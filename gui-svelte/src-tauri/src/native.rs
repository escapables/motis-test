use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader, Write};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use reqwest;
use std::path::Path;
#[cfg(unix)]
use std::io::ErrorKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteLeg {
    pub mode: String,
    pub from_name: String,
    pub to_name: String,
    pub from: LatLon,
    pub to: LatLon,
    pub duration_seconds: i32,
    pub distance_meters: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headsign: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    pub duration_seconds: i32,
    pub transfers: i32,
    pub legs: Vec<RouteLeg>,
}

// Area from C++ API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub admin_level: i32,
    pub matched: bool,
    pub unique: bool,
    #[serde(rename = "default")]
    pub is_default: bool,
}

// Simple location from C++ API (now with full Match data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResult {
    pub name: String,
    pub place_id: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(default)]
    pub areas: Vec<MatchArea>,
    #[serde(default)]
    pub tokens: Vec<Vec<i32>>,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
}

// Area for Match response (API format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArea {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    #[serde(rename = "default", skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

// Token for Match response [start, length]
pub type Token = [i32; 2];

// Full Match object expected by the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub tokens: Vec<Token>,
    pub areas: Vec<MatchArea>,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<f64>,
}

impl Match {
    /// Convert a LocationResult to a full Match using the real data from C++ API
    pub fn from_location_result(loc: &LocationResult) -> Self {
        // Convert tokens from C++ format
        let tokens: Vec<Token> = loc.tokens.iter()
            .filter(|t| t.len() >= 2)
            .map(|t| [t[0], t[1]])
            .collect();
        
        // Determine type - default to PLACE if not specified
        let type_ = loc.type_.clone().unwrap_or_else(|| "PLACE".to_string());
        
        Match {
            type_,
            name: loc.name.clone(),
            id: loc.place_id.clone(),
            lat: loc.lat,
            lon: loc.lon,
            tokens,
            areas: loc.areas.clone(),  // Clone directly from LocationResult
            score: loc.score,
            category: loc.category.clone(),
            street: loc.street.clone(),
            house_number: loc.house_number.clone(),
            country: loc.country.clone(),
            zip: loc.zip.clone(),
            tz: None,  // Not yet implemented in C++ API
            level: None,  // Not yet implemented in C++ API
            modes: loc.modes.clone(),
            importance: loc.importance,
        }
    }
}

// Backend connection modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendMode {
    Ipc,     // Subprocess with stdin/stdout - no HTTP needed
    Server,  // HTTP localhost server - uses web API
}

// Global state
static BACKEND_MODE: Lazy<Mutex<BackendMode>> = Lazy::new(|| Mutex::new(BackendMode::Ipc));
static IPC_PROCESS: Lazy<Mutex<Option<IpcBackend>>> = Lazy::new(|| Mutex::new(None));
static SERVER_URL: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new("http://localhost:8080".to_string()));

pub struct IpcBackend {
    _child: Child,
    stdin: std::process::ChildStdin,
}

impl IpcBackend {
    fn send_command(&mut self, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
        writeln!(self.stdin, "{}", cmd)?;
        
        let stdout = self._child.stdout.as_mut().ok_or("No stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        Ok(line)
    }
}

/// Copy executable to /tmp and set executable permission.
#[cfg(unix)]
fn copy_to_tmp_and_make_executable(exe_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(exe_path);

    let tmp_dir = std::env::temp_dir();
    let exe_name = path
        .file_name()
        .ok_or("Invalid exe path")?
        .to_str()
        .ok_or("Invalid exe name")?;
    let tmp_path = tmp_dir.join(exe_name);

    eprintln!("[MOTIS-GUI] Copying executable to /tmp: {:?}", tmp_path);
    std::fs::copy(exe_path, &tmp_path)?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&tmp_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&tmp_path, perms)?;

    tmp_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid tmp path".into())
}

/// Resolve an executable path suitable for spawn (handles FAT32/non-exec mounts).
fn ensure_executable(exe_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = Path::new(exe_path);

    if !path.exists() {
        return Err(format!("Executable not found: {}", exe_path).into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)?.permissions().mode();
        if mode & 0o111 != 0 {
            return Ok(exe_path.to_string());
        }
        eprintln!(
            "[MOTIS-GUI] Executable bit missing for {}, using /tmp copy",
            exe_path
        );
        return copy_to_tmp_and_make_executable(exe_path);
    }

    #[cfg(not(unix))]
    {
        Ok(exe_path.to_string())
    }
}

