use sqlx::SqlitePool;
use anyhow::Result;
use uuid::Uuid;
use chrono::Utc;
use crate::models::task::Task;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskWithSubtasks {
    #[serde(flatten)]
    pub task: Task,
    pub subtasks: Vec<Task>,
}

pub struct TaskService {
    pool: SqlitePool,
}

impl TaskService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_tasks(&self) -> Result<Vec<TaskWithSubtasks>> {
        let all_tasks = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let (parent_tasks, subtasks): (Vec<Task>, Vec<Task>) = all_tasks
            .into_iter()
            .partition(|t| t.parent_id.is_none());

        let result = parent_tasks
            .into_iter()
            .map(|parent| {
                let id = parent.id.clone();
                let filtered_subtasks = subtasks
                    .iter()
                    .filter(|t| t.parent_id.as_ref() == Some(&id))
                    .cloned()
                    .collect();
                TaskWithSubtasks {
                    task: parent,
                    subtasks: filtered_subtasks,
                }
            })
            .collect();

        Ok(result)
    }

    pub async fn find_task(&self, id: &str) -> Result<Option<Task>> {
        let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(task)
    }

    pub async fn get_task_with_subtasks(&self, id: &str) -> Result<Option<TaskWithSubtasks>> {
        let task = self.find_task(id).await?;
        match task {
            Some(task) => {
                let subtasks = self.find_subtasks(id).await?;
                Ok(Some(TaskWithSubtasks { task, subtasks }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_subtasks(&self, parent_id: &str) -> Result<Vec<Task>> {
        let subtasks = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE parent_id = ?")
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(subtasks)
    }

    pub async fn create_parent_task(
        &self,
        group_title: Option<String>,
        title: String,
        r#type: String,
        year: Option<String>,
        season: Option<String>,
    ) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        sqlx::query(
            r#"
            INSERT INTO tasks (id, group_title, title, type, year, season, status, percentage, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(group_title.as_ref())
        .bind(&title)
        .bind(&r#type)
        .bind(year.as_ref())
        .bind(season.as_ref())
        .bind("active")
        .bind(0.0)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(self.find_task(&id).await?.unwrap())
    }

    pub async fn find_or_create_parent_task(
        &self,
        title: String,
        category: String,
        year: Option<String>,
        season: Option<String>,
    ) -> Result<Task> {
        let existing = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks WHERE group_title = ? AND parent_id IS NULL LIMIT 1"
        )
        .bind(&title)
        .fetch_optional(&self.pool)
        .await?;

        match existing {
            Some(task) => {
                let now = Utc::now();
                sqlx::query(
                    "UPDATE tasks SET status = 'active', year = ?, season = ?, updated_at = ? WHERE id = ?"
                )
                .bind(year)
                .bind(season)
                .bind(now)
                .bind(&task.id)
                .execute(&self.pool)
                .await?;
                Ok(self.find_task(&task.id).await?.unwrap())
            }
            None => {
                self.create_parent_task(Some(title.clone()), title, category, year, season).await
            }
        }
    }

    pub async fn create_sub_task(
        &self,
        parent_id: String,
        title: String,
        m3u8_url: String,
        r#type: String,
    ) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tasks (id, parent_id, title, m3u8_url, type, status, percentage, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&parent_id)
        .bind(&title)
        .bind(&m3u8_url)
        .bind(&r#type)
        .bind("pending")
        .bind(0.0)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(self.find_task(&id).await?.unwrap())
    }

    pub async fn update_task_status(&self, id: &str, status: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_task_progress(&self, id: &str, percentage: f64) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET percentage = ?, updated_at = ? WHERE id = ?")
            .bind(percentage)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_task_segments(&self, id: &str, total_segments: i32) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET total_segments = ?, updated_at = ? WHERE id = ?")
            .bind(total_segments)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_task_completed_segments(&self, id: &str, completed_segments: i32) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET completed_segments = ?, updated_at = ? WHERE id = ?")
            .bind(completed_segments)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_task_estimated_size(&self, id: &str, estimated_size: u64) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET estimated_size = ?, updated_at = ? WHERE id = ?")
            .bind(estimated_size as i64)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_task_output_path(&self, id: &str, output_path: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET output_path = ?, updated_at = ? WHERE id = ?")
            .bind(output_path)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_pending_overwrite(&self, id: &str, is_pending_overwrite: bool) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET is_pending_overwrite = ?, updated_at = ? WHERE id = ?")
            .bind(is_pending_overwrite)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_task(&self, id: &str) -> Result<()> {
        // 先删除子任务
        sqlx::query("DELETE FROM tasks WHERE parent_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        // 再删除任务本身
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_completed_tasks(&self) -> Result<()> {
        // 删除已完成的任务及其子任务
        sqlx::query("DELETE FROM tasks WHERE id IN (SELECT id FROM tasks WHERE status = 'completed' AND parent_id IS NULL)")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM tasks WHERE parent_id NOT IN (SELECT id FROM tasks)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn retry_task(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE tasks SET status = 'pending', percentage = 0.0, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn pause_task(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        // 如果是父任务，暂停所有进行中的子任务
        sqlx::query("UPDATE tasks SET status = 'paused', updated_at = ? WHERE parent_id = ? AND status IN ('pending', 'downloading', 'parsing', 'merging')")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        // 暂停任务本身
        sqlx::query("UPDATE tasks SET status = 'paused', updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn resume_task(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        // 如果是父任务，恢复所有可重试的子任务
        sqlx::query("UPDATE tasks SET status = 'pending', updated_at = ? WHERE parent_id = ? AND status IN ('paused', 'failed')")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        // 子任务恢复时改为 pending；父任务状态交给汇总逻辑计算
        sqlx::query("UPDATE tasks SET status = 'pending', updated_at = ? WHERE id = ? AND status IN ('paused', 'failed')")
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        self.update_parent_status(id).await.ok();
        Ok(())
    }

    pub async fn update_parent_status(&self, parent_id: &str) -> Result<()> {
        let subtasks = self.find_subtasks(parent_id).await?;
        if subtasks.is_empty() {
            return Ok(());
        }

        let total = subtasks.len() as f64;
        let completed_count = subtasks
            .iter()
            .filter(|t| ["completed", "skipped"].contains(&t.status.as_str()))
            .count();
        let failed_count = subtasks.iter().filter(|t| t.status == "failed").count();
        let paused_count = subtasks.iter().filter(|t| t.status == "paused").count();
        let active_count = subtasks
            .iter()
            .filter(|t| ["downloading", "merging", "pending", "parsing"].contains(&t.status.as_str()))
            .count();

        let total_progress: f64 = subtasks.iter().map(|t| t.percentage).sum();
        let avg_progress = total_progress / total;

        let status = if completed_count == subtasks.len() {
            "completed"
        } else if paused_count > 0 && active_count == 0 {
            "paused"
        } else if failed_count > 0 && active_count == 0 {
            "failed"
        } else {
            "active"
        };

        let now = Utc::now();
        sqlx::query("UPDATE tasks SET percentage = ?, status = ?, updated_at = ? WHERE id = ?")
            .bind(avg_progress)
            .bind(status)
            .bind(now)
            .bind(parent_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TaskService;
    use anyhow::Result;
    use sqlx::{sqlite::SqlitePoolOptions, Executor};

    async fn create_test_service() -> Result<TaskService> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        pool.execute(
            r#"
            CREATE TABLE tasks (
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

            CREATE TABLE settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            "#,
        )
        .await?;
        Ok(TaskService::new(pool))
    }

    async fn create_parent_with_subtasks(service: &TaskService) -> Result<(String, String, String)> {
        let parent = service
            .create_parent_task(Some("Group".into()), "Group".into(), "series".into(), None, None)
            .await?;
        let sub1 = service
            .create_sub_task(parent.id.clone(), "Episode.1".into(), "https://example.com/1.m3u8".into(), "series".into())
            .await?;
        let sub2 = service
            .create_sub_task(parent.id.clone(), "Episode.2".into(), "https://example.com/2.m3u8".into(), "series".into())
            .await?;

        Ok((parent.id, sub1.id, sub2.id))
    }

    #[tokio::test]
    async fn update_parent_status_marks_completed_when_all_subtasks_done() -> Result<()> {
        let service = create_test_service().await?;
        let (parent_id, sub1_id, sub2_id) = create_parent_with_subtasks(&service).await?;

        service.update_task_status(&sub1_id, "completed").await?;
        service.update_task_status(&sub2_id, "skipped").await?;
        service.update_task_progress(&sub1_id, 100.0).await?;
        service.update_task_progress(&sub2_id, 100.0).await?;

        service.update_parent_status(&parent_id).await?;

        let parent = service.find_task(&parent_id).await?.unwrap();
        assert_eq!(parent.status, "completed");
        assert_eq!(parent.percentage, 100.0);
        Ok(())
    }

    #[tokio::test]
    async fn update_parent_status_marks_paused_when_all_active_work_is_paused() -> Result<()> {
        let service = create_test_service().await?;
        let (parent_id, sub1_id, sub2_id) = create_parent_with_subtasks(&service).await?;

        service.update_task_status(&sub1_id, "paused").await?;
        service.update_task_status(&sub2_id, "paused").await?;

        service.update_parent_status(&parent_id).await?;

        let parent = service.find_task(&parent_id).await?.unwrap();
        assert_eq!(parent.status, "paused");
        Ok(())
    }

    #[tokio::test]
    async fn update_parent_status_marks_failed_when_only_failures_remain() -> Result<()> {
        let service = create_test_service().await?;
        let (parent_id, sub1_id, sub2_id) = create_parent_with_subtasks(&service).await?;

        service.update_task_status(&sub1_id, "failed").await?;
        service.update_task_status(&sub2_id, "completed").await?;
        service.update_task_progress(&sub2_id, 100.0).await?;

        service.update_parent_status(&parent_id).await?;

        let parent = service.find_task(&parent_id).await?.unwrap();
        assert_eq!(parent.status, "failed");
        Ok(())
    }

    #[tokio::test]
    async fn resume_task_only_requeues_paused_and_failed_subtasks() -> Result<()> {
        let service = create_test_service().await?;
        let (parent_id, sub1_id, sub2_id) = create_parent_with_subtasks(&service).await?;
        let sub3 = service
            .create_sub_task(parent_id.clone(), "Episode.3".into(), "https://example.com/3.m3u8".into(), "series".into())
            .await?;

        service.update_task_status(&sub1_id, "paused").await?;
        service.update_task_status(&sub2_id, "failed").await?;
        service.update_task_status(&sub3.id, "completed").await?;

        service.resume_task(&parent_id).await?;

        assert_eq!(service.find_task(&sub1_id).await?.unwrap().status, "pending");
        assert_eq!(service.find_task(&sub2_id).await?.unwrap().status, "pending");
        assert_eq!(service.find_task(&sub3.id).await?.unwrap().status, "completed");
        Ok(())
    }

    #[tokio::test]
    async fn pause_task_only_pauses_runnable_subtasks() -> Result<()> {
        let service = create_test_service().await?;
        let (parent_id, sub1_id, sub2_id) = create_parent_with_subtasks(&service).await?;

        service.update_task_status(&sub1_id, "downloading").await?;
        service.update_task_status(&sub2_id, "completed").await?;

        service.pause_task(&parent_id).await?;

        assert_eq!(service.find_task(&sub1_id).await?.unwrap().status, "paused");
        assert_eq!(service.find_task(&sub2_id).await?.unwrap().status, "completed");
        Ok(())
    }
}
