use anyhow::{Result, anyhow};
use reqwest::Client;
use m3u8_rs::Playlist;
use url::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct M3U8Info {
    pub segments: Vec<String>,
    pub base_url: String,
    pub total_size: Option<u64>,
}

pub async fn parse_m3u8(m3u8_url: &str) -> Result<M3U8Info> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(m3u8_url).send().await?.text().await?;
    
    match m3u8_rs::parse_playlist_res(response.as_bytes()) {
        Ok(Playlist::MediaPlaylist(playlist)) => {
            let base_url = Url::parse(m3u8_url)?;
            let mut full_urls = Vec::new();
            let mut total_size = 0u64;
            let mut has_size = false;

            for segment in playlist.segments {
                // 处理相对路径和绝对路径
                let segment_url = base_url.join(&segment.uri)?;
                full_urls.push(segment_url.to_string());
                
                if let Some(byte_range) = segment.byte_range {
                    total_size += byte_range.length;
                    has_size = true;
                }
            }

            Ok(M3U8Info {
                segments: full_urls,
                base_url: m3u8_url.to_string(),
                total_size: if has_size { Some(total_size) } else { None },
            })
        },
        Ok(Playlist::MasterPlaylist(_)) => {
            Err(anyhow!("目前暂不支持解析 Master Playlist，请提供具体的 Media Playlist URL"))
        },
        _ => Err(anyhow!("无法解析该 M3U8 文件")),
    }
}
