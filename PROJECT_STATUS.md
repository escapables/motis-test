# MOTIS Portable USB Bundle - Project Status

## Current State: Functional Core Complete ✅

## Session Update (2026-02-05)

### Newly Confirmed Working

- Svelte map vector tiles render correctly in native Tauri (MapLibre) over IPC.
- Glyphs now render correctly (no more white square/tofu placeholders for labels).
- Core interactive map/API flows are working with protocol passthrough:
  - `trip`, `stoptimes` (`v1/v4/v5`), `map/trips` (`v1/v4/v5`)
  - `map/initial`, `map/stops`, `map/levels`
  - `one-to-many`, `one-to-all`, `rentals`

### Technical Changes Landed

- Added tile payload zlib inflate in protocol handler so MapLibre receives decodable MVT bytes.
- Added real glyph serving path in IPC stack:
  - C++ native API `get_glyph(...)` from embedded font resources.
  - IPC command `get_glyph`.
  - Rust sync wrapper + protocol glyph handler wiring.
- Rebuilt and deployed updated `motis-ipc` and `motis-gui-svelte` to USB root bundle.

### Current Priority

- Continue validating remaining UI controls/interaction paths in the Svelte app and patch any endpoint gaps in protocol/native passthrough.

### What Works Now

| Component | Status | Notes |
|-----------|--------|-------|
| IPC Backend | ✅ | `motis-ipc` subprocess communication working |
| Native GUI | ✅ | Basic routing, geocoding functional |
| Map Display | ✅ | Leaflet with OSM tiles, route visualization |
| USB Portable | ✅ | FAT32 compatible via RUN.sh |
| Debug Mode | ✅ | `--debug` flag for troubleshooting |

### File Locations

```
/home/dator/motis-test/
├── gui/                          # SIMPLE HTML VERSION (current)
│   └── src-tauri/
│       ├── src/
│       │   ├── main.rs           # Tauri commands + --debug flag
│       │   └── native.rs         # IPC bridge (SHARE THIS)
│       └── src/index.html        # Vanilla JS frontend with map
│
├── gui-svelte/                   # FULL SVELTE UI (in progress)
│   └── src-tauri/                # Created but empty
│
├── ui/                           # WEB UI SOURCE (Svelte)
│   └── src/                      # 77 Svelte components
│       ├── lib/map/              # MapLibre GL JS components
│       └── routes/+page.svelte   # Main app
│
├── native/                       # C++ IPC BRIDGE
│   ├── api.cc                    # Native MOTIS API wrapper
│   └── example_ipc.cc            # motis-ipc executable
│
└── usb-bundle/                   # USB DEPLOYMENT
    ├── motis-gui                 # Current GUI binary
    ├── motis-ipc                 # IPC bridge (103MB)
    └── data/                     # GTFS/OSM data
```

## Technical Achievements

### 1. IPC Communication Protocol
**Request Format:**
```json
{"cmd":"plan_route","from_lat":59.5,"from_lon":17.5,"to_lat":59.3,"to_lon":18.0}
```

**Response Format:**
```json
{"status":"ok","data":[{"duration_seconds":2640,"transfers":0,"legs":[...]}]}
```

### 2. JSON Serialization Fixed
- Rust structs now match C++ backend output
- Nested `from`/`to` objects with `lat`/`lon` coordinates

### 3. Tauri Command Signatures
```rust
// Use camelCase to match JavaScript
async fn plan_route_cmd(
    #[allow(non_snake_case)] fromLat: f64,
    #[allow(non_snake_case)] fromLon: f64,
    #[allow(non_snake_case)] toLat: f64,
    #[allow(non_snake_case)] toLon: f64,
) -> Result<Vec<Route>, String>
```

### 4. Build Process
- Use `cargo tauri build` (not `cargo build --release`)
- Embeds frontend assets properly
- Works from any location (USB portable)

## Next Phase: Full Svelte Integration

### Goal
Port all 77 Svelte components from `ui/` to work in Tauri via IPC.

