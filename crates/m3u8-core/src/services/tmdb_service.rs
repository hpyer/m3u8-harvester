use crate::services::SettingService;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEFAULT_TMDB_API_BASE_URL: &str = "https://api.themoviedb.org/3";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TmdbMediaType {
    Movie,
    Tv,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TmdbSearchResult {
    pub id: i64,
    pub media_type: TmdbMediaType,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<String>,
    pub season_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TmdbEpisode {
    pub episode_number: i32,
    pub name: Option<String>,
    pub air_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TmdbSeasonDetails {
    pub series_id: i64,
    pub season_number: i32,
    pub episodes: Vec<TmdbEpisode>,
}

#[derive(Debug, Deserialize)]
struct TmdbSearchResponse<T> {
    results: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieResult {
    id: i64,
    title: Option<String>,
    original_title: Option<String>,
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvResult {
    id: i64,
    name: Option<String>,
    original_name: Option<String>,
    first_air_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbTvDetailsResponse {
    number_of_seasons: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct TmdbSeasonResponse {
    season_number: i32,
    episodes: Vec<TmdbSeasonEpisodeResponse>,
}

#[derive(Debug, Deserialize)]
struct TmdbSeasonEpisodeResponse {
    episode_number: i32,
    name: Option<String>,
    air_date: Option<String>,
}

pub struct TmdbService {
    client: Client,
    setting_service: Arc<SettingService>,
}

impl TmdbService {
    pub fn new(setting_service: Arc<SettingService>) -> Self {
        Self {
            client: Client::new(),
            setting_service,
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<TmdbSearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let config = self.load_config().await?;
        let mut movies = self.search_movies(&config, query).await?;
        let mut tv = self.search_tv(&config, query).await?;
        movies.append(&mut tv);
        Ok(movies)
    }

    pub async fn tv_season(&self, series_id: i64, season_number: i32) -> Result<TmdbSeasonDetails> {
        let config = self.load_config().await?;
        let url = format!(
            "{}/tv/{}/season/{}",
            config.base_url, series_id, season_number
        );
        let response = self
            .client
            .get(url)
            .query(&[("api_key", config.api_key.as_str()), ("language", "zh-CN")])
            .send()
            .await?
            .error_for_status()?
            .json::<TmdbSeasonResponse>()
            .await?;

        Ok(TmdbSeasonDetails {
            series_id,
            season_number: response.season_number,
            episodes: response
                .episodes
                .into_iter()
                .map(|episode| TmdbEpisode {
                    episode_number: episode.episode_number,
                    name: episode.name,
                    air_date: episode.air_date,
                })
                .collect(),
        })
    }

    async fn search_movies(
        &self,
        config: &TmdbConfig,
        query: &str,
    ) -> Result<Vec<TmdbSearchResult>> {
        let url = format!("{}/search/movie", config.base_url);
        let response = self
            .client
            .get(url)
            .query(&[
                ("api_key", config.api_key.as_str()),
                ("query", query),
                ("language", "zh-CN"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<TmdbSearchResponse<TmdbMovieResult>>()
            .await?;

        Ok(response
            .results
            .into_iter()
            .map(TmdbMovieResult::into_search_result)
            .collect())
    }

    async fn search_tv(&self, config: &TmdbConfig, query: &str) -> Result<Vec<TmdbSearchResult>> {
        let url = format!("{}/search/tv", config.base_url);
        let response = self
            .client
            .get(url)
            .query(&[
                ("api_key", config.api_key.as_str()),
                ("query", query),
                ("language", "zh-CN"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<TmdbSearchResponse<TmdbTvResult>>()
            .await?;

        let mut results = Vec::new();
        for tv in response.results {
            let mut result = tv.into_search_result();
            result.season_count = self.tv_season_count(config, result.id).await.ok().flatten();
            results.push(result);
        }
        Ok(results)
    }

    async fn tv_season_count(&self, config: &TmdbConfig, series_id: i64) -> Result<Option<i32>> {
        let url = format!("{}/tv/{}", config.base_url, series_id);
        let response = self
            .client
            .get(url)
            .query(&[("api_key", config.api_key.as_str()), ("language", "zh-CN")])
            .send()
            .await?
            .error_for_status()?
            .json::<TmdbTvDetailsResponse>()
            .await?;
        Ok(response.number_of_seasons)
    }

    async fn load_config(&self) -> Result<TmdbConfig> {
        let settings = self.setting_service.get_all().await?;
        let api_key = settings
            .get("tmdbApiKey")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("TMDB API Key 未配置"))?;
        let base_url = settings
            .get("tmdbApiBaseUrl")
            .map(|value| normalize_base_url(value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_TMDB_API_BASE_URL.to_string());

        Ok(TmdbConfig { api_key, base_url })
    }
}

struct TmdbConfig {
    api_key: String,
    base_url: String,
}

impl TmdbMovieResult {
    fn into_search_result(self) -> TmdbSearchResult {
        TmdbSearchResult {
            id: self.id,
            media_type: TmdbMediaType::Movie,
            title: self.title.unwrap_or_else(|| "未命名电影".to_string()),
            original_title: self.original_title,
            year: extract_year(self.release_date.as_deref()),
            season_count: None,
        }
    }
}

impl TmdbTvResult {
    fn into_search_result(self) -> TmdbSearchResult {
        TmdbSearchResult {
            id: self.id,
            media_type: TmdbMediaType::Tv,
            title: self.name.unwrap_or_else(|| "未命名剧集".to_string()),
            original_title: self.original_name,
            year: extract_year(self.first_air_date.as_deref()),
            season_count: None,
        }
    }
}

fn extract_year(value: Option<&str>) -> Option<String> {
    value
        .and_then(|date| date.get(0..4))
        .filter(|year| year.chars().all(|ch| ch.is_ascii_digit()))
        .map(ToString::to_string)
}

fn normalize_base_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        DEFAULT_TMDB_API_BASE_URL.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_year_reads_first_four_digits() {
        assert_eq!(extract_year(Some("2024-03-01")), Some("2024".to_string()));
        assert_eq!(extract_year(Some("")), None);
        assert_eq!(extract_year(None), None);
    }

    #[test]
    fn normalize_base_url_trims_trailing_slashes() {
        assert_eq!(
            normalize_base_url("https://api.themoviedb.org/3///"),
            "https://api.themoviedb.org/3"
        );
        assert_eq!(normalize_base_url(""), "https://api.themoviedb.org/3");
    }

    #[test]
    fn movie_result_uses_title_and_release_year() {
        let item = TmdbMovieResult {
            id: 42,
            title: Some("Arrival".to_string()),
            original_title: Some("Arrival".to_string()),
            release_date: Some("2016-11-10".to_string()),
        };

        assert_eq!(
            item.into_search_result(),
            TmdbSearchResult {
                id: 42,
                media_type: TmdbMediaType::Movie,
                title: "Arrival".to_string(),
                original_title: Some("Arrival".to_string()),
                year: Some("2016".to_string()),
                season_count: None,
            }
        );
    }
}
