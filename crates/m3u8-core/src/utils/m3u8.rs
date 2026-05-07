use anyhow::{anyhow, Result};
use m3u8_rs::{KeyMethod, Playlist};
use reqwest::Client;
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone)]
pub struct M3U8Info {
    pub segments: Vec<SegmentInfo>,
    pub base_url: String,
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SegmentInfo {
    pub url: String,
    pub file_name: String,
    pub duration: f32,
    pub media_sequence: u64,
    pub key: Option<EncryptionKey>,
    pub init_map: Option<InitMapInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionKey {
    pub url: String,
    pub iv: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitMapInfo {
    pub url: String,
    pub file_name: String,
}

pub async fn parse_m3u8(m3u8_url: &str) -> Result<M3U8Info> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(m3u8_url).send().await?.text().await?;

    match m3u8_rs::parse_playlist_res(response.as_bytes()) {
        Ok(Playlist::MediaPlaylist(playlist)) => {
            let base_url = Url::parse(m3u8_url)?;
            let mut full_segments = Vec::new();
            let mut total_size = 0u64;
            let mut has_size = false;
            let mut current_key = None;
            let mut current_map = None;
            let mut map_file_names = HashMap::new();
            let mut map_index = 0usize;
            let media_sequence = playlist.media_sequence;

            for (index, segment) in playlist.segments.into_iter().enumerate() {
                if let Some(key) = segment.key.as_ref() {
                    current_key = normalize_key(&base_url, key)?;
                }
                if let Some(map) = segment.map.as_ref() {
                    current_map = Some(normalize_map(
                        &base_url,
                        map,
                        &mut map_file_names,
                        &mut map_index,
                    )?);
                }

                let segment_url = base_url.join(&segment.uri)?;
                full_segments.push(SegmentInfo {
                    url: segment_url.to_string(),
                    file_name: build_segment_file_name(index, &segment_url),
                    duration: segment.duration,
                    media_sequence: media_sequence + index as u64,
                    key: current_key.clone(),
                    init_map: current_map.clone(),
                });

                if let Some(byte_range) = segment.byte_range {
                    total_size += byte_range.length;
                    has_size = true;
                }
            }

            Ok(M3U8Info {
                segments: full_segments,
                base_url: m3u8_url.to_string(),
                total_size: if has_size { Some(total_size) } else { None },
            })
        }
        Ok(Playlist::MasterPlaylist(_)) => Err(anyhow!(
            "目前暂不支持解析 Master Playlist，请提供具体的 Media Playlist URL"
        )),
        _ => Err(anyhow!("无法解析该 M3U8 文件")),
    }
}

fn normalize_key(base_url: &Url, key: &m3u8_rs::Key) -> Result<Option<EncryptionKey>> {
    match &key.method {
        KeyMethod::None => Ok(None),
        KeyMethod::AES128 => {
            if let Some(key_format) = key.keyformat.as_deref() {
                if key_format != "identity" {
                    return Err(anyhow!("暂不支持 KEYFORMAT={} 的加密 HLS", key_format));
                }
            }

            let key_uri = key
                .uri
                .as_deref()
                .ok_or_else(|| anyhow!("EXT-X-KEY 缺少 URI"))?;
            let key_url = base_url.join(key_uri)?;

            Ok(Some(EncryptionKey {
                url: key_url.to_string(),
                iv: key.iv.clone(),
            }))
        }
        KeyMethod::SampleAES => Err(anyhow!("暂不支持 SAMPLE-AES 加密 HLS")),
        KeyMethod::Other(method) => Err(anyhow!("暂不支持 {} 加密 HLS", method)),
    }
}

fn normalize_map(
    base_url: &Url,
    map: &m3u8_rs::Map,
    file_names: &mut HashMap<String, String>,
    map_index: &mut usize,
) -> Result<InitMapInfo> {
    let map_url = base_url.join(&map.uri)?;
    let map_url_string = map_url.to_string();

    let file_name = if let Some(existing) = file_names.get(&map_url_string) {
        existing.clone()
    } else {
        let file_name = build_map_file_name(*map_index, &map_url);
        *map_index += 1;
        file_names.insert(map_url_string.clone(), file_name.clone());
        file_name
    };

    Ok(InitMapInfo {
        url: map_url_string,
        file_name,
    })
}

fn build_segment_file_name(index: usize, url: &Url) -> String {
    format!("{index:05}.{}", extension_from_url(url, "ts"))
}

fn build_map_file_name(index: usize, url: &Url) -> String {
    format!("init_{index:03}.{}", extension_from_url(url, "mp4"))
}

fn extension_from_url(url: &Url, fallback: &str) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .and_then(|name| name.rsplit_once('.').map(|(_, ext)| ext))
        .filter(|ext| !ext.is_empty())
        .unwrap_or(fallback)
        .to_ascii_lowercase()
}
