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
        video_segments: &[SegmentInfo],
        audio_segments: Option<&[SegmentInfo]>,
    ) -> Result<()> {
        info!("Merging segments in {:?} to {:?}", temp_dir, output_file);
        let ffmpeg_path = resolve_ffmpeg_path()?;

        // 确保输出目录存在
        if let Some(parent) = output_file.parent() {
            tfs::create_dir_all(parent)
                .await
                .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
        }

        if video_segments.is_empty() {
            return Err(anyhow!("没有可合并的分片"));
        }

        let video_playlist_path = temp_dir.join("video_playlist.m3u8");
        let video_playlist_content = build_local_playlist(video_segments);
        tfs::write(&video_playlist_path, video_playlist_content)
            .await
            .with_context(|| {
                format!("Failed to write local playlist: {:?}", video_playlist_path)
            })?;

        let mut command = tokio::process::Command::new(&ffmpeg_path);
        command
            .current_dir(temp_dir)
            .arg("-y")
            .arg("-allowed_extensions")
            .arg("ALL")
            .arg("-i")
            .arg("video_playlist.m3u8");

        if let Some(audio_segments) = audio_segments.filter(|segments| !segments.is_empty()) {
            let audio_playlist_path = temp_dir.join("audio_playlist.m3u8");
            let audio_playlist_content = build_local_playlist(audio_segments);
            tfs::write(&audio_playlist_path, audio_playlist_content)
                .await
                .with_context(|| {
                    format!("Failed to write local playlist: {:?}", audio_playlist_path)
                })?;

            command
                .arg("-allowed_extensions")
                .arg("ALL")
                .arg("-i")
                .arg("audio_playlist.m3u8")
                .arg("-map")
                .arg("0:v:0")
                .arg("-map")
                .arg("1:a:0");
        }

        command.arg("-c").arg("copy").arg(output_file);

        let output = command.output().await.with_context(|| {
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
