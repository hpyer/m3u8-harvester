use crate::utils::m3u8::{EncryptionKey, SegmentInfo};
use aes::Aes128;
use anyhow::{anyhow, Context, Result};
use cbc::cipher::{
    block_padding::{NoPadding, Pkcs7},
    BlockDecryptMut, KeyIvInit,
};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as tfs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Semaphore};

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
    pub fn new(
        options: DownloadOptions,
        task_service: Arc<crate::services::task_service::TaskService>,
    ) -> Result<Self> {
        let mut builder = Client::builder().user_agent(&options.user_agent);

        if let Some(proxy_url) = options
            .proxy
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
        }

        let client = builder.build()?;

        Ok(Self {
            client,
            options,
            task_service,
        })
    }

    pub async fn start_download(
        &self,
        task_id: String,
        segments: Vec<SegmentInfo>,
        save_path: PathBuf,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<()> {
        let total = segments.len();
        let completed = Arc::new(tokio::sync::Mutex::new(0));
        let semaphore = Arc::new(Semaphore::new(self.options.concurrency));
        let key_cache = Arc::new(tokio::sync::Mutex::new(HashMap::<String, Vec<u8>>::new()));

        // 确保目录存在
        tfs::create_dir_all(&save_path).await?;

        self.download_init_maps(&segments, &save_path).await?;

        // 检查已下载的文件（续传支持）
        let expected_files: HashSet<String> = segments
            .iter()
            .map(|segment| segment.file_name.clone())
            .collect();
        let mut initial_completed = 0usize;
        for file_name in &expected_files {
            if tfs::metadata(save_path.join(file_name)).await.is_ok() {
                initial_completed += 1;
            }
        }

        {
            let mut c = completed.lock().await;
            *c = initial_completed;
        }

        let mut handles = Vec::new();
        let client = self.client.clone();

        for segment in segments {
            let semaphore = semaphore.clone();
            let client = client.clone();
            let task_id = task_id.clone();
            let save_path = save_path.clone();
            let completed = completed.clone();
            let progress_tx = progress_tx.clone();
            let task_service = self.task_service.clone();
            let retry_count = self.options.retry_count;
            let retry_delay_ms = self.options.retry_delay_ms;
            let key_cache = key_cache.clone();

            let handle = tokio::spawn(async move {
                let permit = semaphore.acquire_owned().await.unwrap();

                // 检查任务状态，如果是暂停或停止则取消下载
                if let Ok(Some(task)) = task_service.find_task(&task_id).await {
                    if task.status == "paused" {
                        drop(permit);
                        return Ok(());
                    }
                }

                let file_path = save_path.join(&segment.file_name);
                let tmp_path = save_path.join(format!("{}.tmp", segment.file_name));

                // 检查是否已下载
                if tfs::metadata(&file_path).await.is_ok() {
                    drop(permit);
                    return Ok(());
                }

                let mut remaining_retries = retry_count.max(1);
                while remaining_retries > 0 {
                    match Self::download_segment(
                        &client, &segment, &key_cache, &tmp_path, &file_path,
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut c = completed.lock().await;
                            *c += 1;
                            let current_completed = *c;
                            drop(c);

                            let percentage = (current_completed as f64 / total as f64) * 100.0;

                            let _ = progress_tx
                                .send(DownloadProgress {
                                    task_id: task_id.clone(),
                                    total_segments: total,
                                    completed_segments: current_completed,
                                    percentage: (percentage * 10.0).round() / 10.0,
                                    status: "downloading".to_string(),
                                })
                                .await;

                            drop(permit);
                            return Ok(());
                        }
                        Err(e) => {
                            remaining_retries -= 1;
                            if remaining_retries == 0 {
                                drop(permit);
                                return Err(e);
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(retry_delay_ms))
                                .await;
                        }
                    }
                }
                drop(permit);
                Err(anyhow!("下载分片失败: {}", segment.url))
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
        segment: &SegmentInfo,
        key_cache: &tokio::sync::Mutex<HashMap<String, Vec<u8>>>,
        tmp_path: &PathBuf,
        file_path: &PathBuf,
    ) -> Result<()> {
        let mut data = Self::download_bytes(client, &segment.url).await?;

        if let Some(key) = segment.key.as_ref() {
            data = Self::decrypt_segment(client, key_cache, data, key, segment.media_sequence)
                .await
                .with_context(|| format!("解密分片失败: {}", segment.url))?;
        }

        let mut file = tfs::File::create(tmp_path).await?;
        file.write_all(&data).await?;
        file.flush().await?;
        tfs::rename(tmp_path, file_path).await?;
        Ok(())
    }

    async fn download_init_maps(&self, segments: &[SegmentInfo], save_path: &Path) -> Result<()> {
        let mut downloaded = HashSet::new();

        for segment in segments {
            let Some(init_map) = segment.init_map.as_ref() else {
                continue;
            };

            if !downloaded.insert(init_map.file_name.clone()) {
                continue;
            }

            let file_path = save_path.join(&init_map.file_name);
            if tfs::metadata(&file_path).await.is_ok() {
                continue;
            }

            let data = Self::download_bytes(&self.client, &init_map.url).await?;
            let tmp_path = save_path.join(format!("{}.tmp", init_map.file_name));
            let mut file = tfs::File::create(&tmp_path).await?;
            file.write_all(&data).await?;
            file.flush().await?;
            tfs::rename(tmp_path, file_path).await?;
        }

        Ok(())
    }

    async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>> {
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow!("HTTP 错误: {}", response.status()));
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            bytes.extend_from_slice(&chunk?);
        }
        Ok(bytes)
    }

    async fn decrypt_segment(
        client: &Client,
        key_cache: &tokio::sync::Mutex<HashMap<String, Vec<u8>>>,
        data: Vec<u8>,
        key: &EncryptionKey,
        media_sequence: u64,
    ) -> Result<Vec<u8>> {
        let key_bytes = {
            let mut cache = key_cache.lock().await;
            if let Some(bytes) = cache.get(&key.url) {
                bytes.clone()
            } else {
                let bytes = Self::download_bytes(client, &key.url).await?;
                if bytes.len() != 16 {
                    return Err(anyhow!("AES-128 密钥长度无效: {} 字节", bytes.len()));
                }
                cache.insert(key.url.clone(), bytes.clone());
                bytes
            }
        };

        let iv = parse_iv(key.iv.as_deref(), media_sequence)?;
        decrypt_aes128_cbc(data, &key_bytes, &iv)
    }
}

