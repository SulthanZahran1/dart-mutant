#!/usr/bin/env bash
set -euo pipefail

# dart_mutant installer — auto-detects OS + arch, downloads pre-built binary
# Usage: curl -fsSL https://raw.githubusercontent.com/SulthanZahran1/dart-mutant/main/scripts/install.sh | bash

REPO="SulthanZahran1/dart-mutant"
INSTALL_DIR="/usr/local/bin"
FALLBACK_DIR="$HOME/.local/bin"

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux*)  OS="unknown-linux-musl" ;;   # fully static musl build — runs on any Linux
    Darwin*) OS="apple-darwin" ;;
    MINGW*|MSYS*|CYGWIN*) OS="pc-windows-msvc" ;;
    *) echo "❌ Unsupported OS: $OS" >&2; exit 1 ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)  ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "❌ Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

# For Windows, use zip instead of tar.gz
if [[ "$OS" == "pc-windows-msvc" ]]; then
    SUFFIX="zip"
else
    SUFFIX="tar.gz"
fi

BINARY_NAME="dart_mutant"
ASSET_NAME="${BINARY_NAME}-${ARCH}-${OS}.${SUFFIX}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ASSET_NAME}"

echo "📥 Downloading dart_mutant (${ARCH}-${OS})..."
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

if ! curl -fsSL "$DOWNLOAD_URL" -o "$TMPDIR/$ASSET_NAME"; then
    echo "❌ Failed to download. Check that release exists: $DOWNLOAD_URL" >&2
    exit 1
fi

echo "📦 Extracting..."
if [[ "$SUFFIX" == "zip" ]]; then
    unzip -o "$TMPDIR/$ASSET_NAME" -d "$TMPDIR"
else
    tar xzf "$TMPDIR/$ASSET_NAME" -C "$TMPDIR"
fi

# Determine install directory
if [ -w "$INSTALL_DIR" ]; then
    TARGET="$INSTALL_DIR"
elif [ -w "$FALLBACK_DIR" ]; then
    TARGET="$FALLBACK_DIR"
    mkdir -p "$TARGET"
else
    TARGET="$FALLBACK_DIR"
    mkdir -p "$TARGET"
fi

mv "$TMPDIR/$BINARY_NAME" "$TARGET/$BINARY_NAME"
chmod +x "$TARGET/$BINARY_NAME"

# Verify
if "$TARGET/$BINARY_NAME" --version >/dev/null 2>&1; then
    echo "✅ dart_mutant installed to $TARGET/$BINARY_NAME"
    "$TARGET/$BINARY_NAME" --version
    if [ "$TARGET" == "$FALLBACK_DIR" ]; then
        echo ""
        echo "⚠️  Installed to $FALLBACK_DIR — ensure it's on your PATH:"
        echo "    export PATH=\"$FALLBACK_DIR:\$PATH\""
    fi
else
    echo "❌ Installation failed — binary verification failed" >&2
    exit 1
fi
