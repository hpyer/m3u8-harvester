use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, oneshot, Mutex};
use m3u8_core::{TaskService, SettingService, Downloader, DownloadOptions, parse_m3u8, VideoMerger, DownloadProgress};
use tracing::{info, error};

pub struct DownloadService {
    task_service: Arc<TaskService>,
    setting_service: Arc<SettingService>,
    storage_path: PathBuf,
    active_tasks: Arc<Mutex<HashSet<String>>>,
    overwrite_waiters: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
}

impl DownloadService {
    pub fn new(task_service: Arc<TaskService>, setting_service: Arc<SettingService>, storage_path: PathBuf) -> Self {
        Self {
            task_service,
            setting_service,
            storage_path,
            active_tasks: Arc::new(Mutex::new(HashSet::new())),
            overwrite_waiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run_task(&self, task_id: String) {
        {
            let mut active_tasks = self.active_tasks.lock().await;
            if !active_tasks.insert(task_id.clone()) {
                info!("Task {} is already running", task_id);
                return;
            }
        }

        let task_service = self.task_service.clone();
        let setting_service = self.setting_service.clone();
        let storage_path = self.storage_path.clone();
        let active_tasks = self.active_tasks.clone();
        let overwrite_waiters = self.overwrite_waiters.clone();
        let task_id_for_cleanup = task_id.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_task(
                task_id,
                task_service,
                setting_service,
                storage_path,
                overwrite_waiters,
            ).await {
                error!("Task failed: {}", e);
            }

            active_tasks.lock().await.remove(&task_id_for_cleanup);
        });
    }

    pub async fn handle_overwrite_response(&self, task_id: String, overwrite: bool) -> anyhow::Result<()> {
        if let Some(sender) = self.overwrite_waiters.lock().await.remove(&task_id) {
            let _ = sender.send(overwrite);
        } else {
            self.task_service.update_task_status(&task_id, if overwrite { "pending" } else { "skipped" }).await?;
            if !overwrite {
                self.task_service.update_task_progress(&task_id, 100.0).await?;
                if let Some(parent_id) = self
                    .task_service
                    .find_task(&task_id)
                    .await?
                    .and_then(|task| task.parent_id)
                {
                    self.task_service.update_parent_status(&parent_id).await?;
                }
            }
        }

        self.task_service
            .set_pending_overwrite(&task_id, false)
            .await?;

        if overwrite {
            self.run_task(task_id).await;
        }

        Ok(())
    }

    async fn execute_task(
        task_id: String,
        task_service: Arc<TaskService>,
        setting_service: Arc<SettingService>,
        storage_path: PathBuf,
        overwrite_waiters: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    ) -> anyhow::Result<()> {
        let task = task_service.find_task(&task_id).await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if task.m3u8_url.is_none() {
            return Err(anyhow::anyhow!("No M3U8 URL for task"));
        }

        let m3u8_url = task.m3u8_url.clone().unwrap();
        let settings = setting_service.get_all().await?;

        let output_file = Self::build_output_path(&storage_path, &task_service, &task).await?;
        task_service
            .update_task_output_path(&task_id, &output_file.to_string_lossy())
            .await?;

        if tokio::fs::metadata(&output_file).await.is_ok() {
            task_service.update_task_status(&task_id, "pending").await?;
            task_service.set_pending_overwrite(&task_id, true).await?;

            let (tx, rx) = oneshot::channel();
            overwrite_waiters.lock().await.insert(task_id.clone(), tx);

            let overwrite = match tokio::time::timeout(std::time::Duration::from_secs(3600), rx).await {
                Ok(Ok(val)) => val,
                _ => false,
            };

            overwrite_waiters.lock().await.remove(&task_id);
            task_service.set_pending_overwrite(&task_id, false).await?;

            if !overwrite {
                task_service.update_task_status(&task_id, "skipped").await?;
                task_service.update_task_progress(&task_id, 100.0).await?;
                if let Some(parent_id) = &task.parent_id {
                    task_service.update_parent_status(parent_id).await?;
                }
                return Ok(());
            }

            let _ = tokio::fs::remove_file(&output_file).await;
        }
        
        // 1. 解析 M3U8
        task_service.update_task_status(&task_id, "parsing").await?;
        let m3u8_info = parse_m3u8(&m3u8_url).await?;

        // 更新总分片数和预计大小
        task_service.update_task_segments(&task_id, m3u8_info.segments.len() as i32).await?;
        if let Some(size) = m3u8_info.total_size {
            task_service.update_task_estimated_size(&task_id, size).await?;
        }

        // 2. 准备下载路径
        let temp_dir = storage_path.join("temp").join(&task_id);
        let (tx, mut rx) = mpsc::channel::<DownloadProgress>(100);

        // 3. 启动进度监听
        let ts_id_clone = task_id.clone();
        let ts_service_clone = task_service.clone();
        tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let _ = ts_service_clone.update_task_progress(&ts_id_clone, progress.percentage).await;
                let _ = ts_service_clone.update_task_completed_segments(&ts_id_clone, progress.completed_segments as i32).await;
                if let Some(parent_id) = ts_service_clone.find_task(&ts_id_clone).await.ok().flatten().and_then(|t| t.parent_id) {
                    let _ = ts_service_clone.update_parent_status(&parent_id).await;
                }
            }
        });

        // 4. 开始下载
        task_service.update_task_status(&task_id, "downloading").await?;
        let downloader = Arc::new(Downloader::new(
            DownloadOptions {
                proxy: settings
                    .get("proxy")
                    .filter(|v| !v.trim().is_empty())
                    .cloned(),
                concurrency: settings
                    .get("concurrency")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(5),
                retry_count: settings
                    .get("retryCount")
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(3),
                retry_delay_ms: settings
                    .get("retryDelay")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2000),
                user_agent: settings
                    .get("userAgent")
                    .filter(|v| !v.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| DownloadOptions::default().user_agent),
            },
            task_service.clone(),
        )?);
        downloader.start_download(task_id.clone(), m3u8_info.segments, temp_dir.clone(), tx).await?;

        // 重新检查任务状态和完成度，防止因暂停而过早合并
        let current_task = task_service.find_task(&task_id).await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;
        
        if current_task.status == "paused" || current_task.completed_segments < current_task.total_segments {
            info!("Task {} is paused or incomplete, skipping merge. ({} / {})", task_id, current_task.completed_segments, current_task.total_segments);
            return Ok(());
        }

        // 5. 合并视频
        task_service.update_task_status(&task_id, "merging").await?;
        VideoMerger::merge(&temp_dir, &output_file).await?;

        // 6. 完成
        task_service.update_task_status(&task_id, "completed").await?;
        task_service.update_task_progress(&task_id, 100.0).await?;
        task_service
            .update_task_output_path(&task_id, &output_file.to_string_lossy())
            .await?;
        
        if let Some(parent_id) = task.parent_id {
            task_service.update_parent_status(&parent_id).await?;
        }

        // 清理临时文件
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        info!("Task {} completed successfully", task_id);
        Ok(())
    }

    async fn build_output_path(
        storage_path: &PathBuf,
        task_service: &Arc<TaskService>,
        task: &m3u8_core::Task,
    ) -> anyhow::Result<PathBuf> {
        let parent_title = if let Some(parent_id) = &task.parent_id {
            task_service
                .find_task(parent_id)
                .await?
                .and_then(|t| t.group_title)
                .unwrap_or_else(|| "Others".to_string())
        } else {
            task.group_title.clone().unwrap_or_else(|| "Others".to_string())
        };

        let downloads_dir = storage_path.join("downloads").join(&parent_title);
        tokio::fs::create_dir_all(&downloads_dir).await?;

        Ok(downloads_dir.join(format!("{}.mp4", task.title)))
    }
}

