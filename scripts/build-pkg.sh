#!/bin/zsh

# Build a macOS GUI .pkg installer for github-shell-helpers.
#
# Produces a productbuild archive with four selectable components:
#   1. Core Git Commands    (required) — git-upload, git-get, etc. + lib/ + man pages
#   2. MCP Research Tools   (optional) — git-research-mcp, git-shell-helpers-mcp + lib/mcp-*.js
#   3. DevOps Audit Agents  (optional) — audit commands + copilot-config/ + community-cache/
#   4. VS Code Integration  (optional) — VSIX + vision-tool + patches + proposed API
#
# The installer shows a welcome screen, license, component checkboxes, and a
# post-install conclusion page. Core is always selected and cannot be deselected.
#
# Result:
#   dist/github-shell-helpers-<version>.pkg

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build/pkgroot"
COMPONENTS_DIR="${ROOT_DIR}/build/components"
DIST_DIR="${ROOT_DIR}/dist"
PKG_DIR="${ROOT_DIR}/scripts/pkg"
RESOURCES_DIR="${PKG_DIR}/resources"

DATA_ROOT="usr/local/share/github-shell-helpers"

VERSION_FILE="${ROOT_DIR}/VERSION"
if [ -f "$VERSION_FILE" ]; then
  VERSION="$(tr -d '\n' <"$VERSION_FILE")"
else
  VERSION="0.0.0"
fi

PKG_PATH="${DIST_DIR}/github-shell-helpers-${VERSION}.pkg"

echo "[build-pkg] Building Git Shell Helpers ${VERSION} installer..."

rm -rf "$BUILD_DIR" "$COMPONENTS_DIR" "$DIST_DIR"
mkdir -p "$DIST_DIR" "$COMPONENTS_DIR"

bash "${ROOT_DIR}/scripts/build-vsix.sh"

# ── Helper ────────────────────────────────────────────────────────────────────

copy_exec() {
  local src="$1" dest="$2"
  cp "$src" "$dest"
  chmod 755 "$dest"
}

ensure_dir() { mkdir -p "$@"; }

pkg_size_kb() {
  du -sk "$1" 2>/dev/null | awk '{print $1}'
}

# ── Component 1: Core Git Commands ───────────────────────────────────────────

echo "[build-pkg] Assembling core component..."
CORE_ROOT="${BUILD_DIR}/core"
CORE_BIN="${CORE_ROOT}/usr/local/bin"
CORE_LIB="${CORE_ROOT}/usr/local/bin/lib"
CORE_MAN="${CORE_ROOT}/usr/local/share/man/man1"

ensure_dir "$CORE_BIN" "$CORE_LIB" "$CORE_MAN"

for cmd in git-upload git-get git-initialize git-checkpoint \
           git-fucked-the-push git-remerge git-resolve \
           git-scan-for-leaked-envs git-help-i-pushed-an-env \
           git-copilot-quickstart; do
  if [ -f "${ROOT_DIR}/${cmd}" ]; then
    copy_exec "${ROOT_DIR}/${cmd}" "${CORE_BIN}/${cmd}"
  fi
done

for lib in upload-ai-message.sh upload-diff-analysis.sh upload-spinner.sh \
           upload-test-detection.sh upload-test-output.sh \
           env-batch-ops.sh env-cache.sh env-git-ops.sh env-history-clean.sh \
           env-patterns.sh env-scan.sh env-ui.sh \
           quickstart-detect.sh quickstart-models.sh; do
  if [ -f "${ROOT_DIR}/lib/${lib}" ]; then
    cp "${ROOT_DIR}/lib/${lib}" "${CORE_LIB}/${lib}"
  fi
done

for man in git-upload.1 git-get.1 git-initialize.1 git-fucked-the-push.1 \
           git-checkpoint.1; do
  if [ -f "${ROOT_DIR}/man/man1/${man}" ]; then
    cp "${ROOT_DIR}/man/man1/${man}" "${CORE_MAN}/${man}"
  fi
done

chmod +x "${PKG_DIR}/core-scripts/postinstall"
CORE_KB="$(pkg_size_kb "$CORE_ROOT")"

pkgbuild --root "$CORE_ROOT" \
  --scripts "${PKG_DIR}/core-scripts" \
  --identifier "com.rockywearsahat.gsh.core" \
  --version "$VERSION" \
  --install-location / \
  "${COMPONENTS_DIR}/core.pkg"

# ── Component 2: MCP Research Tools ──────────────────────────────────────────

