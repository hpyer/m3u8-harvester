use std::path::Path;
use anyhow::{Result, anyhow};
use tokio::fs as tfs;
use tracing::info;

pub struct VideoMerger;

impl VideoMerger {
    pub async fn merge(temp_dir: &Path, output_file: &Path) -> Result<()> {
        info!("Merging segments in {:?} to {:?}", temp_dir, output_file);
        
        // 确保输出目录存在
        if let Some(parent) = output_file.parent() {
            tfs::create_dir_all(parent).await?;
        }

        // 创建 ffmpeg 列表文件
        let list_file_path = temp_dir.join("file_list.txt");
        let mut content = String::new();
        
        let mut entries = Vec::new();
        let mut dir = tfs::read_dir(temp_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ts") {
                entries.push(path);
            }
        }
        
        // 按文件名排序 (00000.ts, 00001.ts, ...)
        entries.sort();

        for entry in entries {
            if let Some(file_name) = entry.file_name().and_then(|n| n.to_str()) {
                content.push_str(&format!("file '{}'\n", file_name));
            }
        }

        tfs::write(&list_file_path, content).await?;

        // 调用 ffmpeg 合并
        let output = tokio::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-f")
            .arg("concat")
            .arg("-safe")
            .arg("0")
            .arg("-i")
            .arg(&list_file_path)
            .arg("-c")
            .arg("copy")
            .arg(output_file)
            .output()
            .await?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg merge failed: {}", err));
        }

        info!("Successfully merged video to {:?}", output_file);
        Ok(())
    }
}
