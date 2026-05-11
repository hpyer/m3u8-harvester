# TMDB Task Metadata Assistant Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional TMDB-powered helper that fills movie/series task fields and generates editable M3U8 row titles during task creation.

**Architecture:** Put TMDB request and response normalization in `m3u8-core`, expose it through both Axum and Tauri, then consume it from the shared frontend API client. Keep task creation payloads compatible by rebuilding `rawSubtasks` before the existing master-playlist probing and create-task flow.

**Tech Stack:** Rust, Axum, Tauri 2 commands, reqwest, serde, Vue 3 Composition API, Pinia, TypeScript, Tailwind CSS, DaisyUI.

---

## File Structure

- Create `crates/m3u8-core/src/services/tmdb_service.rs`: TMDB settings resolution, HTTP calls, response normalization, and pure helpers.
- Modify `crates/m3u8-core/src/services/mod.rs`: export `tmdb_service`.
- Modify `crates/m3u8-core/src/lib.rs`: re-export TMDB service types.
- Modify `apps/server/src/main.rs`: create `TmdbService` and add TMDB routes.
- Modify `apps/server/src/handlers/mod.rs`: export the new handler module.
- Create `apps/server/src/handlers/tmdb_handler.rs`: HTTP handlers for search and TV season details.
- Modify `apps/desktop/src-tauri/src/main.rs`: create `TmdbService` and expose matching Tauri commands.
- Modify `apps/web/src/types/app.ts`: add TMDB types and extend settings.
- Modify `apps/web/src/services/api.ts`: add HTTP/Tauri TMDB calls and parsers.
- Modify `apps/web/src/stores/appStore.ts`: add TMDB state/actions, M3U8 row parsing, title override handling, and rebuilt submit payload.
- Modify `apps/web/src/components/modals/SettingsModal.vue`: add TMDB settings fields.
- Modify `apps/web/src/components/modals/AddTaskModal.vue`: add TMDB search UI and naming preview/editor.

## Task 1: Core TMDB Service

**Files:**

- Create: `crates/m3u8-core/src/services/tmdb_service.rs`
- Modify: `crates/m3u8-core/src/services/mod.rs`
- Modify: `crates/m3u8-core/src/lib.rs`

- [ ] **Step 1: Write pure normalization tests**

Add this test module to the new service file with the implementation stubs in the same file:

```rust
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
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test -p m3u8-core services::tmdb_service`

Expected: FAIL because `tmdb_service` and its types/functions do not exist yet.

- [ ] **Step 3: Implement the TMDB service**

Create `crates/m3u8-core/src/services/tmdb_service.rs`:

```rust
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
```

Modify `crates/m3u8-core/src/services/mod.rs`:

```rust
pub mod download_service;
pub mod file_service;
pub mod setting_service;
pub mod task_service;
pub mod tmdb_service;
```

Modify `crates/m3u8-core/src/lib.rs` to re-export the service and types:

```rust
pub use services::tmdb_service::{
    TmdbEpisode, TmdbMediaType, TmdbSearchResult, TmdbSeasonDetails, TmdbService,
};
```

- [ ] **Step 4: Run the focused test and verify it passes**

Run: `cargo test -p m3u8-core services::tmdb_service`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/m3u8-core/src/services/tmdb_service.rs crates/m3u8-core/src/services/mod.rs crates/m3u8-core/src/lib.rs
git commit -m "feat: add TMDB core service"
```

## Task 2: Server TMDB Endpoints

**Files:**

- Modify: `apps/server/src/main.rs`
- Modify: `apps/server/src/handlers/mod.rs`
- Create: `apps/server/src/handlers/tmdb_handler.rs`

- [ ] **Step 1: Add the handler module**

Create `apps/server/src/handlers/tmdb_handler.rs`:

```rust
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use m3u8_core::{TmdbSearchResult, TmdbSeasonDetails};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SearchTmdbQuery {
    pub query: String,
}

