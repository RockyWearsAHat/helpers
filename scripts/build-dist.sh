#!/usr/bin/env bash

# Build script to assemble distributable artifacts for github-shell-helpers.
#
# Currently this just copies the standalone installer into dist/ so it can
# be uploaded as a release asset or CI artifact.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${SCRIPT_DIR}/dist"
VERSION_FILE="${SCRIPT_DIR}/VERSION"

if [ -f "$VERSION_FILE" ]; then
	VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs)"
else
	VERSION="0.0.0"
fi

STABLE_INSTALLER_PATH="${DIST_DIR}/Git-Shell-Helpers-Installer.sh"
VERSIONED_INSTALLER_PATH="${DIST_DIR}/Git-Shell-Helpers-Installer-${VERSION}.sh"

mkdir -p "$DIST_DIR"

rm -f "$STABLE_INSTALLER_PATH" "$VERSIONED_INSTALLER_PATH"

cp "${SCRIPT_DIR}/Git-Shell-Helpers-Installer.sh" "$STABLE_INSTALLER_PATH"
cp "${SCRIPT_DIR}/Git-Shell-Helpers-Installer.sh" "$VERSIONED_INSTALLER_PATH"
chmod +x "$STABLE_INSTALLER_PATH" "$VERSIONED_INSTALLER_PATH"

echo "[build-dist] Wrote dist/Git-Shell-Helpers-Installer.sh"
echo "[build-dist] Wrote dist/Git-Shell-Helpers-Installer-${VERSION}.sh"
