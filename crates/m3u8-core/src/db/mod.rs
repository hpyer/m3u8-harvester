use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use anyhow::Result;
use std::fs;
use std::path::Path;

pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // 确保数据库文件所在目录存在
    if let Some(path_str) = database_url.strip_prefix("sqlite:") {
        if let Some(parent) = Path::new(path_str).parent() {
            fs::create_dir_all(parent)?;
        }
    } else if let Some(path_str) = database_url.strip_prefix("file:") {
        if let Some(parent) = Path::new(path_str).parent() {
            fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    // 手动执行基础迁移
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL,
            parent_id TEXT,
            group_title TEXT,
            title TEXT NOT NULL,
            type TEXT NOT NULL,
            year TEXT,
            season TEXT,
            m3u8_url TEXT,
            status TEXT NOT NULL,
            is_pending_overwrite BOOLEAN NOT NULL DEFAULT 0,
            percentage REAL NOT NULL DEFAULT 0,
            total_segments INTEGER NOT NULL DEFAULT 0,
            completed_segments INTEGER NOT NULL DEFAULT 0,
            estimated_size INTEGER,
            output_path TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // 尝试添加 missing columns (简单的迁移逻辑)
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN estimated_size INTEGER").execute(&pool).await;

    Ok(pool)
}
