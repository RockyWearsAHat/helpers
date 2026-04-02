#!/usr/bin/env bash

# build-npm-package.sh
#
# Usage:
#   ./scripts/build-npm-package.sh
#
# Description:
#   Sync the npm package version from VERSION and build a publishable .tgz.
#
# Options:
#   None.
#
# Examples:
#   ./scripts/build-npm-package.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION_FILE="${ROOT_DIR}/VERSION"
PKG_JSON="${ROOT_DIR}/package.json"

# shellcheck source=./package-manifest.sh
source "${ROOT_DIR}/scripts/package-manifest.sh"

if ! command -v npm >/dev/null 2>&1; then
	echo "[build-npm-package] ERROR: npm is required to build the npm package." >&2
	exit 1
fi

VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs 2>/dev/null || echo "0.0.0")"

if [[ "$OSTYPE" == darwin* ]]; then
	sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" "$PKG_JSON"
else
	sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" "$PKG_JSON"
fi

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	chmod +x "${ROOT_DIR}/${command_file}"
done < <(gsh_core_commands)

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	chmod +x "${ROOT_DIR}/${command_file}"
done < <(gsh_audit_commands)

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	chmod +x "${ROOT_DIR}/${command_file}"
done < <(gsh_mcp_commands)

mkdir -p "$DIST_DIR"
PACKAGE_FILE="$(cd "$ROOT_DIR" && npm pack --pack-destination "$DIST_DIR" | tail -n 1)"

echo "[build-npm-package] Built ${DIST_DIR}/${PACKAGE_FILE}"