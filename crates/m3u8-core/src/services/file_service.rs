use anyhow::Result;
use async_recursion::async_recursion;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Serialize, Clone)]
pub struct FileInfo {
    pub id: String, // 相对路径，如 "folder/sub/file.mp4"
    pub name: String,
    pub size: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct FolderInfo {
    pub id: String, // 相对路径
    pub name: String,
    #[serde(rename = "fileCount")]
    pub file_count: usize,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub folders: Vec<FolderInfo>,
    pub files: Vec<FileInfo>,
}

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct FileService {
    base_path: Arc<Mutex<PathBuf>>,
}

impl FileService {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path: Arc::new(Mutex::new(base_path)),
        }
    }

    pub async fn get_base_path(&self) -> String {
        self.base_path.lock().await.to_string_lossy().to_string()
    }

    pub async fn set_base_path(&self, new_path: PathBuf) {
        let mut base = self.base_path.lock().await;
        *base = new_path;
    }

    pub async fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        let mut folders = Vec::new();
        let base_path = self.base_path.lock().await.clone();

        if !fs::try_exists(&base_path).await? {
            return Ok(folders);
        }

        let mut entries = fs::read_dir(&base_path).await?;
        let mut root_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            if metadata.is_dir() {
                folders.push(
                    self.build_folder_info(name.clone(), name, &path, &base_path)
                        .await?,
                );
            } else if metadata.is_file() {
                root_files.push(FileInfo {
                    id: name.clone(),
                    name: name.clone(),
                    size: format_size(metadata.len()),
                    updated_at: format_datetime(metadata.modified()?),
                });
            }
        }

        if !root_files.is_empty() {
            let latest_update = root_files
                .iter()
                .map(|f| f.updated_at.clone())
                .max()
                .unwrap_or_else(|| "0000-00-00 00:00".to_string());

            folders.push(FolderInfo {
                id: "others".to_string(),
                name: "其他".to_string(),
                file_count: root_files.len(),
                updated_at: latest_update,
                folders: Vec::new(),
                files: root_files,
            });
        }

        // 按更新时间降序排列
        folders.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(folders)
    }

    #[async_recursion]
    #[allow(clippy::only_used_in_recursion)]
    async fn build_folder_info(
        &self,
        relative_id: String,
        name: String,
        dir: &Path,
        base_path: &Path, // 将 base_path 作为参数传入以避免在递归中反复加锁
    ) -> Result<FolderInfo> {
        let mut entries = fs::read_dir(dir).await?;
        let metadata = fs::metadata(dir).await?;
        let mut folders = Vec::new();
        let mut files = Vec::new();
        let mut latest_update = format_datetime(metadata.modified()?);
        let mut file_count = 0;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if entry_name.starts_with('.') {
                continue;
            }

            if metadata.is_dir() {
                let child_relative_id = relative_id_path(&relative_id, &entry_name);
                let child = self
                    .build_folder_info(child_relative_id, entry_name, &path, base_path)
                    .await?;
                file_count += child.file_count;
                latest_update = latest_update.max(child.updated_at.clone());
                folders.push(child);
            } else if metadata.is_file() {
                let relative_id = path.strip_prefix(base_path)?.to_string_lossy().to_string();

                let file = FileInfo {
                    id: relative_id,
                    name: entry_name,
                    size: format_size(metadata.len()),
                    updated_at: format_datetime(metadata.modified()?),
                };
                latest_update = latest_update.max(file.updated_at.clone());
                file_count += 1;
                files.push(file);
            }
        }

        folders.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        files.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(FolderInfo {
            id: relative_id,
            name,
            file_count,
            updated_at: latest_update,
            folders,
            files,
        })
    }

    pub async fn delete_file(&self, id: &str) -> Result<()> {
        let base_path = self.base_path.lock().await;
        let file_path = base_path.join(id);
        if !file_path.starts_with(&*base_path) {
            return Err(anyhow::anyhow!("Invalid file path"));
        }
        if fs::try_exists(&file_path).await? {
            fs::remove_file(file_path).await?;
        }
        Ok(())
    }

    pub async fn delete_folder(&self, id: &str) -> Result<()> {
        let base_path = self.base_path.lock().await;
        let folder_path = base_path.join(id);
        if !folder_path.starts_with(&*base_path) {
            return Err(anyhow::anyhow!("Invalid folder path"));
        }
        if fs::try_exists(&folder_path).await? {
            fs::remove_dir_all(folder_path).await?;
        }
        Ok(())
    }

    pub async fn rename(&self, id: &str, new_name: &str) -> Result<()> {
        let base_path = self.base_path.lock().await;
        let old_path = base_path.join(id);
        if !old_path.starts_with(&*base_path) {
            return Err(anyhow::anyhow!("Invalid old path"));
        }

        // 构造新路径
        let new_path = if let Some(parent) = old_path.parent() {
            parent.join(new_name)
        } else {
            return Err(anyhow::anyhow!("Invalid parent path"));
        };

        if !new_path.starts_with(&*base_path) {
            return Err(anyhow::anyhow!("Invalid new path"));
        }

        fs::rename(old_path, new_path).await?;
        Ok(())
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn format_datetime(dt: std::time::SystemTime) -> String {
    let dt: DateTime<Utc> = dt.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn relative_id_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else {
        format!("{}/{}", parent, child)
    }
}
