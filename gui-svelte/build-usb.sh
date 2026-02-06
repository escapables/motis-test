#!/bin/bash
# Build MOTIS Transit Svelte UI for USB bundle

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../" && pwd)"
USB_BUNDLE="$PROJECT_ROOT/usb-bundle-svelte"

echo "================================"
echo "MOTIS Transit - Svelte UI Build"
echo "================================"
echo ""

# Check prerequisites
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found"; exit 1; }
cargo tauri --version >/dev/null 2>&1 || { echo "ERROR: cargo-tauri not found (install with: cargo install tauri-cli)"; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "ERROR: pnpm not found"; exit 1; }

# Step 1: Build Svelte UI
echo "[1/4] Building Svelte UI..."
cd "$PROJECT_ROOT/ui"
pnpm install
pnpm build

# Step 2: Build Tauri app
echo ""
echo "[2/4] Building Tauri application..."
cd "$SCRIPT_DIR/src-tauri"
cargo tauri build

# Step 3: Copy to USB bundle
echo ""
echo "[3/4] Copying to USB bundle..."
mkdir -p "$USB_BUNDLE"
rm -f "$USB_BUNDLE"/*.desktop

# Copy GUI executable
cp "$SCRIPT_DIR/src-tauri/target/release/motis-gui-svelte" "$USB_BUNDLE/"

# Copy motis-ipc (from native build)
if [ -f "$PROJECT_ROOT/build/native/motis-ipc" ]; then
    cp "$PROJECT_ROOT/build/native/motis-ipc" "$USB_BUNDLE/"
else
    echo "WARNING: motis-ipc not found at build/native/motis-ipc"
    echo "        Please build it first: cmake --build build --target motis-ipc"
fi

# Copy MOTIS import binary (for data import script)
# Preferred build output is build/motis in this repo.
if [ -f "$PROJECT_ROOT/build/motis" ]; then
    cp "$PROJECT_ROOT/build/motis" "$USB_BUNDLE/motis"
elif [ -f "$PROJECT_ROOT/build/motis-transit" ]; then
    # Backward compatibility for older local layouts.
    cp "$PROJECT_ROOT/build/motis-transit" "$USB_BUNDLE/motis"
else
    echo "WARNING: motis binary not found at build/motis (or build/motis-transit)"
    echo "        Import functionality won't work until MOTIS is built."
fi

# Copy UI files (for reference, though embedded in executable)
mkdir -p "$USB_BUNDLE/ui"
cp -r "$PROJECT_ROOT/ui/build"/* "$USB_BUNDLE/ui/" 2>/dev/null || true

# Create data directory placeholder
mkdir -p "$USB_BUNDLE/data"

# Step 4: Make scripts executable
echo ""
echo "[4/4] Setting permissions..."
chmod +x "$USB_BUNDLE/RUN.sh"
chmod +x "$USB_BUNDLE/motis-import.sh"
chmod +x "$USB_BUNDLE/motis-gui-svelte" 2>/dev/null || true
chmod +x "$USB_BUNDLE/motis-ipc" 2>/dev/null || true

echo ""
echo "================================"
echo "Build complete!"
echo "================================"
echo ""
echo "USB bundle location: $USB_BUNDLE"
echo ""
echo "To use:"
echo "  1. Import transit data:"
echo "     cd $USB_BUNDLE"
echo "     ./motis-import.sh /path/to/gtfs.zip /path/to/osm.pbf"
echo ""
echo "  2. Run the application:"
echo "     ./RUN.sh"
echo ""
echo "  3. Or copy $USB_BUNDLE to your USB stick"
echo ""

# Show bundle contents
echo "Bundle contents:"
ls -lh "$USB_BUNDLE/"
