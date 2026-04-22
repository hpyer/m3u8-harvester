# M3U8 Harvester

轻量级 M3U8 视频流下载合并工具，支持 Web 服务端部署和 Tauri 桌面版，提供并发下载、实时进度监控及自动 FFmpeg 合并。

## 核心特性

- ✅ **M3U8 解析**: 自动补全相对路径，处理重定向。
- ✅ **并发下载**: 后端受控并发（默认 5 线程），无内存溢出的流式下载。
- ✅ **实时进度**: 自动刷新机制，实时展示下载百分比及状态变化。
- ✅ **FFmpeg 合并**: 无损快速合并，支持用户自定义文件名。
- ✅ **任务管理**: 完整的 CRUD，支持物理文件清理。
- ✅ **桌面版**: 基于 Tauri 2 复用同一套 Vue UI，可直接调用本地 Rust 服务能力。
- ✅ **单镜像部署**: 前后端合一，轻量高效。

## 技术栈

- **Frontend:** Vue 3 (Composition API), TypeScript, Vite, Tailwind CSS, DaisyUI
- **Backend:** Rust, Axum, SQLx (SQLite), Tokio (Async runtime)
- **Desktop:** Tauri 2, Rust, shared Vue frontend
- **Engine:** Reqwest (Download), FFmpeg (Merge)

## 快速开始 (Docker)

推荐直接使用 GitHub Container Registry 中已构建好的镜像，镜像内已包含 Rust 编译产物及 FFmpeg 依赖。

### 使用 Docker Run

```bash
docker pull ghcr.io/hpyer/m3u8-harvester:latest

# 建议先准备持久化目录
mkdir -p ./storage/db ./storage/downloads ./storage/temp

# 运行镜像
docker run -d \
  -p 6868:6868 \
  -v $(pwd)/storage:/app/storage \
  -e DATABASE_URL='sqlite:/app/storage/db/app.db?mode=rwc' \
  --name m3u8-downloader \
  ghcr.io/hpyer/m3u8-harvester:latest
```

### 使用 Docker Compose

如果你更喜欢使用 `docker-compose`，可以创建 `docker-compose.yml` 文件或直接使用项目根目录下的文件：

```yaml
services:
  m3u8-harvester:
    image: ghcr.io/hpyer/m3u8-harvester:latest
    container_name: m3u8-harvester
    ports:
      - '6868:6868'
    volumes:
      - ./storage:/app/storage
    environment:
      - RUST_LOG=info
      - DATABASE_URL=sqlite:/app/storage/db/app.db?mode=rwc
    restart: unless-stopped
```

运行：

```bash
mkdir -p ./storage/db ./storage/downloads ./storage/temp
docker-compose up -d
```

### GitHub Actions 自动构建

仓库已提供 GitHub Actions workflow：

- 文件：`.github/workflows/docker-publish.yml`
- 推送 `main` 分支时自动构建并发布 `latest`
- 推送 `v*` 标签时自动发布对应版本标签
- `pull_request` 到 `main` 时只执行构建，不推送镜像

默认发布地址：

```text
ghcr.io/hpyer/m3u8-harvester
```

如果仓库是首次发布 GHCR 包，请到 GitHub 的 `Packages` 设置中确认该镜像已设为公开可拉取。

### 挂载说明

为了保证数据持久化和方便导出视频，请务必挂载 `/app/storage` 目录：

- `/app/storage/db`: SQLite 数据库
- `/app/storage/downloads`: 下载完成后的 MP4 文件
- `/app/storage/temp`: 下载中的临时分片与中间文件

如果宿主机上对应目录不存在，建议先手动创建：

```bash
mkdir -p ./storage/db ./storage/downloads ./storage/temp
```

### 本地自行构建镜像

如果你不想使用 GHCR，也可以继续本地构建：

```bash
docker build -t m3u8-harvester .
mkdir -p ./storage/db ./storage/downloads ./storage/temp
docker run -d \
  -p 6868:6868 \
  -v $(pwd)/storage:/app/storage \
  -e DATABASE_URL='sqlite:/app/storage/db/app.db?mode=rwc' \
  --name m3u8-downloader \
  m3u8-harvester
```

### Docker 常见问题

如果容器启动后立即退出，并出现以下错误：

```text
Failed to initialize database: error returned from database: (code: 14) unable to open database file
```

通常需要检查这几项：

- 宿主机挂载的 `./storage` 是否存在，且 Docker 有权限读写。
- 是否使用了较旧的镜像版本。重新 `docker pull` 或本地 `docker build` 后再启动。
- `DATABASE_URL` 是否指向容器内可写路径。推荐使用 `sqlite:/app/storage/db/app.db?mode=rwc`。

