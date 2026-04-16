use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use serde::Deserialize;
use crate::AppState;
use m3u8_core::{Task, TaskWithSubtasks};
use std::sync::Arc;
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref RE_YEAR: Regex = Regex::new(r"\d{4}").unwrap();
    static ref RE_SEASON: Regex = Regex::new(r"\d+").unwrap();
    static ref RE_SAFE_TITLE: Regex = Regex::new(r#"[\\/:*?"<>|]"#).unwrap();
    static ref RE_SEASON_IN_SUB: Regex = Regex::new(r"(?i)(?:S|Season\s*)(\d+)").unwrap();
    static ref RE_EP_MATCH: Regex = Regex::new(r"(?i)(?:E|Episode|第|集\s*)(\d+)").unwrap();
    static ref RE_ONLY_NUM: Regex = Regex::new(r"^\d+$").unwrap();
}

pub async fn get_tasks(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<TaskWithSubtasks>> {
    let tasks = state.task_service.get_tasks().await.unwrap_or_default();
    Json(tasks)
}

pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TaskWithSubtasks>, StatusCode> {
    match state.task_service.get_task_with_subtasks(&id).await {
        Ok(Some(task)) => Ok(Json(task)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub category: String,
    pub year: Option<String>,
    pub season: Option<String>,
    #[serde(rename = "rawSubtasks")]
    pub raw_subtasks: String,
}

#[derive(Deserialize)]
pub struct OverwriteResponse {
    pub overwrite: bool,
}

pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTaskRequest>,
) -> Json<Task> {
    // 自动优化修复年份格式
    let final_year = payload.year.as_ref().and_then(|y| {
        RE_YEAR.find(y).map(|m| m.as_str().to_string())
    });

    // 自动优化修复季号格式
    let final_season = payload.season.as_ref().and_then(|s| {
        RE_SEASON.find(s).map(|m| m.as_str().to_string())
    });

    let parent_task = state.task_service.find_or_create_parent_task(
        payload.title.clone(),
        payload.category.clone(),
        final_year.clone(),
        final_season.clone(),
    ).await.expect("Failed to create parent task");

    let safe_group_title = RE_SAFE_TITLE.replace_all(&payload.title, "_").into_owned();
    let lines: Vec<&str> = payload.raw_subtasks.lines().filter(|l| !l.trim().is_empty()).collect();

    for (index, line) in lines.iter().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }
        
        let url = parts[0];
        let sub_title_raw = parts[1..].join(" ");

        let final_sub_title = if payload.category == "series" {
            let mut season_str = final_season.as_ref()
                .map(|s| format!("S{:02}", s.parse::<u32>().unwrap_or(1)))
                .unwrap_or_else(|| "S01".to_string());

            if let Some(caps) = RE_SEASON_IN_SUB.captures(&sub_title_raw) {
                if let Some(m) = caps.get(1) {
                    season_str = format!("S{:02}", m.as_str().parse::<u32>().unwrap_or(1));
                }
            }

            let mut episode = String::new();
            if let Some(caps) = RE_EP_MATCH.captures(&sub_title_raw) {
                if let Some(m) = caps.get(1) {
                    episode = format!("E{:02}", m.as_str().parse::<u32>().unwrap_or(1));
                }
            } else if sub_title_raw.is_empty() || RE_ONLY_NUM.is_match(&sub_title_raw) {
                let num = if sub_title_raw.is_empty() { index + 1 } else { sub_title_raw.parse().unwrap_or(index + 1) };
                episode = format!("E{:02}", num);
            }

            if !episode.is_empty() {
                format!("{}.{}{}", safe_group_title, season_str, episode)
            } else if !sub_title_raw.is_empty() {
                format!("{}.{}", safe_group_title, sub_title_raw)
            } else {
                format!("{}.{}", safe_group_title, season_str)
            }
        } else if payload.category == "movie" {
            let year_str = final_year.as_ref().map(|y| format!(".{}", y)).unwrap_or_default();
            let sequence = if lines.len() > 1 { format!(".{:02}", index + 1) } else { "".to_string() };
            if !sub_title_raw.is_empty() {
                format!("{}{}.{}{}", safe_group_title, year_str, sub_title_raw, sequence)
            } else {
                format!("{}{}{}", safe_group_title, year_str, sequence)
            }
        } else {
            let sequence = if lines.len() > 1 { format!(".{:02}", index + 1) } else { "".to_string() };
            if !sub_title_raw.is_empty() {
                format!("{}.{}{}", safe_group_title, sub_title_raw, sequence)
            } else {
                format!("{}{}", safe_group_title, sequence)
            }
        };

        let sub_task = state.task_service.create_sub_task(
            parent_task.id.clone(),
            final_sub_title,
            url.to_string(),
            payload.category.clone(),
        ).await.expect("Failed to create subtask");

        // 启动下载任务
        state.download_service.run_task(sub_task.id).await;
    }

    Json(parent_task)
}

pub async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.task_service.delete_task(&id).await.ok();
    Json(serde_json::json!({ "success": true }))
}

pub async fn delete_completed_tasks(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    state.task_service.delete_completed_tasks().await.ok();
    Json(serde_json::json!({ "success": true }))
}

pub async fn retry_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.task_service.retry_task(&id).await.ok();
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if let Some(parent_id) = task.parent_id {
            state.task_service.update_parent_status(&parent_id).await.ok();
        }
    }
    state.download_service.run_task(id).await;
    Json(serde_json::json!({ "success": true }))
}

pub async fn pause_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.task_service.pause_task(&id).await.ok();
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if let Some(parent_id) = task.parent_id {
            state.task_service.update_parent_status(&parent_id).await.ok();
        } else {
            state.task_service.update_parent_status(&id).await.ok();
        }
    }
    Json(serde_json::json!({ "success": true }))
}

pub async fn resume_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.task_service.resume_task(&id).await.ok();
    
    // 如果是子任务，重新启动下载
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if task.parent_id.is_some() {
            if let Some(parent_id) = task.parent_id.clone() {
                state.task_service.update_parent_status(&parent_id).await.ok();
            }
            state.download_service.run_task(id).await;
        } else {
            // 如果是父任务，启动所有待处理或已恢复的子任务
            state.task_service.update_parent_status(&id).await.ok();
            if let Ok(subtasks) = state.task_service.find_subtasks(&id).await {
                for sub in subtasks {
                    if sub.status == "pending" {
                        state.download_service.run_task(sub.id).await;
                    }
                }
            }
        }
    }
    
    Json(serde_json::json!({ "success": true }))
}

pub async fn respond_overwrite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<OverwriteResponse>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .download_service
        .handle_overwrite_response(id, payload.overwrite)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({ "success": true })))
}
