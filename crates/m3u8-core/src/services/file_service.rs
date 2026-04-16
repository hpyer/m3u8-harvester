use anyhow::Result;
use std::path::{Path, PathBuf};
use serde::Serialize;
use tokio::fs;
use chrono::{DateTime, Utc};
use async_recursion::async_recursion;

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
    pub files: Vec<FileInfo>,
}

pub struct FileService {
    base_path: PathBuf,
}

impl FileService {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn get_base_path(&self) -> String {
        self.base_path.to_string_lossy().to_string()
    }

    pub async fn list_folders(&self) -> Result<Vec<FolderInfo>> {
        let mut folders = Vec::new();
        
        if !fs::try_exists(&self.base_path).await? {
            return Ok(folders);
        }

        let mut entries = fs::read_dir(&self.base_path).await?;
        let mut root_files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            if name.starts_with('.') { continue; }

            if metadata.is_dir() {
                let mut files = Vec::new();
                self.scan_files_recursive(&path, &path, &mut files).await?;
                
                let modified = metadata.modified()?;
                let latest_update = files.iter()
                    .map(|f| f.updated_at.clone())
                    .max()
                    .unwrap_or_else(|| format_datetime(modified));

                folders.push(FolderInfo {
                    id: name.clone(),
                    name,
                    file_count: files.len(),
                    updated_at: latest_update,
                    files,
                });
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
            let latest_update = root_files.iter()
                .map(|f| f.updated_at.clone())
                .max()
                .unwrap_or_else(|| "0000-00-00 00:00".to_string());

            folders.push(FolderInfo {
                id: "others".to_string(),
                name: "其他".to_string(),
                file_count: root_files.len(),
                updated_at: latest_update,
                files: root_files,
            });
        }

        // 按更新时间降序排列
        folders.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(folders)
    }

    #[async_recursion]
    async fn scan_files_recursive(&self, _folder_root: &Path, current_dir: &Path, files: &mut Vec<FileInfo>) -> Result<()> {
        let mut entries = fs::read_dir(current_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }

            if metadata.is_dir() {
                self.scan_files_recursive(_folder_root, &path, files).await?;
            } else if metadata.is_file() {
                let relative_id = path.strip_prefix(&self.base_path)?
                    .to_string_lossy()
                    .to_string();
                
                files.push(FileInfo {
                    id: relative_id,
                    name,
                    size: format_size(metadata.len()),
                    updated_at: format_datetime(metadata.modified()?),
                });
            }
        }
        Ok(())
    }

    pub async fn delete_file(&self, id: &str) -> Result<()> {
        let file_path = self.base_path.join(id);
        if !file_path.starts_with(&self.base_path) {
            return Err(anyhow::anyhow!("Invalid file path"));
        }
        if fs::try_exists(&file_path).await? {
            fs::remove_file(file_path).await?;
        }
        Ok(())
    }

    pub async fn delete_folder(&self, id: &str) -> Result<()> {
        let folder_path = self.base_path.join(id);
        if !folder_path.starts_with(&self.base_path) {
            return Err(anyhow::anyhow!("Invalid folder path"));
        }
        if fs::try_exists(&folder_path).await? {
            fs::remove_dir_all(folder_path).await?;
        }
        Ok(())
    }

    pub async fn rename(&self, id: &str, new_name: &str) -> Result<()> {
        let old_path = self.base_path.join(id);
        if !old_path.starts_with(&self.base_path) {
            return Err(anyhow::anyhow!("Invalid old path"));
        }

        // 构造新路径
        let new_path = if let Some(parent) = old_path.parent() {
            parent.join(new_name)
        } else {
            return Err(anyhow::anyhow!("Invalid parent path"));
        };

        if !new_path.starts_with(&self.base_path) {
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
