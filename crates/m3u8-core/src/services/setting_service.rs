use crate::models::setting::Setting;
use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub struct SettingService {
    pool: SqlitePool,
}

impl SettingService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> Result<HashMap<String, String>> {
        let rows = sqlx::query_as::<_, Setting>("SELECT * FROM settings")
            .fetch_all(&self.pool)
            .await?;

        let mut settings = HashMap::new();
        for row in rows {
            settings.insert(row.key, row.value);
        }

        // 默认设置
        if !settings.contains_key("concurrency") {
            settings.insert("concurrency".to_string(), "5".to_string());
        }
        if !settings.contains_key("retryCount") {
            settings.insert("retryCount".to_string(), "3".to_string());
        }
        if !settings.contains_key("retryDelay") {
            settings.insert("retryDelay".to_string(), "2000".to_string());
        }
        if !settings.contains_key("userAgent") {
            settings.insert(
                "userAgent".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            );
        }
        if !settings.contains_key("proxy") {
            settings.insert("proxy".to_string(), "".to_string());
        }

        Ok(settings)
    }

    pub async fn update(&self, settings: HashMap<String, String>) -> Result<()> {
        for (key, value) in settings {
            sqlx::query(
                "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
            )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn get_value(&self, key: &str) -> Result<Option<String>> {
        let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(setting.map(|s| s.value))
    }
}
