use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::fs::{self, OpenOptions};
use std::path::Path;

fn sqlite_file_path(database_url: &str) -> Option<&str> {
    let raw_path = if let Some(path) = database_url.strip_prefix("sqlite:") {
        path
    } else if let Some(path) = database_url.strip_prefix("file:") {
        path
    } else {
        return None;
    };

    let path_without_query = raw_path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(raw_path);

    if path_without_query.is_empty() || path_without_query == ":memory:" {
        return None;
    }

    Some(path_without_query)
}

pub async fn init_db(database_url: &str) -> Result<SqlitePool> {
    // 确保数据库文件及其父目录在连接前已经存在，避免 SQLite 因路径不存在而启动失败。
    if let Some(path_str) = sqlite_file_path(database_url) {
        if let Some(parent) = Path::new(path_str).parent() {
            fs::create_dir_all(parent)?;
        }

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path_str)?;
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
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN estimated_size INTEGER")
        .execute(&pool)
        .await;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::sqlite_file_path;

    #[test]
    fn extracts_relative_sqlite_path() {
        assert_eq!(
            sqlite_file_path("sqlite:storage/db/app.db"),
            Some("storage/db/app.db")
        );
    }

    #[test]
    fn extracts_absolute_sqlite_path_with_query() {
        assert_eq!(
            sqlite_file_path("sqlite:/app/storage/db/app.db?mode=rwc"),
            Some("/app/storage/db/app.db")
        );
    }

    #[test]
    fn skips_memory_database() {
        assert_eq!(sqlite_file_path("sqlite::memory:"), None);
    }
}
