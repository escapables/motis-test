#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../" && pwd)"
USB_BUNDLE="$PROJECT_ROOT/usb-bundle"
USB_TEMPLATE="$SCRIPT_DIR/usb-template"
BUILD_DIR="$PROJECT_ROOT/build"
OFFLINE_MODE=0
SKIP_PNPM_INSTALL=0

usage() {
    cat <<EOF
Usage: $0 [--offline] [--skip-pnpm-install]

Options:
  --offline            Build without network access for pnpm commands.
                       Requires existing local dependencies in node_modules.
                       Implies --skip-pnpm-install.
  --skip-pnpm-install  Skip 'pnpm install' and reuse existing node_modules.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --offline)
            OFFLINE_MODE=1
            SKIP_PNPM_INSTALL=1
            shift
            ;;
        --skip-pnpm-install)
            SKIP_PNPM_INSTALL=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "ERROR: Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

echo "================================"
echo "MOTIS Transit - Svelte UI Build"
echo "================================"
echo ""

command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found"; exit 1; }
cargo tauri --version >/dev/null 2>&1 || { echo "ERROR: cargo-tauri not found (install with: cargo install tauri-cli)"; exit 1; }
command -v pnpm >/dev/null 2>&1 || { echo "ERROR: pnpm not found"; exit 1; }
command -v cmake >/dev/null 2>&1 || { echo "ERROR: cmake not found"; exit 1; }

if [ ! -d "$USB_TEMPLATE" ]; then
    echo "ERROR: Missing USB template directory: $USB_TEMPLATE"
    exit 1
fi

echo "[1/5] Building Svelte UI..."
cd "$PROJECT_ROOT/ui"

if [ "$SKIP_PNPM_INSTALL" -eq 1 ]; then
    echo "Skipping pnpm install (requested)."
    if [ ! -d "$PROJECT_ROOT/ui/node_modules" ] || [ ! -d "$SCRIPT_DIR/node_modules" ]; then
        echo "ERROR: Missing node_modules for offline/skip mode."
        echo "Run once online first: pnpm install"
        exit 1
    fi
else
    if [ "$OFFLINE_MODE" -eq 1 ]; then
        pnpm --offline install --frozen-lockfile --config.confirmModulesPurge=false
    else
        pnpm install --frozen-lockfile --config.confirmModulesPurge=false
    fi
fi

if [ "$OFFLINE_MODE" -eq 1 ]; then
    pnpm --offline --filter @motis-project/motis-client build
    pnpm --offline build
else
    pnpm --filter @motis-project/motis-client build
    pnpm build
fi

echo ""
echo "[2/5] Ensuring native binaries..."
if [ ! -f "$BUILD_DIR/native/motis-ipc" ] || { [ ! -f "$BUILD_DIR/motis" ] && [ ! -f "$BUILD_DIR/motis-transit" ]; }; then
    cmake -S "$PROJECT_ROOT" -B "$BUILD_DIR" -DCMAKE_BUILD_TYPE=Release
    cmake --build "$BUILD_DIR" --target motis motis-ipc -j"$(nproc)"
fi

if [ ! -f "$BUILD_DIR/native/motis-ipc" ]; then
    echo "ERROR: motis-ipc not found at $BUILD_DIR/native/motis-ipc"
    exit 1
fi

if [ -f "$BUILD_DIR/motis" ]; then
    MOTIS_BIN="$BUILD_DIR/motis"
elif [ -f "$BUILD_DIR/motis-transit" ]; then
    MOTIS_BIN="$BUILD_DIR/motis-transit"
else
    echo "ERROR: motis binary not found at $BUILD_DIR/motis or $BUILD_DIR/motis-transit"
    exit 1
fi

echo ""
echo "[3/5] Building Tauri application..."
cd "$SCRIPT_DIR/src-tauri"
cargo tauri build

echo ""
echo "[4/5] Copying to USB bundle..."
mkdir -p "$USB_BUNDLE"
rm -rf "$USB_BUNDLE/ui" "$USB_BUNDLE/data"
rm -f "$USB_BUNDLE"/*.desktop
cp "$USB_TEMPLATE/RUN.sh" "$USB_TEMPLATE/motis-import.sh" "$USB_TEMPLATE/README.txt" "$USB_TEMPLATE/sweden-route-fix.lua" "$USB_BUNDLE/"
cp "$SCRIPT_DIR/src-tauri/target/release/motis-gui-svelte" "$USB_BUNDLE/"
cp "$BUILD_DIR/native/motis-ipc" "$USB_BUNDLE/"
cp "$MOTIS_BIN" "$USB_BUNDLE/motis"
mkdir -p "$USB_BUNDLE/ui"
cp -r "$PROJECT_ROOT/ui/build"/* "$USB_BUNDLE/ui/"
mkdir -p "$USB_BUNDLE/data"

echo ""
echo "[5/5] Setting permissions..."
chmod +x "$USB_BUNDLE/RUN.sh"
chmod +x "$USB_BUNDLE/motis-import.sh"
chmod +x "$USB_BUNDLE/motis-gui-svelte"
chmod +x "$USB_BUNDLE/motis-ipc"
chmod +x "$USB_BUNDLE/motis"

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

echo "Bundle contents:"
ls -lh "$USB_BUNDLE/"
