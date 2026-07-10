#!/usr/bin/env bash

# Helpers-Installer.sh
#
# Standalone installer script that fetches the latest versions of the
# helper commands and man pages from GitHub and installs them into:
#   - ~/bin
#   - ~/man/man1
# then wires PATH and MANPATH into the user's active shell profiles.
#
# Usage (one line):
#   curl -fsSL \
#     https://raw.githubusercontent.com/RockyWearsAHat/helpers/main/Helpers-Installer.sh \
#     | bash

set -euo pipefail

REPO_RAW_BASE="https://raw.githubusercontent.com/RockyWearsAHat/helpers/main"

BIN_DIR="${HOME}/bin"
MAN_DIR="${HOME}/man/man1"
COMMUNITY_SETTINGS_DIR="${HOME}/.copilot"
COMMUNITY_SETTINGS_FILE="${COMMUNITY_SETTINGS_DIR}/devops-audit-community-settings.json"
DEFAULT_COMMUNITY_REPO="RockyWearsAHat/helpers"
DEFAULT_COMMUNITY_BRANCH="main"
SHELL_ENV_DIR="${HOME}/.config/helpers"
LOCAL_PATH_SNIPPET="${SHELL_ENV_DIR}/paths-local.sh"
LOCAL_PATH_SOURCE='[ -f "$HOME/.config/helpers/paths-local.sh" ] && . "$HOME/.config/helpers/paths-local.sh"'

ensure_dir() {
  local dir="$1"
  if [ ! -d "$dir" ]; then
    mkdir -p "$dir"
  fi
}

ensure_line_in_file() {
  local file="$1"
  local line="$2"

  if [ ! -f "$file" ]; then
    touch "$file"
  fi

  if ! grep -qxF "$line" "$file" 2>/dev/null; then
    printf '%s\n' "$line" >>"$file"
  fi
}

write_local_path_snippet() {
  ensure_dir "$SHELL_ENV_DIR"
  printf '%s\n' \
    '# Added by Helpers-Installer.sh.' \
    'case ":${PATH:-}:" in' \
    '  *":$HOME/bin:"*) ;;' \
    '  *) export PATH="$HOME/bin${PATH:+:$PATH}" ;;' \
    'esac' \
    'case ":${MANPATH:-}:" in' \
    '  *":$HOME/man:"*) ;;' \
    '  *) export MANPATH="$HOME/man${MANPATH:+:$MANPATH}" ;;' \
    'esac' \
    >"$LOCAL_PATH_SNIPPET"
}

default_profile_path() {
  case "${SHELL:-/bin/bash}" in
    */zsh|zsh)
      printf '%s\n' "${HOME}/.zshrc"
      ;;
    */bash|bash)
      if [ "$(uname -s)" = "Darwin" ]; then
        printf '%s\n' "${HOME}/.bash_profile"
      else
        printf '%s\n' "${HOME}/.bashrc"
      fi
      ;;
    *)
      printf '%s\n' "${HOME}/.profile"
      ;;
  esac
}

list_shell_profiles() {
  local found=false
  local candidate
  for candidate in \
    "${HOME}/.zshrc" \
    "${HOME}/.zprofile" \
    "${HOME}/.bash_profile" \
    "${HOME}/.bashrc" \
    "${HOME}/.profile"
  do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      found=true
    fi
  done

  if [ "$found" = false ]; then
    default_profile_path
  fi
}

install_shell_path_setup() {
  local profile
  write_local_path_snippet
  while IFS= read -r profile; do
    [ -n "$profile" ] || continue
    ensure_line_in_file "$profile" "$LOCAL_PATH_SOURCE"
  done < <(list_shell_profiles)
}

fetch() {
  local src="$1"
  local dest="$2"

  echo "[Helpers-Installer] Fetching $src -> $dest"
  curl -fsSL "$src" -o "$dest"
}

