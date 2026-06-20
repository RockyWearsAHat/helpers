#!/usr/bin/env bash

# build-deb.sh
#
# Usage:
#   ./scripts/build-deb.sh
#
# Description:
#   Build a Debian package from the shared release manifest.
#
# Options:
#   None.
#
# Examples:
#   ./scripts/build-deb.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION_FILE="${ROOT_DIR}/VERSION"
DEB_BUILD_ROOT="${ROOT_DIR}/build/deb-root"
DEBIAN_DIR="${DEB_BUILD_ROOT}/DEBIAN"
PKG_BIN="${DEB_BUILD_ROOT}/usr/bin"
PKG_LIB="${PKG_BIN}/lib"
PKG_SCRIPTS="${PKG_BIN}/scripts"
PKG_MAN="${DEB_BUILD_ROOT}/usr/share/man/man1"

# shellcheck source=./package-manifest.sh
source "${ROOT_DIR}/scripts/package-manifest.sh"

if ! command -v dpkg-deb >/dev/null 2>&1; then
	echo "[build-deb] ERROR: dpkg-deb is required to build .deb packages." >&2
	exit 1
fi

VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs 2>/dev/null || echo "0.0.0")"
DEB_PATH="${DIST_DIR}/github-shell-helpers_${VERSION}_all.deb"

copy_exec() {
	local src="$1"
	local dest="$2"
	cp "$src" "$dest"
	chmod 755 "$dest"
}

mkdir -p "$DIST_DIR"
rm -rf "$DEB_BUILD_ROOT"
rm -f "$DEB_PATH"
mkdir -p "$DEBIAN_DIR" "$PKG_BIN" "$PKG_LIB" "$PKG_SCRIPTS" "$PKG_MAN"

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	copy_exec "${ROOT_DIR}/${command_file}" "${PKG_BIN}/${command_file}"
done < <(helpers_core_commands)

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	copy_exec "${ROOT_DIR}/${command_file}" "${PKG_BIN}/${command_file}"
done < <(helpers_audit_commands)

while IFS= read -r shell_lib; do
	[ -n "$shell_lib" ] || continue
	cp "${ROOT_DIR}/lib/${shell_lib}" "${PKG_LIB}/${shell_lib}"
done < <(helpers_shell_libs)

while IFS= read -r support_script; do
	[ -n "$support_script" ] || continue
	copy_exec "${ROOT_DIR}/scripts/${support_script}" "${PKG_SCRIPTS}/${support_script}"
done < <(helpers_support_scripts)

while IFS= read -r data_dir; do
	[ -n "$data_dir" ] || continue
	cp -R "${ROOT_DIR}/${data_dir}" "${PKG_BIN}/${data_dir}"
done < <(helpers_data_dirs)

# Ship VERSION so the postinstall knows which prebuilt binary to download.
cp "${VERSION_FILE}" "${PKG_BIN}/VERSION"

while IFS= read -r man_page; do
	[ -n "$man_page" ] || continue
	cp "${ROOT_DIR}/man/man1/${man_page}" "${PKG_MAN}/${man_page}"
done < <(helpers_man_pages)

printf '%s\n' \
	"Package: github-shell-helpers" \
	"Version: ${VERSION}" \
	"Section: utils" \
	"Priority: optional" \
	"Architecture: all" \
	"Maintainer: RockyWearsAHat" \
	"Depends: bash, git, curl, tar" \
	"Homepage: https://github.com/RockyWearsAHat/helpers" \
	"Description: Native AI-agent tooling (MCP server + CLI), Node-free" \
	" Downloads the prebuilt helpers-native binary for this host on install" \
	" and symlinks the helpers/git-* CLIs — no Node or build toolchain." > "${DEBIAN_DIR}/control"

# Postinstall: download the prebuilt native binary for the host, symlink the CLIs,
# and register the MCP server (Node-free; runs as root → writes /usr/bin).
cat > "${DEBIAN_DIR}/postinst" <<'POSTINST'
#!/bin/bash
set -e
VERSION="$(cat /usr/bin/VERSION 2>/dev/null || echo '')"
if [ -f /usr/bin/scripts/fetch-prebuilt.sh ]; then
	bash /usr/bin/scripts/fetch-prebuilt.sh /usr/bin "$VERSION" --register || \
		echo "helpers: finish setup with: bash /usr/bin/scripts/fetch-prebuilt.sh /usr/bin --register"
fi
exit 0
POSTINST
chmod 0755 "${DEBIAN_DIR}/postinst"

dpkg-deb --build "$DEB_BUILD_ROOT" "$DEB_PATH"

echo "[build-deb] Wrote ${DEB_PATH}"