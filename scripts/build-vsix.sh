#!/usr/bin/env bash

# Build the VS Code extension .vsix, syncing the version from the repo
# VERSION file into vscode-extension/package.json first.
#
# Usage: ./scripts/build-vsix.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
EXT_DIR="${ROOT_DIR}/vscode-extension"
VERSION_FILE="${ROOT_DIR}/VERSION"
PKG_JSON="${EXT_DIR}/package.json"

if [ -f "$VERSION_FILE" ]; then
	VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs)"
else
	echo "[build-vsix] ERROR: VERSION file not found" >&2
	exit 1
fi

# Patch package.json version in-place (portable sed)
if [[ "$OSTYPE" == darwin* ]]; then
	sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" "$PKG_JSON"
else
	sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" "$PKG_JSON"
fi

echo "[build-vsix] Synced version ${VERSION} into package.json"

cd "$EXT_DIR"
rm -f *.vsix
npx @vscode/vsce package --no-dependencies --allow-missing-repository

echo "[build-vsix] Built ${EXT_DIR}/git-shell-helpers-${VERSION}.vsix"
