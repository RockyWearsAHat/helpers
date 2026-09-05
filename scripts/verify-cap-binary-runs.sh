#!/usr/bin/env bash
# Verify cap-binary-runs gate: ensure helpers-native binary is in project root and MCP schemas work

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Determine binary name based on platform
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    BINARY="$ROOT/helpers-native.exe"
else
    BINARY="$ROOT/helpers-native"
fi

# Gate check 1: Binary exists in project root
if [ ! -f "$BINARY" ]; then
    echo "cap-binary-runs FAILED: Binary not found at $BINARY" >&2
    exit 1
fi

echo "✓ Binary exists at $BINARY"

# Gate check 2: Binary can run schemas command and output valid JSON
if ! output=$("$BINARY" schemas 2>&1); then
    echo "cap-binary-runs FAILED: Binary failed to run schemas command" >&2
    exit 1
fi

# Gate check 3: Output starts with [ (JSON array)
if ! echo "$output" | grep -q '^\['; then
    echo "cap-binary-runs FAILED: Output does not look like JSON array" >&2
    exit 1
fi

# Gate check 4: Check for expected tool names in output
if ! echo "$output" | grep -q '"checkpoint"' || ! echo "$output" | grep -q '"lint"'; then
    echo "cap-binary-runs FAILED: Expected tools not found in schemas" >&2
    exit 1
fi

echo "✓ Binary outputs valid JSON with MCP tools"

echo "✓ All expected MCP tools are present"
echo ""
echo "✅ cap-binary-runs gate PASSED"
exit 0
