use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader, Write};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteLeg {
    pub mode: String,
    pub from_name: String,
    pub to_name: String,
    pub from_lat: f64,
    pub from_lon: f64,
    pub to_lat: f64,
    pub to_lon: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResult {
    pub name: String,
    pub place_id: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
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

pub fn init_ipc(exe_path: &str, data_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new(exe_path)
        .arg(data_path)
        .arg("--ipc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    
    let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
    
    let mut guard = IPC_PROCESS.lock()?;
    *guard = Some(IpcBackend { _child: child, stdin });
    
    let mut mode_guard = BACKEND_MODE.lock()?;
    *mode_guard = BackendMode::Ipc;
    
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
pub async fn geocode(query: &str) -> Result<Vec<LocationResult>, Box<dyn std::error::Error>> {
    let mode = get_mode();
    
    match mode {
        BackendMode::Ipc => {
            let mut guard = IPC_PROCESS.lock()?;
            let backend = guard.as_mut().ok_or("IPC not initialized")?;
            
            let cmd = format!(r#"{{"cmd":"geocode","query":"{}"}}"#, query.replace('"', "\\\""));
            let response = backend.send_command(&cmd)?;
            
            let result: serde_json::Value = serde_json::from_str(&response)?;
            if result["status"] == "ok" {
                let locations: Vec<LocationResult> = serde_json::from_value(result["data"].clone())?;
                Ok(locations)
            } else {
                Err(result["message"].as_str().unwrap_or("Unknown error").into())
            }
        }
        BackendMode::Server => {
            let url = SERVER_URL.lock()?.clone();
            let client = reqwest::Client::new();
            let resp = client
                .get(format!("{}/api/v1/geocode", url))
                .query(&[("text", query)])
                .send()
                .await?;
            
            let locations: Vec<LocationResult> = resp.json().await?;
            Ok(locations)
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
            let mut guard = IPC_PROCESS.lock()?;
            let backend = guard.as_mut().ok_or("IPC not initialized")?;
            
            let cmd = format!(
                r#"{{"cmd":"plan_route","from_lat":{},"from_lon":{},"to_lat":{},"to_lon":{}}}"#,
                from_lat, from_lon, to_lat, to_lon
            );
            let response = backend.send_command(&cmd)?;
            
            let result: serde_json::Value = serde_json::from_str(&response)?;
            if result["status"] == "ok" {
                let routes: Vec<RouteResult> = serde_json::from_value(result["data"].clone())?;
                Ok(routes)
            } else {
                Err(result["message"].as_str().unwrap_or("Unknown error").into())
            }
        }
        BackendMode::Server => {
            let url = SERVER_URL.lock()?.clone();
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
            Ok(routes)
        }
    }
}

// Reverse geocode
pub async fn reverse_geocode(lat: f64, lon: f64) -> Result<Option<LocationResult>, Box<dyn std::error::Error>> {
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
                Ok(Some(loc))
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
            
            let loc: Option<LocationResult> = resp.json().await?;
            Ok(loc)
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
pub async fn auto_init(exe_path: Option<&str>, data_path: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    // Try IPC mode first
    if let (Some(exe), Some(data)) = (exe_path, data_path) {
        if std::path::Path::new(exe).exists() && std::path::Path::new(data).exists() {
            match init_ipc(exe, data) {
                Ok(()) => return Ok("IPC mode initialized".to_string()),
                Err(e) => println!("IPC init failed: {}, trying server mode...", e),
            }
        }
    }
    
    // Try server mode
    let url = SERVER_URL.lock()?.clone();
    match reqwest::get(format!("{}/api/v1/geocode?text=test", url)).await {
        Ok(_) => {
            init_server(&url);
            Ok("Server mode initialized".to_string())
        }
        Err(_) => Err("Neither IPC nor server backend available".into())
    }
}
