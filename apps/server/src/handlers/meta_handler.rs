use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVersionInfo {
    pub server_version: &'static str,
    pub web_version: &'static str,
    pub docker_image: &'static str,
    pub docker_version: &'static str,
    pub tauri_version: Option<&'static str>,
}

pub async fn get_app_version() -> Json<AppVersionInfo> {
    let tauri_version = option_env!("APP_TAURI_VERSION").and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    });

    Json(AppVersionInfo {
        server_version: env!("CARGO_PKG_VERSION"),
        web_version: env!("APP_WEB_VERSION"),
        docker_image: env!("APP_DOCKER_IMAGE"),
        docker_version: env!("APP_DOCKER_VERSION"),
        tauri_version,
    })
}