echo "[build-pkg] Assembling MCP tools component..."
MCP_ROOT="${BUILD_DIR}/mcp"
MCP_BIN="${MCP_ROOT}/usr/local/bin"
MCP_LIB="${MCP_ROOT}/usr/local/bin/lib"
MCP_MAN="${MCP_ROOT}/usr/local/share/man/man1"

ensure_dir "$MCP_BIN" "$MCP_LIB" "$MCP_MAN"

copy_exec "${ROOT_DIR}/git-research-mcp" "${MCP_BIN}/git-research-mcp"
copy_exec "${ROOT_DIR}/git-shell-helpers-mcp" "${MCP_BIN}/git-shell-helpers-mcp"

for lib in mcp-google-headless.js mcp-knowledge-index.js mcp-knowledge-rw.js \
           mcp-utils.js mcp-web-search.js; do
  if [ -f "${ROOT_DIR}/lib/${lib}" ]; then
    cp "${ROOT_DIR}/lib/${lib}" "${MCP_LIB}/${lib}"
  fi
done

if [ -f "${ROOT_DIR}/man/man1/git-research-mcp.1" ]; then
  cp "${ROOT_DIR}/man/man1/git-research-mcp.1" "${MCP_MAN}/git-research-mcp.1"
fi

chmod +x "${PKG_DIR}/mcp-scripts/postinstall"
MCP_KB="$(pkg_size_kb "$MCP_ROOT")"

pkgbuild --root "$MCP_ROOT" \
  --scripts "${PKG_DIR}/mcp-scripts" \
  --identifier "com.rockywearsahat.gsh.mcp" \
  --version "$VERSION" \
  --install-location / \
  "${COMPONENTS_DIR}/mcp.pkg"

# ── Component 3: DevOps Audit Agents ─────────────────────────────────────────

echo "[build-pkg] Assembling DevOps Audit component..."
AUDIT_ROOT="${BUILD_DIR}/audit"
AUDIT_BIN="${AUDIT_ROOT}/usr/local/bin"
AUDIT_MAN="${AUDIT_ROOT}/usr/local/share/man/man1"
AUDIT_DATA="${AUDIT_ROOT}/${DATA_ROOT}"
AUDIT_SCRIPTS="${AUDIT_DATA}/scripts"

ensure_dir "$AUDIT_BIN" "$AUDIT_MAN" "$AUDIT_DATA" "$AUDIT_SCRIPTS"

copy_exec "${ROOT_DIR}/git-copilot-devops-audit" "${AUDIT_BIN}/git-copilot-devops-audit"
copy_exec "${ROOT_DIR}/scripts/community-cache-submit.sh" "${AUDIT_BIN}/git-copilot-devops-audit-community-submit"
copy_exec "${ROOT_DIR}/scripts/community-cache-pull.sh" "${AUDIT_BIN}/git-copilot-devops-audit-community-pull"
copy_exec "${ROOT_DIR}/scripts/community-research-submit.sh" "${AUDIT_BIN}/git-copilot-devops-audit-community-research-submit"

if [ -d "${ROOT_DIR}/copilot-config" ]; then
  cp -R "${ROOT_DIR}/copilot-config" "${AUDIT_DATA}/copilot-config"
fi
if [ -d "${ROOT_DIR}/community-cache" ]; then
  cp -R "${ROOT_DIR}/community-cache" "${AUDIT_DATA}/community-cache"
fi
if [ -d "${ROOT_DIR}/templates" ]; then
  cp -R "${ROOT_DIR}/templates" "${AUDIT_DATA}/templates"
fi
if [ -f "${ROOT_DIR}/scripts/build-knowledge-index.js" ]; then
  cp "${ROOT_DIR}/scripts/build-knowledge-index.js" "${AUDIT_SCRIPTS}/build-knowledge-index.js"
  chmod +x "${AUDIT_SCRIPTS}/build-knowledge-index.js"
fi

ln -sf "/usr/local/share/github-shell-helpers/copilot-config" "${AUDIT_BIN}/copilot-config"
ln -sf "/usr/local/share/github-shell-helpers/community-cache" "${AUDIT_BIN}/community-cache"
ln -sf "/usr/local/share/github-shell-helpers/templates" "${AUDIT_BIN}/templates"

if [ -f "${ROOT_DIR}/man/man1/git-copilot-devops-audit.1" ]; then
  cp "${ROOT_DIR}/man/man1/git-copilot-devops-audit.1" "${AUDIT_MAN}/git-copilot-devops-audit.1"
