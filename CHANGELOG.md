# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses semantic version tags.

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
