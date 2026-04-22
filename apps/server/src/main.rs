use axum::{
    routing::{delete, get, post},
    Router,
};
use m3u8_core::{init_db, DownloadService, FileService, SettingService, TaskService};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;

pub struct AppState {
    pub task_service: Arc<TaskService>,
    pub setting_service: Arc<SettingService>,
    pub file_service: Arc<FileService>,
    pub download_service: Arc<DownloadService>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "m3u8_server=info,m3u8_core=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:storage/db/app.db".into());

    let pool = init_db(&database_url)
        .await
        .expect("Failed to initialize database");

    let task_service = Arc::new(TaskService::new(pool.clone()));
    let setting_service = Arc::new(SettingService::new(pool.clone()));

    let storage_path = PathBuf::from(env::var("STORAGE_PATH").unwrap_or_else(|_| "storage".into()));
    let abs_storage_path =
        std::fs::canonicalize(&storage_path).unwrap_or_else(|_| storage_path.clone());
    tracing::info!("Storage directory: {}", abs_storage_path.display());

    // Web/server mode is controlled by STORAGE_PATH. Desktop path selection is Tauri-only.
    let downloads_path = storage_path.join("downloads");

    // 确保下载目录存在
    tokio::fs::create_dir_all(&downloads_path).await.ok();

    let file_service = Arc::new(FileService::new(downloads_path.clone()));
    let download_service = Arc::new(DownloadService::new(
        task_service.clone(),
        setting_service.clone(),
        downloads_path,
        false,
    ));

    let state = Arc::new(AppState {
        task_service,
        setting_service,
        file_service,
        download_service,
    });

    // 静态文件目录
    let static_dir = env::var("STATIC_DIR").unwrap_or_else(|_| "dist".into());
    tracing::info!("Serving static files from: {}", static_dir);

    let app = Router::new()
        // API routes
        .route(
            "/api/tasks",
            get(handlers::task_handler::get_tasks)
                .post(handlers::task_handler::create_task)
                .delete(handlers::task_handler::delete_completed_tasks),
        )
        .route(
            "/api/tasks/:id",
            get(handlers::task_handler::get_task).delete(handlers::task_handler::delete_task),
        )
        .route(
            "/api/tasks/:id/retry",
            post(handlers::task_handler::retry_task),
        )
        .route(
            "/api/tasks/:id/pause",
            post(handlers::task_handler::pause_task),
        )
        .route(
            "/api/tasks/:id/resume",
            post(handlers::task_handler::resume_task),
        )
        .route(
            "/api/tasks/:id/overwrite",
            post(handlers::task_handler::respond_overwrite),
        )
        .route(
            "/api/settings",
            get(handlers::setting_handler::get_settings)
                .post(handlers::setting_handler::update_settings),
        )
        .route(
            "/api/meta/version",
            get(handlers::meta_handler::get_app_version),
        )
        .route("/api/files", get(handlers::file_handler::list_files))
        .route(
            "/api/files/:id",
            delete(handlers::file_handler::delete_file),
        )
        .route(
            "/api/files/folders/:id",
            delete(handlers::file_handler::delete_folder),
        )
        .route(
            "/api/files/:id/rename",
            post(handlers::file_handler::rename_file_or_folder),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
        // Static files and fallback for SPA
        .fallback_service(
            ServeDir::new(&static_dir)
                .not_found_service(ServeFile::new(format!("{}/index.html", static_dir))),
        );

    let port = env::var("PORT").unwrap_or_else(|_| "6868".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("Server running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await.unwrap();
}
