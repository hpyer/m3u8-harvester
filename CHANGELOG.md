# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses semantic version tags.

## [1.2.2] - 2026-07-09

### Changed

- Reused the existing Master Playlist selection flow for single-variant playlists so users can confirm the concrete stream even when only one quality is available.
- Added change detection for web and desktop icon generation to avoid rewriting unchanged icon files during local development.

### Fixed

- Fixed M3U8 probing and parsing to use configured User-Agent and proxy settings consistently across HTTP server and Tauri desktop modes.
- Improved M3U8 parse errors with HTTP status, content type, and response preview details for easier diagnosis.
- Fixed direct retry/download of Master Playlists that contain exactly one downloadable variant by automatically following that concrete media playlist.

## [1.2.1] - 2026-05-20

### Changed

- Centralized release version management to the root workspace version in `Cargo.toml`, with desktop release checks and local version bump commands reading from the same source.

### Fixed

- Fixed FFmpeg executable detection by adding fallback lookup paths for common macOS, Linux, and Windows installations when `PATH` lookup fails.

## [1.2.0] - 2026-05-11

### Added

- Added TMDB search integration for task creation: users can search movies and TV series, auto-fill title/category/year, and for TV series auto-generate `SxxExx` naming rows with episode names.
- Added editable naming row preview in the task creation modal with per-row URL, generated title, and manual override support.
- Added TMDB settings fields (API Key, API Base URL) in the settings dialog.
- Added TMDB API proxy endpoints for HTTP server (tmdb_handler) and Tauri desktop commands.
- Added `M3U8NamingRow`, `TmdbSearchResult`, `TmdbSeasonDetails`, and `TmdbEpisode` types to the frontend type system.

### Changed

- Compacted the add task modal layout: smaller header, smaller input controls, reduced padding and spacing.
- Compacted the settings modal layout: removed version info section, smaller inputs, reduced padding and spacing.
- Updated task submission to rebuild subtask strings from editable naming rows instead of raw textarea content.
- Improved Docker build with layer caching for pnpm store, cargo registry, and cargo target directories.
- Bumped server, core, desktop, web fallback, and Docker version metadata to `1.2.0`.

## [1.1.0] - 2026-05-07

### Added

- Added master-playlist probing before task creation so users can choose a concrete HLS variant.
- Added a resolution selection modal with 30-second auto-continue behavior that defaults to the highest-resolution stream.

### Changed

- Updated download flow to support variant streams with separate `EXT-X-MEDIA` audio renditions and merge the selected audio/video pair into one MP4.
- Bumped server, core, desktop, web fallback, Tauri config, and Docker version metadata to `1.1.0`.

## [1.0.2] - 2026-04-22

### Added

- Added GitHub Actions workflow to build desktop installers for Windows, macOS Intel, macOS Apple Silicon, and Linux.
- Added automatic GitHub Release publishing for version tags with normalized desktop asset names and changelog-based release notes.

### Changed

- Enabled Tauri bundle generation for desktop builds.
- Stopped local desktop builds from regenerating icons unless the icon command is run explicitly.
- Bumped server, core, desktop, web fallback, and Docker version metadata to `1.0.2`.

## [1.0.1] - 2026-04-17

### Added

- Added root `.env.example` to align local development defaults with the documented environment setup.
- Added Docker troubleshooting guidance for SQLite initialization failures and explicit storage directory preparation steps.

### Changed

- Updated Docker image defaults and compose examples to use `sqlite:/app/storage/db/app.db?mode=rwc`.
- Created `/app/storage/temp` in the runtime image to match the project's expected storage layout.
- Bumped server, core, web, and Docker version metadata to `1.0.1`.

### Fixed

- Fixed SQLite initialization to create the database file before opening the connection.
- Fixed SQLite path extraction for file URLs that include query parameters such as `?mode=rwc`.

## [1.0.0] - 2026-04-17

### Added

- Added GitHub Actions workflow to build and publish Docker images to `ghcr.io/hpyer/m3u8-harvester`.
- Added app version metadata API for server, web, docker, and future tauri versions.
- Added dedicated version display in the web settings modal and footer.
- Added root `AGENTS.md` for project-specific agent guidance.

### Changed

- Updated Docker deployment documentation to use GHCR as the default image source.
- Split runtime download settings from build/version metadata instead of mixing them in one settings model.
- Standardized project release versions to start from `1.0.0`.
- Improved local file tree rendering to preserve season directory hierarchy such as `S01` and `S02`.

### Fixed

- Fixed completed segment downloads not entering merge immediately after download completion.
- Fixed series output paths so downloads are stored under season subdirectories.
- Fixed season directory resolution to prefer subtask filename/title season markers over parent default season.
- Fixed Husky hook activation and adjusted Rust `pre-commit` clippy execution for `lint-staged`.
