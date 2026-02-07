//! Custom protocol handler for motis:// requests
//! 
//! This module intercepts HTTP requests from the Svelte UI and routes them
//! to the IPC backend. This allows the Svelte UI to use standard fetch()
//! while communicating via IPC subprocess (no localhost HTTP server needed).

use tauri::http::{Request, Response, StatusCode, header::CONTENT_TYPE};
use std::borrow::Cow;
use serde_json::json;
use crate::native;
use std::io::Read;

fn error_response(
    status: StatusCode,
    stage: &str,
    path: &str,
    message: &str,
) -> Response<Cow<'static, [u8]>> {
    let payload = json!({
        "error": message,
        "stage": stage,
        "path": path
    });

    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Cow::Owned(payload.to_string().into_bytes()))
        .unwrap()
}

fn classify_error(error: &str) -> (StatusCode, &'static str) {
    let lower = error.to_ascii_lowercase();
    if lower.starts_with("unknown endpoint:") {
        return (StatusCode::NOT_FOUND, "endpoint");
    }
    if lower.contains("initialization")
        || lower.contains("not initialized")
        || lower.contains("data directory")
        || lower.contains("motis_data_path")
        || lower.contains("config.yml")
    {
        return (StatusCode::SERVICE_UNAVAILABLE, "initialization");
    }
    if lower.contains("ipc")
        || lower.contains("motis-ipc")
        || lower.contains("broken pipe")
        || lower.contains("unexpected eof")
    {
        return (StatusCode::BAD_GATEWAY, "ipc");
    }
    (StatusCode::INTERNAL_SERVER_ERROR, "endpoint")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteKind {
    Geocode,
    ReverseGeocode,
    Passthrough,
    Glyph,
    Tiles,
    DebugTransfers,
    Unknown,
}

fn classify_path(path: &str) -> RouteKind {
    match path {
        // Geocoding
        "/api/v1/geocode" | "/api/v5/geocode" => RouteKind::Geocode,
        "/api/v1/reverse-geocode" | "/api/v5/reverse-geocode" => RouteKind::ReverseGeocode,

        // Route planning + core transit API
        "/api/v1/plan"
        | "/api/v5/plan"
        | "/api/v1/trip"
        | "/api/v5/trip"
        | "/api/v1/stoptimes"
        | "/api/v4/stoptimes"
        | "/api/v5/stoptimes"
        | "/api/v1/map/trips"
        | "/api/v4/map/trips"
        | "/api/v5/map/trips"
        | "/api/v1/map/initial"
        | "/api/v1/map/stops"
        | "/api/v1/map/levels"
        | "/api/v1/one-to-many"
        | "/api/v1/one-to-all"
        | "/api/experimental/one-to-all"
        | "/api/v1/rentals"
        | "/api/v1/map/rentals" => RouteKind::Passthrough,

        // Glyph requests used by MapLibre text rendering
        _ if path.starts_with("/tiles/glyphs/") => RouteKind::Glyph,

        // Tiles (vector tiles for map)
        _ if path.starts_with("/api/v1/tiles/")
            || path.starts_with("/api/v5/tiles/")
            || (path.starts_with("/tiles/") && path.ends_with(".mvt")) =>
        {
            RouteKind::Tiles
        }

        // Debug
        "/api/debug/transfers" => RouteKind::DebugTransfers,
        _ => RouteKind::Unknown,
    }
}

/// Handle motis:// scheme requests
pub fn handle_motis_request(
    request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let query = request.uri().query().unwrap_or("");
    
    eprintln!("[MOTIS-PROTOCOL] Request: {}?{}", path, query);
    
    // Check if IPC is initialized.
    let mut is_initialized = native::is_ipc_initialized();
    
    // Try auto-initialization if not initialized (for USB/FAT32 launcher compatibility)
    if !is_initialized {
        is_initialized = native::try_auto_init();
    }
    
    eprintln!("[MOTIS-PROTOCOL] IPC initialized: {}, path: {}", is_initialized, path);
    
    if !is_initialized {
        let message = native::get_startup_diagnostics().unwrap_or_else(|| {
            "MOTIS IPC not initialized. Next action: launch via RUN.sh or set MOTIS_IPC_PATH and MOTIS_DATA_PATH."
                .to_string()
        });
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "initialization", path, &message);
    }
    
    // Parse query parameters
    let params: std::collections::HashMap<String, String> = query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|s| {
            let mut parts = s.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").to_string();
            Some((key, urlencoding::decode(&value).unwrap_or_default().to_string()))
        })
        .collect();
    
    // Debug: log all parameters
    if !params.is_empty() {
        eprintln!("[MOTIS-PROTOCOL] Params: {:?}", params);
    }
    
    // Route to appropriate handler
    let result = match classify_path(path) {
        RouteKind::Geocode => handle_geocode(&params),
        RouteKind::ReverseGeocode => handle_reverse_geocode(&params),
        RouteKind::Passthrough => handle_api_passthrough(path, query),
        RouteKind::Glyph => handle_glyphs(path),
        RouteKind::Tiles => handle_tiles(path),
        RouteKind::DebugTransfers => handle_debug_transfers(&params),
        RouteKind::Unknown => Err(format!("Unknown endpoint: {}", path)),
    };
    
    match result {
        Ok((body, content_type)) => {
            Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type)
                .header("Access-Control-Allow-Origin", "*")
                .body(Cow::Owned(body))
                .unwrap()
        }
        Err(e) => {
            eprintln!("[MOTIS-PROTOCOL] Error: {}", e);
            let (status, stage) = classify_error(&e);
            error_response(status, stage, path, &e)
        }
    }
}

