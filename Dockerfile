# syntax=docker/dockerfile:1.7

# --- Stage 1: Frontend Builder ---
FROM --platform=$BUILDPLATFORM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/package.json
COPY apps/desktop/package.json apps/desktop/package.json
RUN corepack enable && corepack install
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && \
    pnpm install --frozen-lockfile
COPY . .
RUN pnpm --filter @m3u8-harvester/web build

# --- Stage 2: Backend Builder ---
FROM rust:alpine AS backend-builder
WORKDIR /app
ARG TARGETPLATFORM
RUN apk add --no-cache musl-dev
ARG APP_DOCKER_IMAGE=ghcr.io/hpyer/m3u8-harvester
ARG APP_DOCKER_VERSION=1.1.0
ARG APP_TAURI_VERSION=
ENV APP_DOCKER_IMAGE=${APP_DOCKER_IMAGE}
ENV APP_DOCKER_VERSION=${APP_DOCKER_VERSION}
ENV APP_TAURI_VERSION=${APP_TAURI_VERSION}
COPY . .
# 使用锁文件保证本地与 CI 构建结果一致
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target-${TARGETPLATFORM},target=/app/target \
    cargo build --release --locked -p m3u8-server && \
    cp target/release/m3u8-server /tmp/m3u8-server

# --- Stage 3: Final Runner ---
FROM alpine:3.19 AS runner
WORKDIR /app

# 安装运行时依赖
RUN apk add --no-cache ffmpeg openssl libgcc libstdc++ && \
    rm -rf /var/cache/apk/*

# 环境变量配置
ENV RUST_LOG=info \
    PORT=6868 \
    STORAGE_PATH=/app/storage \
    DATABASE_URL="sqlite:/app/storage/db/app.db?mode=rwc" \
    STATIC_DIR=/app/dist

# 复制产物
COPY --from=backend-builder /tmp/m3u8-server ./m3u8-server
COPY --from=frontend-builder /app/apps/web/dist ./dist

# 创建必要的目录
RUN mkdir -p /app/storage/db /app/storage/downloads /app/storage/temp && \
    chmod -R 777 /app/storage

EXPOSE 6868

CMD ["./m3u8-server"]