fn parse_iv(iv: Option<&str>, media_sequence: u64) -> Result<[u8; 16]> {
    if let Some(value) = iv {
        let trimmed = value.trim();
        let hex = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed);

        if hex.len() != 32 {
            return Err(anyhow!("AES-128 IV 长度无效: {}", value));
        }

        let mut iv_bytes = [0u8; 16];
        for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let part = std::str::from_utf8(chunk)?;
            iv_bytes[index] = u8::from_str_radix(part, 16)?;
        }
        return Ok(iv_bytes);
    }

    let mut iv_bytes = [0u8; 16];
    iv_bytes[8..].copy_from_slice(&media_sequence.to_be_bytes());
    Ok(iv_bytes)
}

fn decrypt_aes128_cbc(data: Vec<u8>, key: &[u8], iv: &[u8; 16]) -> Result<Vec<u8>> {
    type Aes128CbcDec = cbc::Decryptor<Aes128>;

    let decrypt_with_padding = |mut buffer: Vec<u8>| -> Result<Vec<u8>> {
        let decrypted = Aes128CbcDec::new_from_slices(key, iv)
            .map_err(|_| anyhow!("AES-128 解密器初始化失败"))?
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|_| anyhow!("AES-128 CBC 解密失败"))?;
        Ok(decrypted.to_vec())
    };

    match decrypt_with_padding(data.clone()) {
        Ok(bytes) => Ok(bytes),
        Err(_) if data.len().is_multiple_of(16) => {
            let mut buffer = data;
            let decrypted = Aes128CbcDec::new_from_slices(key, iv)
                .map_err(|_| anyhow!("AES-128 解密器初始化失败"))?
                .decrypt_padded_mut::<NoPadding>(&mut buffer)
                .map_err(|_| anyhow!("AES-128 CBC 解密失败"))?;
            Ok(decrypted.to_vec())
        }
        Err(err) => Err(err),
    }
}
