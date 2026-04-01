#!/usr/bin/env bash

# build-aur-package.sh
#
# Usage:
#   ./scripts/build-aur-package.sh
#
# Description:
#   Generate AUR packaging metadata for the portable release archive.
#
# Options:
#   None.
#
# Examples:
#   ./scripts/build-aur-package.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION_FILE="${ROOT_DIR}/VERSION"
AUR_DIR="${DIST_DIR}/aur"
VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs 2>/dev/null || echo "0.0.0")"
PKG_NAME="github-shell-helpers"
ARCHIVE_NAME="${PKG_NAME}-${VERSION}.tar.gz"
ARCHIVE_PATH="${DIST_DIR}/${ARCHIVE_NAME}"
PKGBUILD_PATH="${AUR_DIR}/PKGBUILD"
SRCINFO_PATH="${AUR_DIR}/.SRCINFO"
SOURCE_URL="https://github.com/RockyWearsAHat/github-shell-helpers/releases/download/v${VERSION}/${ARCHIVE_NAME}"

if [ ! -f "$ARCHIVE_PATH" ]; then
	bash "${ROOT_DIR}/scripts/build-dist.sh"
fi

mkdir -p "$AUR_DIR"
SHA256="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"

printf '%s\n' \
	"pkgname=${PKG_NAME}" \
	"pkgver=${VERSION}" \
	'pkgrel=1' \
	"pkgdesc='Git helpers, MCP tools, and Copilot audit workflow'" \
	"arch=('any')" \
	"url='https://github.com/RockyWearsAHat/github-shell-helpers'" \
	"license=('MIT')" \
	"depends=('bash' 'zsh' 'git' 'curl' 'jq' 'nodejs')" \
	"source=(\"${ARCHIVE_NAME}::${SOURCE_URL}\")" \
	"sha256sums=('${SHA256}')" \
	'' \
	'package() {' \
	'  install -dm755 "$pkgdir/usr/bin" "$pkgdir/usr/share/man"' \
	"  cp -R \"\$srcdir/${PKG_NAME}-${VERSION}/bin/.\" \"\$pkgdir/usr/bin/\"" \
	"  cp -R \"\$srcdir/${PKG_NAME}-${VERSION}/man/.\" \"\$pkgdir/usr/share/man/\"" \
	'}' > "$PKGBUILD_PATH"

printf '%s\n' \
	"pkgbase = ${PKG_NAME}" \
	"\tpkgdesc = Git helpers, MCP tools, and Copilot audit workflow" \
	"\tpkgver = ${VERSION}" \
	'\tpkgrel = 1' \
	"\turl = https://github.com/RockyWearsAHat/github-shell-helpers" \
	'\tarch = any' \
	'\tlicense = MIT' \
	'\tdepends = bash' \
	'\tdepends = zsh' \
	'\tdepends = git' \
	'\tdepends = curl' \
	'\tdepends = jq' \
	'\tdepends = nodejs' \
	"\tsource = ${ARCHIVE_NAME}::${SOURCE_URL}" \
	"\tsha256sums = ${SHA256}" \
	'' \
	"pkgname = ${PKG_NAME}" > "$SRCINFO_PATH"

echo "[build-aur-package] Wrote ${PKGBUILD_PATH}"
echo "[build-aur-package] Wrote ${SRCINFO_PATH}"