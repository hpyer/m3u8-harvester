# AGENTS.md

## Purpose

This repository is a monorepo for `m3u8-harvester`, a local-first M3U8 download and merge tool.

- Frontend: Vue 3 + TypeScript + Vite + Tailwind CSS + DaisyUI
- Backend: Rust + Axum
- Desktop: Tauri 2 app in `apps/desktop`, reusing the Vue frontend
- Core domain: Rust library crate in `crates/m3u8-core`
- Runtime dependencies: SQLite and FFmpeg

This file gives coding agents the minimum project-specific context needed to make correct changes quickly.

## Repository Layout

- `apps/server`
  - Rust HTTP server entrypoint
  - Axum handlers and server-side orchestration
  - Uses `m3u8-core` services for task, settings, file, and download workflows
- `apps/desktop`
  - Tauri 2 desktop app
  - Reuses `apps/web` as its frontend
  - Exposes local commands in `src-tauri/src/main.rs` and directly wires `m3u8-core` services
- `crates/m3u8-core`
  - Shared domain crate
  - Database models and services
  - File tree and download workflow logic
  - Downloader, merger, and M3U8 parsing utilities
- `apps/web`
  - Vue 3 SPA
  - Task management UI and local file browser
- `storage`
  - Runtime data directory
  - `storage/db`: SQLite database
  - `storage/downloads`: merged video output
  - `storage/temp`: in-progress download artifacts
- `.husky`
  - Git hooks
  - `pre-commit` runs `lint-staged`
  - `commit-msg` runs `commitlint`

## Key Code Paths

- `apps/server/src/main.rs`
  - server bootstrap
- `apps/server/src/handlers`
  - HTTP handlers for tasks, files, and settings
- `apps/desktop/src-tauri/src/main.rs`
  - Tauri command handlers and desktop service bootstrap
- `crates/m3u8-core/src/services/download_service.rs`
  - task download lifecycle, segment completion handling, merge entry, output path selection
- `crates/m3u8-core/src/services/task_service.rs`
  - task persistence and task-related domain logic
- `crates/m3u8-core/src/services/file_service.rs`
  - local file tree generation for the web UI
- `crates/m3u8-core/src/core/downloader.rs`
  - segment download engine
- `crates/m3u8-core/src/utils/merger.rs`
  - FFmpeg merge integration
- `apps/web/src/components/features/TaskList.vue`
  - main task list UI
- `apps/web/src/components/features/LocalFiles.vue`
  - local files page
- `apps/web/src/components/features/LocalFolderTree.vue`
  - recursive folder tree rendering
- `apps/web/src/services/api.ts`
  - frontend API client

## Setup Assumptions

- Rust toolchain is installed.
- Node.js `>=20` is expected.
- `pnpm` is the package manager.
- Tauri 2 platform dependencies are required for desktop development and packaging.
- `ffmpeg` must be available on `PATH` for merge-related flows to work.

Optional local `.env` values from the project README:

```env
PORT=6868
STORAGE_PATH=storage
DATABASE_URL=sqlite:storage/db/app.db
RUST_LOG=info
```

## Canonical Commands

Use the root scripts unless there is a good reason not to.

### Development

```bash
pnpm dev:web
pnpm dev:server
pnpm dev:desktop
```

### Build

```bash
pnpm build:web
pnpm build:server
pnpm build:desktop
pnpm build
```

### Lint and Format

```bash
pnpm lint:web
pnpm lint:rust
pnpm lint

pnpm format:web
pnpm format:rust
pnpm format
```

### Rust Test

```bash
cargo test -p m3u8-server
cargo test -p m3u8-core
cargo test --workspace
```

### Targeted Checks

```bash
pnpm --filter @m3u8-harvester/web build
cargo fmt --all
cargo clippy --workspace -- -D warnings
```

## Working Rules

### 1. Keep changes in the correct layer

- Put HTTP-only orchestration in `apps/server`.
- Put desktop-only Tauri command wiring in `apps/desktop/src-tauri`.
- Put reusable domain logic in `crates/m3u8-core`.
- Do not move business logic into Vue components if it belongs in backend or shared Rust services.

