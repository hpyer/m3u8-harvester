# TMDB Task Metadata Assistant Design

## Purpose

Add an optional TMDB assistant to the task creation flow. The feature helps users search movie and TV metadata, fill task form fields, and generate better file names for M3U8 subtasks.

This feature is not a media library feature. It does not store posters, overviews, cast data, or TMDB IDs in the task table. Manual task creation remains fully supported when TMDB is not configured or when the user does not want metadata help.

## Goals

- Let users configure a TMDB API key and TMDB API base URL in the settings modal.
- Let users search TMDB by title from the add-task modal.
- Support both movies and TV series.
- Fill task fields from the selected TMDB result.
- Preserve the textarea-based batch M3U8 input.
- Add a structured naming preview below the textarea so each M3U8 line can be mapped to a movie part or TV episode.
- Let manual per-line titles override TMDB-generated titles.
- Keep the existing task creation payload and download path semantics intact.

## Non-Goals

- Do not persist TMDB metadata on tasks.
- Do not add poster, backdrop, overview, or cast display.
- Do not change the database schema for tasks.
- Do not replace the M3U8 textarea with only individual URL inputs.
- Do not change master-playlist variant selection behavior.
- Do not change the existing series season directory rules.

## Architecture

Use the existing cross-runtime API pattern:

- `apps/web/src/services/api.ts` exposes TMDB methods on `AppApi`.
- HTTP mode calls new Axum endpoints in `apps/server`.
- Tauri mode calls new commands in `apps/desktop/src-tauri`.
- Both HTTP and Tauri implementations read `tmdbApiKey` and `tmdbApiBaseUrl` from `SettingService`.

This keeps the TMDB API key local to the server or desktop runtime and avoids exposing it directly from frontend code in HTTP mode.

## Settings

Add two settings:

- `tmdbApiKey`: user-provided TMDB API key.
- `tmdbApiBaseUrl`: configurable TMDB API base URL, defaulting to `https://api.themoviedb.org/3`.

The settings modal should show both fields near the existing runtime settings. If `tmdbApiKey` is empty, TMDB search controls remain visible but disabled with a concise Chinese helper message.

The API base URL should be normalized by trimming trailing slashes. Invalid or unreachable URLs should fail during TMDB requests, not while saving settings.

## TMDB API Surface

Add local API methods for the frontend:

- `searchTmdb(query: string): Promise<TmdbSearchResult[]>`
- `getTmdbTvSeason(seriesId: number, seasonNumber: number): Promise<TmdbSeasonDetails>`

Search returns a normalized list containing only fields needed for task creation:

- `id`
- `mediaType`: `movie` or `tv`
- `title`
- `originalTitle`
- `year`
- `seasonCount` for TV when available

Season details return:

- `seriesId`
- `seasonNumber`
- `episodes`: array of `episodeNumber`, `name`, and optional `airDate`

Implementation should use TMDB v3 endpoints through the configured base URL:

- `/search/movie`
- `/search/tv`
- `/tv/{series_id}/season/{season_number}`

The local backend should send the configured API key to TMDB. Error responses should be normalized to short user-facing messages in the frontend.

## Add Task Flow

The add-task modal keeps the existing fields:

- Task title
- Category
- Year for movie
- Season for series
- Raw M3U8 textarea

Add a TMDB helper section:

1. User enters a title and searches TMDB.
2. Results show movies and TV series together with media type and year.
3. Selecting a movie fills:
   - `title`
   - `category = movie`
   - `year`
4. Selecting a TV series fills:
   - `title`
   - `category = series`
   - `year`
   - default `season = 1` when no season is already set
5. For TV series, the user can choose a season and load its episode list.

The user can still edit every filled field before submission.

## M3U8 Input and Naming Preview

Keep the textarea as the primary batch input because users commonly paste multiple M3U8 URLs at once.

Below the textarea, show a parsed naming preview when there are non-empty lines. Each row represents one textarea line and includes:

- Row number.
- M3U8 URL or compact URL display.
- Generated title.
- Manual title input.

The manual title input has highest priority. If it is filled, it becomes the subtask title part for that line.

For movie tasks:

- One line defaults to the movie title with year through the existing backend naming behavior.
- Multiple lines keep existing sequence behavior.
- Manual row titles can name parts such as `上`, `下`, `Part 1`, or a custom file name segment.

For series tasks:

- User selects a season and starting episode number.
- Rows map sequentially from the starting episode.
- Generated row titles should use `SxxEyy` so the existing backend parser preserves season-specific directory behavior.
- If TMDB episode names are available, the preview displays them for context, but the generated title sent to the backend remains parser-friendly.
- Manual row title overrides the generated title and can include an explicit season marker such as `S03E05`.

Before submitting, the store rebuilds `rawSubtasks` from the parsed rows:

```text
<url> <final-title>
<url> <final-title>
```

This lets the existing create-task handlers keep their current normalization and download naming semantics.

## Data Flow

1. Settings modal saves `tmdbApiKey` and `tmdbApiBaseUrl` through the existing settings API.
2. Add-task modal requests TMDB search through `api.ts`.
3. HTTP/Tauri runtime reads settings and queries TMDB.
4. User selects a TMDB result and optionally a TV season.
5. Frontend computes naming preview from textarea lines and selected metadata.
6. On submit, frontend rebuilds `rawSubtasks` with final row titles.
7. Existing `submitNewTask` continues to probe master playlists and send `streamSelections`.
8. Existing HTTP/Tauri create-task logic creates parent and subtask records.

## Error Handling

- Missing API key: disable search and show a concise message asking the user to configure TMDB settings.
- Empty query: do not call the backend.
- TMDB 401/403: show an authentication error.
- TMDB network or invalid base URL errors: show a connection/configuration error.
- No search results: show an empty state, not an alert.
- Season details unavailable: leave series metadata selected but keep manual naming available.

TMDB failure must never block manual task creation.

## Testing

Add focused tests where practical:

- Frontend unit coverage for rebuilding `rawSubtasks` from parsed rows and manual title overrides, if the project test setup supports it.
- Rust tests for local TMDB response normalization can be added around pure parsing helpers.
- Build validation:
  - `cargo test -p m3u8-server`
  - `cargo test -p m3u8-core` if shared Rust helpers are added
  - `pnpm --filter @m3u8-harvester/web build`

## Open Decisions Resolved

- TMDB configuration is stored in settings, not environment-only configuration.
- API base URL is configurable.
- HTTP and Tauri use local proxy methods instead of frontend direct TMDB calls.
- Movies and TV series are both supported in the first version.
- Textarea batch input stays as the primary M3U8 entry method.
- Structured row editing is added as a preview/editor layer under the textarea.
