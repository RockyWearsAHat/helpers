#!/usr/bin/env bash

# build-dist.sh
#
# Usage:
#   ./scripts/build-dist.sh
#
# Description:
#   Build portable release artifacts from the shared package manifest.
#   Outputs the standalone installer scripts plus a tar.gz archive containing
#   the full command and support-file tree used by package-manager releases.
#
# Options:
#   None.
#
# Examples:
#   ./scripts/build-dist.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION_FILE="${ROOT_DIR}/VERSION"
RELEASE_ROOT_PARENT="${ROOT_DIR}/build/release-root"

# shellcheck source=./package-manifest.sh
source "${ROOT_DIR}/scripts/package-manifest.sh"

if [ -f "$VERSION_FILE" ]; then
	VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs)"
else
	VERSION="0.0.0"
fi

STABLE_INSTALLER_PATH="${DIST_DIR}/Helpers-Installer.sh"
VERSIONED_INSTALLER_PATH="${DIST_DIR}/Helpers-Installer-${VERSION}.sh"
ARCHIVE_ROOT_NAME="github-shell-helpers-${VERSION}"
ARCHIVE_ROOT="${RELEASE_ROOT_PARENT}/${ARCHIVE_ROOT_NAME}"
ARCHIVE_BIN="${ARCHIVE_ROOT}/bin"
ARCHIVE_LIB="${ARCHIVE_BIN}/lib"
ARCHIVE_SCRIPTS="${ARCHIVE_BIN}/scripts"
ARCHIVE_MAN="${ARCHIVE_ROOT}/man/man1"
TARBALL_PATH="${DIST_DIR}/${ARCHIVE_ROOT_NAME}.tar.gz"
CHECKSUM_PATH="${DIST_DIR}/${ARCHIVE_ROOT_NAME}-checksums.txt"

copy_exec() {
	local src="$1"
	local dest="$2"
	cp "$src" "$dest"
	chmod 755 "$dest"
}

mkdir -p "$DIST_DIR"
rm -f "$STABLE_INSTALLER_PATH" "$VERSIONED_INSTALLER_PATH" "$TARBALL_PATH" "$CHECKSUM_PATH"
rm -rf "$RELEASE_ROOT_PARENT"
mkdir -p "$ARCHIVE_BIN" "$ARCHIVE_LIB" "$ARCHIVE_SCRIPTS" "$ARCHIVE_MAN"

cp "${ROOT_DIR}/Helpers-Installer.sh" "$STABLE_INSTALLER_PATH"
cp "${ROOT_DIR}/Helpers-Installer.sh" "$VERSIONED_INSTALLER_PATH"
chmod +x "$STABLE_INSTALLER_PATH" "$VERSIONED_INSTALLER_PATH"

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	copy_exec "${ROOT_DIR}/${command_file}" "${ARCHIVE_BIN}/${command_file}"
done < <(helpers_core_commands)

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	copy_exec "${ROOT_DIR}/${command_file}" "${ARCHIVE_BIN}/${command_file}"
done < <(helpers_audit_commands)

while IFS= read -r command_file; do
	[ -n "$command_file" ] || continue
	copy_exec "${ROOT_DIR}/${command_file}" "${ARCHIVE_BIN}/${command_file}"
done < <(helpers_mcp_commands)

while IFS= read -r shell_lib; do
	[ -n "$shell_lib" ] || continue
	cp "${ROOT_DIR}/lib/${shell_lib}" "${ARCHIVE_LIB}/${shell_lib}"
done < <(helpers_shell_libs)

while IFS= read -r mcp_lib; do
	[ -n "$mcp_lib" ] || continue
	cp "${ROOT_DIR}/lib/${mcp_lib}" "${ARCHIVE_LIB}/${mcp_lib}"
done < <(helpers_mcp_libs)

while IFS= read -r support_script; do
	[ -n "$support_script" ] || continue
	copy_exec "${ROOT_DIR}/scripts/${support_script}" "${ARCHIVE_SCRIPTS}/${support_script}"
done < <(helpers_support_scripts)

while IFS= read -r data_dir; do
	[ -n "$data_dir" ] || continue
	cp -R "${ROOT_DIR}/${data_dir}" "${ARCHIVE_BIN}/${data_dir}"
done < <(helpers_data_dirs)

# Rust crate sources, staged next to the bins so `helpers build` (run by the
# installer) can compile the native binary in place. Copy sources only — never
# the multi-hundred-MB target/ build cache.
while IFS= read -r crate_dir; do
	[ -n "$crate_dir" ] || continue
	dest="${ARCHIVE_BIN}/${crate_dir}"
	mkdir -p "$dest"
	cp "${ROOT_DIR}/${crate_dir}/Cargo.toml" "$dest/"
	[ -f "${ROOT_DIR}/${crate_dir}/Cargo.lock" ] && cp "${ROOT_DIR}/${crate_dir}/Cargo.lock" "$dest/"
	cp -R "${ROOT_DIR}/${crate_dir}/src" "$dest/src"
done < <(helpers_crate_dirs)

while IFS= read -r man_page; do
	[ -n "$man_page" ] || continue
	cp "${ROOT_DIR}/man/man1/${man_page}" "${ARCHIVE_MAN}/${man_page}"
done < <(helpers_man_pages)

tar -czf "$TARBALL_PATH" -C "$RELEASE_ROOT_PARENT" "$ARCHIVE_ROOT_NAME"

(
	cd "$DIST_DIR"
	# Portable SHA-256: GNU coreutils ships `sha256sum`; macOS/Perl ship
	# `shasum`. Windows Git Bash typically has `sha256sum` but not `shasum`.
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$(basename "$VERSIONED_INSTALLER_PATH")" "$(basename "$TARBALL_PATH")" > "$CHECKSUM_PATH"
	elif command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$(basename "$VERSIONED_INSTALLER_PATH")" "$(basename "$TARBALL_PATH")" > "$CHECKSUM_PATH"
	else
		echo "[build-dist] ERROR: no SHA-256 tool (need sha256sum or shasum)" >&2
		exit 1
	fi
)

echo "[build-dist] Wrote dist/Helpers-Installer.sh"
echo "[build-dist] Wrote dist/Helpers-Installer-${VERSION}.sh"
echo "[build-dist] Wrote dist/${ARCHIVE_ROOT_NAME}.tar.gz"
echo "[build-dist] Wrote dist/${ARCHIVE_ROOT_NAME}-checksums.txt"
