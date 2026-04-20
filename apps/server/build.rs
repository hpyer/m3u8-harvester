use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    let root_package_path = manifest_dir.join("../../package.json");

    println!("cargo:rerun-if-changed={}", root_package_path.display());

    let app_version = fs::read_to_string(&root_package_path)
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .and_then(|json| {
            json.get("version")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unknown".to_string());

    let docker_image =
        env::var("APP_DOCKER_IMAGE").unwrap_or_else(|_| "ghcr.io/hpyer/m3u8-harvester".to_string());
    let docker_version = env::var("APP_DOCKER_VERSION").unwrap_or_else(|_| app_version.clone());
    let tauri_version = env::var("APP_TAURI_VERSION").unwrap_or_default();

    println!("cargo:rustc-env=APP_WEB_VERSION={app_version}");
    println!("cargo:rustc-env=APP_DOCKER_IMAGE={docker_image}");
    println!("cargo:rustc-env=APP_DOCKER_VERSION={docker_version}");
    println!("cargo:rustc-env=APP_TAURI_VERSION={tauri_version}");
}
