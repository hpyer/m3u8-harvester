---
name: m3u8-harvester-release
description: Release the m3u8-harvester project: prepare its workspace version, Cargo lockfile, changelog, Git tag, GitHub Release, desktop installers, and Docker image. Use only for release, publish, tag, or release-workflow requests in this repository.
---

# m3u8-harvester 发布

只在仓库根目录执行本流程。发布由根目录 `Cargo.toml` 的
`[workspace.package].version` 管理；桌面端、服务端、核心库和 Docker 镜像版本
都从它派生。

## 先确认发布状态

1. 读取 `git status --short`、当前分支、最近标签和 `Cargo.toml` 的版本。
2. 保留与发布无关的改动，尤其不要暂存 `storage/` 或自动生成但与当前发布无关的
   `apps/desktop/src-tauri/gen/schemas/` 文件。
3. 阅读 `CHANGELOG.md` 顶部条目和 `.github/workflows/desktop-release.yml`、
   `.github/workflows/docker-publish.yml`。两个工作流都要求发布标签指向
   `main` 上的提交；桌面发布还要求标签版本与 `Cargo.toml` 完全一致。

## 准备版本

1. 仅在用户明确要求的版本号下更新版本。使用 `pnpm version:set <x.y.z>`；若
   Corepack 无法取得 pnpm，可改用 `node scripts/version.mjs set <x.y.z>`。
2. 在 `CHANGELOG.md` 顶部添加 `## [x.y.z] - YYYY-MM-DD`，说明用户可见的改动。
3. 运行 `cargo check --workspace` 更新 `Cargo.lock`，确认其中
   `m3u8-core`、`m3u8-server`、`m3u8-desktop` 三个包的版本与 `Cargo.toml`
   一致。
4. 必须运行 `cargo check --workspace --locked`。这一步模拟 Docker 发布中的
   `cargo build --release --locked`，避免锁文件未同步。
5. 若发布包含 Web 改动，额外运行
   `pnpm --filter @m3u8-harvester/web build`。无法下载 pnpm 或依赖时，如实报告，
   不要把网络失败误判为代码失败。

## 提交并触发发布

1. 只暂存发布相关文件和已验证的功能改动；通常至少包括 `Cargo.toml`、
   `Cargo.lock`、`CHANGELOG.md`。
2. 提交后先推送 `main`，确认待发布提交已位于远程 `main`。
3. 创建注释标签 `v<x.y.z>`，然后推送该标签。标签触发：
   - `desktop-release.yml`：构建 Windows、macOS Intel、macOS Apple Silicon、Linux
     安装包，并创建 GitHub Release；
   - `docker-publish.yml`：发布 GHCR 多架构镜像及 `latest`、`v<x.y.z>`、
     `<x.y.z>`、`<major>.<minor>` 标签。
4. 发布完成前检查两个 GitHub Actions 工作流；优先用 `gh run`，不可用时查看
   GitHub Actions 页面。报告桌面与 Docker 的状态及链接。

## 失败处理

- **不要因为工作流失败自动增加版本号。** 保留用户指定的发布版本。
- Docker 构建在 `auth.docker.io/token` 发生 `connection reset`、超时或限流时，
  属于 Docker Hub/Runner 的暂时性网络错误。使用 GitHub Actions 的 **Re-run jobs**
  重跑同一工作流；不要为了重试创建新版本或移动标签。
- `cargo ... --locked` 失败且显示内部 `m3u8-*` 包版本不同步时，更新
  `Cargo.lock`，但保持 `Cargo.toml` 的版本不变。若已推送的发布标签必须移动到
  新提交，先告知用户影响并取得确认；已发布的标签不应擅自重写。
- GitHub Release、Docker 发布或桌面构建失败时，读取失败步骤和错误摘要；先区分
  代码、锁文件、凭据、权限和上游网络问题，再决定修复或重试。

## 交付

最终说明版本号、发布提交、标签、验证结果，以及 GitHub Release、桌面安装包和
Docker 镜像的实际状态。若工作流仍在运行，明确说明尚未发布完成，不要声称成功。