fi

chmod +x "${PKG_DIR}/audit-scripts/postinstall"
AUDIT_KB="$(pkg_size_kb "$AUDIT_ROOT")"

pkgbuild --root "$AUDIT_ROOT" \
  --scripts "${PKG_DIR}/audit-scripts" \
  --identifier "com.rockywearsahat.gsh.audit" \
  --version "$VERSION" \
  --install-location / \
  "${COMPONENTS_DIR}/audit.pkg"

# ── Component 4: VS Code Integration ─────────────────────────────────────────

echo "[build-pkg] Assembling VS Code component..."
VSCODE_ROOT="${BUILD_DIR}/vscode"
VSCODE_DATA="${VSCODE_ROOT}/${DATA_ROOT}"
VSCODE_VSIX="${VSCODE_DATA}/vscode"
VSCODE_SCRIPTS="${VSCODE_DATA}/scripts"
VSCODE_VISION="${VSCODE_DATA}/vision-tool"

ensure_dir "$VSCODE_VSIX" "$VSCODE_SCRIPTS" "$VSCODE_VISION"

VSIX_FILE="${ROOT_DIR}/vscode-extension/git-shell-helpers-${VERSION}.vsix"
if [ -f "$VSIX_FILE" ]; then
  cp "$VSIX_FILE" "$VSCODE_VSIX/"
fi

if [ -f "${ROOT_DIR}/scripts/patch-vscode-apply-all.js" ]; then
  cp "${ROOT_DIR}/scripts/patch-vscode-apply-all.js" "$VSCODE_SCRIPTS/"
fi

for f in mcp-server.js extension.js package.json README.md LICENSE.txt; do
  if [ -f "${ROOT_DIR}/vision-tool/${f}" ]; then
    cp "${ROOT_DIR}/vision-tool/${f}" "$VSCODE_VISION/"
  fi
done

vision_vsix="$(find "${ROOT_DIR}/vision-tool" -maxdepth 1 -name '*.vsix' -print -quit 2>/dev/null || true)"
if [ -n "$vision_vsix" ]; then
  cp "$vision_vsix" "$VSCODE_VISION/"
fi

chmod +x "${PKG_DIR}/vscode-scripts/postinstall"
VSCODE_KB="$(pkg_size_kb "$VSCODE_ROOT")"

pkgbuild --root "$VSCODE_ROOT" \
  --scripts "${PKG_DIR}/vscode-scripts" \
  --identifier "com.rockywearsahat.gsh.vscode" \
  --version "$VERSION" \
  --install-location / \
  "${COMPONENTS_DIR}/vscode.pkg"

# ── Build Distribution XML with real sizes ────────────────────────────────────

echo "[build-pkg] Generating distribution..."
DIST_XML="${BUILD_DIR}/distribution.xml"

sed -e "s/__VERSION__/${VERSION}/g" \
    -e "s/__CORE_KB__/${CORE_KB}/g" \
    -e "s/__MCP_KB__/${MCP_KB}/g" \
    -e "s/__AUDIT_KB__/${AUDIT_KB}/g" \
    -e "s/__VSCODE_KB__/${VSCODE_KB}/g" \
    "${PKG_DIR}/distribution.xml" > "$DIST_XML"

RESOURCES_BUILD="${BUILD_DIR}/resources"
mkdir -p "$RESOURCES_BUILD"
sed "s/__VERSION__/${VERSION}/g" "${RESOURCES_DIR}/welcome.html" > "${RESOURCES_BUILD}/welcome.html"
cp "${RESOURCES_DIR}/license.html" "${RESOURCES_BUILD}/license.html"
cp "${RESOURCES_DIR}/conclusion.html" "${RESOURCES_BUILD}/conclusion.html"

# ── Assemble final product archive ───────────────────────────────────────────

productbuild \
  --distribution "$DIST_XML" \
  --resources "$RESOURCES_BUILD" \
  --package-path "$COMPONENTS_DIR" \
  "$PKG_PATH"

echo ""
echo "[build-pkg] ✓ Built GUI installer: $PKG_PATH"
echo "[build-pkg]   Components: core (${CORE_KB}KB) + mcp (${MCP_KB}KB) + audit (${AUDIT_KB}KB) + vscode (${VSCODE_KB}KB)"
echo ""
echo "[build-pkg] To install: open $PKG_PATH"
echo "[build-pkg] To sign:    productsign --sign 'Developer ID Installer: ...' $PKG_PATH signed.pkg"