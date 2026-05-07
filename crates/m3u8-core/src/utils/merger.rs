use crate::utils::m3u8::SegmentInfo;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::path::Path;
use tokio::fs as tfs;
use tracing::info;

pub struct VideoMerger;

impl VideoMerger {
    pub async fn merge(
        temp_dir: &Path,
        output_file: &Path,
        segments: &[SegmentInfo],
    ) -> Result<()> {
        info!("Merging segments in {:?} to {:?}", temp_dir, output_file);
        let ffmpeg_path = resolve_ffmpeg_path()?;

        // 确保输出目录存在
        if let Some(parent) = output_file.parent() {
            tfs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
        }

        if segments.is_empty() {
            return Err(anyhow!("没有可合并的分片"));
        }

        let playlist_path = temp_dir.join("local_playlist.m3u8");
        let playlist_content = build_local_playlist(segments);
        tfs::write(&playlist_path, playlist_content)
            .await
            .with_context(|| format!("Failed to write local playlist: {:?}", playlist_path))?;

        // 调用 ffmpeg 合并
        let output = tokio::process::Command::new(&ffmpeg_path)
            .current_dir(temp_dir)
            .arg("-y")
            .arg("-allowed_extensions")
            .arg("ALL")
            .arg("-i")
            .arg("local_playlist.m3u8")
            .arg("-c")
            .arg("copy")
            .arg(output_file)
            .output()
            .await
            .with_context(|| {
                format!(
                    "Failed to start ffmpeg {:?} in {:?} for output {:?}",
                    ffmpeg_path, temp_dir, output_file
                )
            })?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg merge failed: {}", err));
        }

        info!("Successfully merged video to {:?}", output_file);
        Ok(())
    }
}

fn build_local_playlist(segments: &[SegmentInfo]) -> String {
    let target_duration = segments
        .iter()
        .map(|segment| segment.duration.ceil() as u64)
        .max()
        .unwrap_or(1)
        .max(1);

    let mut content = format!(
        "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:{target_duration}\n#EXT-X-MEDIA-SEQUENCE:0\n"
    );
    let mut current_map: Option<&str> = None;

    for segment in segments {
        let next_map = segment.init_map.as_ref().map(|map| map.file_name.as_str());
        if next_map != current_map {
            if let Some(map_file) = next_map {
                content.push_str(&format!("#EXT-X-MAP:URI=\"{}\"\n", map_file));
            }
            current_map = next_map;
        }

        content.push_str(&format!(
            "#EXTINF:{},\n{}\n",
            segment.duration, segment.file_name
        ));
    }

    content.push_str("#EXT-X-ENDLIST\n");
    content
}

fn resolve_ffmpeg_path() -> Result<std::path::PathBuf> {
    if let Some(path) = env::var_os("FFMPEG_PATH") {
        let path = std::path::PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(anyhow!(
            "FFMPEG_PATH is set but does not point to a file: {:?}",
            path
        ));
    }

    which::which("ffmpeg").map_err(|_| {
        anyhow!(
            "FFmpeg executable not found. Install ffmpeg and make sure it is in PATH, or set FFMPEG_PATH to the absolute ffmpeg binary path."
        )
    })
}
