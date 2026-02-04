pub mod native;

use native::{LocationResult as Location, RouteResult as Route};
use std::path::PathBuf;

fn get_exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to get executable directory".to_string())
}

#[tauri::command]
async fn init_backend(
    exe_path: Option<String>,
    data_path: Option<String>,
    server_url: Option<String>
) -> Result<String, String> {
    // If server URL provided, use server mode
    if let Some(url) = server_url {
        native::init_server(&url);
        return Ok(format!("Server mode: {}", url));
    }
    
    // Otherwise try auto-detection
    let exe = exe_path.as_deref().or_else(|| {
        get_exe_dir().ok().map(|d| d.join("motis-ipc"))
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .map(|s| s.leak() as &str)
    });
    
    let data = data_path.as_deref().or_else(|| {
        get_exe_dir().ok().map(|d| d.join("data"))
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .map(|s| s.leak() as &str)
    });
    
    native::auto_init(exe, data).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_backend_mode() -> Result<String, String> {
    match native::get_mode() {
        native::BackendMode::Ipc => Ok("ipc".to_string()),
        native::BackendMode::Server => Ok("server".to_string()),
    }
}

#[tauri::command]
async fn plan_route_cmd(
    from_lat: f64, from_lon: f64,
    to_lat: f64, to_lon: f64,
) -> Result<Vec<Route>, String> {
    native::plan_route(from_lat, from_lon, to_lat, to_lon)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn geocode_cmd(query: String) -> Result<Vec<Location>, String> {
    native::geocode(&query).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reverse_geocode_cmd(lat: f64, lon: f64) -> Result<Option<Location>, String> {
    native::reverse_geocode(lat, lon).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn destroy_backend() {
    native::destroy();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            init_backend,
            get_backend_mode,
            plan_route_cmd,
            geocode_cmd,
            reverse_geocode_cmd,
            destroy_backend,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
