#!/usr/bin/env bash
# Build helpers-native and copy the binary to the project root

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/native"

# Detect the target to determine where cargo puts the binary
TARGET="${CARGO_BUILD_TARGET:-}"
if [ -z "$TARGET" ]; then
    # Use the host target by default
    TARGET="$(rustc --version --verbose | grep '^host:' | awk '{print $2}')" || TARGET=""
fi

# Build in release mode
echo "[build-native] Compiling helpers-native in release mode..."
cargo build --release ${TARGET:+--target "$TARGET"} "$@"

# Determine the binary path based on target
if [ -n "$TARGET" ]; then
    BINARY="native/target/$TARGET/release/helpers-native"
else
    BINARY="native/target/release/helpers-native"
fi

# Handle Windows .exe extension
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    BINARY="${BINARY}.exe"
fi

# Verify the binary exists
if [ ! -f "$BINARY" ]; then
    echo "[build-native] ERROR: Binary not found at $BINARY" >&2
    exit 1
fi

# Copy to project root
echo "[build-native] Copying $BINARY to project root..."
cp "$BINARY" "$ROOT/"
echo "[build-native] Binary copied successfully: $ROOT/$(basename "$BINARY")"
