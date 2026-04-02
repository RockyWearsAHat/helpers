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

# shellcheck source=./package-manifest.sh
source "${ROOT_DIR}/scripts/package-manifest.sh"

PKG_PATH="${DIST_DIR}/github-shell-helpers-${VERSION}.pkg"

echo "[build-pkg] Building Git Shell Helpers ${VERSION} installer..."

rm -rf "$BUILD_DIR" "$COMPONENTS_DIR"
rm -f "$PKG_PATH" "${PKG_PATH%.pkg}-unsigned.pkg"
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

while IFS= read -r cmd; do
  [ -n "$cmd" ] || continue
  if [ -f "${ROOT_DIR}/${cmd}" ]; then
    copy_exec "${ROOT_DIR}/${cmd}" "${CORE_BIN}/${cmd}"
  fi
done < <(gsh_core_commands)

while IFS= read -r lib; do
  [ -n "$lib" ] || continue
  if [ -f "${ROOT_DIR}/lib/${lib}" ]; then
    cp "${ROOT_DIR}/lib/${lib}" "${CORE_LIB}/${lib}"
  fi
done < <(gsh_shell_libs)

while IFS= read -r man; do
  [ -n "$man" ] || continue
  if [ -f "${ROOT_DIR}/man/man1/${man}" ]; then
    cp "${ROOT_DIR}/man/man1/${man}" "${CORE_MAN}/${man}"
  fi
done < <(gsh_core_man_pages)

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

while IFS= read -r entry; do
  [ -n "$entry" ] || continue
  if [ -f "${ROOT_DIR}/${entry}" ]; then
    copy_exec "${ROOT_DIR}/${entry}" "${MCP_BIN}/${entry}"
  fi
done < <(gsh_mcp_commands)

while IFS= read -r lib; do
  [ -n "$lib" ] || continue
  if [ -f "${ROOT_DIR}/lib/${lib}" ]; then
    cp "${ROOT_DIR}/lib/${lib}" "${MCP_LIB}/${lib}"
  fi
done < <(gsh_mcp_libs)

while IFS= read -r man; do
  [ -n "$man" ] || continue
  if [ -f "${ROOT_DIR}/man/man1/${man}" ]; then
    cp "${ROOT_DIR}/man/man1/${man}" "${MCP_MAN}/${man}"
  fi
done < <(gsh_mcp_man_pages)

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

while IFS= read -r cmd; do
  [ -n "$cmd" ] || continue
  if [ -f "${ROOT_DIR}/${cmd}" ]; then
    copy_exec "${ROOT_DIR}/${cmd}" "${AUDIT_BIN}/${cmd}"
  fi
done < <(gsh_audit_commands)

while IFS= read -r data_dir; do
  [ -n "$data_dir" ] || continue
  if [ -d "${ROOT_DIR}/${data_dir}" ]; then
    cp -R "${ROOT_DIR}/${data_dir}" "${AUDIT_DATA}/${data_dir}"
  fi
done < <(gsh_data_dirs)

while IFS= read -r support_script; do
  [ -n "$support_script" ] || continue
  if [ -f "${ROOT_DIR}/scripts/${support_script}" ]; then
    cp "${ROOT_DIR}/scripts/${support_script}" "${AUDIT_SCRIPTS}/${support_script}"
    chmod +x "${AUDIT_SCRIPTS}/${support_script}"
  fi
done < <(gsh_support_scripts)

ln -sf "/usr/local/share/github-shell-helpers/copilot-config" "${AUDIT_BIN}/copilot-config"
ln -sf "/usr/local/share/github-shell-helpers/community-cache" "${AUDIT_BIN}/community-cache"
ln -sf "/usr/local/share/github-shell-helpers/scripts" "${AUDIT_BIN}/scripts"
ln -sf "/usr/local/share/github-shell-helpers/templates" "${AUDIT_BIN}/templates"

while IFS= read -r man; do
  [ -n "$man" ] || continue
  if [ -f "${ROOT_DIR}/man/man1/${man}" ]; then
    cp "${ROOT_DIR}/man/man1/${man}" "${AUDIT_MAN}/${man}"
  fi
done < <(gsh_audit_man_pages)

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

# ── Sign & Notarize (optional) ───────────────────────────────────────────────
#
# Set these environment variables to enable signing and notarization:
#   PKG_SIGN_IDENTITY   — "Developer ID Installer: Name (TEAMID)"
#   NOTARIZE_APPLE_ID   — Apple ID email for notarytool
#   NOTARIZE_PASSWORD    — App-specific password or keychain reference
#   NOTARIZE_TEAM_ID    — 10-char Apple Developer Team ID

if [ -n "${PKG_SIGN_IDENTITY:-}" ]; then
  echo "[build-pkg] Signing with: ${PKG_SIGN_IDENTITY}"
  UNSIGNED_PATH="${PKG_PATH%.pkg}-unsigned.pkg"
  mv "$PKG_PATH" "$UNSIGNED_PATH"

  productsign --sign "${PKG_SIGN_IDENTITY}" "$UNSIGNED_PATH" "$PKG_PATH"
  rm -f "$UNSIGNED_PATH"

  pkgutil --check-signature "$PKG_PATH"
  echo "[build-pkg] ✓ Package signed"

  if [ -n "${NOTARIZE_APPLE_ID:-}" ] && [ -n "${NOTARIZE_PASSWORD:-}" ] && [ -n "${NOTARIZE_TEAM_ID:-}" ]; then
    echo "[build-pkg] Submitting for notarization..."
    xcrun notarytool submit "$PKG_PATH" \
      --apple-id "${NOTARIZE_APPLE_ID}" \
      --password "${NOTARIZE_PASSWORD}" \
      --team-id "${NOTARIZE_TEAM_ID}" \
      --wait --timeout 15m

    xcrun stapler staple "$PKG_PATH"
    echo "[build-pkg] ✓ Package notarized and stapled"
  else
    echo "[build-pkg] ⚠ Signed but not notarized (set NOTARIZE_APPLE_ID, NOTARIZE_PASSWORD, NOTARIZE_TEAM_ID)"
  fi
else
  echo "[build-pkg] ⚠ Package is unsigned (set PKG_SIGN_IDENTITY to sign)"
fi

echo ""
echo "[build-pkg] ✓ Built installer: $PKG_PATH"
echo "[build-pkg]   Components: core (${CORE_KB}KB) + mcp (${MCP_KB}KB) + audit (${AUDIT_KB}KB) + vscode (${VSCODE_KB}KB)"