# Source-build fallback for platforms without a prebuilt binary: clone the repo,
# compile helpers-native with cargo, symlink the CLIs, and register. Needs Rust.
install_from_source() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "[Helpers-Installer] No prebuilt for $(uname -s)/$(uname -m) and no Rust toolchain (cargo)." >&2
    echo "[Helpers-Installer] Install Rust (https://rustup.rs), then re-run this installer." >&2
    return 1
  fi
  local extract_dir; extract_dir="$(mktemp -d "${TMPDIR:-/tmp}/helpers-source.XXXXXX")"
  if ! curl -fsSL "https://codeload.github.com/RockyWearsAHat/helpers/tar.gz/refs/heads/main" | tar -xz -C "$extract_dir"; then
    rm -rf "$extract_dir"; return 1
  fi
  local root; root="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d -name 'helpers-*' | head -n 1)"
  [ -n "$root" ] || { rm -rf "$extract_dir"; return 1; }
  echo "[Helpers-Installer] Building helpers-native from source (cargo)…"
  if ! cargo build --release --manifest-path "$root/native/Cargo.toml"; then
    rm -rf "$extract_dir"; return 1
  fi
  ensure_dir "$BIN_DIR"
  install -m 0755 "$root/native/target/release/helpers-native" "$BIN_DIR/helpers-native" 2>/dev/null ||
    { cp "$root/native/target/release/helpers-native" "$BIN_DIR/helpers-native" && chmod 0755 "$BIN_DIR/helpers-native"; }
  ( cd "$BIN_DIR" && for n in helpers git-resolve git-remerge git-fucked-the-push git-initialize \
      git-get git-scan-for-leaked-envs git-upload git-checkpoint git-help-i-pushed-an-env; do
      ln -sf helpers-native "$n"; done )
  rm -rf "$extract_dir"
  "$BIN_DIR/helpers" install --agent auto || true
  echo "[Helpers-Installer] Installed helpers-native from source."
  return 0
}

# Node-free primary install: fetch the shared bootstrap (scripts/fetch-prebuilt.sh)
# and run it to download the prebuilt binary, symlink the CLIs, and register. Keeps
# the curl|bash installer self-contained (the bootstrap is pulled from raw). Returns
# non-zero if there's no prebuilt for this platform (caller falls back to source).
fetch_and_register_prebuilt() {
  local version="$1"
  local boot; boot="$(mktemp "${TMPDIR:-/tmp}/helpers-boot.XXXXXX")"
  if ! curl -fsSL -o "$boot" "$REPO_RAW_BASE/scripts/fetch-prebuilt.sh" 2>/dev/null; then
    rm -f "$boot"; return 1
  fi
  if bash "$boot" "$BIN_DIR" "$version" --register; then rm -f "$boot"; return 0; fi
  rm -f "$boot"; return 1
}


write_community_settings() {
  local mode="$1"
  local local_clone=""

  if [ -f "$(pwd)/community-cache/manifest.json" ]; then
    local_clone="$(pwd)"
  fi

  ensure_dir "$COMMUNITY_SETTINGS_DIR"
  cat >"$COMMUNITY_SETTINGS_FILE" <<EOF
{
  "schemaVersion": 1,
  "mode": "${mode}",
  "communityRepo": "${DEFAULT_COMMUNITY_REPO}",
  "baseBranch": "${DEFAULT_COMMUNITY_BRANCH}",
  "branchPrefix": "automation/community-cache-submission"$( [ -n "$local_clone" ] && printf ',\n  "localClone": "%s"' "$local_clone" )
}
EOF
  echo "[Helpers-Installer] Wrote community cache settings: $COMMUNITY_SETTINGS_FILE"
}

configure_community_cache() {
  local reply=""
  local mode="pull-only"

  printf '[Helpers-Installer] Enable privacy-safe community cache uploads after successful audits? [y/N]: '
  read -r reply || reply=""
  if [[ "$reply" == "y" || "$reply" == "Y" ]]; then
    mode="pull-and-auto-submit"
  fi

  write_community_settings "$mode"

  if "${BIN_DIR}/git-copilot-devops-audit-community-pull" >/dev/null 2>&1; then
    echo "[Helpers-Installer] Pulled the latest shared DevOps audit community cache."
  else
    echo "[Helpers-Installer] WARNING: failed to pull the shared DevOps audit community cache." >&2
  fi
}

