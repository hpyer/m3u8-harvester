# M3U8 Harvester

轻量级 M3U8 视频流下载合并工具，支持并发下载、实时进度监控及自动 FFmpeg 合并。

## 技术栈

- **Frontend:** Vue 3 (Composition API), TypeScript, Vite, Tailwind CSS, DaisyUI
- **Backend:** Rust, Axum, SQLx (SQLite), Tokio (Async runtime)
- **Engine:** Reqwest (Download), FFmpeg (Merge)

## 快速开始 (Docker)

推荐使用 Docker 一键部署，镜像内已包含 Rust 编译产物及 FFmpeg 依赖。

### 使用 Docker Run

```bash
docker build -t m3u8-harvester .

# 运行镜像
docker run -d \
  -p 6868:6868 \
  -v $(pwd)/storage:/app/storage \
  --name m3u8-downloader \
  m3u8-harvester
```

### 使用 Docker Compose

如果你更喜欢使用 `docker-compose`，可以创建 `docker-compose.yml` 文件或直接使用项目根目录下的文件：

```yaml
services:
  m3u8-harvester:
    build: .
    container_name: m3u8-harvester
    ports:
      - "6868:6868"
    volumes:
      - ./storage:/app/storage
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

运行：

```bash
docker-compose up -d
```

### 挂载说明

为了保证数据持久化和方便导出视频，请务必挂载 `/app/storage` 目录：

- `/app/storage/db`: SQLite 数据库
- `/app/storage/downloads`: 下载完成后的 MP4 文件

## 本地开发指南 (不使用 Docker)

如果你是开发者，想要在本地调试代码，请按照以下步骤操作：

### 1. 准备环境

- **Rust**: 建议版本 v1.75+
- **Node.js**: 建议版本 v20+ (用于前端构建)
- **FFmpeg**: 确保你的电脑已安装 `ffmpeg` 并已添加到系统环境变量 (Path) 中。
  - macOS: `brew install ffmpeg`
  - Windows: [下载并配置环境变量](https://ffmpeg.org/download.html)
  - Ubuntu/Debian: `sudo apt install ffmpeg`

### 2. 运行开发模式

你可以使用根目录配置的快捷脚本同时启动：

- **启动后端**: `pnpm dev:server` (内部执行 `cargo run`)
- **启动前端**: `pnpm dev:web` (内部执行 `vite`)

访问 `http://localhost:5173` 即可进行实时调试。

### 3. 环境变量 (.env)

你可以在根目录下创建 `.env` 文件：

```env
PORT=6868
STORAGE_PATH=storage
DATABASE_URL=sqlite:storage/db/app.db
RUST_LOG=info
```

## 核心特性

- ✅ **M3U8 解析**: 自动补全相对路径，处理重定向。
- ✅ **并发下载**: 后端受控并发（默认 5 线程），无内存溢出的流式下载。
- ✅ **实时进度**: 自动刷新机制，实时展示下载百分比及状态变化。
- ✅ **FFmpeg 合并**: 无损快速合并，支持用户自定义文件名。
- ✅ **任务管理**: 完整的 CRUD，支持物理文件清理。
- ✅ **单镜像部署**: 前后端合一，轻量高效。

## 免责声明

**本项目仅供个人学习和技术交流使用，下载的视频版权归原作者所有，请勿用于任何商业用途或非法传播。使用者因违规使用带来的法律责任由使用者自行承担。**