### Architecture Change

**Current (gui/):**
```
Vanilla JS ──invoke()──► Rust ──IPC──► motis-ipc
```

**Target (gui-svelte/):**
```
Svelte UI ──fetch(/api/...)──► Tauri Custom Protocol ──IPC──► motis-ipc
```

### Implementation Plan

#### Step 1: Foundation
1. Copy `gui/src-tauri/src/native.rs` → `gui-svelte/src-tauri/src/`
2. Set up basic Tauri structure
3. Verify IPC still works

#### Step 2: Custom Protocol
Add to `main.rs`:
```rust
.register_uri_scheme_protocol("motis", |app, request| {
    // Parse /api/v1/plan?...
    // Route to IPC
    // Return JSON response
})
```

#### Step 3: API Bridge
Map HTTP endpoints to IPC:
| Endpoint | IPC Command | Status |
|----------|-------------|--------|
| /api/v1/plan | plan_route_cmd | ✅ exists |
| /api/v1/geocode | geocode_cmd | ✅ exists |
| /tiles/{z}/{x}/{y}.mvt | get_tile | 🟡 Working backend, rendering issue |
| /api/v1/stoptimes | **NEW** stop_times | ⏳ needed |
| /api/v1/isochrones | **NEW** isochrones | ⏳ needed |
| /api/v1/trips | **NEW** trips (RailViz) | ⏳ needed |

#### Step 4: C++ API Extensions
Add to `native/api.cc`:
```cpp
// Vector tiles
std::vector<uint8_t> get_tile(int z, int x, int y);

// Schedules
std::string get_stop_times(const std::string& stop_id);

// Isochrones
std::string get_isochrones(double lat, double lon, int minutes);

// Real-time vehicles
std::string get_trips_in_bounds(Bounds bounds);
```

#### Step 5: Build Integration
```bash
# Build Svelte UI
cd ui && pnpm build

# Copy to Tauri
cp -r ui/build gui-svelte/src-tauri/web-build

# Build Tauri app
cd gui-svelte/src-tauri && cargo tauri build
```

## Key Challenges Ahead

### 1. Vector Tiles (Biggest Challenge) - 🟡 IN PROGRESS
- ✅ Backend tile serving works (200KB+ tiles served via IPC)
- 🟡 Frontend rendering issue - tiles not showing in MapLibre
- **Investigation needed:**
  - Tauri v2 custom protocol CSP issues
  - MapLibre worker access to `motis://` protocol
  - Try `http://motis.localhost` format
  - Alternative: proxy through Tauri invoke commands
- Fallback: Use raster tiles (OSM) instead of vector tiles

### 2. Binary Data in IPC
- Current IPC uses JSON text
- Tiles are binary (application/x-protobuf)
- May need base64 encoding or shared memory

### 3. Real-time Updates
- RailViz polls for vehicle positions
- Need to convert to Tauri events or keep polling

### 4. Web Workers
- Svelte UI uses workers for trip calculations
- Workers can work in Tauri but need path adjustments

## Decision Points

### Q: Keep HTML version?
**A:** YES - as fallback/lightweight option
- `gui/` → simple, fast, basic routing
- `gui-svelte/` → full features, larger binary

### Q: Use raster or vector tiles?
**A:** Start with raster (faster), migrate to vector
- Raster: Change map style URL in Svelte
- Vector: Implement tile serving in C++/Rust

### Q: Implement all features at once?
**A:** NO - incremental approach
1. Get basic Svelte UI loading
2. Add route planning (existing IPC)
3. Add tiles (raster first)
4. Add schedules
5. Add isochrones
6. Add RailViz

## Current Blockers

None - ready to proceed with Svelte integration.

## Success Criteria

- [ ] Svelte UI loads in Tauri window
- [ ] Route planning works
- [ ] Map displays with tiles
- [ ] All web UI features functional
- [ ] USB portable (no localhost)
- [ ] FAT32 compatible

## Ready to Start

Next action: Copy native.rs and set up Tauri custom protocol handler.
