// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use m3u8_core::{
    init_db, DownloadService, FileService, SettingService, Task, TaskService, TaskWithSubtasks,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::{Manager, State};

static RE_YEAR: OnceLock<Regex> = OnceLock::new();
static RE_SEASON: OnceLock<Regex> = OnceLock::new();
static RE_SAFE_TITLE: OnceLock<Regex> = OnceLock::new();
static RE_SEASON_IN_SUB: OnceLock<Regex> = OnceLock::new();
static RE_EP_MATCH: OnceLock<Regex> = OnceLock::new();
static RE_ONLY_NUM: OnceLock<Regex> = OnceLock::new();

fn get_re_year() -> &'static Regex {
    RE_YEAR.get_or_init(|| Regex::new(r"\d{4}").unwrap())
}
fn get_re_season() -> &'static Regex {
    RE_SEASON.get_or_init(|| Regex::new(r"\d+").unwrap())
}
fn get_re_safe_title() -> &'static Regex {
    RE_SAFE_TITLE.get_or_init(|| Regex::new(r#"[\\/:*?"<>|]"#).unwrap())
}
fn get_re_season_in_sub() -> &'static Regex {
    RE_SEASON_IN_SUB.get_or_init(|| Regex::new(r"(?i)(?:S|Season\s*)(\d+)").unwrap())
}
fn get_re_ep_match() -> &'static Regex {
    RE_EP_MATCH.get_or_init(|| Regex::new(r"(?i)(?:E|Episode|第|集\s*)(\d+)").unwrap())
}
fn get_re_only_num() -> &'static Regex {
    RE_ONLY_NUM.get_or_init(|| Regex::new(r"^\d+$").unwrap())
}