find_vscode_cli() {
  local candidate

  if command -v code >/dev/null 2>&1; then
    command -v code
    return 0
  fi

  for candidate in \
    "$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" \
    "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" \
    "$HOME/Applications/Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code" \
    "/Applications/Visual Studio Code - Insiders.app/Contents/Resources/app/bin/code"
  do
    if [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

maybe_install_vscode_extensions() {
  local code_cli="$1"
  local reply=""

  printf '[Helpers-Installer] Install or update VS Code GitHub Copilot extensions now? [Y/n]: '
  read -r reply || reply=""

  if [[ -n "$reply" && "$reply" != "y" && "$reply" != "Y" ]]; then
    echo "[Helpers-Installer] Skipped VS Code extension install. Install GitHub.copilot and GitHub.copilot-chat later if needed."
    return
  fi

  if "$code_cli" --install-extension GitHub.copilot --force >/dev/null 2>&1; then
    echo "[Helpers-Installer] Installed or updated VS Code extension: GitHub.copilot"
  else
    echo "[Helpers-Installer] Failed to install VS Code extension: GitHub.copilot" >&2
  fi

  if "$code_cli" --install-extension GitHub.copilot-chat --force >/dev/null 2>&1; then
    echo "[Helpers-Installer] Installed or updated VS Code extension: GitHub.copilot-chat"
  else
    echo "[Helpers-Installer] Failed to install VS Code extension: GitHub.copilot-chat" >&2
  fi
}

maybe_install_helpers_extension() {
  local code_cli="$1"
  local version="$2"
  local reply=""

  echo ""
  echo "[Helpers-Installer] Helpers VS Code Extension (RECOMMENDED)"
  echo "  Provides:"
  echo "    - MCP server auto-registration (no manual mcp.json editing)"
  echo "    - Branch sessions: per-chat isolated git worktrees for parallel work"
  echo "    - AI checkpoint commits with auto-generated messages"
  echo "    - Strict lint bridge: run VS Code diagnostics from Copilot agents"
  echo "    - Community cache sidebar: GitHub sign-in, audit settings, repo whitelist"
  echo "    - Session memory: per-workspace learning across agent conversations"
  echo "    - Model selector: switch Copilot models from the status bar"
  printf '[Helpers-Installer] Install the Helpers extension? [Y/n]: '
  read -r reply || reply=""

  if [[ -n "$reply" && "$reply" != "y" && "$reply" != "Y" ]]; then
    echo "[Helpers-Installer] Skipped. Install later from a GitHub release .vsix or the extension marketplace."
    return
  fi

  local vsix_url="https://github.com/RockyWearsAHat/helpers/releases/download/v${version}/helpers-${version}.vsix"
  local vsix_tmp="${TMPDIR:-/tmp}/helpers-${version}.vsix"

  if curl -fsSL -o "$vsix_tmp" "$vsix_url" 2>/dev/null; then
    if "$code_cli" --install-extension "$vsix_tmp" --force >/dev/null 2>&1; then
      echo "[Helpers-Installer] Installed Helpers extension v${version}"
    else
      echo "[Helpers-Installer] Failed to install VSIX. Install manually from: $vsix_url" >&2
    fi
    rm -f "$vsix_tmp"
  else
    echo "[Helpers-Installer] Could not download VSIX from release. Trying marketplace ID..."
    if "$code_cli" --install-extension RockyWearsAHat.helpers --force >/dev/null 2>&1; then
      echo "[Helpers-Installer] Installed Helpers from marketplace."
    else
      echo "[Helpers-Installer] Extension not yet on marketplace. Download the .vsix from:" >&2
      echo "  https://github.com/RockyWearsAHat/helpers/releases/latest" >&2
    fi
  fi
}

setup_vscode_proposed_api() {
  local argv_file="$HOME/.vscode/argv.json"
  local ext_id="RockyWearsAHat.helpers"
  local patch_argv_script="${BIN_DIR}/scripts/patch-vscode-argv.js"

  if [ ! -f "$argv_file" ]; then
    echo "[Helpers-Installer] No ~/.vscode/argv.json found - skipping proposed API setup."
    return
  fi

  if grep -q "$ext_id" "$argv_file" 2>/dev/null; then
    echo "[Helpers-Installer] Proposed API already enabled for $ext_id."
    return
  fi

  if [ ! -f "$patch_argv_script" ]; then
    echo "[Helpers-Installer] Proposed API patch helper not installed - skipping argv.json update."
    return
  fi

  if command -v node >/dev/null 2>&1 && node "$patch_argv_script" "$argv_file" "$ext_id" >/dev/null 2>&1; then
    echo "[Helpers-Installer] Enabled proposed API for $ext_id in argv.json."
    return
  fi

  echo "[Helpers-Installer] Could not auto-patch argv.json."
  echo "  Add \"$ext_id\" to the enable-proposed-api array in $argv_file manually."
}

apply_vscode_patches() {
  local patch_script="${BIN_DIR}/scripts/patch-vscode-apply-all.js"

  if [ ! -f "$patch_script" ]; then
    echo "[Helpers-Installer] VS Code patch helpers were not installed - skipping repatch step."
    return
  fi

  if ! command -v node >/dev/null 2>&1; then
    echo "[Helpers-Installer] Node.js not found - cannot apply VS Code patches."
    return
  fi

  local all_patched
  all_patched="$(node "$patch_script" --all-patched 2>/dev/null)" || {
    echo "[Helpers-Installer] Could not check patch status."
    return
  }

  if [ "$all_patched" = "true" ]; then
    echo "[Helpers-Installer] VS Code patches already applied."
    return
  fi

  local missing
  missing="$(node "$patch_script" --missing 2>/dev/null)" || missing="unknown"

  echo ""
  echo "[Helpers-Installer] VS Code Branch Session Patches"
  echo "  Missing patches: $missing"
  echo ""
  echo "  These patches improve branch-per-chat in Copilot:"
  echo "    - folder-switch: removes confirmation dialog on workspace folder swap"
  echo "    - git-head-display: shows worktree branch name in the status bar"
  echo ""
  echo "  Originals are backed up and can be restored with:"
  echo "    node ~/bin/scripts/patch-vscode-apply-all.js --revert"
  echo ""
  printf '[Helpers-Installer] Apply VS Code patches now? [Y/n]: '
  read -r patch_reply || patch_reply=""

  if [[ -z "$patch_reply" || "$patch_reply" == "y" || "$patch_reply" == "Y" ]]; then
    if node "$patch_script" 2>&1 | while IFS= read -r line; do
      echo "  $line"
    done; then
      if node "$patch_script" --check >/dev/null 2>&1; then
        echo "[Helpers-Installer] Patches applied and verified. Quit and restart VS Code (Cmd+Q -> reopen) to activate."
      else
        echo "[Helpers-Installer] Patch verification failed. Re-run 'node ~/bin/scripts/patch-vscode-apply-all.js --check'."
      fi
    else
      echo "[Helpers-Installer] Patch application failed. Re-run 'node ~/bin/scripts/patch-vscode-apply-all.js'."
    fi
  else
    echo "[Helpers-Installer] Skipped. Apply later with: node ~/bin/scripts/patch-vscode-apply-all.js"
  fi
}

# ---------------------------------------------------------------------------
# MCP tools installation
# ---------------------------------------------------------------------------

VSCODE_MCP_JSON="$HOME/Library/Application Support/Code/User/mcp.json"

remove_mcp_server() {
  local name="$1"
  local mcp_file="$VSCODE_MCP_JSON"

  [ -f "$mcp_file" ] || return 0

  # Single -c one-liner (no heredoc / multi-line body) per shell-safety; a
  # malformed mcp.json throws and is swallowed by `2>/dev/null || true`.
  python3 -c "import json,sys; p,n=sys.argv[1],sys.argv[2]; d=json.load(open(p)); s=d.get('servers'); (s.pop(n,None), open(p,'w').write(json.dumps(d,indent=2)+chr(10))) if isinstance(s,dict) and n in s else None" "$mcp_file" "$name" 2>/dev/null || true
}

# Purge pre-rebrand static registrations; the current "helpers" server is
# extension-managed and must not be removed here.
remove_legacy_helpers_mcp_servers() {
  remove_mcp_server "gsh"
  remove_mcp_server "git-shell-helpers"
  remove_mcp_server "git-shell-helpers-mcp"
}

configure_mcp_tools() {
  local install_research=true
  local install_vision=true

  echo ""
  echo "[Helpers-Installer] MCP Tools (global via Helpers extension)"
  echo "  Bundled tool modules:"
  echo "    1) git-research-mcp  — web search & knowledge cache for Copilot agents"
  echo "    2) helpers-vision        — screenshot analysis with vision models"
  echo ""
  printf '[Helpers-Installer] Install MCP tools into VS Code? [Y/n/pick]: '
  read -r mcp_reply || mcp_reply=""

  if [[ "$mcp_reply" == "n" || "$mcp_reply" == "N" ]]; then
    echo "[Helpers-Installer] Skipped MCP tool installation."
    return
  fi

  if [[ "$mcp_reply" == "pick" || "$mcp_reply" == "p" ]]; then
    printf '  Include research tools (web search, knowledge cache)? [Y/n]: '
    read -r r1 || r1=""
    if [[ "$r1" == "n" || "$r1" == "N" ]]; then
      install_research=false
    fi

    printf '  Include vision tools (screenshot analysis)? [Y/n]: '
    read -r r2 || r2=""
    if [[ "$r2" == "n" || "$r2" == "N" ]]; then
      install_vision=false
    fi
  fi

  # Fetch vision tool files if selected
  if [ "$install_vision" = true ]; then
    local vision_dir="${BIN_DIR}/vision-tool"
    ensure_dir "$vision_dir"
    fetch "$REPO_RAW_BASE/vision-tool/mcp-server.js" "$vision_dir/mcp-server.js"
    fetch "$REPO_RAW_BASE/vision-tool/screenshot.js" "$vision_dir/screenshot.js"
  fi

  # Remove legacy static mcp.json entries — the VS Code extension now
  # registers Helpers automatically.
  remove_legacy_helpers_mcp_servers

  echo "[Helpers-Installer] Installed MCP server runtime: ${BIN_DIR}/helpers-server"
  if [ "$install_research" = true ]; then
    echo "  ✓ research tools (web search, knowledge cache, fetch pages)"
  else
    echo "  - research tools left available for later install"
  fi
  if [ "$install_vision" = true ]; then
    echo "  ✓ vision tools (screenshot analysis)"
  else
    echo "  - vision tools not installed"
  fi
  echo "[Helpers-Installer] Helpers is registered by the Helpers VS Code extension."
  echo "[Helpers-Installer] Reload VS Code after installing the extension to refresh MCP server discovery."
}

install_all() {
  echo "[Helpers-Installer] Installing helpers..."

  ensure_dir "$BIN_DIR"
  ensure_dir "$MAN_DIR"

  local helpers_version=""
  helpers_version="$(curl -fsSL "$REPO_RAW_BASE/VERSION" 2>/dev/null | tr -d '\n' || echo "")"

  # Primary path: download the prebuilt helpers-native binary for THIS platform,
  # symlink the helpers/git-* CLIs to it, and register — entirely Node-free, no
  # source, no toolchain (the binary embeds its agent config). Fallback: build
  # from source with cargo (needs Rust) for platforms without a prebuilt.
  if fetch_and_register_prebuilt "$helpers_version"; then
    :
  elif install_from_source; then
    :
  else
    echo "[Helpers-Installer] ERROR: could not install a prebuilt binary or build from source." >&2
    exit 1
  fi

  configure_community_cache

  install_shell_path_setup

  # -----------------------------------------------------------------------------
  # HIGHLY RECOMMENDED: Install/Update GitHub Copilot CLI
  # Enables AI commit messages (git upload -ai) and improves Copilot integration
  # -----------------------------------------------------------------------------
  echo ""
  if command -v gh >/dev/null 2>&1; then
    if gh extension list 2>/dev/null | grep -q 'gh-copilot'; then
      printf '[Helpers-Installer] (HIGHLY RECOMMENDED) GitHub Copilot CLI is installed. Update it now? [Y/n]: ' >&2
      read -r copilot_reply || copilot_reply=""
      if [[ -z "$copilot_reply" || "$copilot_reply" == "y" || "$copilot_reply" == "Y" ]]; then
        gh extension upgrade gh-copilot && \
          echo "[Helpers-Installer] GitHub Copilot CLI updated." >&2 || \
          echo "[Helpers-Installer] Update failed — try: gh extension upgrade gh-copilot" >&2
      fi
    else
      printf '[Helpers-Installer] (HIGHLY RECOMMENDED) Install GitHub Copilot CLI? Enables AI commit messages and better Copilot integration. [Y/n]: ' >&2
      read -r copilot_reply || copilot_reply=""
      if [[ -z "$copilot_reply" || "$copilot_reply" == "y" || "$copilot_reply" == "Y" ]]; then
        gh extension install github/gh-copilot && \
          echo "[Helpers-Installer] GitHub Copilot CLI installed. Try: gh copilot suggest" >&2 || \
          echo "[Helpers-Installer] Install failed — try manually: gh extension install github/gh-copilot" >&2
      else
        echo "[Helpers-Installer] Skipped. Install later with: gh extension install github/gh-copilot" >&2
      fi
    fi
  else
    echo "[Helpers-Installer] (HIGHLY RECOMMENDED) GitHub CLI (gh) not found." >&2
    echo "  Install it to enable AI commit messages and better Copilot integration:" >&2
    if command -v brew >/dev/null 2>&1; then
      echo "  brew install gh && gh extension install github/gh-copilot" >&2
    else
      echo "  https://cli.github.com  →  then: gh extension install github/gh-copilot" >&2
    fi
  fi

  # Optional: install private audit agents and the slash command into standard user-level Copilot locations
  VSCODE_USER_DIR="$HOME/Library/Application Support/Code/User"
  local code_cli=""
  local should_offer_vscode_setup=false

  if code_cli="$(find_vscode_cli)"; then
    should_offer_vscode_setup=true
    mkdir -p "$VSCODE_USER_DIR"

    echo ""
    maybe_install_vscode_extensions "$code_cli"

    if [ -n "$helpers_version" ]; then
      maybe_install_helpers_extension "$code_cli" "$helpers_version"
    fi
  elif [ -d "$VSCODE_USER_DIR" ]; then
    should_offer_vscode_setup=true
    echo ""
    echo "[Helpers-Installer] VS Code user profile found, but no usable 'code' CLI was detected."
    echo "[Helpers-Installer] Install the GitHub Copilot and GitHub Copilot Chat extensions manually for the best experience in VS Code."
  fi

  if [ "$should_offer_vscode_setup" = true ]; then
    # MCP tools — always offer when VS Code is available
    configure_mcp_tools

    # Branch-per-chat: proposed API flag + bundle patches
    setup_vscode_proposed_api
    apply_vscode_patches
  fi

  echo "[Helpers-Installer] Done. Open a new terminal to reload PATH and MANPATH changes."
  echo "Then you can use: git upload, git get, git initialize, git fucked-the-push,"
  echo "  git copilot-devops-audit, and view docs via git help <command>."
}

install_all