可用下面命令快速重建并启动：

```bash
docker rm -f m3u8-harvester 2>/dev/null || true
docker build -t m3u8-harvester .
mkdir -p ./storage/db ./storage/downloads ./storage/temp
docker run -d \
  -p 6868:6868 \
  -v $(pwd)/storage:/app/storage \
  -e DATABASE_URL='sqlite:/app/storage/db/app.db?mode=rwc' \
  --name m3u8-harvester \
  m3u8-harvester
```

## 本地开发指南 (不使用 Docker)

如果你是开发者，想要在本地调试代码，请按照以下步骤操作：

### 1. 准备环境

- **Rust**: 建议版本 v1.75+
- **Node.js**: 建议版本 v20+ (用于前端构建)
- **pnpm**: 用于前端依赖与根脚本管理
- **Tauri 桌面依赖**: 如需调试或构建桌面版，请先准备对应平台的 Tauri 2 系统依赖。
- **FFmpeg**: 确保你的电脑已安装 `ffmpeg` 并已添加到系统环境变量 (Path) 中。
  - macOS: `brew install ffmpeg`
  - Windows: [下载并配置环境变量](https://ffmpeg.org/download.html)
  - Ubuntu/Debian: `sudo apt install ffmpeg`

安装依赖：

```bash
pnpm install
```

### 2. 运行开发模式

你可以使用根目录配置的快捷脚本同时启动：

- **启动后端**: `pnpm dev:server` (内部执行 `cargo run`)
- **启动前端**: `pnpm dev:web` (内部执行 `vite`)
- **启动桌面版**: `pnpm dev:desktop` (内部执行 `tauri dev`，并自动启动 Web 前端)

访问 `http://localhost:5173` 即可进行实时调试。

桌面版会复用 `apps/web` 的前端界面，但在 Tauri 环境中通过 `invoke` 直接调用本地命令，不依赖 Axum HTTP 服务。桌面应用的数据库位于系统应用数据目录，默认下载目录优先使用系统下载目录；下载设置中选择的路径会作为后续下载根目录。

### 3. 环境变量 (.env)

你可以直接基于根目录下的 `.env.example` 创建 `.env`：

```bash
cp .env.example .env
```

默认内容如下：

```env
PORT=6868
STORAGE_PATH=storage
DATABASE_URL=sqlite:storage/db/app.db
RUST_LOG=info
```

### 4. 仓库结构

```text
apps/
  server/       # Axum 服务端，处理 HTTP 接口与服务端启动编排
  web/          # Vue 3 前端，同时服务 Web 与桌面版
  desktop/      # Tauri 2 桌面壳，直接调用 m3u8-core 服务
crates/
  m3u8-core/ # 核心领域逻辑、数据库服务、下载器、文件树与合并逻辑
storage/
  db/       # SQLite 数据库
  downloads/# 已合并输出的视频文件
  temp/     # 下载中的临时文件
```

### 5. 常用命令

开发：

```bash
pnpm dev:web
pnpm dev:server
pnpm dev:desktop
```

构建：

```bash
pnpm build:web
pnpm build:server
pnpm build:desktop
pnpm build
```

检查与格式化：

```bash
pnpm lint:web
pnpm lint:rust
pnpm lint

pnpm format:web
pnpm format:rust
pnpm format
```

测试：

```bash
cargo test -p m3u8-server
cargo test -p m3u8-core
cargo test --workspace
```

常见最小验证集：

```bash
pnpm --filter @m3u8-harvester/web build
cargo fmt --all
cargo clippy --workspace -- -D warnings
```

### 6. 提交约束

项目已启用 Husky：

- `pre-commit` 会执行 `lint-staged`
- `commit-msg` 会执行 `commitlint`

其中：

- `*.ts`、`*.js`、`*.vue` 会自动执行 `eslint --fix` 和 `prettier --write`
- `*.rs` 会自动执行 `cargo fmt --` 和 `cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings`

建议在提交前先自行跑一遍对应命令，避免在 hook 阶段才发现问题。

### 7. 开发注意事项

- `storage/` 是运行时目录，不是源码目录，不要随意提交其中内容。
- 剧集/综艺/动漫下载会保留 `Sxx` season 子目录，修改下载路径或文件树接口时不要把该层级拍平。
- 文件列表接口与前端目录树需要保持一致，涉及本地文件展示时通常要同步检查服务端和前端类型。
- 更详细的协作约定见 [AGENTS.md](./AGENTS.md)。

## 免责声明

**本项目仅供个人学习和技术交流使用，下载的视频版权归原作者所有，请勿用于任何商业用途或非法传播。使用者因违规使用带来的法律责任由使用者自行承担。**
