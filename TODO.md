# MOTIS Portable USB Bundle - TODO List

## Project Structure

We now have **two GUI versions**:

| Directory | Type | Purpose | Status |
|-----------|------|---------|--------|
| `gui/` | Simple HTML | Lightweight, basic routing | ✅ Working |
| `gui-svelte/` | Full Svelte UI | Map working, tiles serving! | 🚧 In Progress |

## 🔴 CRITICAL - Blocking Issues

### Session Notes (2026-02-05)
- [x] Vector tiles render in MapLibre (native Tauri build).
- [x] Glyph label rendering fixed (no placeholder white squares).
- [x] USB root bundle refreshed with rebuilt `motis-ipc` + `motis-gui-svelte`.
- [x] IPC protocol passthrough now covers major interactive endpoints (`trip`, `stoptimes`, `map/trips`, `map/initial`, `map/stops`, `map/levels`, `one-to-many`, `one-to-all`, `rentals`).

### 1. "Connection Refused" / Frontend Loading Wrong HTML
**Status:** ✅ **FIXED** - Use `cargo tauri build` instead of `cargo build --release`

**Issue:** Tauri was showing "Connection refused" because it couldn't find the frontend assets when the binary was moved from the build directory.

**Root Cause Discovered:**
1. **Config parent hardcoding:** Tauri v2 embeds the `config_parent` path (directory containing `tauri.conf.json`) at build time. When using `cargo build --release`, this path is hardcoded to the build machine's path (`/home/dator/motis-test/gui/src-tauri/`). When the binary is moved to the USB bundle, it still looks for `src/` in the original build location.

2. **Invalid devUrl:** `"devUrl": ""` is not valid in Tauri v2 (must be `null` or a valid URI).

**Fix Applied:**

1. Fixed `tauri.conf.json`:
```json
{
  "build": {
    "frontendDist": "src",
    "devUrl": null,  // Changed from "" to null
    ...
  }
}
```

2. Use `cargo tauri build` instead of `cargo build --release`:
```bash
cd ~/motis-test/gui/src-tauri
cargo tauri build  # Properly embeds assets and uses custom-protocol
cp target/release/motis-gui /run/media/dator/1CB7A3D87F361348/usb-bundle/
```

**Why `cargo tauri build` is needed:**
- It compiles with the `custom-protocol` feature which embeds frontend assets into the binary
- It properly resolves paths for distribution
- The resulting binary works regardless of where it's placed

**Verification:**
- [x] App starts without "Connection refused" error
- [x] Debug panel shows "Tauri v2 API found at __TAURI__.core.invoke"
- [x] IPC mode initializes correctly
- [x] Frontend loads from embedded assets, not filesystem

---

### 2. Tauri Core Loading (RESOLVED)
**Status:** ✅ Fixed in `gui/src-tauri/src/index.html`  
**Issue:** `TAURI.core not found` / `invoke not found in __TAURI__`

**Root Cause:**
- Line 9 had: `window.__TAURI__ = window.__TAURI__ || {};`
- This polyfill was **overwriting** Tauri's injected API with an empty object!

**Fix Applied:**
- Removed the polyfill
- Fixed script structure (was malformed with code outside `<script>` tags)
- Added proper Tauri v2 API detection for `window.__TAURI__.core.invoke`

---

### 3. IPC Mode vs Server Mode (RESOLVED)
**Status:** ✅ Fixed in Rust code  
**Issue:** When IPC failed, code fell back to server mode trying `localhost:8080`

**Fix Applied:**
- Modified `native.rs` to **only** use IPC mode (no server fallback)
- Better error messages for missing files
- Added `ensure_executable()` helper to handle NTFS/FAT32 permission issues by copying to `/tmp`

---

## 🟡 HIGH PRIORITY - Pending Testing

### 4. Verify End-to-End IPC Communication
**Status:** ✅ **COMPLETED**  
**Test checklist:**
- [x] `init_native(data_path)` initializes MOTIS backend
- [x] `geocode_cmd(query)` returns location results
- [x] `plan_route_cmd(...)` returns route options
- [x] `check_data_path_exists(path)` validates data directory

