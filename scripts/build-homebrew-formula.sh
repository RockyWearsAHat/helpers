#!/usr/bin/env bash

# build-homebrew-formula.sh
#
# Usage:
#   ./scripts/build-homebrew-formula.sh
#
# Description:
#   Generate a Homebrew formula that installs the portable release archive.
#
# Options:
#   None.
#
# Examples:
#   ./scripts/build-homebrew-formula.sh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
VERSION_FILE="${ROOT_DIR}/VERSION"
FORMULA_DIR="${DIST_DIR}/homebrew"
VERSION="$(tr -d '\n' <"$VERSION_FILE" | xargs 2>/dev/null || echo "0.0.0")"
ARCHIVE_NAME="github-shell-helpers-${VERSION}.tar.gz"
ARCHIVE_PATH="${DIST_DIR}/${ARCHIVE_NAME}"
FORMULA_PATH="${FORMULA_DIR}/github-shell-helpers.rb"

if [ ! -f "$ARCHIVE_PATH" ]; then
	bash "${ROOT_DIR}/scripts/build-dist.sh"
fi

mkdir -p "$FORMULA_DIR"
SHA256="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"
RELEASE_URL="https://github.com/RockyWearsAHat/github-shell-helpers/releases/download/v${VERSION}/${ARCHIVE_NAME}"

printf '%s\n' \
	'class GithubShellHelpers < Formula' \
	'  desc "Git helpers, MCP tools, and Copilot audit workflow"' \
	'  homepage "https://github.com/RockyWearsAHat/github-shell-helpers"' \
	"  url \"${RELEASE_URL}\"" \
	"  sha256 \"${SHA256}\"" \
	"  version \"${VERSION}\"" \
	'' \
	'  depends_on "git"' \
	'  depends_on "jq"' \
	'  depends_on "node"' \
	'' \
	'  def install' \
	'    prefix.install "bin", "man"' \
	'  end' \
	'' \
	'  def caveats' \
	"    \"The VS Code extension is shipped separately as git-shell-helpers-${VERSION}.vsix. Install it manually after brew install if you want the extension-managed MCP and branch-session features.\"" \
	'  end' \
	'' \
	'  test do' \
	'    assert_match "git-checkpoint", shell_output("#{bin}/git-checkpoint --help")' \
	'  end' \
	'end' > "$FORMULA_PATH"

echo "[build-homebrew-formula] Wrote ${FORMULA_PATH}"