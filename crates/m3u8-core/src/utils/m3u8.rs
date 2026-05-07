use anyhow::{anyhow, Result};
use m3u8_rs::{AlternativeMediaType, KeyMethod, Playlist};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone)]
pub struct M3U8Info {
    pub segments: Vec<SegmentInfo>,
    pub base_url: String,
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DownloadSource {
    pub video: M3U8Info,
    pub audio: Option<M3U8Info>,
}

impl DownloadSource {
    pub fn total_segments(&self) -> usize {
        self.video.segments.len()
            + self
                .audio
                .as_ref()
                .map(|audio| audio.segments.len())
                .unwrap_or(0)
    }

    pub fn total_size(&self) -> Option<u64> {
        match (
            self.video.total_size,
            self.audio.as_ref().and_then(|audio| audio.total_size),
        ) {
            (Some(video), Some(audio)) => Some(video + audio),
            (Some(video), None) => Some(video),
            (None, Some(audio)) => Some(audio),
            (None, None) => None,
        }
    }

    pub fn all_segments(&self) -> Vec<SegmentInfo> {
        let mut segments = self.video.segments.clone();
        if let Some(audio) = &self.audio {
            segments.extend(audio.segments.clone());
        }
        segments
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct M3U8VariantOption {
    pub video_url: String,
    pub audio_url: Option<String>,
    pub resolution: Option<String>,
    pub bandwidth: u64,
    pub average_bandwidth: Option<u64>,
    pub codecs: Option<String>,
    pub audio_name: Option<String>,
    pub has_separate_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct M3U8ProbeResult {
    pub is_master: bool,
    pub default_variant_index: Option<usize>,
    pub variants: Vec<M3U8VariantOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct M3U8StreamSelection {
    pub original_url: String,
    pub video_url: String,
    pub audio_url: Option<String>,
    pub resolution: Option<String>,
    pub bandwidth: u64,
    pub average_bandwidth: Option<u64>,
    pub codecs: Option<String>,
    pub audio_name: Option<String>,
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
    let client = build_client()?;
    parse_media_playlist_with_client(&client, m3u8_url).await
}

pub async fn probe_m3u8(m3u8_url: &str) -> Result<M3U8ProbeResult> {
    let client = build_client()?;
    let response = client.get(m3u8_url).send().await?.text().await?;

    match m3u8_rs::parse_playlist_res(response.as_bytes()) {
        Ok(Playlist::MediaPlaylist(_)) => Ok(M3U8ProbeResult {
            is_master: false,
            default_variant_index: None,
            variants: Vec::new(),
        }),
        Ok(Playlist::MasterPlaylist(playlist)) => {
            let base_url = Url::parse(m3u8_url)?;
            let mut variants = Vec::new();

            for variant in playlist.variants {
                if variant.is_i_frame {
                    continue;
                }

                let video_url = base_url.join(&variant.uri)?.to_string();
                let audio_rendition = variant.audio.as_ref().and_then(|group_id| {
                    playlist
                        .alternatives
                        .iter()
                        .filter(|media| {
                            media.media_type == AlternativeMediaType::Audio
                                && media.group_id == *group_id
                                && media.uri.is_some()
                        })
                        .max_by_key(|media| {
                            let default_score = if media.default {
                                2
                            } else if media.autoselect {
                                1
                            } else {
                                0
                            };
                            (default_score, media.name.clone())
                        })
                });

                let audio_url = audio_rendition
                    .and_then(|media| media.uri.as_deref())
                    .map(|uri| base_url.join(uri))
                    .transpose()?
                    .map(|url| url.to_string());

                variants.push(M3U8VariantOption {
                    video_url,
                    audio_url,
                    resolution: variant
                        .resolution
                        .as_ref()
                        .map(|resolution| format!("{}x{}", resolution.width, resolution.height)),
                    bandwidth: variant.bandwidth,
                    average_bandwidth: variant.average_bandwidth,
                    codecs: variant.codecs.clone(),
                    audio_name: audio_rendition.map(|media| media.name.clone()),
                    has_separate_audio: audio_rendition.is_some(),
                });
            }

            let default_variant_index = variants
                .iter()
                .enumerate()
                .max_by_key(|(_, variant)| variant_priority(variant))
                .map(|(index, _)| index);

            Ok(M3U8ProbeResult {
                is_master: true,
                default_variant_index,
                variants,
            })
        }
        _ => Err(anyhow!("无法解析该 M3U8 文件")),
    }
}

pub async fn parse_download_source(input: &str) -> Result<DownloadSource> {
    let client = build_client()?;

    if let Ok(selection) = serde_json::from_str::<M3U8StreamSelection>(input) {
        let video = prefix_m3u8_info(
            parse_media_playlist_with_client(&client, &selection.video_url).await?,
            "video",
        );
        let audio = match selection.audio_url.as_deref() {
            Some(audio_url) => Some(prefix_m3u8_info(
                parse_media_playlist_with_client(&client, audio_url).await?,
                "audio",
            )),
            None => None,
        };

        return Ok(DownloadSource { video, audio });
    }

    let video = parse_media_playlist_with_client(&client, input).await?;
    Ok(DownloadSource { video, audio: None })
}

fn build_client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?)
}

async fn parse_media_playlist_with_client(client: &Client, m3u8_url: &str) -> Result<M3U8Info> {
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
            "目前暂不支持直接下载 Master Playlist，请先选择具体清晰度"
        )),
        _ => Err(anyhow!("无法解析该 M3U8 文件")),
    }
}

fn prefix_m3u8_info(mut info: M3U8Info, prefix: &str) -> M3U8Info {
    for segment in &mut info.segments {
        segment.file_name = format!("{prefix}/{}", segment.file_name);
        if let Some(init_map) = &mut segment.init_map {
            init_map.file_name = format!("{prefix}/{}", init_map.file_name);
        }
    }
    info
}

fn variant_priority(variant: &M3U8VariantOption) -> (u64, u64, u64) {
    let resolution_score = variant
        .resolution
        .as_deref()
        .and_then(|value| value.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<u64>().ok()?, h.parse::<u64>().ok()?)))
        .map(|(w, h)| w * h)
        .unwrap_or(0);
    (
        resolution_score,
        variant.average_bandwidth.unwrap_or(0),
        variant.bandwidth,
    )
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