### 5. Data Path Resolution on USB
**Status:** ✅ Partially tested  
**Test Cases:**
- [x] Run from `/media/user/USB/motis-gui/` with `./data`
- [ ] Run from `/tmp/` (RUN.sh FAT32 workaround)
- [ ] Custom absolute path works
- [ ] Symlink to USB data directory works

### 6. motis-ipc Process Management
**Status:** ✅ Partially verified  
**Checks:**
- [x] Process spawns correctly on NTFS USB
- [x] Process terminates on GUI close
- [ ] No zombie processes left behind
- [ ] Handles motis-ipc crashes gracefully

---

## 🟢 MEDIUM PRIORITY

### 7. Map Integration
**Status:** ✅ **COMPLETED**  
**Current:** Leaflet map with OpenStreetMap tiles  
**Features:**
- [x] OpenStreetMap tiles via Leaflet
- [x] Display route geometry on map (polylines)
- [x] Show start/end markers
- [x] Auto-fit map to show routes
- [ ] Show intermediate stops/transfers
- [ ] Current location marker
- [ ] Switch to vector tiles (match web UI)

### 8. UI Polish
**Status:** 📋 Planned  
**Tasks:**
- [ ] Better loading states during initialization
- [ ] Error message styling
- [ ] Dark mode support
- [ ] Responsive layout for small screens
- [ ] Better route display (timeline view)

### 9. Build Automation
**Status:** 📋 Planned  
**Tasks:**
- [ ] Create `build-usb-bundle.sh` script
- [ ] Cross-compilation for Windows
- [ ] Cross-compilation for macOS
- [ ] Strip debug symbols for smaller binaries
- [ ] Use `cargo tauri build` to embed HTML in binary (no `src` folder needed)

---

## 🟣 MAJOR PROJECT: Full Svelte UI Integration

### Overview
Port the complete Svelte web UI (`ui/` - 77 components) to work in Tauri via custom protocol + IPC.

**Location:** `gui-svelte/`
**Reference:** `PROJECT_STATUS.md` for detailed architecture

### Phase 1: Foundation
- [ ] Copy `gui/src-tauri/src/native.rs` to `gui-svelte/`
- [ ] Create basic Tauri app structure
- [ ] Test IPC communication

### Phase 2: Custom Protocol Handler
- [ ] Register `motis://` custom protocol in Tauri
- [ ] Create HTTP-to-IPC router
- [ ] Handle JSON responses

### Phase 3: Core API (Existing)
- [ ] `GET /api/v1/plan` → `plan_route_cmd`
- [ ] `GET /api/v1/geocode` → `geocode_cmd`
- [ ] `GET /api/v1/reverse-geocode` → `reverse_geocode_cmd`

### Phase 4: Vector Tiles (CRITICAL) 🟡 PARTIAL
- [x] C++: Add `get_tile(z, x, y)` to native API
- [x] Rust: Add `get_tile_sync` IPC command
- [x] Handle binary MVT data via base64 in IPC
- [x] Serve tiles via custom protocol (200KB+ tiles being served!)
- [x] **DEBUG FIXED:** Tiles render visually in MapLibre GL JS
- [x] Glyph endpoint wired through IPC and rendering correctly

### Current Blocker: Tile Rendering
**Status:** ✅ Resolved.

**Resolved with:**
```
- Tile payload inflate handling in protocol layer
- Real glyph loading path via `get_glyph` IPC command
```

**Next steps:**
1. Continue validating remaining interactive controls against protocol endpoints.
2. Add any missing endpoint passthroughs discovered during UI testing.
3. Reduce/clean legacy unused protocol handlers once behavior is fully stable.

### Phase 5: Advanced Features
- [ ] **Stop Times:** `GET /api/v1/stoptimes` → schedules
- [ ] **Isochrones:** `GET /api/v1/isochrones` → travel time maps
- [ ] **RailViz:** `GET /api/v1/trips` → real-time vehicles

