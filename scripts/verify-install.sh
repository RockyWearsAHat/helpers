#!/usr/bin/env bash
# Verify that a Helpers install provisions working native tools WITHOUT any build
# toolchain — i.e. by downloading the prebuilt `helpers-native` for this platform.
# Used by the install-test CI across every install method (curl installer, npm,
# .deb, .pkg, tarball) and OS.
#
#   usage: verify-install.sh <path-to-helpers-cli>
#
# Proof model:
#   1. Hide cargo/rustc/cc so a source build is impossible (defense-in-depth).
#   2. Run `helpers build` and require the "installed prebuilt" path (not a compile).
#   3. Require helpers-native to report > 0 tools.
# Any failure exits non-zero so CI fails loudly.
set -euo pipefail

helpers_cli="${1:?usage: verify-install.sh <path-to-helpers-cli>}"
node_bin="${NODE_BIN:-node}"

# Hide any build toolchain so success can only come from the prebuilt download.
# Best-effort and non-fatal: the assertion on "installed prebuilt" below is the
# real proof regardless of whether hiding succeeds.
for tool in cargo rustc cc gcc clang; do
	p="$(command -v "$tool" 2>/dev/null || true)"
	if [ -n "$p" ] && [ -f "$p" ]; then
		mv "$p" "$p.hidden" 2>/dev/null || sudo mv "$p" "$p.hidden" 2>/dev/null || true
	fi
done

echo "[verify-install] provisioning via: $helpers_cli"
# Capture without letting `set -e` abort on a non-zero `helpers build` — we want
# to print its output and give a clear diagnosis either way.
set +e
build_out="$("$node_bin" "$helpers_cli" build 2>&1)"
build_rc=$?
set -e
printf '%s\n' "$build_out"
echo "[verify-install] helpers build exit code: $build_rc"

if ! printf '%s\n' "$build_out" | grep -q "installed prebuilt"; then
	echo "[verify-install] FAIL: native tools were not provisioned from a prebuilt download." >&2
	echo "[verify-install] (no external build should be required on a supported platform)" >&2
	exit 1
fi

status_out="$("$node_bin" "$helpers_cli" status 2>&1 || true)"
tools="$(printf '%s\n' "$status_out" | grep -oE '[0-9]+ total' | grep -oE '[0-9]+' | head -1)"
if [ -z "${tools:-}" ] || [ "$tools" -le 0 ]; then
	echo "[verify-install] FAIL: 0 tools after install." >&2
	printf '%s\n' "$status_out" >&2
	exit 1
fi

# The native binary must also run standalone, with no Node — proving the heavy
# tools themselves carry no Node dependency (only the CLI/MCP server need Node).
native_bin="$(dirname "$helpers_cli")/helpers-native"
[ -f "${native_bin}.exe" ] && native_bin="${native_bin}.exe"
if [ -x "$native_bin" ]; then
	"$native_bin" schemas >/dev/null 2>&1 || {
		echo "[verify-install] FAIL: helpers-native could not run standalone." >&2
		exit 1
	}
fi

echo "[verify-install] OK: $tools tools provisioned via prebuilt download, no build toolchain used."
