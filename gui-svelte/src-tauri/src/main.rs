pub mod native;
pub mod protocol;

use native::{Match as Location, RouteResult as Route};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

// Global debug flag
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

struct IpcShutdownGuard;

impl Drop for IpcShutdownGuard {
    fn drop(&mut self) {
        native::destroy();
    }
}

#[cfg(target_os = "linux")]
fn install_linux_zoom_lock<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    use webkit2gtk::WebViewExt;

    window
        .with_webview(|webview| {
            let view = webview.inner();
            view.set_zoom_level(1.0);
            view.connect_zoom_level_notify(|view| {
                // Force app/webview zoom to remain neutral; map zoom is handled by MapLibre itself.
                if (view.zoom_level() - 1.0).abs() > 0.000_001 {
                    view.set_zoom_level(1.0);
                }
            });
        })
        .map_err(|e| format!("Failed to configure Linux webview zoom lock: {}", e))
}

#[cfg(not(target_os = "linux"))]
fn install_linux_zoom_lock<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    Ok(())
}

fn get_exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| "Failed to get executable directory".to_string())
}

#[tauri::command]
async fn get_default_data_path_cmd() -> Result<String, String> {
    get_exe_dir()
        .map(|d| d.join("data"))
        .and_then(|p| p.to_str().map(|s| s.to_string()).ok_or("Invalid path".to_string()))
}

#[tauri::command]
async fn check_data_path_exists(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).exists())
}

#[tauri::command]
async fn init_native(data_path: Option<String>) -> Result<String, String> {
    init_backend(None, data_path, None).await
}

#[tauri::command]
async fn init_backend(
    exe_path: Option<String>,
    data_path: Option<String>,
    _server_url: Option<String>
) -> Result<String, String> {
    // Check environment variable first (set by RUN.sh for USB/FAT32 support)
    let env_ipc_path = std::env::var("MOTIS_IPC_PATH").ok();
    
    let exe = exe_path.as_deref()
        .or_else(|| env_ipc_path.as_deref())
        .or_else(|| {
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
    #[allow(non_snake_case)] fromLat: f64,
    #[allow(non_snake_case)] fromLon: f64,
    #[allow(non_snake_case)] toLat: f64,
    #[allow(non_snake_case)] toLon: f64,
) -> Result<Vec<Route>, String> {
    native::plan_route(fromLat, fromLon, toLat, toLon)
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

#[tauri::command]
async fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Some(main_webview) = app.get_webview_window("main") {
                if let Err(err) = install_linux_zoom_lock(&main_webview) {
                    eprintln!("[MOTIS-GUI] {}", err);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_backend,
            init_native,
            get_default_data_path_cmd,
            check_data_path_exists,
            get_backend_mode,
            plan_route_cmd,
            geocode_cmd,
            reverse_geocode_cmd,
            destroy_backend,
            is_debug_mode,
        ])
        .register_uri_scheme_protocol("motis", |_app, request| {
            protocol::handle_motis_request(request)
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    let _ipc_shutdown_guard = IpcShutdownGuard;
    let args: Vec<String> = std::env::args().collect();
    let debug_mode = args.contains(&"--debug".to_string());
    DEBUG_MODE.store(debug_mode, Ordering::Relaxed);
    
    if debug_mode {
        eprintln!("[MOTIS-GUI] Debug mode enabled");
    }
    
    // Auto-initialize IPC if data path provided via CLI
    let data_path_flag = args.iter().position(|a| a == "--data-path");
    if let Some(pos) = data_path_flag {
        if let Some(path) = args.get(pos + 1) {
            eprintln!("[MOTIS-GUI] Auto-initializing with data path: {}", path);
            
            // Try multiple locations for motis-ipc
            // First check environment variable (set by RUN.sh for USB/FAT32)
            let env_path = std::env::var("MOTIS_IPC_PATH").ok().map(PathBuf::from);
            
            let possible_exes = vec![
                env_path,
                // Same directory as executable (normal case)
                get_exe_dir().ok().map(|d| d.join("motis-ipc")),
                // Parent of data directory (USB case: data/../motis-ipc)
                Path::new(path).parent().map(|p| p.join("motis-ipc")),
            ];
            
            let mut initialized = false;
            for exe_path in possible_exes.into_iter().flatten() {
                if exe_path.exists() {
                    eprintln!("[MOTIS-GUI] Trying motis-ipc at: {:?}", exe_path);
                    if let Some(exe_str) = exe_path.to_str() {
                        if let Err(e) = native::init_ipc(exe_str, path) {
                            eprintln!("[MOTIS-GUI] Auto-init failed: {}", e);
                        } else {
                            eprintln!("[MOTIS-GUI] IPC auto-initialized successfully");
                            initialized = true;
                            break;
                        }
                    }
                }
            }
            
            if !initialized {
                eprintln!("[MOTIS-GUI] Could not find or initialize motis-ipc");
            }
        }
    }
    
    run();
}
