#!/bin/bash
set -euo pipefail

# Prepare OpenClaw bundle for Tauri resources
# Downloads Node.js binary and builds pruned OpenClaw deployment

usage() {
    echo "Usage: $0 [--target TARGET] [--node-version VERSION]"
    echo "  TARGET:  darwin-arm64, darwin-x64, linux-x64, linux-arm64"
    echo "  VERSION: Node.js version (default: 24.13.1)"
    exit 1
}

# Check prerequisites
for cmd in curl tar pnpm; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "Error: required command '$cmd' not found"
        exit 1
    fi
done

# Defaults
NODE_VERSION="24.13.1"
TARGET=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            [[ $# -lt 2 ]] && { echo "Error: --target requires a value"; usage; }
            TARGET="$2"
            shift 2
            ;;
        --node-version)
            [[ $# -lt 2 ]] && { echo "Error: --node-version requires a value"; usage; }
            NODE_VERSION="$2"
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Auto-detect platform if --target not provided
if [[ -z "$TARGET" ]]; then
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"

    case "$OS" in
        darwin) PLATFORM="darwin" ;;
        linux)  PLATFORM="linux" ;;
        *)      echo "Error: Unsupported OS: $OS"; exit 1 ;;
    esac

    case "$ARCH" in
        x86_64|amd64)  NODE_ARCH="x64" ;;
        arm64|aarch64) NODE_ARCH="arm64" ;;
        *)             echo "Error: Unsupported arch: $ARCH"; exit 1 ;;
    esac

    TARGET="${PLATFORM}-${NODE_ARCH}"
else
    # Validate and parse provided target
    case "$TARGET" in
        darwin-arm64|darwin-x64|linux-x64|linux-arm64) ;;
        *) echo "Error: Invalid target: $TARGET"; usage ;;
    esac
    PLATFORM="${TARGET%-*}"
    NODE_ARCH="${TARGET##*-}"
fi

# Paths
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
OPENCLAW_DIR="${OPENCLAW_SRC:-$APP_DIR/../openclaw}"
RESOURCES_DIR="$APP_DIR/src-tauri/resources"

# Temp directory with cleanup trap
TMPDIR_WORK="$(mktemp -d)"
cleanup() { rm -rf "$TMPDIR_WORK"; }
trap cleanup EXIT

# Validate source directories
if [[ ! -d "$OPENCLAW_DIR" ]]; then
    echo "Error: OpenClaw source directory not found at $OPENCLAW_DIR"
    exit 1
fi

# Node.js download URL
NODE_DIST="node-v${NODE_VERSION}-${PLATFORM}-${NODE_ARCH}"
if [[ "$PLATFORM" == "darwin" ]]; then
    NODE_ARCHIVE="${NODE_DIST}.tar.gz"
else
    NODE_ARCHIVE="${NODE_DIST}.tar.xz"
fi
NODE_URL="https://nodejs.org/dist/v${NODE_VERSION}/${NODE_ARCHIVE}"

echo "=== OpenClaw Desktop Bundle Preparation ==="
echo "Target:    ${TARGET}"
echo "Node.js:   v${NODE_VERSION}"
echo "OpenClaw:  ${OPENCLAW_DIR}"
echo "Resources: ${RESOURCES_DIR}"
echo ""

# --- Step 1: Create resources directory ---
mkdir -p "$RESOURCES_DIR/openclaw"

# --- Step 2: Download Node.js binary ---
NODE_VERSION_FILE="$RESOURCES_DIR/.node-version"
if [[ -f "$RESOURCES_DIR/node" ]] && [[ -f "$NODE_VERSION_FILE" ]] && [[ "$(cat "$NODE_VERSION_FILE")" == "v${NODE_VERSION}-${PLATFORM}-${NODE_ARCH}" ]]; then
    echo ">>> Node.js v${NODE_VERSION} binary already present, skipping download"
else
    rm -f "$RESOURCES_DIR/node" "$NODE_VERSION_FILE"
    echo ">>> Downloading Node.js v${NODE_VERSION} for ${TARGET}..."

    if [[ "$PLATFORM" == "darwin" ]]; then
        curl -fsSL "$NODE_URL" | tar xz -C "$TMPDIR_WORK"
    else
        curl -fsSL "$NODE_URL" | tar xJ -C "$TMPDIR_WORK"
    fi

    cp "$TMPDIR_WORK/${NODE_DIST}/bin/node" "$RESOURCES_DIR/node"
    chmod +x "$RESOURCES_DIR/node"
    echo "v${NODE_VERSION}-${PLATFORM}-${NODE_ARCH}" > "$NODE_VERSION_FILE"
    echo "    Node.js binary: $(du -sh "$RESOURCES_DIR/node" | cut -f1)"
fi

# --- Step 3: Build OpenClaw ---
echo ">>> Building OpenClaw..."
(cd "$OPENCLAW_DIR" && pnpm install --frozen-lockfile && pnpm build)

# --- Step 4: Create pruned production deployment ---
echo ">>> Creating pruned production deployment (pnpm deploy --prod)..."
DEPLOY_DIR="$TMPDIR_WORK/openclaw-deploy"
(cd "$OPENCLAW_DIR" && pnpm --filter openclaw deploy --prod "$DEPLOY_DIR")

# --- Step 5: Copy to resources ---
echo ">>> Copying to resources..."

# Verify build outputs exist
for required in "$OPENCLAW_DIR/openclaw.mjs" "$OPENCLAW_DIR/dist" "$DEPLOY_DIR/package.json" "$DEPLOY_DIR/node_modules"; do
    if [[ ! -e "$required" ]]; then
        echo "Error: Required build output not found: $required"
        exit 1
    fi
done

# Entry point
cp "$OPENCLAW_DIR/openclaw.mjs" "$RESOURCES_DIR/openclaw/openclaw.mjs"
echo "    openclaw.mjs copied"

# package.json (needed for ESM module resolution)
cp "$DEPLOY_DIR/package.json" "$RESOURCES_DIR/openclaw/package.json"
echo "    package.json copied"

# dist/ (tsdown bundle)
rm -rf "$RESOURCES_DIR/openclaw/dist"
cp -r "$OPENCLAW_DIR/dist" "$RESOURCES_DIR/openclaw/dist"
echo "    dist: $(du -sh "$RESOURCES_DIR/openclaw/dist" | cut -f1)"

# node_modules/ (pruned production deps with native addons)
rm -rf "$RESOURCES_DIR/openclaw/node_modules"
cp -r "$DEPLOY_DIR/node_modules" "$RESOURCES_DIR/openclaw/node_modules"
echo "    node_modules: $(du -sh "$RESOURCES_DIR/openclaw/node_modules" | cut -f1)"

# --- Summary ---
echo ""
echo "=== Bundle Summary ==="
echo "Node binary:   $(du -sh "$RESOURCES_DIR/node" | cut -f1)"
echo "OpenClaw dist: $(du -sh "$RESOURCES_DIR/openclaw/dist" | cut -f1)"
echo "node_modules:  $(du -sh "$RESOURCES_DIR/openclaw/node_modules" | cut -f1)"
echo "Total:         $(du -sh "$RESOURCES_DIR" | cut -f1)"
echo ""
echo "Done! Resources ready at: $RESOURCES_DIR"
