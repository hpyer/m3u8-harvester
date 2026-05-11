pub mod core;
pub mod db;
pub mod models;
pub mod services;
pub mod utils;

pub use core::downloader::{DownloadOptions, DownloadProgress, Downloader};
pub use db::init_db;
pub use models::setting::Setting;
pub use models::task::{Task, TaskStatus};
pub use services::download_service::DownloadService;
pub use services::file_service::{FileInfo, FileService, FolderInfo};
pub use services::setting_service::SettingService;
pub use services::task_service::{TaskService, TaskWithSubtasks};
pub use services::tmdb_service::{
    TmdbEpisode, TmdbMediaType, TmdbSearchResult, TmdbSeasonDetails, TmdbService,
};
pub use utils::m3u8::{
    parse_download_source, parse_m3u8, probe_m3u8, DownloadSource, InitMapInfo, M3U8Info,
    M3U8ProbeResult, M3U8StreamSelection, M3U8VariantOption, SegmentInfo,
};
pub use utils::merger::VideoMerger;
