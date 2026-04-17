use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use m3u8_core::FolderInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
pub struct ListFilesResponse {
    pub folders: Vec<FolderInfo>,
    #[serde(rename = "downloadPath")]
    pub download_path: String,
}

pub async fn list_files(State(state): State<Arc<AppState>>) -> Json<ListFilesResponse> {
    let folders = state.file_service.list_folders().await.unwrap_or_default();
    let download_path = state.file_service.get_base_path();

    Json(ListFilesResponse {
        folders,
        download_path,
    })
}

pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // id 可能是 "folder/file.mp4"，在 axum 中 Path 提取时需要注意编码
    // 但前端用了 encodeURIComponent，且 axum 默认会解码
    state.file_service.delete_file(&id).await.ok();
    Json(serde_json::json!({ "success": true }))
}

pub async fn delete_folder(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.file_service.delete_folder(&id).await.ok();
    Json(serde_json::json!({ "success": true }))
}

#[derive(Deserialize)]
pub struct RenameRequest {
    #[serde(rename = "newName")]
    pub new_name: String,
}

pub async fn rename_file_or_folder(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<RenameRequest>,
) -> Json<serde_json::Value> {
    state.file_service.rename(&id, &payload.new_name).await.ok();
    Json(serde_json::json!({ "success": true }))
}
