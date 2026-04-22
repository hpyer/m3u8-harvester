use crate::core::downloader::{DownloadOptions, DownloadProgress, Downloader};
use crate::services::setting_service::SettingService;
use crate::services::task_service::TaskService;
use crate::utils::m3u8::parse_m3u8;
use crate::utils::merger::VideoMerger;
use regex::Regex;
use std::sync::OnceLock;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{error, info};

static RE_SEASON_IN_TITLE: OnceLock<Regex> = OnceLock::new();

fn get_re_season_in_title() -> &'static Regex {
    RE_SEASON_IN_TITLE
        .get_or_init(|| Regex::new(r"(?i)(?:^|[.\s_-])S(\d{1,2})(?:E\d{1,3}\b|[.\s_-]|$)").unwrap())
}

pub struct DownloadService {
    task_service: Arc<TaskService>,
    setting_service: Arc<SettingService>,
    default_download_path: PathBuf,
    use_configured_download_path: bool,
    active_tasks: Arc<Mutex<HashSet<String>>>,
    overwrite_waiters: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
}

impl DownloadService {
    pub fn new(
        task_service: Arc<TaskService>,
        setting_service: Arc<SettingService>,
        default_download_path: PathBuf,
        use_configured_download_path: bool,
    ) -> Self {
        Self {
            task_service,
            setting_service,
            default_download_path,
            use_configured_download_path,
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
        let default_download_path = self.default_download_path.clone();
        let use_configured_download_path = self.use_configured_download_path;
        let active_tasks = self.active_tasks.clone();
        let overwrite_waiters = self.overwrite_waiters.clone();
        let task_id_for_cleanup = task_id.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::execute_task(
                task_id,
                task_service,
                setting_service,
                default_download_path,
                use_configured_download_path,
                overwrite_waiters,
            )
            .await
            {
                error!("Task failed: {}", e);
            }

            active_tasks.lock().await.remove(&task_id_for_cleanup);
        });
    }

    pub async fn handle_overwrite_response(
        &self,
        task_id: String,
        overwrite: bool,
    ) -> anyhow::Result<()> {
        if let Some(sender) = self.overwrite_waiters.lock().await.remove(&task_id) {
            let _ = sender.send(overwrite);
        } else {
            self.task_service
                .update_task_status(&task_id, if overwrite { "pending" } else { "skipped" })
                .await?;
            if !overwrite {
                self.task_service
                    .update_task_progress(&task_id, 100.0)
                    .await?;
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
        default_download_path: PathBuf,
        use_configured_download_path: bool,
        overwrite_waiters: Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>,
    ) -> anyhow::Result<()> {
        let task = task_service
            .find_task(&task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if task.m3u8_url.is_none() {
            return Err(anyhow::anyhow!("No M3U8 URL for task"));
        }

        let m3u8_url = task.m3u8_url.clone().unwrap();
        let settings = setting_service.get_all().await?;

        // The configured download path must match the file browser root.
        // If no explicit setting exists, use the app-specific startup default.
        let download_root = if use_configured_download_path {
            settings
                .get("downloadPath")
                .filter(|path| !path.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or(default_download_path)
        } else {
            default_download_path
        };

        let output_file = Self::build_output_path(&download_root, &task_service, &task).await?;
        task_service
            .update_task_output_path(&task_id, &output_file.to_string_lossy())
            .await?;

        if tokio::fs::metadata(&output_file).await.is_ok() {
            // ... (rest of the overwrite logic remains same)
            task_service.update_task_status(&task_id, "pending").await?;
            task_service.set_pending_overwrite(&task_id, true).await?;

            let (tx, rx) = oneshot::channel();
            overwrite_waiters.lock().await.insert(task_id.clone(), tx);

            let overwrite =
                match tokio::time::timeout(std::time::Duration::from_secs(3600), rx).await {
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
        // ... (parsing logic remains same)
        task_service.update_task_status(&task_id, "parsing").await?;
        let m3u8_info = parse_m3u8(&m3u8_url).await?;

        // 更新总分片数和预计大小
        task_service
            .update_task_segments(&task_id, m3u8_info.segments.len() as i32)
            .await?;
        if let Some(size) = m3u8_info.total_size {
            task_service
                .update_task_estimated_size(&task_id, size)
                .await?;
        }

        // 2. 准备下载路径：在下载根目录下创建 .temp
        let temp_dir = download_root.join(".temp").join(&task_id);
        let (tx, mut rx) = mpsc::channel::<DownloadProgress>(100);

        // 3. 启动进度监听
        // ... (listener logic remains same)
        let ts_id_clone = task_id.clone();
        let ts_service_clone = task_service.clone();
        let progress_listener = tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                let _ = ts_service_clone
                    .update_task_progress(&ts_id_clone, progress.percentage)
                    .await;
                let _ = ts_service_clone
                    .update_task_completed_segments(
                        &ts_id_clone,
                        progress.completed_segments as i32,
                    )
                    .await;
                if let Some(parent_id) = ts_service_clone
                    .find_task(&ts_id_clone)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|t| t.parent_id)
                {
                    let _ = ts_service_clone.update_parent_status(&parent_id).await;
                }
            }
        });

        // 4. 开始下载
        task_service
            .update_task_status(&task_id, "downloading")
            .await?;
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
        downloader
            .start_download(task_id.clone(), m3u8_info.segments, temp_dir.clone(), tx)
            .await?;
        let _ = progress_listener.await;

        // ... (rest of the merge and completion logic remains same)
        // 重新检查任务状态和完成度，防止因暂停而过早合并
        let current_task = task_service
            .find_task(&task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if current_task.status == "paused"
            || current_task.completed_segments < current_task.total_segments
        {
            info!(
                "Task {} is paused or incomplete, skipping merge. ({} / {})",
                task_id, current_task.completed_segments, current_task.total_segments
            );
            return Ok(());
        }

        // 5. 合并视频
        task_service.update_task_status(&task_id, "merging").await?;
        VideoMerger::merge(&temp_dir, &output_file).await?;

        // 6. 完成
        task_service
            .update_task_status(&task_id, "completed")
            .await?;
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
        download_root: &std::path::Path, // 接收动态下载根目录
        task_service: &Arc<TaskService>,
        task: &crate::models::task::Task,
    ) -> anyhow::Result<PathBuf> {
        let (parent_title, parent_type, parent_season) = if let Some(parent_id) = &task.parent_id {
            let parent_task = task_service.find_task(parent_id).await?;
            let title = parent_task
                .as_ref()
                .and_then(|t| t.group_title.clone())
                .unwrap_or_else(|| "Others".to_string());
            let task_type = parent_task
                .as_ref()
                .map(|t| t.r#type.clone())
                .unwrap_or_else(|| task.r#type.clone());
            let season = parent_task.and_then(|t| t.season);
            (title, task_type, season)
        } else {
            (
                task.group_title
                    .clone()
                    .unwrap_or_else(|| "Others".to_string()),
                task.r#type.clone(),
                task.season.clone(),
            )
        };

        let mut downloads_dir = download_root.join(&parent_title);
        if parent_type == "series" {
            if let Some(season) = extract_season_from_title(&task.title)
                .or_else(|| parse_season_number(parent_season.as_deref()))
            {
                downloads_dir = downloads_dir.join(format!("S{:02}", season));
            }
        }
        tokio::fs::create_dir_all(&downloads_dir).await?;

        Ok(downloads_dir.join(format!("{}.mp4", task.title)))
    }
}

fn extract_season_from_title(title: &str) -> Option<u32> {
    get_re_season_in_title()
        .captures(title)
        .and_then(|caps| caps.get(1))
        .and_then(|season| season.as_str().parse::<u32>().ok())
}

fn parse_season_number(season: Option<&str>) -> Option<u32> {
    season.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse::<u32>().ok()
        }
    })
}
