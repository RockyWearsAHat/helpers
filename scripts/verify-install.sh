#!/usr/bin/env bash
# Verify a Helpers install is a working, fully Node-free native binary. Used by the
# install-test CI across every method (curl installer, prebuilt tarball, .deb,
# .pkg, npm) and OS. NODE plus the build toolchain are hidden first, so a green run
# proves the install needs nothing external — it's a single prebuilt binary.
#
#   usage: verify-install.sh <path-to-helpers-cli>
set -euo pipefail

helpers_cli="${1:?usage: verify-install.sh <path-to-helpers-cli>}"

# Hide node + the build toolchain so success can only come from the native binary.
for tool in node cargo rustc cc gcc clang; do
	p="$(command -v "$tool" 2>/dev/null || true)"
	if [ -n "$p" ] && [ -f "$p" ]; then
		mv "$p" "$p.hidden" 2>/dev/null || sudo mv "$p" "$p.hidden" 2>/dev/null || true
	fi
done
echo "[verify-install] node on PATH after hiding: $(command -v node || echo none)"

if [ ! -e "$helpers_cli" ]; then
	echo "[verify-install] FAIL: helpers CLI not found at $helpers_cli" >&2
	exit 1
fi

# It must be a native binary (or a symlink to one), NOT a Node script.
real="$(readlink -f "$helpers_cli" 2>/dev/null || echo "$helpers_cli")"
if head -c2 "$real" 2>/dev/null | grep -q '#!' && head -n1 "$real" 2>/dev/null | grep -qi 'node'; then
	echo "[verify-install] FAIL: helpers is still a Node script ($real)" >&2
	exit 1
fi

# The CLI runs with no Node and lists tools.
set +e
status_out="$("$helpers_cli" status 2>&1)"
status_rc=$?
set -e
printf '%s\n' "$status_out"
if [ "$status_rc" -ne 0 ]; then
	echo "[verify-install] FAIL: 'helpers status' exited $status_rc" >&2
	exit 1
fi
tools="$(printf '%s\n' "$status_out" | grep -oE '[0-9]+ total' | grep -oE '[0-9]+' | head -1)"
if [ -z "${tools:-}" ] || [ "$tools" -le 0 ]; then
	echo "[verify-install] FAIL: 0 tools after install" >&2
	exit 1
fi

# The MCP server is the same native binary — confirm it speaks MCP with no Node.
native_bin="$(dirname "$real")/helpers-native"
[ -f "${native_bin}.exe" ] && native_bin="${native_bin}.exe"
if [ -x "$native_bin" ]; then
	echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | "$native_bin" mcp >/dev/null 2>&1 ||
		{ echo "[verify-install] FAIL: helpers-native mcp did not respond" >&2; exit 1; }
fi

echo "[verify-install] OK: $tools tools via a Node-free native binary ($helpers_cli)."
