# --- Stage 1: Frontend Builder ---
FROM node:20-alpine AS frontend-builder
WORKDIR /app
RUN corepack enable && corepack prepare pnpm@latest --activate
COPY . .
RUN pnpm install --frozen-lockfile
RUN pnpm --filter @m3u8-harvester/web build

# --- Stage 2: Backend Builder ---
FROM rust:alpine AS backend-builder
WORKDIR /app
RUN apk add --no-cache musl-dev
COPY . .
# 更新依赖并编译二进制文件
RUN cargo update && cargo build --release -p m3u8-server

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
    DATABASE_URL="sqlite:/app/storage/db/app.db" \
    STATIC_DIR=/app/dist

# 复制产物
COPY --from=backend-builder /app/target/release/m3u8-server ./m3u8-server
COPY --from=frontend-builder /app/apps/web/dist ./dist

# 创建必要的目录
RUN mkdir -p /app/storage/db /app/storage/downloads && \
    chmod -R 777 /app/storage

EXPOSE 6868

CMD ["./m3u8-server"]