pub fn init_ipc(exe_path: &str, data_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[MOTIS-GUI] Starting motis-ipc...");
    eprintln!("[MOTIS-GUI] Original exe path: {}", exe_path);
    eprintln!("[MOTIS-GUI] Data path: {}", data_path);
    
    // Check data path exists
    if !Path::new(data_path).exists() {
        return Err(format!("Data directory not found: {}", data_path).into());
    }
    
    // Ensure executable is actually executable (handles FAT32 USB)
    let actual_exe_path = ensure_executable(exe_path)?;
    eprintln!("[MOTIS-GUI] Using executable: {}", actual_exe_path);

    let spawn = |binary: &str| {
        Command::new(binary)
            .arg(data_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())  // Show motis-ipc's stderr for debugging
            .spawn()
    };

    let mut child = match spawn(&actual_exe_path) {
        Ok(child) => child,
        #[cfg(unix)]
        Err(err) if err.kind() == ErrorKind::PermissionDenied => {
            eprintln!(
                "[MOTIS-GUI] Spawn failed due to permission/noexec mount, retrying from /tmp: {}",
                err
            );
            let tmp_exe = copy_to_tmp_and_make_executable(exe_path)?;
            eprintln!("[MOTIS-GUI] Retrying spawn with executable: {}", tmp_exe);
            spawn(&tmp_exe).map_err(|e| format!("Failed to spawn motis-ipc from /tmp: {}", e))?
        }
        Err(err) => {
            return Err(format!(
                "Failed to spawn motis-ipc: {}. Path: {}",
                err, actual_exe_path
            )
            .into())
        }
    };

    eprintln!("[MOTIS-GUI] Process spawned, PID: {:?}", child.id());

    // Catch immediate loader/startup failures early (e.g. missing GLIBCXX symbol).
    std::thread::sleep(std::time::Duration::from_millis(150));
    if let Some(status) = child.try_wait()? {
        return Err(format!(
            "motis-ipc exited immediately (status: {}). Check executable compatibility and startup logs.",
            status
        ).into());
    }
    
    let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
    
    // Store the backend first
    let backend = IpcBackend { _child: child, stdin };
    
    let mut guard = IPC_PROCESS.lock()?;
    *guard = Some(backend);
    
    let mut mode_guard = BACKEND_MODE.lock()?;
    *mode_guard = BackendMode::Ipc;
    
    eprintln!("[MOTIS-GUI] IPC backend initialized (data loading in progress...)");
    
    Ok(())
}

pub fn init_server(url: &str) {
    let mut guard = SERVER_URL.lock().unwrap();
    *guard = url.to_string();
    
    let mut mode_guard = BACKEND_MODE.lock().unwrap();
    *mode_guard = BackendMode::Server;
}

pub fn get_mode() -> BackendMode {
    *BACKEND_MODE.lock().unwrap()
}

