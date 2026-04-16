use anyhow::{Result, anyhow};
use reqwest::Client;
use std::path::PathBuf;
use tokio::sync::{mpsc, Semaphore};
use std::sync::Arc;
use tokio::fs as tfs;
use tokio::io::AsyncWriteExt;
use futures_util::StreamExt;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub task_id: String,
    pub total_segments: usize,
    pub completed_segments: usize,
    pub percentage: f64,
    pub status: String,
}

pub struct DownloadOptions {
    pub user_agent: String,
    pub proxy: Option<String>,
    pub concurrency: usize,
    pub retry_count: usize,
    pub retry_delay_ms: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            proxy: None,
            concurrency: 5,
            retry_count: 3,
            retry_delay_ms: 2000,
        }
    }
}

pub struct Downloader {
    client: Client,
    options: DownloadOptions,
    task_service: Arc<crate::services::task_service::TaskService>,
}

impl Downloader {
    pub fn new(options: DownloadOptions, task_service: Arc<crate::services::task_service::TaskService>) -> Result<Self> {
        let mut builder = Client::builder().user_agent(&options.user_agent);

        if let Some(proxy_url) = options.proxy.as_ref().filter(|value| !value.trim().is_empty()) {
            builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
        }

        let client = builder.build()?;
        
        Ok(Self { client, options, task_service })
    }

    pub async fn start_download(
        &self,
        task_id: String,
        urls: Vec<String>,
        save_path: PathBuf,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        let total = urls.len();
        let completed = Arc::new(tokio::sync::Mutex::new(0));
        let semaphore = Arc::new(Semaphore::new(self.options.concurrency));
        
        // 确保目录存在
        tfs::create_dir_all(&save_path).await?;

        // 检查已下载的文件（续传支持）
        let mut initial_completed = 0;
        let mut entries = tfs::read_dir(&save_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_name().to_string_lossy().ends_with(".ts") {
                initial_completed += 1;
            }
        }
        
        {
            let mut c = completed.lock().await;
            *c = initial_completed;
        }

        let mut handles = Vec::new();
        let client = self.client.clone();
        
        for (index, url) in urls.into_iter().enumerate() {
            let semaphore = semaphore.clone();
            let client = client.clone();
            let task_id = task_id.clone();
            let save_path = save_path.clone();
            let completed = completed.clone();
            let progress_tx = progress_tx.clone();
            let task_service = self.task_service.clone();
            let retry_count = self.options.retry_count;
            let retry_delay_ms = self.options.retry_delay_ms;
            
            let handle = tokio::spawn(async move {
                let permit = semaphore.acquire_owned().await.unwrap();
                
                // 检查任务状态，如果是暂停或停止则取消下载
                if let Ok(Some(task)) = task_service.find_task(&task_id).await {
                    if task.status == "paused" {
                        drop(permit);
                        return Ok(());
                    }
                }

                let file_name = format!("{:05}.ts", index);
                let file_path = save_path.join(&file_name);
                let tmp_path = save_path.join(format!("{}.tmp", file_name));

                // 检查是否已下载
                if tfs::metadata(&file_path).await.is_ok() {
                    drop(permit);
                    return Ok(());
                }

                let mut remaining_retries = retry_count.max(1);
                while remaining_retries > 0 {
                    match Self::download_segment(&client, &url, &tmp_path, &file_path).await {
                        Ok(_) => {
                            let mut c = completed.lock().await;
                            *c += 1;
                            let current_completed = *c;
                            drop(c);
                            
                            let percentage = (current_completed as f64 / total as f64) * 100.0;
                            
                            let _ = progress_tx.send(DownloadProgress {
                                task_id: task_id.clone(),
                                total_segments: total,
                                completed_segments: current_completed,
                                percentage: (percentage * 10.0).round() / 10.0,
                                status: "downloading".to_string(),
                            }).await;
                            
                            drop(permit);
                            return Ok(());
                        }
                        Err(e) => {
                            remaining_retries -= 1;
                            if remaining_retries == 0 {
                                drop(permit);
                                return Err(e);
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms)).await;
                        }
                    }
                }
                drop(permit);
                Err(anyhow!("下载分片失败: {}", url))
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }

        Ok(())
    }

    async fn download_segment(
        client: &Client,
        url: &str,
        tmp_path: &PathBuf,
        file_path: &PathBuf,
    ) -> Result<()> {
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("HTTP 错误: {}", response.status()));
        }

        let mut file = tfs::File::create(tmp_path).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        tfs::rename(tmp_path, file_path).await?;
        Ok(())
    }
}
