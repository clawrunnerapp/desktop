#!/bin/bash
set -euo pipefail

# Prepare OpenClaw bundle for Tauri resources
# Downloads Node.js binary and copies OpenClaw dist + node_modules

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
RESOURCES_DIR="$APP_DIR/src-tauri/resources"
OPENCLAW_SRC="${OPENCLAW_SRC:-$APP_DIR/../openclaw}"

NODE_VERSION="v22.15.0"

# Detect platform
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  darwin) PLATFORM="darwin" ;;
  linux)  PLATFORM="linux" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) NODE_ARCH="x64" ;;
  arm64|aarch64) NODE_ARCH="arm64" ;;
  *)             echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

NODE_FILENAME="node-${NODE_VERSION}-${PLATFORM}-${NODE_ARCH}"
NODE_URL="https://nodejs.org/dist/${NODE_VERSION}/${NODE_FILENAME}.tar.xz"

echo "=== OpenClaw Desktop Bundle Preparation ==="
echo "Platform: ${PLATFORM}-${NODE_ARCH}"
echo "Node.js: ${NODE_VERSION}"
echo "OpenClaw source: ${OPENCLAW_SRC}"
echo ""

# Create resources directory
mkdir -p "$RESOURCES_DIR/openclaw"

# Download Node.js binary if not present
if [ ! -f "$RESOURCES_DIR/node" ]; then
  echo ">>> Downloading Node.js ${NODE_VERSION} for ${PLATFORM}-${NODE_ARCH}..."
  TMPDIR="$(mktemp -d)"
  curl -sL "$NODE_URL" | tar xJ -C "$TMPDIR"
  cp "$TMPDIR/${NODE_FILENAME}/bin/node" "$RESOURCES_DIR/node"
  chmod +x "$RESOURCES_DIR/node"
  rm -rf "$TMPDIR"
  echo "    Node.js binary: $(du -sh "$RESOURCES_DIR/node" | cut -f1)"
else
  echo ">>> Node.js binary already present, skipping download"
fi

# Copy OpenClaw dist
if [ -d "$OPENCLAW_SRC/dist" ]; then
  echo ">>> Copying OpenClaw dist..."
  rm -rf "$RESOURCES_DIR/openclaw/dist"
  cp -r "$OPENCLAW_SRC/dist" "$RESOURCES_DIR/openclaw/dist"
  echo "    dist: $(du -sh "$RESOURCES_DIR/openclaw/dist" | cut -f1)"
else
  echo "WARNING: OpenClaw dist not found at $OPENCLAW_SRC/dist"
  echo "         Build OpenClaw first: cd $OPENCLAW_SRC && pnpm build"
fi

# Copy OpenClaw entry point
if [ -f "$OPENCLAW_SRC/openclaw.mjs" ]; then
  echo ">>> Copying openclaw.mjs..."
  cp "$OPENCLAW_SRC/openclaw.mjs" "$RESOURCES_DIR/openclaw/openclaw.mjs"
else
  echo "WARNING: openclaw.mjs not found at $OPENCLAW_SRC/openclaw.mjs"
fi

# Copy node_modules (production only)
if [ -d "$OPENCLAW_SRC/node_modules" ]; then
  echo ">>> Copying node_modules (this may take a moment)..."
  rm -rf "$RESOURCES_DIR/openclaw/node_modules"
  cp -r "$OPENCLAW_SRC/node_modules" "$RESOURCES_DIR/openclaw/node_modules"
  echo "    node_modules: $(du -sh "$RESOURCES_DIR/openclaw/node_modules" | cut -f1)"
else
  echo "WARNING: node_modules not found at $OPENCLAW_SRC/node_modules"
  echo "         Install deps first: cd $OPENCLAW_SRC && pnpm install"
fi

# Copy package.json for module resolution
if [ -f "$OPENCLAW_SRC/package.json" ]; then
  cp "$OPENCLAW_SRC/package.json" "$RESOURCES_DIR/openclaw/package.json"
fi

echo ""
echo "=== Bundle Summary ==="
echo "Total resources: $(du -sh "$RESOURCES_DIR" | cut -f1)"
echo "Done!"
