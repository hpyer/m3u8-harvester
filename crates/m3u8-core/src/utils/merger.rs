use crate::utils::m3u8::SegmentInfo;
use anyhow::{anyhow, Context, Result};
use std::env;
use std::path::{Path, PathBuf};
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

fn resolve_ffmpeg_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("FFMPEG_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(anyhow!(
            "FFMPEG_PATH is set but does not point to a file: {:?}",
            path
        ));
    }

    if let Ok(path) = which::which("ffmpeg") {
        return Ok(path);
    }

    for candidate in fallback_ffmpeg_candidates() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "FFmpeg executable not found. Install ffmpeg and make sure it is in PATH, or set FFMPEG_PATH to the absolute ffmpeg binary path."
    ))
}

fn fallback_ffmpeg_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/opt/homebrew/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
        ]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
            PathBuf::from("/snap/bin/ffmpeg"),
        ]
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = vec![
            PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe"),
            PathBuf::from(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe"),
        ];

        if let Some(program_files) = env::var_os("ProgramFiles") {
            candidates.push(
                PathBuf::from(program_files)
                    .join("ffmpeg")
                    .join("bin")
                    .join("ffmpeg.exe"),
            );
        }

        if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
            candidates.push(
                PathBuf::from(program_files_x86)
                    .join("ffmpeg")
                    .join("bin")
                    .join("ffmpeg.exe"),
            );
        }

        if let Some(choco_install) = env::var_os("ChocolateyInstall") {
            candidates.push(PathBuf::from(choco_install).join("bin").join("ffmpeg.exe"));
        }

        if let Some(user_profile) = env::var_os("USERPROFILE") {
            candidates.push(
                PathBuf::from(&user_profile)
                    .join("scoop")
                    .join("shims")
                    .join("ffmpeg.exe"),
            );
        }

        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local_app_data)
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join("ffmpeg.exe"),
            );
        }

        candidates
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::fallback_ffmpeg_candidates;
    use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "macos")]
    fn includes_common_homebrew_ffmpeg_locations_on_macos() {
        let candidates = fallback_ffmpeg_candidates();
        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/ffmpeg")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/ffmpeg")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn includes_common_system_locations_on_linux() {
        let candidates = fallback_ffmpeg_candidates();
        assert!(candidates.contains(&PathBuf::from("/usr/bin/ffmpeg")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/ffmpeg")));
        assert!(candidates.contains(&PathBuf::from("/snap/bin/ffmpeg")));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn includes_common_install_locations_on_windows() {
        let candidates = fallback_ffmpeg_candidates();
        assert!(candidates.contains(&PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe")));
        assert!(candidates.contains(&PathBuf::from(r"C:\ProgramData\chocolatey\bin\ffmpeg.exe")));
    }

    #[test]
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    fn has_no_platform_fallbacks_for_unknown_targets() {
        assert!(fallback_ffmpeg_candidates().is_empty());
    }
}