fn handle_geocode(params: &std::collections::HashMap<String, String>) 
    -> Result<(Vec<u8>, &'static str), String> {
    
    // Support multiple common parameter names
    let query = params.get("text")
        .or_else(|| params.get("q"))
        .or_else(|| params.get("query"))
        .or_else(|| params.get("search"))
        .ok_or("Missing query parameter (expected 'text', 'q', 'query', or 'search')")?;
    
    eprintln!("[MOTIS-PROTOCOL] Geocoding query: '{}'", query);
    
    match native::geocode_sync(query) {
        Ok(locations) => {
            eprintln!("[MOTIS-PROTOCOL] Geocode found {} results", locations.len());
            // Return array directly (not wrapped in content)
            Ok((serde_json::to_vec(&locations).unwrap(), "application/json"))
        }
        Err(e) => {
            eprintln!("[MOTIS-PROTOCOL] Geocode error: {}", e);
            Err(e.to_string())
        }
    }
}

fn handle_reverse_geocode(params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    
    let lat: f64 = params.get("lat")
        .and_then(|s| s.parse().ok())
        .ok_or("Missing or invalid 'lat' parameter")?;
    
    let lon: f64 = params.get("lon")
        .and_then(|s| s.parse().ok())
        .ok_or("Missing or invalid 'lon' parameter")?;
    
    match native::reverse_geocode_sync(lat, lon) {
        Ok(Some(loc)) => {
            Ok((serde_json::to_vec(&loc).unwrap(), "application/json"))
        }
        Ok(None) => {
            Ok(("null".as_bytes().to_vec(), "application/json"))
        }
        Err(e) => Err(e.to_string())
    }
}

fn handle_trip(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy trip handler should not be used".to_string())
}

fn handle_stoptimes(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy stoptimes handler should not be used".to_string())
}

fn handle_map_trips(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy map trips handler should not be used".to_string())
}

fn handle_map_initial(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy map initial handler should not be used".to_string())
}

fn handle_map_stops(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy map stops handler should not be used".to_string())
}

fn handle_map_levels(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy map levels handler should not be used".to_string())
}

fn handle_one_to_many(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy one-to-many handler should not be used".to_string())
}

fn handle_one_to_all(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy one-to-all handler should not be used".to_string())
}

fn handle_rentals(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Err("Legacy rentals handler should not be used".to_string())
}

fn build_passthrough_path_and_query(path: &str, query: &str) -> String {
    if query.is_empty() {
        path.to_string()
    } else {
        // Preserve the original query bytes exactly (no parse/rebuild),
        // so STOP/place ids and encoded values keep their semantics.
        format!("{}?{}", path, query)
    }
}

fn handle_api_passthrough(path: &str, query: &str) -> Result<(Vec<u8>, &'static str), String> {
    let path_and_query = build_passthrough_path_and_query(path, query);
    let value = native::api_get_sync(&path_and_query).map_err(|e| e.to_string())?;
    Ok((serde_json::to_vec(&value).unwrap_or_else(|_| b"{}".to_vec()), "application/json"))
}

fn handle_glyphs(path: &str) -> Result<(Vec<u8>, &'static str), String> {
    eprintln!("[MOTIS-PROTOCOL] Glyph request: {}", path);
    match native::get_glyph_sync(path) {
        Ok(Some(base64_data)) => {
            match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data) {
                Ok(binary_data) => {
                    return Ok((binary_data, "application/x-protobuf"));
                }
                Err(e) => {
                    eprintln!("[MOTIS-PROTOCOL] Glyph base64 decode error: {}", e);
                }
            }
        }
        Ok(None) => {
            eprintln!("[MOTIS-PROTOCOL] Glyph not found: {}", path);
        }
        Err(e) => {
            eprintln!("[MOTIS-PROTOCOL] Glyph fetch error: {}", e);
        }
    }

    // Return empty glyph payload on error so map rendering can continue.
    Ok((vec![], "application/x-protobuf"))
}

fn maybe_inflate_zlib(data: &[u8]) -> Vec<u8> {
    // MOTIS tile payloads are often zlib-compressed. MapLibre expects
    // decodable MVT protobuf bytes at this stage.
    if data.len() < 2 || data[0] != 0x78 {
        return data.to_vec();
    }

    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut inflated = Vec::new();
    match decoder.read_to_end(&mut inflated) {
        Ok(_) if !inflated.is_empty() => inflated,
        _ => data.to_vec(),
    }
}

