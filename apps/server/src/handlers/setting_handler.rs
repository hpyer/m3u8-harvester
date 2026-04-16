use axum::{
    extract::State,
    Json,
};
use crate::AppState;
use std::sync::Arc;
use std::collections::HashMap;

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> Json<HashMap<String, String>> {
    let settings = state.setting_service.get_all().await.unwrap_or_default();
    Json(settings)
}

pub async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    state.setting_service.update(payload).await.ok();
    Json(serde_json::json!({ "success": true }))
}