pub async fn search_tmdb(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchTmdbQuery>,
) -> Result<Json<Vec<TmdbSearchResult>>, (StatusCode, String)> {
    state
        .tmdb_service
        .search(&query.query)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

pub async fn get_tmdb_tv_season(
    State(state): State<Arc<AppState>>,
    Path((series_id, season_number)): Path<(i64, i32)>,
) -> Result<Json<TmdbSeasonDetails>, (StatusCode, String)> {
    state
        .tmdb_service
        .tv_season(series_id, season_number)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}
```

- [ ] **Step 2: Wire the handler and state**

Modify `apps/server/src/handlers/mod.rs`:

```rust
pub mod file_handler;
pub mod meta_handler;
pub mod setting_handler;
pub mod task_handler;
pub mod tmdb_handler;
```

Modify `apps/server/src/main.rs` imports and state:

```rust
use m3u8_core::{init_db, DownloadService, FileService, SettingService, TaskService, TmdbService};

pub struct AppState {
    pub task_service: Arc<TaskService>,
    pub setting_service: Arc<SettingService>,
    pub file_service: Arc<FileService>,
    pub download_service: Arc<DownloadService>,
    pub tmdb_service: Arc<TmdbService>,
}
```

When creating state, initialize the service from the existing `setting_service`:

```rust
let tmdb_service = Arc::new(TmdbService::new(setting_service.clone()));
let state = Arc::new(AppState {
    task_service,
    setting_service,
    file_service,
    download_service,
    tmdb_service,
});
```

Add routes near the existing task/settings routes:

```rust
.route("/api/tmdb/search", get(handlers::tmdb_handler::search_tmdb))
.route(
    "/api/tmdb/tv/:series_id/season/:season_number",
    get(handlers::tmdb_handler::get_tmdb_tv_season),
)
```

- [ ] **Step 3: Run server tests**

Run: `cargo test -p m3u8-server`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/server/src/main.rs apps/server/src/handlers/mod.rs apps/server/src/handlers/tmdb_handler.rs
git commit -m "feat: expose TMDB server endpoints"
```

## Task 3: Tauri TMDB Commands

**Files:**

- Modify: `apps/desktop/src-tauri/src/main.rs`

- [ ] **Step 1: Add state and command imports**

Update the `m3u8_core` import:

```rust
use m3u8_core::{
    init_db, probe_m3u8, DownloadService, FileService, M3U8ProbeResult, M3U8StreamSelection,
    SettingService, Task, TaskService, TaskWithSubtasks, TmdbSearchResult, TmdbSeasonDetails,
    TmdbService,
};
```

Add `tmdb_service` to `AppState`:

```rust
pub struct AppState {
    pub task_service: Arc<TaskService>,
    pub setting_service: Arc<SettingService>,
    pub file_service: Arc<FileService>,
    pub download_service: Arc<DownloadService>,
    pub tmdb_service: Arc<TmdbService>,
}
```

- [ ] **Step 2: Add commands**

Add these commands after `probe_task_m3u8`:

```rust
#[tauri::command]
async fn search_tmdb(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<TmdbSearchResult>, String> {
    state
        .tmdb_service
        .search(&query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_tmdb_tv_season(
    state: State<'_, Arc<AppState>>,
    series_id: i64,
    season_number: i32,
) -> Result<TmdbSeasonDetails, String> {
    state
        .tmdb_service
        .tv_season(series_id, season_number)
        .await
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Initialize service and register commands**

Where the desktop state is built, add:

```rust
let tmdb_service = Arc::new(TmdbService::new(setting_service.clone()));
```

Include it in `AppState`:

```rust
tmdb_service,
```

Register the commands in `tauri::generate_handler!`:

```rust
search_tmdb,
get_tmdb_tv_season,
```

- [ ] **Step 4: Run desktop check**

Run: `cargo check -p m3u8-desktop`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/main.rs
git commit -m "feat: expose TMDB desktop commands"
```

## Task 4: Frontend Types and API Client

**Files:**

- Modify: `apps/web/src/types/app.ts`
- Modify: `apps/web/src/services/api.ts`

- [ ] **Step 1: Add TypeScript types**

In `apps/web/src/types/app.ts`, extend `AppSettings`:

```ts
export interface AppSettings {
  concurrency: string;
  retryCount: string;
  retryDelay: string;
  userAgent: string;
  proxy: string;
  downloadPath?: string;
  tmdbApiKey: string;
  tmdbApiBaseUrl: string;
}
```

Add TMDB types:

```ts
export type TmdbMediaType = 'movie' | 'tv';

export interface TmdbSearchResult {
  id: number;
  mediaType: TmdbMediaType;
  title: string;
  originalTitle: string | null;
  year: string | null;
  seasonCount: number | null;
}

export interface TmdbEpisode {
  episodeNumber: number;
  name: string | null;
  airDate: string | null;
}

export interface TmdbSeasonDetails {
  seriesId: number;
  seasonNumber: number;
  episodes: TmdbEpisode[];
}

export interface M3U8NamingRow {
  lineIndex: number;
  url: string;
  originalTitle: string;
  generatedTitle: string;
  manualTitle: string;
  episodeNumber: number | null;
  episodeName: string | null;
}
```

- [ ] **Step 2: Add API parsers and methods**

In `apps/web/src/services/api.ts`, import the TMDB types:

```ts
  TmdbSearchResult,
  TmdbSeasonDetails,
```

Add parsers:

```ts
const parseTmdbSearchResults = (value: unknown): TmdbSearchResult[] =>
  Array.isArray(value)
    ? value
        .map((item) => {
          if (!isRecord(item)) return null;
          const id = asNumber(item.id, 0);
          const mediaType =
            item.mediaType === 'movie' || item.mediaType === 'tv' ? item.mediaType : null;
          const title = asString(item.title);
          if (!id || !mediaType || !title) return null;
          return {
            id,
            mediaType,
            title,
            originalTitle: asNullableString(item.originalTitle),
            year: asNullableString(item.year),
            seasonCount: asNullableNumber(item.seasonCount),
          };
        })
        .filter((item): item is TmdbSearchResult => item !== null)
    : [];

const parseTmdbSeasonDetails = (value: unknown): TmdbSeasonDetails => {
  if (!isRecord(value)) {
    return { seriesId: 0, seasonNumber: 0, episodes: [] };
  }

  return {
    seriesId: asNumber(value.seriesId),
    seasonNumber: asNumber(value.seasonNumber),
    episodes: Array.isArray(value.episodes)
      ? value.episodes
          .map((episode) => {
            if (!isRecord(episode)) return null;
            return {
              episodeNumber: asNumber(episode.episodeNumber),
              name: asNullableString(episode.name),
              airDate: asNullableString(episode.airDate),
            };
          })
          .filter((episode): episode is TmdbSeasonDetails['episodes'][number] => episode !== null)
      : [],
  };
};
```

Extend `AppApi`:

```ts
  searchTmdb(query: string): Promise<TmdbSearchResult[]>;
  getTmdbTvSeason(seriesId: number, seasonNumber: number): Promise<TmdbSeasonDetails>;
```

Add HTTP methods:

```ts
  async searchTmdb(query: string) {
    const res = await axios.get<unknown>(`${API_BASE}/api/tmdb/search`, { params: { query } });
    return parseTmdbSearchResults(res.data);
  }

  async getTmdbTvSeason(seriesId: number, seasonNumber: number) {
    const res = await axios.get<unknown>(`${API_BASE}/api/tmdb/tv/${seriesId}/season/${seasonNumber}`);
    return parseTmdbSeasonDetails(res.data);
  }
```

Add Tauri methods:

```ts
  async searchTmdb(query: string) {
    const data = await invoke<unknown>('search_tmdb', { query });
    return parseTmdbSearchResults(data);
  }

  async getTmdbTvSeason(seriesId: number, seasonNumber: number) {
    const data = await invoke<unknown>('get_tmdb_tv_season', {
      seriesId,
      seasonNumber,
    });
    return parseTmdbSeasonDetails(data);
  }
```

Export wrappers:

```ts
  searchTmdb: (query: string) => apiClient.searchTmdb(query),
  getTmdbTvSeason: (seriesId: number, seasonNumber: number) =>
    apiClient.getTmdbTvSeason(seriesId, seasonNumber),
```

- [ ] **Step 3: Build frontend types**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/types/app.ts apps/web/src/services/api.ts
git commit -m "feat: add TMDB frontend API client"
```

## Task 5: Settings UI and Store Persistence

**Files:**

- Modify: `apps/web/src/stores/appStore.ts`
- Modify: `apps/web/src/components/modals/SettingsModal.vue`

- [ ] **Step 1: Add default settings and parsing**

In `apps/web/src/stores/appStore.ts`, update `DEFAULT_SETTINGS`:

```ts
const DEFAULT_SETTINGS: AppSettings = {
  concurrency: '5',
  retryCount: '3',
  retryDelay: '2000',
  userAgent:
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  proxy: '',
  tmdbApiKey: '',
  tmdbApiBaseUrl: 'https://api.themoviedb.org/3',
};
```

In `loadSettings`, merge the new fields:

```ts
tmdbApiKey: String(settings.tmdbApiKey ?? this.settings.tmdbApiKey),
tmdbApiBaseUrl: String(settings.tmdbApiBaseUrl ?? this.settings.tmdbApiBaseUrl),
```

In `saveSettings`, include the new fields:

```ts
tmdbApiKey: String(newSettings.tmdbApiKey ?? this.settings.tmdbApiKey),
tmdbApiBaseUrl: String(newSettings.tmdbApiBaseUrl ?? this.settings.tmdbApiBaseUrl),
```

- [ ] **Step 2: Add settings fields**

In `apps/web/src/components/modals/SettingsModal.vue`, add this section between download configuration and version information:

```vue
<section class="rounded-xl border border-base-300 bg-base-100 p-4">
  <div class="mb-4">
    <h4 class="font-semibold text-sm text-primary">TMDB 配置</h4>
    <p class="text-xs opacity-60 mt-1">用于新建任务时搜索电影和剧集信息，只辅助填表和命名。</p>
  </div>

  <div class="flex flex-col gap-4">
    <div class="form-control">
      <label class="label pb-1"><span class="label-text font-medium">TMDB API Key</span></label>
      <input
        v-model="localSettings.tmdbApiKey"
        type="password"
        autocomplete="off"
        class="input input-bordered w-full"
        placeholder="在 TMDB 账户设置中获取 API Key"
      />
    </div>

    <div class="form-control">
      <label class="label pb-1"><span class="label-text font-medium">TMDB API 地址</span></label>
      <input
        v-model="localSettings.tmdbApiBaseUrl"
        type="text"
        class="input input-bordered w-full"
        placeholder="https://api.themoviedb.org/3"
      />
    </div>
  </div>
</section>
```

- [ ] **Step 3: Build frontend**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/stores/appStore.ts apps/web/src/components/modals/SettingsModal.vue
git commit -m "feat: add TMDB settings"
```

## Task 6: Store TMDB Actions and Naming Rows

**Files:**

- Modify: `apps/web/src/stores/appStore.ts`

- [ ] **Step 1: Add store state**

Import new types:

```ts
  M3U8NamingRow,
  TmdbSearchResult,
  TmdbSeasonDetails,
```

Add state fields:

```ts
tmdbSearchQuery: '',
tmdbSearchResults: [] as TmdbSearchResult[],
tmdbSearchLoading: false,
tmdbSearchError: '',
selectedTmdbResult: null as TmdbSearchResult | null,
selectedTmdbSeason: null as TmdbSeasonDetails | null,
tmdbSeasonNumber: '1',
tmdbStartEpisode: '1',
namingRows: [] as M3U8NamingRow[],
```

- [ ] **Step 2: Add row helper methods**

Add these actions:

```ts
parseNamingRows(rawSubtasks: string) {
  const existingManualTitles = new Map(
    this.namingRows.map((row) => [row.lineIndex, row.manualTitle]),
  );

  this.namingRows = rawSubtasks
    .split('\n')
    .map((line, lineIndex) => ({ line: line.trim(), lineIndex }))
    .filter(({ line }) => Boolean(line))
    .map(({ line, lineIndex }) => {
      const [url, ...titleParts] = line.split(/\s+/);
      const originalTitle = titleParts.join(' ');
      return {
        lineIndex,
        url,
        originalTitle,
        generatedTitle: this.generateRowTitle(lineIndex),
        manualTitle: existingManualTitles.get(lineIndex) ?? originalTitle,
        episodeNumber: this.getEpisodeNumberForRow(lineIndex),
        episodeName: this.getEpisodeNameForRow(lineIndex),
      };
    });
},
generateRowTitle(lineIndex: number) {
  if (this.addTaskData.category !== 'series') {
    return '';
  }

  const season = Number.parseInt(this.tmdbSeasonNumber || this.addTaskData.season || '1', 10);
  const episode = this.getEpisodeNumberForRow(lineIndex) ?? lineIndex + 1;
  return `S${String(Number.isFinite(season) ? season : 1).padStart(2, '0')}E${String(episode).padStart(2, '0')}`;
},
getEpisodeNumberForRow(lineIndex: number) {
  if (this.addTaskData.category !== 'series') return null;
  const start = Number.parseInt(this.tmdbStartEpisode || '1', 10);
  const safeStart = Number.isFinite(start) && start > 0 ? start : 1;
  return safeStart + lineIndex;
},
getEpisodeNameForRow(lineIndex: number) {
  const episodeNumber = this.getEpisodeNumberForRow(lineIndex);
  if (!episodeNumber || !this.selectedTmdbSeason) return null;
  return (
    this.selectedTmdbSeason.episodes.find((episode) => episode.episodeNumber === episodeNumber)
      ?.name ?? null
  );
},
setNamingRowManualTitle(lineIndex: number, manualTitle: string) {
  const row = this.namingRows.find((item) => item.lineIndex === lineIndex);
  if (row) {
    row.manualTitle = manualTitle;
  }
},
rebuildRawSubtasksFromNamingRows(task: AddTaskPayload): AddTaskPayload {
  if (this.namingRows.length === 0) {
    return task;
  }

  const rawSubtasks = this.namingRows
    .map((row) => {
      const finalTitle = row.manualTitle.trim() || row.generatedTitle.trim();
      return finalTitle ? `${row.url} ${finalTitle}` : row.url;
    })
    .join('\n');

  return { ...task, rawSubtasks };
},
```

- [ ] **Step 3: Use rebuilt rows on submit**

At the start of `submitNewTask`, replace direct use of `task` with:

```ts
const finalTask = this.rebuildRawSubtasksFromNamingRows(task);
const streamSelections = await this.resolveStreamSelections(finalTask.rawSubtasks);
```

And submit `finalTask`:

```ts
await api.createTask({ ...finalTask, streamSelections });
```

When resetting after submit, clear TMDB state:

```ts
this.resetTmdbTaskHelper();
```

Add the reset action:

```ts
resetTmdbTaskHelper() {
  this.tmdbSearchQuery = '';
  this.tmdbSearchResults = [];
  this.tmdbSearchLoading = false;
  this.tmdbSearchError = '';
  this.selectedTmdbResult = null;
  this.selectedTmdbSeason = null;
  this.tmdbSeasonNumber = '1';
  this.tmdbStartEpisode = '1';
  this.namingRows = [];
},
```

- [ ] **Step 4: Run frontend build**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/stores/appStore.ts
git commit -m "feat: add TMDB naming row state"
```

## Task 7: Store TMDB Search and Season Actions

**Files:**

- Modify: `apps/web/src/stores/appStore.ts`

- [ ] **Step 1: Add TMDB actions**

Add these actions:

```ts
async searchTmdb() {
  const query = this.tmdbSearchQuery.trim() || this.addTaskData.title.trim();
  if (!query) {
    this.tmdbSearchResults = [];
    this.tmdbSearchError = '';
    return;
  }

  if (!this.settings.tmdbApiKey.trim()) {
    this.tmdbSearchError = '请先在设置中填写 TMDB API Key';
    this.tmdbSearchResults = [];
    return;
  }

  this.tmdbSearchLoading = true;
  this.tmdbSearchError = '';
  try {
    this.tmdbSearchResults = await api.searchTmdb(query);
  } catch (_error) {
    this.tmdbSearchResults = [];
    this.tmdbSearchError = 'TMDB 查询失败，请检查 API Key 或 API 地址';
  } finally {
    this.tmdbSearchLoading = false;
  }
},
async selectTmdbResult(result: TmdbSearchResult) {
  this.selectedTmdbResult = result;
  this.addTaskData.title = result.title;
  this.addTaskData.category = result.mediaType === 'movie' ? 'movie' : 'series';
  this.addTaskData.year = result.year ?? '';

  if (result.mediaType === 'tv') {
    if (!this.addTaskData.season) {
      this.addTaskData.season = '1';
    }
    this.tmdbSeasonNumber = this.addTaskData.season || '1';
    await this.loadTmdbSeason();
  } else {
    this.selectedTmdbSeason = null;
  }

  this.parseNamingRows(this.addTaskData.rawSubtasks);
},
async loadTmdbSeason() {
  if (!this.selectedTmdbResult || this.selectedTmdbResult.mediaType !== 'tv') {
    this.selectedTmdbSeason = null;
    return;
  }

  const seasonNumber = Number.parseInt(this.tmdbSeasonNumber || this.addTaskData.season || '1', 10);
  const safeSeason = Number.isFinite(seasonNumber) && seasonNumber >= 0 ? seasonNumber : 1;
  this.tmdbSeasonNumber = String(safeSeason);
  this.addTaskData.season = String(safeSeason);

  try {
    this.selectedTmdbSeason = await api.getTmdbTvSeason(this.selectedTmdbResult.id, safeSeason);
  } catch (_error) {
    this.selectedTmdbSeason = null;
    this.tmdbSearchError = '季集信息加载失败，仍可手动命名';
  }

  this.parseNamingRows(this.addTaskData.rawSubtasks);
},
updateTmdbEpisodeMapping() {
  this.parseNamingRows(this.addTaskData.rawSubtasks);
},
```

- [ ] **Step 2: Reset helper when opening the modal**

At the start of `openAddTaskModal`, add:

```ts
this.resetTmdbTaskHelper();
```

Keep the existing `closeVariantSelectionModal()` call before it.

- [ ] **Step 3: Run frontend build**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/stores/appStore.ts
git commit -m "feat: add TMDB task helper actions"
```

## Task 8: Add Task Modal UI

**Files:**

- Modify: `apps/web/src/components/modals/AddTaskModal.vue`

- [ ] **Step 1: Add helpers to script**

In the script block, add:

```ts
import { computed } from 'vue';
```

Add helpers:

```ts
const isTmdbConfigured = computed(() => Boolean(store.settings.tmdbApiKey.trim()));

const parseRows = () => {
  store.parseNamingRows(store.addTaskData.rawSubtasks);
};

const searchTmdb = async () => {
  await store.searchTmdb();
};

const selectTmdbResult = async (result: (typeof store.tmdbSearchResults)[number]) => {
  await store.selectTmdbResult(result);
};
```

Update `setCategory`:

```ts
const setCategory = (category: TaskCategory) => {
  store.addTaskData.category = category;
  parseRows();
};
```

- [ ] **Step 2: Add TMDB search section above task name**

Add this block below the heading:

```vue
<section class="mb-5 rounded-lg border border-base-300 bg-base-200/40 p-4">
  <div class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-3">
      <div>
        <h4 class="font-semibold text-sm">TMDB 辅助填表</h4>
        <p class="text-xs opacity-60 mt-1">可选，用于匹配电影/剧集信息并生成命名预览。</p>
      </div>
      <span v-if="!isTmdbConfigured" class="badge badge-warning badge-sm">未配置</span>
    </div>

    <div class="join w-full">
      <input
        v-model="store.tmdbSearchQuery"
        type="text"
        class="input input-bordered join-item w-full"
        :disabled="!isTmdbConfigured"
        placeholder="输入电影或剧集名称"
        @keyup.enter="searchTmdb"
      />
      <button class="btn btn-primary join-item" :disabled="!isTmdbConfigured || store.tmdbSearchLoading" @click="searchTmdb">
        {{ store.tmdbSearchLoading ? '查询中' : '查询' }}
      </button>
    </div>

    <p v-if="!isTmdbConfigured" class="text-xs text-warning">请先在设置中填写 TMDB API Key 和 API 地址。</p>
    <p v-if="store.tmdbSearchError" class="text-xs text-error">{{ store.tmdbSearchError }}</p>

    <div v-if="store.tmdbSearchResults.length" class="grid gap-2 max-h-40 overflow-y-auto">
      <button
        v-for="result in store.tmdbSearchResults"
        :key="`${result.mediaType}-${result.id}`"
        type="button"
        class="btn btn-sm justify-between"
        :class="store.selectedTmdbResult?.id === result.id && store.selectedTmdbResult?.mediaType === result.mediaType ? 'btn-primary' : 'btn-outline'"
        @click="selectTmdbResult(result)"
      >
        <span class="truncate">{{ result.title }}</span>
        <span class="opacity-70">{{ result.mediaType === 'movie' ? '电影' : '剧集' }} {{ result.year || '' }}</span>
      </button>
    </div>
  </div>
</section>
```

- [ ] **Step 3: Add series season controls**

After the category/year/season controls, add:

```vue
<div v-if="store.addTaskData.category === 'series' && store.selectedTmdbResult?.mediaType === 'tv'" class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-3">
  <div class="form-control">
    <label class="label pb-1"><span class="label-text font-medium">TMDB 季号</span></label>
    <input
      v-model="store.tmdbSeasonNumber"
      type="number"
      min="0"
      class="input input-bordered w-full"
      @change="store.loadTmdbSeason"
    />
  </div>
  <div class="form-control">
    <label class="label pb-1"><span class="label-text font-medium">起始集</span></label>
    <input
      v-model="store.tmdbStartEpisode"
      type="number"
      min="1"
      class="input input-bordered w-full"
      @input="store.updateTmdbEpisodeMapping"
    />
  </div>
  <div class="flex items-end">
    <button class="btn btn-outline w-full" @click="store.loadTmdbSeason">刷新季集</button>
  </div>
</div>
```

- [ ] **Step 4: Parse textarea into rows**

Add `@input="parseRows"` to the textarea:

```vue
<textarea
  v-model="store.addTaskData.rawSubtasks"
  class="textarea textarea-bordered h-48 font-mono text-sm"
  placeholder="https://example.com/a.m3u8 第01集&#10;https://example.com/b.m3u8 第02集"
  @input="parseRows"
></textarea>
```

- [ ] **Step 5: Add naming preview**

Add this block below the textarea:

```vue
<div v-if="store.namingRows.length" class="mt-4 rounded-lg border border-base-300 overflow-hidden">
  <div class="bg-base-200 px-3 py-2 text-sm font-semibold">命名预览</div>
<div class="divide-y divide-base-300">
    <div v-for="row in store.namingRows" :key="row.lineIndex" class="grid grid-cols-1 md:grid-cols-[4rem_1fr_1fr] gap-3 p-3 items-center">
      <span class="badge badge-ghost">#{{ row.lineIndex + 1 }}</span>
      <div class="min-w-0">
        <p class="truncate font-mono text-xs opacity-70">{{ row.url }}</p>
        <p v-if="row.generatedTitle" class="text-xs mt-1">
          建议：<span class="font-mono">{{ row.generatedTitle }}</span>
          <span v-if="row.episodeName" class="opacity-60"> · {{ row.episodeName }}</span>
        </p>
      </div>
<input
  :value="row.manualTitle"
  type="text"
  class="input input-bordered input-sm w-full"
  placeholder="手动标题，留空则用建议标题"
  @input="store.setNamingRowManualTitle(row.lineIndex, ($event.target as HTMLInputElement).value)"
/>
```

- [ ] **Step 6: Run frontend build**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/components/modals/AddTaskModal.vue
git commit -m "feat: add TMDB task creation UI"
```

## Task 9: Final Verification

**Files:**

- Verify all changed files.

- [ ] **Step 1: Format Rust**

Run: `cargo fmt --all`

Expected: command exits with code 0.

- [ ] **Step 2: Run core tests**

Run: `cargo test -p m3u8-core`

Expected: PASS.

- [ ] **Step 3: Run server tests**

Run: `cargo test -p m3u8-server`

Expected: PASS.

- [ ] **Step 4: Build frontend**

Run: `pnpm --filter @m3u8-harvester/web build`

Expected: PASS.

- [ ] **Step 5: Check git diff**

Run: `git status --short`

Expected: only intentional files are modified, or the tree is clean after commits.

- [ ] **Step 6: Final commit if formatting changed files**

If `cargo fmt --all` or frontend build tooling changed files after the previous commits, run:

```bash
git add crates/m3u8-core/src/services/tmdb_service.rs crates/m3u8-core/src/services/mod.rs crates/m3u8-core/src/lib.rs apps/server/src/main.rs apps/server/src/handlers/mod.rs apps/server/src/handlers/tmdb_handler.rs apps/desktop/src-tauri/src/main.rs apps/web/src/types/app.ts apps/web/src/services/api.ts apps/web/src/stores/appStore.ts apps/web/src/components/modals/SettingsModal.vue apps/web/src/components/modals/AddTaskModal.vue
git commit -m "chore: finalize TMDB assistant formatting"
```

Expected: commit succeeds if there are formatting changes; skip this step when `git status --short` is clean.

## Self-Review

- Spec coverage: settings, HTTP/Tauri local proxy, movie and TV support, textarea preservation, naming preview, manual title override, unchanged task schema, and unchanged download semantics are covered by Tasks 1-8.
- Placeholder scan: the plan contains no unresolved placeholders or deferred implementation notes.
- Type consistency: Rust exports `TmdbSearchResult` and `TmdbSeasonDetails`; frontend parsers use `mediaType`, `seasonCount`, `episodeNumber`, and `airDate` consistently with serde camelCase.