### Phase 6: Build Integration
- [ ] Build script: `ui/` → `gui-svelte/src-tauri/web-build/`
- [ ] Configure `tauri.conf.json` for Svelte build
- [ ] Create unified build script

### Challenges
| Issue | Approach |
|-------|----------|
| Binary tile data | Base64 encode or shared memory |
| MapLibre GL JS | Works in Tauri, needs tile endpoint |
| Web workers | Adjust baseUrl to custom protocol |
| Real-time updates | Convert polling to Tauri events |

---

## 🔵 LOW PRIORITY / FUTURE

### 10. Windows Support
**Status:** 📋 Future  
**Tasks:**
- [ ] Build Windows executable
- [ ] Test FAT32/exFAT compatibility
- [ ] Windows batch launcher
- [ ] Windows desktop shortcut

### 11. macOS Support
**Status:** 📋 Future  
**Tasks:**
- [ ] Build macOS app bundle
- [ ] Code signing (optional)
- [ ] Test on Intel and Apple Silicon

### 12. Additional Features
**Status:** 📋 Ideas  
**Ideas:**
- [ ] Favorite locations
- [ ] Recent searches
- [ ] Departure/arrival time selection
- [ ] Transit mode filtering
- [ ] Export route to PDF/image

---

## ✅ COMPLETED

- [x] Native API wrapper (`native/api.cc`)
- [x] IPC bridge executable (`motis-ipc`)
- [x] Tauri app structure
- [x] Rust backend commands with IPC-only mode
- [x] Frontend HTML/JS with proper Tauri v2 API
- [x] USB bundle packaging
- [x] FAT32 launcher script (`RUN.sh`)
- [x] Desktop entry (`MOTIS-Transit.desktop`)
- [x] Build system integration (CMake)
- [x] Fixed Tauri `frontendDist` path configuration
- [x] Removed Tauri API polyfill that was breaking injection
- [x] Debug console hidden by default, enabled with `--debug` CLI flag
- [x] Fixed route planning JSON serialization (nested `from`/`to` objects)
- [x] Map integration with Leaflet and OpenStreetMap tiles

---

## Debug Notes

### File Locations

**Source files (correct HTML with IPC):**
```
~/motis-test/gui/src-tauri/src/index.html     (31,088 bytes - custom HTML)
~/motis-test/gui/src/index.html               (24,645 bytes - Svelte UI)
```

**USB bundle:**
```
/run/media/dator/1CB7A3D87F361348/usb-bundle/
├── motis-gui              ← executable
├── motis-ipc              ← IPC bridge
├── data/                  ← GTFS/OSM data
├── src/index.html         ← must be custom HTML (31KB), NOT Svelte UI (24KB)
└── ...
```

### Key Configuration Files

**`gui/src-tauri/tauri.conf.json`:**
```json
{
  "build": {
    "frontendDist": "src",      // Points to gui/src-tauri/src/
    "devUrl": ""                  // Empty = no dev server fallback
  }
}
```

**`gui/src-tauri/capabilities/default.json`:**
```json
{
  "permissions": [
    "core:default",
    "opener:default"
  ]
}
```

### Troubleshooting Commands

```bash
# Run GUI with debug console visible
/run/media/dator/*/usb-bundle/motis-gui --debug

# Check which HTML is being used
md5sum ~/motis-test/gui/src-tauri/src/index.html
md5sum /run/media/dator/1CB7A3D87F361348/usb-bundle/src/index.html

# Verify binary has IPC code
strings /run/media/dator/*/usb-bundle/motis-gui | grep "init_native"

# Check for localhost references (should be none in new binary)
strings /run/media/dator/*/usb-bundle/motis-gui | grep "localhost:8080"
```

---

## Resources

- [Tauri v2 Migration Guide](https://tauri.app/start/migrate/from-tauri-1/)
- [Tauri v2 Core Module](https://tauri.app/reference/javascript/api/namespacecore/)
- [Tauri Security - Capabilities](https://tauri.app/security/capabilities/)
