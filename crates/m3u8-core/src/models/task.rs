use serde::{Deserialize, Serialize};
use sqlx::Type;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Downloading,
    Merging,
    Completed,
    Failed,
    Paused,
    Skipped,
    Active, // 对应父任务活跃状态
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub group_title: Option<String>,
    pub title: String,
    pub r#type: String, // movie, series, show
    pub year: Option<String>,
    pub season: Option<String>,
    pub m3u8_url: Option<String>,
    pub status: String, // SQLx 处理 Enum 有时较繁琐，先用 String 存，业务逻辑用 TaskStatus
    pub is_pending_overwrite: bool,
    pub percentage: f64,
    pub total_segments: i32,
    pub completed_segments: i32,
    pub estimated_size: Option<i64>,
    pub output_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