pub struct AppState {
    pub task_service: Arc<TaskService>,
    pub setting_service: Arc<SettingService>,
    pub file_service: Arc<FileService>,
    pub download_service: Arc<DownloadService>,
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

#[tauri::command]
async fn get_tasks(state: State<'_, Arc<AppState>>) -> Result<Vec<TaskWithSubtasks>, String> {
    state
        .task_service
        .get_tasks()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_task(state: State<'_, Arc<AppState>>, id: String) -> Result<TaskWithSubtasks, String> {
    state
        .task_service
        .get_task_with_subtasks(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())
}

#[tauri::command]
async fn create_task(
    state: State<'_, Arc<AppState>>,
    payload: CreateTaskRequest,
) -> Result<Task, String> {
    let final_year = payload
        .year
        .as_ref()
        .and_then(|y| get_re_year().find(y).map(|m| m.as_str().to_string()));

    let final_season = payload
        .season
        .as_ref()
        .and_then(|s| get_re_season().find(s).map(|m| m.as_str().to_string()));

    let parent_task = state
        .task_service
        .find_or_create_parent_task(
            payload.title.clone(),
            payload.category.clone(),
            final_year.clone(),
            final_season.clone(),
        )
        .await
        .map_err(|e| e.to_string())?;

    let safe_group_title = get_re_safe_title()
        .replace_all(&payload.title, "_")
        .into_owned();
    let lines: Vec<&str> = payload
        .raw_subtasks
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    for (index, line) in lines.iter().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let url = parts[0];
        let sub_title_raw = parts[1..].join(" ");

        let final_sub_title = if payload.category == "series" {
            let mut season_str = final_season
                .as_ref()
                .map(|s| format!("S{:02}", s.parse::<u32>().unwrap_or(1)))
                .unwrap_or_else(|| "S01".to_string());

            if let Some(caps) = get_re_season_in_sub().captures(&sub_title_raw) {
                if let Some(m) = caps.get(1) {
                    season_str = format!("S{:02}", m.as_str().parse::<u32>().unwrap_or(1));
                }
            }

            let mut episode = String::new();
            if let Some(caps) = get_re_ep_match().captures(&sub_title_raw) {
                if let Some(m) = caps.get(1) {
                    episode = format!("E{:02}", m.as_str().parse::<u32>().unwrap_or(1));
                }
            } else if sub_title_raw.is_empty() || get_re_only_num().is_match(&sub_title_raw) {
                let num = if sub_title_raw.is_empty() {
                    index + 1
                } else {
                    sub_title_raw.parse().unwrap_or(index + 1)
                };
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
            let year_str = final_year
                .as_ref()
                .map(|y| format!(".{}", y))
                .unwrap_or_default();
            let sequence = if lines.len() > 1 {
                format!(".{:02}", index + 1)
            } else {
                "".to_string()
            };
            if !sub_title_raw.is_empty() {
                format!(
                    "{}{}.{}{}",
                    safe_group_title, year_str, sub_title_raw, sequence
                )
            } else {
                format!("{}{}{}", safe_group_title, year_str, sequence)
            }
        } else {
            let sequence = if lines.len() > 1 {
                format!(".{:02}", index + 1)
            } else {
                "".to_string()
            };
            if !sub_title_raw.is_empty() {
                format!("{}.{}{}", safe_group_title, sub_title_raw, sequence)
            } else {
                format!("{}{}", safe_group_title, sequence)
            }
        };

        let sub_task = state
            .task_service
            .create_sub_task(
                parent_task.id.clone(),
                final_sub_title,
                url.to_string(),
                payload.category.clone(),
            )
            .await
            .map_err(|e| e.to_string())?;

        state.download_service.run_task(sub_task.id).await;
    }

    Ok(parent_task)
}

#[tauri::command]
async fn delete_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .task_service
        .delete_task(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_completed_tasks(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .task_service
        .delete_completed_tasks()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn retry_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .task_service
        .retry_task(&id)
        .await
        .map_err(|e| e.to_string())?;
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if let Some(parent_id) = task.parent_id {
            let _ = state.task_service.update_parent_status(&parent_id).await;
        }
    }
    state.download_service.run_task(id).await;
    Ok(())
}

#[tauri::command]
async fn pause_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .task_service
        .pause_task(&id)
        .await
        .map_err(|e| e.to_string())?;
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if let Some(parent_id) = task.parent_id {
            let _ = state.task_service.update_parent_status(&parent_id).await;
        } else {
            let _ = state.task_service.update_parent_status(&id).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn resume_task(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .task_service
        .resume_task(&id)
        .await
        .map_err(|e| e.to_string())?;
    if let Ok(Some(task)) = state.task_service.find_task(&id).await {
        if let Some(parent_id) = task.parent_id.clone() {
            let _ = state.task_service.update_parent_status(&parent_id).await;
            state.download_service.run_task(id).await;
        } else {
            let _ = state.task_service.update_parent_status(&id).await;
            if let Ok(subtasks) = state.task_service.find_subtasks(&id).await {
                for sub in subtasks {
                    if sub.status == "pending" {
                        state.download_service.run_task(sub.id).await;
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn respond_overwrite(
    state: State<'_, Arc<AppState>>,
    id: String,
    overwrite: bool,
) -> Result<(), String> {
    state
        .download_service
        .handle_overwrite_response(id, overwrite)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_settings(
    state: State<'_, Arc<AppState>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut settings = state
        .setting_service
        .get_all()
        .await
        .map_err(|e| e.to_string())?;
    settings.insert(
        "downloadPath".to_string(),
        state.file_service.get_base_path().await,
    );
    Ok(settings)
}

#[tauri::command]
async fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if let Some(new_path) = settings.get("downloadPath") {
        state
            .file_service
            .set_base_path(PathBuf::from(new_path))
            .await;
    }
    state
        .setting_service
        .update(settings)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_files(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let folders = state
        .file_service
        .list_folders()
        .await
        .map_err(|e| e.to_string())?;
    let download_path = state.file_service.get_base_path().await;
    Ok(serde_json::json!({
        "folders": folders,
        "downloadPath": download_path
    }))
}

#[tauri::command]
async fn delete_file(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .file_service
        .delete_file(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_folder(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state
        .file_service
        .delete_folder(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_file_or_folder(
    state: State<'_, Arc<AppState>>,
    id: String,
    new_name: String,
) -> Result<(), String> {
    state
        .file_service
        .rename(&id, &new_name)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct VersionInfo {
    #[serde(rename = "appVersion")]
    app_version: String,
    #[serde(rename = "tauriVersion")]
    tauri_version: String,
}

#[tauri::command]
async fn get_app_version() -> VersionInfo {
    VersionInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // 1. 基础存储路径 (用于数据库)
            let storage_path = app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("storage"));
            std::fs::create_dir_all(&storage_path).ok();

            let db_dir = storage_path.join("db");
            std::fs::create_dir_all(&db_dir).ok();
            let database_url = format!("sqlite:{}/app.db", db_dir.to_string_lossy());

            // 2. 异步初始化服务
            tauri::async_runtime::block_on(async move {
                let pool = init_db(&database_url).await.expect("Failed to init db");
                let setting_service = Arc::new(SettingService::new(pool.clone()));
                let task_service = Arc::new(TaskService::new(pool.clone()));

                // 3. 获取下载路径：数据库设置 -> 系统下载目录 -> 应用数据目录
                let saved_download_path = setting_service
                    .get_value("downloadPath")
                    .await
                    .ok()
                    .flatten();
                let downloads_path = saved_download_path
                    .map(PathBuf::from)
                    .or_else(dirs::download_dir)
                    .unwrap_or_else(|| storage_path.join("downloads"));

                std::fs::create_dir_all(&downloads_path).ok();

                // 4. 临时目录放在下载目录下的 .temp
                let temp_path = downloads_path.join(".temp");
                std::fs::create_dir_all(&temp_path).ok();

                let file_service = Arc::new(FileService::new(downloads_path.clone()));
                let download_service = Arc::new(DownloadService::new(
                    task_service.clone(),
                    setting_service.clone(),
                    downloads_path,
                    true,
                ));

                app_handle.manage(Arc::new(AppState {
                    task_service,
                    setting_service,
                    file_service,
                    download_service,
                }));
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_tasks,
            get_task,
            create_task,
            delete_task,
            delete_completed_tasks,
            retry_task,
            pause_task,
            resume_task,
            respond_overwrite,
            get_settings,
            update_settings,
            list_files,
            delete_file,
            delete_folder,
            rename_file_or_folder,
            get_app_version,
            open_select_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn open_select_directory(app_handle: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app_handle.dialog().file().pick_folder(move |folder| {
        let path = folder.map(|f| f.to_string());
        tx.send(path).unwrap();
    });

    rx.recv().map_err(|e| e.to_string())
}