### 2. Preserve download path semantics

The project already has logic for series-style season directories.

- Series content should keep `Sxx` directory levels.
- If a subtask title or explicit filename contains a season marker like `S03E05`, that season wins over the parent task default season.
- Do not flatten season folders in backend file APIs or in the web UI unless the feature explicitly asks for it.

### 3. Be careful with runtime data

- `storage/` is runtime state, not source code.
- Do not casually delete or rewrite files under `storage/db`, `storage/downloads`, or `storage/temp`.
- If a change touches path generation, confirm it does not break existing download directory conventions.

### 4. Respect current UI patterns

- The web app uses Vue 3 Composition API with TypeScript.
- Existing UI is built with Tailwind and DaisyUI. Stay consistent unless the task asks for a redesign.
- Recent file-browser behavior depends on hierarchical folder rendering. Keep folder recursion intact.

### 5. Keep hooks green

Pre-commit is active through Husky and `lint-staged`.

- `*.rs` staged files run:
  - `cargo fmt --`
  - `cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings`
- `*.ts`, `*.js`, `*.vue` staged files run:
  - `eslint --fix`
  - `prettier --write`
- commit messages are checked by `commitlint`

Do not assume a commit will pass if local format or lint has not been run.

## Change Guidance

### When changing backend download behavior

Also inspect:

- `crates/m3u8-core/src/services/download_service.rs`
- `apps/server/src/handlers/task_handler.rs`
- `apps/desktop/src-tauri/src/main.rs`
- `crates/m3u8-core/src/models/task.rs`
- `crates/m3u8-core/src/services/task_service.rs`
- `crates/m3u8-core/src/core/downloader.rs`
- `crates/m3u8-core/src/utils/merger.rs`

Validate with:

```bash
cargo test -p m3u8-server
```

If the change affects shared logic, also run:

```bash
cargo test -p m3u8-core
```

If the change affects desktop command wiring or packaging, also run:

```bash
pnpm --filter @m3u8-harvester/desktop build
```

### When changing file tree or local file display

Backend and frontend must stay aligned.

Check together:

- `crates/m3u8-core/src/services/file_service.rs`
- `apps/web/src/types/app.ts`
- `apps/web/src/services/api.ts`
- `apps/web/src/components/features/LocalFiles.vue`
- `apps/web/src/components/features/LocalFolderTree.vue`

Validate with:

```bash
cargo test -p m3u8-server
pnpm --filter @m3u8-harvester/web build
```

### When changing web UI only

At minimum run:

```bash
pnpm --filter @m3u8-harvester/web build
```

If types or payload shape changed, re-check backend endpoints too.

## Known Project Conventions

- Use `pnpm`, not `npm`.
- Prefer root scripts for routine tasks.
- Prefer targeted validation over full rebuilds when the change scope is small.
- The web frontend must work in both HTTP mode and Tauri mode; keep `apps/web/src/services/api.ts` aligned with both Axum endpoints and Tauri commands when API shape changes.
- Keep responses and labels in Chinese if matching surrounding UI/content.
- Avoid unnecessary renames or structural refactors in Rust modules unless the task requires them.

## Good Final Verification Sets

Choose the smallest set that matches the change:

### Rust-only

```bash
cargo fmt --all
cargo test -p m3u8-server
```

### Web-only

```bash
pnpm --filter @m3u8-harvester/web build
```

### Desktop-only

```bash
cargo fmt --all
pnpm --filter @m3u8-harvester/desktop build
```

### Cross-layer task, file tree, or API change

```bash
cargo fmt --all
cargo test -p m3u8-server
pnpm --filter @m3u8-harvester/desktop build
pnpm --filter @m3u8-harvester/web build
```

## Avoid

- Do not edit generated build output under `apps/web/dist`.
- Do not commit runtime files from `storage/`.
- Do not bypass Husky/commit hooks unless explicitly instructed.
- Do not assume download path logic is trivial; season and filename rules matter here.
