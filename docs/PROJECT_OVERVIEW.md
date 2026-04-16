# Project Overview

## Summary

`m3u8-harvester` 是一个用于下载和合并 M3U8 视频流的全栈项目，当前形态是：

- 前端：`apps/web`，Vue 3 + Pinia + Vite + Tailwind + DaisyUI
- 后端：`apps/server`，Rust + Axum
- 核心能力库：`crates/m3u8-core`，负责数据库、任务管理、M3U8 解析、下载与文件管理

仓库正处于一次明显的后端迁移阶段：旧的 TypeScript 服务端基本已删除，新的 Rust 服务端已经接管主要 API。

## Monorepo Structure

```text
.
├── apps/
│   ├── server/              # Axum HTTP server，静态资源托管，路由入口
│   └── web/                 # Vue 前端
├── crates/
│   └── m3u8-core/          # Rust 核心库
├── Dockerfile              # 单镜像部署
├── docker-compose.yml
├── Cargo.toml              # Rust workspace
├── package.json            # 前端与 Rust 的统一开发命令
└── README.md
```

## Runtime Flow

### 1. Frontend

- 入口：`apps/web/src/main.ts`
- 根组件：`apps/web/src/App.vue`
- 全局状态：`apps/web/src/stores/appStore.ts`

前端通过 `appStore` 调用后端 API：

- `GET /api/tasks`
- `POST /api/tasks`
- `POST /api/tasks/:id/pause`
- `POST /api/tasks/:id/resume`
- `POST /api/tasks/:id/retry`
- `GET /api/settings`
- `POST /api/settings`
- `GET /api/files`
- `DELETE /api/files/:id`
- `DELETE /api/files/folders/:id`
- `POST /api/files/:id/rename`

开发模式下，前端会把 API 指向 `http://localhost:6868`；生产环境下走同源地址。

### 2. Backend

入口文件：`apps/server/src/main.rs`

后端职责：

- 初始化日志
- 初始化 SQLite
- 创建 `TaskService`、`SettingService`、`FileService`、`DownloadService`
- 注册 Axum 路由
- 托管前端构建产物目录 `dist`

也就是说，生产部署是“单服务”模式：Rust 后端同时提供 API 和前端静态页面。

### 3. Core Download Pipeline

主流程位于 `apps/server/src/services/download_service.rs`：

1. 从数据库读取任务
2. 将任务状态置为 `parsing`
3. 调用 `parse_m3u8` 解析播放列表并得到分片 URL
4. 更新分片总数和预估大小
5. 调用 `Downloader::start_download` 并发下载 `.ts` 分片
6. 监听下载进度，回写任务百分比与已完成分片数
7. 下载完成后调用 `VideoMerger::merge`
8. 合并成功后标记任务完成并清理临时目录

## Core Modules

### `crates/m3u8-core/src/db/mod.rs`

- 初始化 SQLite 连接
- 自动创建 `tasks`、`settings` 两张表
- 用很轻量的方式处理缺失字段补充

### `crates/m3u8-core/src/services/task_service.rs`

负责：

- 父任务/子任务创建
- 查询任务树
- 更新状态、进度、分片统计、输出路径
- 暂停、恢复、重试、删除任务
- 依据子任务状态回推父任务状态

当前任务模型是“父任务 + 子任务”结构：

- 父任务：表示一个合集、剧集组、影片组
- 子任务：真正绑定 `m3u8_url` 的下载单元

### `crates/m3u8-core/src/core/downloader.rs`

负责：

- 基于 `reqwest` 并发下载分片
- 用 `Semaphore` 控制并发
- 对单个分片做最多 3 次重试
- 检查已存在分片，提供基础续传能力

### `crates/m3u8-core/src/services/file_service.rs`

负责：

- 列出下载目录中的文件夹和文件
- 删除文件、删除目录
- 重命名文件或目录

下载输出目录默认是：

- `storage/db`：数据库
- `storage/downloads`：输出视频
- `storage/temp/<task_id>`：下载中的分片临时目录

## Data Model

`tasks` 表关键字段：

- `id`
- `parent_id`
- `group_title`
- `title`
- `type`
- `m3u8_url`
- `status`
- `is_pending_overwrite`
- `percentage`
- `total_segments`
- `completed_segments`
- `estimated_size`
- `output_path`
- `created_at`
- `updated_at`

常见状态值：

- `pending`
- `parsing`
- `downloading`
- `merging`
- `completed`
- `failed`
- `paused`
- `skipped`
- `active`（父任务）

## Development Commands

根目录统一命令在 `package.json`：

- `pnpm dev:server`：启动 Rust 后端
- `pnpm dev:web`：启动 Vite 前端
- `pnpm build`：构建前端和后端
- `pnpm lint`：前端 ESLint + Rust clippy
- `pnpm format`：前端 Prettier + Rust fmt

Rust workspace：

- `cargo run -p m3u8-server`
- `cargo build --release -p m3u8-server`

## Current Codebase State

当前代码库有几个重要现实需要记住：

1. 仓库不是“干净基线”。
   `git status` 显示旧的 TypeScript 服务端文件被删除，Rust 服务端和核心库被新增，说明这是迁移过程中的工作树。

2. `target/` 已经进入仓库工作树视野。
   它不是理解业务所必需，后续分析时应默认忽略。

3. 前后端接口存在迁移期不一致。
   例如前端 store 中仍有 `POST /api/tasks/:id/overwrite` 的调用，但当前 Rust 路由里没有对应 handler。

4. 设置字段存在命名不一致。
   后端默认设置是 `concurrency`、`retryCount`、`retryDelay`，前端状态里仍保留 `downloadDir`、`maxConcurrent`、`userAgent`、`proxy` 这种旧模型字段。

5. README 基本反映了新架构方向，但细节未必完全跟当前实现逐项对齐。

## Suggested Reading Order

后续如果要继续改功能，建议按这个顺序读：

1. `README.md`
2. `apps/server/src/main.rs`
3. `apps/server/src/handlers/task_handler.rs`
4. `apps/server/src/services/download_service.rs`
5. `crates/m3u8-core/src/services/task_service.rs`
6. `crates/m3u8-core/src/core/downloader.rs`
7. `apps/web/src/stores/appStore.ts`
8. `apps/web/src/components/features/TaskList.vue`
9. `apps/web/src/components/features/LocalFiles.vue`

## Practical Notes For Future Work

- 如果后续要修功能，优先先确认“前端期望的 API”是否已在 Rust 后端落地。
- 如果要改下载逻辑，主落点是 `m3u8-core`，不是 `apps/server`。
- 如果要改页面交互，先看 `appStore`，因为页面很多行为都依赖 store 的轮询和乐观更新。
- 如果要评估稳定性，优先检查暂停/恢复、重复文件覆盖、失败重试和父子任务状态同步。