#[cfg(test)]
mod tests {
    use super::DownloadService;
    use anyhow::Result;
    use m3u8_core::{SettingService, TaskService};
    use sqlx::{sqlite::SqlitePoolOptions, Executor};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_suffix() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos()
    }

    async fn create_services() -> Result<(Arc<TaskService>, Arc<SettingService>, DownloadService)> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        pool.execute(
            r#"
            CREATE TABLE tasks (
                id TEXT PRIMARY KEY NOT NULL,
                parent_id TEXT,
                group_title TEXT,
                title TEXT NOT NULL,
                type TEXT NOT NULL,
                year TEXT,
                season TEXT,
                m3u8_url TEXT,
                status TEXT NOT NULL,
                is_pending_overwrite BOOLEAN NOT NULL DEFAULT 0,
                percentage REAL NOT NULL DEFAULT 0,
                total_segments INTEGER NOT NULL DEFAULT 0,
                completed_segments INTEGER NOT NULL DEFAULT 0,
                estimated_size INTEGER,
                output_path TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            "#,
        )
        .await?;
        let task_service = Arc::new(TaskService::new(pool.clone()));
        let setting_service = Arc::new(SettingService::new(pool));
        let storage_path = std::env::current_dir()?
            .join(".tmp-tests")
            .join(format!("m3u8-server-storage-{}", unique_suffix()));
        let download_service =
            DownloadService::new(task_service.clone(), setting_service.clone(), storage_path);

        Ok((task_service, setting_service, download_service))
    }

    #[tokio::test]
    async fn handle_overwrite_response_without_waiter_marks_task_skipped_and_clears_flag() -> Result<()> {
        let (task_service, _setting_service, download_service) = create_services().await?;

        let parent = task_service
            .create_parent_task(Some("Group".into()), "Group".into(), "movie".into(), None, None)
            .await?;
        let subtask = task_service
            .create_sub_task(
                parent.id.clone(),
                "Movie.2025".into(),
                "https://example.com/movie.m3u8".into(),
                "movie".into(),
            )
            .await?;

        task_service.set_pending_overwrite(&subtask.id, true).await?;
        download_service
            .handle_overwrite_response(subtask.id.clone(), false)
            .await?;

        let updated = task_service.find_task(&subtask.id).await?.unwrap();
        assert_eq!(updated.status, "skipped");
        assert_eq!(updated.percentage, 100.0);
        assert!(!updated.is_pending_overwrite);

        let parent = task_service.find_task(&parent.id).await?.unwrap();
        assert_eq!(parent.status, "completed");
        Ok(())
    }
}