// Geocode - works in both modes
pub async fn geocode(query: &str) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
    let mode = get_mode();
    
    match mode {
        BackendMode::Ipc => {
            eprintln!("[MOTIS-GUI] geocode() called with query: '{}'", query);
            
            let cmd = format!(r#"{{"cmd":"geocode","query":"{}"}}"#, query.replace('"', "\\\""));
            eprintln!("[MOTIS-GUI] Sending command: {}", cmd);
            
            // Retry logic for IPC
            let max_retries = 5;
            let retry_delay = std::time::Duration::from_millis(1000);
            
            for attempt in 1..=max_retries {
                let mut guard = IPC_PROCESS.lock()?;
                let backend = guard.as_mut().ok_or("IPC not initialized")?;
                
                match backend.send_command(&cmd) {
                    Ok(response) => {
                        eprintln!("[MOTIS-GUI] Got response: {}", response.trim());
                        
                        let result: serde_json::Value = serde_json::from_str(&response)?;
                        if result["status"] == "ok" {
                            let locations: Vec<LocationResult> = serde_json::from_value(result["data"].clone())?;
                            eprintln!("[MOTIS-GUI] Found {} locations", locations.len());
                            // Convert to Match format
                            let matches: Vec<Match> = locations.iter()
                                .map(|loc| Match::from_location_result(loc))
                                .collect();
                            return Ok(matches);
                        } else {
                            let msg = result["message"].as_str().unwrap_or("Unknown error");
                            eprintln!("[MOTIS-GUI] Backend error: {}", msg);
                            return Err(msg.into());
                        }
                    }
                    Err(e) => {
                        eprintln!("[MOTIS-GUI] Attempt {}/{} failed: {}", attempt, max_retries, e);
                        if attempt < max_retries {
                            eprintln!("[MOTIS-GUI] Retrying in 1 second...");
                            drop(guard); // Release lock before sleeping
                            std::thread::sleep(retry_delay);
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            
            Err("Max retries exceeded".into())
        }
        BackendMode::Server => {
            let url = SERVER_URL.lock()?.clone();
            eprintln!("[MOTIS-GUI] geocode() using server: {}", url);
            
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/api/v1/geocode", url))
                .query(&[("text", query)])
                .send()
                .await?;
            
            let matches: Vec<Match> = resp.json().await?;
            eprintln!("[MOTIS-GUI] Found {} matches", matches.len());
            Ok(matches)
        }
    }
}

// Plan route - works in both modes
pub async fn plan_route(
    from_lat: f64, from_lon: f64,
    to_lat: f64, to_lon: f64
) -> Result<Vec<RouteResult>, Box<dyn std::error::Error>> {
    let mode = get_mode();
    
    match mode {
        BackendMode::Ipc => {
            eprintln!("[MOTIS-GUI] plan_route() called: ({}, {}) to ({}, {})", from_lat, from_lon, to_lat, to_lon);
            
            let cmd = format!(
                r#"{{"cmd":"plan_route","from_lat":{},"from_lon":{},"to_lat":{},"to_lon":{}}}"#,
                from_lat, from_lon, to_lat, to_lon
            );
            eprintln!("[MOTIS-GUI] Sending command: {}", cmd);
            
            // Retry logic for IPC
            let max_retries = 5;
            let retry_delay = std::time::Duration::from_millis(1000);
            
            for attempt in 1..=max_retries {
                let mut guard = IPC_PROCESS.lock()?;
                let backend = guard.as_mut().ok_or("IPC not initialized")?;
                
                match backend.send_command(&cmd) {
                    Ok(response) => {
                        eprintln!("[MOTIS-GUI] Got response: {}", response.trim());
                        
                        let result: serde_json::Value = serde_json::from_str(&response)?;
                        if result["status"] == "ok" {
                            let routes: Vec<RouteResult> = serde_json::from_value(result["data"].clone())?;
                            eprintln!("[MOTIS-GUI] Found {} routes", routes.len());
                            return Ok(routes);
                        } else {
                            let msg = result["message"].as_str().unwrap_or("Unknown error");
                            eprintln!("[MOTIS-GUI] Backend error: {}", msg);
                            return Err(msg.into());
                        }
                    }
                    Err(e) => {
                        eprintln!("[MOTIS-GUI] Attempt {}/{} failed: {}", attempt, max_retries, e);
                        if attempt < max_retries {
                            eprintln!("[MOTIS-GUI] Retrying in 1 second...");
                            drop(guard); // Release lock before sleeping
                            std::thread::sleep(retry_delay);
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            
            Err("Max retries exceeded".into())
        }
        BackendMode::Server => {
            let url = SERVER_URL.lock()?.clone();
            eprintln!("[MOTIS-GUI] plan_route() using server: {}", url);
            
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/api/v1/plan", url))
                .query(&[
                    ("fromPlace", &format!("{}, {}", from_lat, from_lon)),
                    ("toPlace", &format!("{}, {}", to_lat, to_lon)),
                ])
                .send()
                .await?;
            
            let result: serde_json::Value = resp.json().await?;
            // Parse from MOTIS plan response format
            let itineraries = result["itineraries"].as_array()
                .ok_or("No itineraries")?;
            
            let mut routes = Vec::new();
            for itin in itineraries {
                let route = RouteResult {
                    duration_seconds: itin["duration"].as_i64().unwrap_or(0) as i32,
                    transfers: itin["transfers"].as_i64().unwrap_or(0) as i32,
                    legs: Vec::new(), // Simplified - would parse legs here
                };
                routes.push(route);
            }
            eprintln!("[MOTIS-GUI] Found {} routes", routes.len());
            Ok(routes)
        }
    }
}

// Reverse geocode
pub async fn reverse_geocode(lat: f64, lon: f64) -> Result<Option<Match>, Box<dyn std::error::Error>> {
    let mode = get_mode();
    
    match mode {
        BackendMode::Ipc => {
            let mut guard = IPC_PROCESS.lock()?;
            let backend = guard.as_mut().ok_or("IPC not initialized")?;
            
            let cmd = format!(r#"{{"cmd":"reverse_geocode","lat":{},"lon":{}}}"#, lat, lon);
            let response = backend.send_command(&cmd)?;
            
            let result: serde_json::Value = serde_json::from_str(&response)?;
            if result["status"] == "ok" && !result["data"].is_null() {
                let loc: LocationResult = serde_json::from_value(result["data"].clone())?;
                Ok(Some(Match::from_location_result(&loc)))
            } else {
                Ok(None)
            }
        }
        BackendMode::Server => {
            let url = SERVER_URL.lock()?.clone();
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/api/v1/reverse-geocode", url))
                .query(&[("lat", lat.to_string()), ("lon", lon.to_string())])
                .send()
                .await?;
            
            let matches: Vec<Match> = resp.json().await?;
            Ok(matches.into_iter().next())
        }
    }
}

pub fn destroy() {
    let mut guard = IPC_PROCESS.lock().unwrap();
    if let Some(_) = guard.take() {
        // Process will be killed when dropped
    }
}

// Auto-detect mode based on what's available
// For USB portable: ONLY use IPC mode, never fall back to server
pub async fn auto_init(exe_path: Option<&str>, data_path: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    eprintln!("[MOTIS-GUI] auto_init() called");
    eprintln!("[MOTIS-GUI] exe_path: {:?}", exe_path);
    eprintln!("[MOTIS-GUI] data_path: {:?}", data_path);
    
    // USB PORTABLE MODE: Only use IPC, never fall back to server
    let exe = exe_path.ok_or("No executable path provided")?;
    let data = data_path.ok_or("No data path provided")?;
    
    eprintln!("[MOTIS-GUI] Checking if files exist...");
    let exe_exists = Path::new(exe).exists();
    let data_exists = Path::new(data).exists();
    eprintln!("[MOTIS-GUI] exe exists: {}, data exists: {}", exe_exists, data_exists);
    
    if !exe_exists {
        return Err(format!(
            "motis-ipc executable not found: {}\n\nPlease ensure:\n1. motis-ipc is in the same folder as motis-gui\n2. You're running from the correct directory", 
            exe
        ).into());
    }
    
    if !data_exists {
        return Err(format!(
            "Data directory not found: {}\n\nPlease ensure:\n1. The 'data' folder exists next to motis-gui\n2. You've imported GTFS/OSM data using ./motis-transit import", 
            data
        ).into());
    }
    
    eprintln!("[MOTIS-GUI] Attempting IPC initialization...");
    match init_ipc(exe, data) {
        Ok(()) => {
            eprintln!("[MOTIS-GUI] IPC mode initialized successfully");
            Ok("IPC mode initialized".to_string())
        }
        Err(e) => {
            eprintln!("[MOTIS-GUI] IPC init failed: {}", e);
            Err(format!(
                "Failed to start IPC backend: {}\n\nTroubleshooting:\n1. Check that motis-ipc has executable permissions:\n   chmod +x {}\n2. If on NTFS/USB, ensure it's mounted with exec permissions\n3. Try running: {} --version\n4. Check that data directory contains valid MOTIS data", 
                e, exe, exe
            ).into())
        }
    }
}

// ============================================================================
// Synchronous wrappers for protocol handler
// These are blocking versions for use in the custom protocol handler
// ============================================================================

/// Synchronous geocode for protocol handler
pub fn geocode_sync(query: &str) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
    let cmd = format!(r#"{{"cmd":"geocode","query":"{}"}}"#, query.replace('"', "\\\""));
    
    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;
    
    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;
    
    if result["status"] == "ok" {
        let locations: Vec<LocationResult> = serde_json::from_value(result["data"].clone())?;
        // Convert to Match format
        let matches: Vec<Match> = locations.iter()
            .map(|loc| Match::from_location_result(loc))
            .collect();
        Ok(matches)
    } else {
        let msg = result["message"].as_str().unwrap_or("Unknown error");
        Err(msg.into())
    }
}

/// Synchronous route planning for protocol handler
pub fn plan_route_sync(
    from_lat: f64, from_lon: f64,
    to_lat: f64, to_lon: f64
) -> Result<Vec<RouteResult>, Box<dyn std::error::Error>> {
    let cmd = format!(
        r#"{{"cmd":"plan_route","from_lat":{},"from_lon":{},"to_lat":{},"to_lon":{}}}"#,
        from_lat, from_lon, to_lat, to_lon
    );
    
    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;
    
    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;
    
    if result["status"] == "ok" {
        let routes: Vec<RouteResult> = serde_json::from_value(result["data"].clone())?;
        Ok(routes)
    } else {
        let msg = result["message"].as_str().unwrap_or("Unknown error");
        Err(msg.into())
    }
}

/// Synchronous reverse geocode for protocol handler
pub fn reverse_geocode_sync(lat: f64, lon: f64) -> Result<Option<Match>, Box<dyn std::error::Error>> {
    let cmd = format!(r#"{{"cmd":"reverse_geocode","lat":{},"lon":{}}}"#, lat, lon);
    
    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;
    
    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;
    
    if result["status"] == "ok" && !result["data"].is_null() {
        let loc: LocationResult = serde_json::from_value(result["data"].clone())?;
        Ok(Some(Match::from_location_result(&loc)))
    } else {
        Ok(None)
    }
}

/// Check if IPC backend is initialized
pub fn is_ipc_initialized() -> bool {
    IPC_PROCESS.lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false)
}

/// Try to auto-initialize IPC from environment or default paths
/// Returns true if initialization succeeded
pub fn try_auto_init() -> bool {
    if is_ipc_initialized() {
        return true;
    }
    
    eprintln!("[MOTIS-GUI] Trying auto-initialization...");
    
    // Try environment variable for motis-ipc path
    let ipc_path = std::env::var("MOTIS_IPC_PATH").ok();
    
    // Try to find data path
    let data_path = std::env::var("MOTIS_DATA_PATH").ok()
        .or_else(|| {
            // Try default locations
            let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
            let data_dir = exe_dir.join("data");
            if data_dir.exists() {
                data_dir.to_str().map(|s| s.to_string())
            } else {
                None
            }
        });
    
    if let (Some(ipc), Some(data)) = (&ipc_path, &data_path) {
        eprintln!("[MOTIS-GUI] Auto-init with IPC: {}, Data: {}", ipc, data);
        if let Err(e) = init_ipc(ipc, data) {
            eprintln!("[MOTIS-GUI] Auto-init failed: {}", e);
            return false;
        }
        eprintln!("[MOTIS-GUI] Auto-init succeeded!");
        return true;
    }
    
    eprintln!("[MOTIS-GUI] Auto-init: missing paths. IPC: {:?}, Data: {:?}", ipc_path, data_path);
    false
}

/// Synchronous get tile for protocol handler (returns base64 string)
pub fn get_tile_sync(z: i32, x: i32, y: i32) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let cmd = format!(r#"{{"cmd":"get_tile","z":{},"x":{},"y":{}}}"#, z, x, y);
    
    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;
    
    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;
    
    if result["status"] == "ok" && result["data"]["found"].as_bool().unwrap_or(false) {
        let base64_data = result["data"]["data_base64"].as_str()
            .ok_or("Invalid tile data")?;
        Ok(Some(base64_data.to_string()))
    } else {
        Ok(None)
    }
}

/// Synchronous get glyph for protocol handler (returns base64 string)
pub fn get_glyph_sync(path: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let cmd = serde_json::json!({
        "cmd": "get_glyph",
        "path": path
    }).to_string();

    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;

    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;

    if result["status"] == "ok" && result["data"]["found"].as_bool().unwrap_or(false) {
        let base64_data = result["data"]["data_base64"]
            .as_str()
            .ok_or("Invalid glyph data")?;
        Ok(Some(base64_data.to_string()))
    } else {
        Ok(None)
    }
}

/// Generic synchronous GET endpoint call for protocol passthrough.
pub fn api_get_sync(path_and_query: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cmd = serde_json::json!({
        "cmd": "api_get",
        "path": path_and_query
    }).to_string();

    let mut guard = IPC_PROCESS.lock()?;
    let backend = guard.as_mut().ok_or("IPC not initialized")?;

    let response = backend.send_command(&cmd)?;
    let result: serde_json::Value = serde_json::from_str(&response)?;

    if result["status"] == "ok" {
        Ok(result["data"].clone())
    } else {
        let msg = result["message"].as_str().unwrap_or("Unknown error");
        Err(msg.into())
    }
}
