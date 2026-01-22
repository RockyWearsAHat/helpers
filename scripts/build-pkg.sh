#!/bin/zsh

# Build a macOS .pkg installer for github-shell-helpers.
#
# This creates a package that installs:
#   - git-upload, git-get, git-initialize, git-fucked-the-push into /usr/local/bin
#   - their man pages into /usr/local/share/man/man1
#
# Result:
#   dist/github-shell-helpers-<version>.pkg

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build/pkgroot"
DIST_DIR="${ROOT_DIR}/dist"

BIN_PAYLOAD="${BUILD_DIR}/usr/local/bin"
MAN_PAYLOAD="${BUILD_DIR}/usr/local/share/man/man1"

VERSION_FILE="${ROOT_DIR}/VERSION"
if [ -f "$VERSION_FILE" ]; then
  VERSION="$(tr -d '\n' <"$VERSION_FILE")"
else
  VERSION="0.0.0"
fi

IDENTIFIER="com.rockywearsahat.github-shell-helpers"
PKG_PATH="${DIST_DIR}/github-shell-helpers-${VERSION}.pkg"

rm -rf "$BUILD_DIR" "$DIST_DIR"
mkdir -p "$BIN_PAYLOAD" "$MAN_PAYLOAD" "$DIST_DIR"

# Copy scripts
cp "${ROOT_DIR}/git-upload"     "$BIN_PAYLOAD/git-upload"
cp "${ROOT_DIR}/git-get"        "$BIN_PAYLOAD/git-get"
cp "${ROOT_DIR}/git-initialize" "$BIN_PAYLOAD/git-initialize"
cp "${ROOT_DIR}/git-fucked-the-push" "$BIN_PAYLOAD/git-fucked-the-push"
chmod 755 "$BIN_PAYLOAD"/git-*

# Copy man pages
cp "${ROOT_DIR}/man/man1/git-upload.1"     "$MAN_PAYLOAD/git-upload.1"
cp "${ROOT_DIR}/man/man1/git-get.1"        "$MAN_PAYLOAD/git-get.1"
cp "${ROOT_DIR}/man/man1/git-initialize.1" "$MAN_PAYLOAD/git-initialize.1"
cp "${ROOT_DIR}/man/man1/git-fucked-the-push.1" "$MAN_PAYLOAD/git-fucked-the-push.1"

# Build the package (no scripts; pure payload into /usr/local)
pkgbuild --root "$BUILD_DIR" \
  --identifier "$IDENTIFIER" \
  --version "$VERSION" \
  --install-location / \
  "$PKG_PATH"

echo "[build-pkg] Wrote $PKG_PATH"