fn handle_tiles(path: &str) -> Result<(Vec<u8>, &'static str), String> {
    // Vector tiles are binary MVT format
    eprintln!("[MOTIS-PROTOCOL] Tile request: {}", path);
    
    // Parse tile coordinates from path: /api/v1/tiles/{z}/{x}/{y}.mvt or /tiles/{z}/{x}/{y}.mvt
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    eprintln!("[MOTIS-PROTOCOL] Path parts: {:?}", parts);
    
    // Need at least 3 parts: tiles/z/x/y.mvt
    if parts.len() >= 3 {
        let z: i32 = parts[parts.len()-3].parse().map_err(|e| format!("Invalid z: {}", e))?;
        let x: i32 = parts[parts.len()-2].parse().map_err(|e| format!("Invalid x: {}", e))?;
        let y_str = parts.last().unwrap_or(&"");
        let y: i32 = y_str.trim_end_matches(".mvt").parse().map_err(|e| format!("Invalid y: {}", e))?;
        
        eprintln!("[MOTIS-PROTOCOL] Tile: z={}, x={}, y={}", z, x, y);
        
        // Fetch tile from IPC backend
        match native::get_tile_sync(z, x, y) {
            Ok(Some(base64_data)) => {
                eprintln!("[MOTIS-PROTOCOL] Got base64 data: {} chars", base64_data.len());
                // Decode base64 to binary
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data) {
                    Ok(binary_data) => {
                        let tile_bytes = maybe_inflate_zlib(&binary_data);
                        eprintln!(
                            "[MOTIS-PROTOCOL] Tile decoded: {} bytes (inflated: {} bytes)",
                            binary_data.len(),
                            tile_bytes.len()
                        );
                        if tile_bytes.is_empty() {
                            eprintln!("[MOTIS-PROTOCOL] Warning: tile data is empty!");
                        }
                        return Ok((tile_bytes, "application/vnd.mapbox-vector-tile"));
                    }
                    Err(e) => {
                        eprintln!("[MOTIS-PROTOCOL] Base64 decode error: {}", e);
                    }
                }
            }
            Ok(None) => {
                eprintln!("[MOTIS-PROTOCOL] Tile not found");
            }
            Err(e) => {
                eprintln!("[MOTIS-PROTOCOL] Tile fetch error: {}", e);
            }
        }
    }
    
    // Return empty tile on error (this is what map libraries expect for missing tiles)
    eprintln!("[MOTIS-PROTOCOL] Returning empty tile");
    Ok((vec![], "application/vnd.mapbox-vector-tile"))
}

fn handle_debug_transfers(_params: &std::collections::HashMap<String, String>)
    -> Result<(Vec<u8>, &'static str), String> {
    Ok(("[]".as_bytes().to_vec(), "application/json"))
}

#[cfg(test)]
mod tests {
    use super::{build_passthrough_path_and_query, classify_path, RouteKind};

    #[test]
    fn passthrough_path_without_query() {
        assert_eq!(
            build_passthrough_path_and_query("/api/v1/plan", ""),
            "/api/v1/plan"
        );
    }

    #[test]
    fn passthrough_preserves_stop_id_query_for_plan() {
        let query = "fromPlace=sweden_1617&toPlace=sweden_765&time=2026-02-07T08%3A00%3A00Z";
        assert_eq!(
            build_passthrough_path_and_query("/api/v1/plan", query),
            "/api/v1/plan?fromPlace=sweden_1617&toPlace=sweden_765&time=2026-02-07T08%3A00%3A00Z"
        );
    }

    #[test]
    fn passthrough_preserves_coordinate_query_for_plan() {
        let query = "fromPlace=59.331139%2C18.059447&toPlace=59.313578%2C18.06192";
        assert_eq!(
            build_passthrough_path_and_query("/api/v5/plan", query),
            "/api/v5/plan?fromPlace=59.331139%2C18.059447&toPlace=59.313578%2C18.06192"
        );
    }

    #[test]
    fn openapi_endpoints_are_routed_in_protocol_mode() {
        // Keep this aligned with ui/api/openapi/services.gen.ts
        let openapi_paths = [
            "/api/v5/plan",
            "/api/v1/one-to-many",
            "/api/v1/one-to-all",
            "/api/v1/reverse-geocode",
            "/api/v1/geocode",
            "/api/v5/trip",
            "/api/v5/stoptimes",
            "/api/v5/map/trips",
            "/api/v1/map/initial",
            "/api/v1/map/stops",
            "/api/v1/map/levels",
            "/api/v1/rentals",
            "/api/debug/transfers",
        ];

        for path in openapi_paths {
            assert_ne!(
                classify_path(path),
                RouteKind::Unknown,
                "UI OpenAPI path not routed in protocol mode: {path}"
            );
        }
    }
}
