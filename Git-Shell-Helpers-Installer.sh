#!/usr/bin/env bash

# Git-Shell-Helpers-Installer.sh
#
# Standalone installer script that fetches the latest versions of the
# helper commands and man pages from GitHub and installs them into:
#   - ~/bin
#   - ~/man/man1
# then wires PATH and MANPATH into the user's active shell profiles.
#
# Usage (one line):
#   curl -fsSL \
#     https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
#     | bash

set -euo pipefail

REPO_RAW_BASE="https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main"

BIN_DIR="${HOME}/bin"
MAN_DIR="${HOME}/man/man1"
COMMUNITY_SETTINGS_DIR="${HOME}/.copilot"
COMMUNITY_SETTINGS_FILE="${COMMUNITY_SETTINGS_DIR}/devops-audit-community-settings.json"
DEFAULT_COMMUNITY_REPO="RockyWearsAHat/github-shell-helpers"
DEFAULT_COMMUNITY_BRANCH="main"
SHELL_ENV_DIR="${HOME}/.config/git-shell-helpers"
LOCAL_PATH_SNIPPET="${SHELL_ENV_DIR}/paths-local.sh"
LOCAL_PATH_SOURCE='[ -f "$HOME/.config/git-shell-helpers/paths-local.sh" ] && . "$HOME/.config/git-shell-helpers/paths-local.sh"'

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
    '# Added by Git-Shell-Helpers-Installer.sh.' \
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

  echo "[Git-Shell-Helpers-Installer] Fetching $src -> $dest"
  curl -fsSL "$src" -o "$dest"
}

copy_packaged_tree() {
  local source_root="$1"

  if [ ! -d "$source_root/bin" ]; then
    return 1
  fi

  ensure_dir "$BIN_DIR"
  ensure_dir "$MAN_DIR"
  cp -R "$source_root/bin/." "$BIN_DIR/"
  if [ -d "$source_root/man/man1" ]; then
    cp -R "$source_root/man/man1/." "$MAN_DIR/"
  fi

  find "$BIN_DIR" -maxdepth 1 -type f -name 'git-*' -exec chmod +x {} + 2>/dev/null || true
  return 0
}

install_release_archive() {
  local version="$1"
  local archive_url="https://github.com/RockyWearsAHat/github-shell-helpers/releases/download/v${version}/github-shell-helpers-${version}.tar.gz"
  local archive_tmp="${TMPDIR:-/tmp}/github-shell-helpers-${version}.tar.gz"
  local extract_dir="$(mktemp -d "${TMPDIR:-/tmp}/gsh-release.XXXXXX")"
  local source_root="${extract_dir}/github-shell-helpers-${version}"

  if ! curl -fsSL -o "$archive_tmp" "$archive_url" 2>/dev/null; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  if ! tar -xzf "$archive_tmp" -C "$extract_dir"; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  if ! copy_packaged_tree "$source_root"; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  rm -rf "$extract_dir" "$archive_tmp"
  echo "[Git-Shell-Helpers-Installer] Installed packaged release tree for v${version}"
  return 0
}

install_source_archive() {
  local archive_url="https://codeload.github.com/RockyWearsAHat/github-shell-helpers/tar.gz/refs/heads/main"
  local archive_tmp="${TMPDIR:-/tmp}/github-shell-helpers-main.tar.gz"
  local extract_dir="$(mktemp -d "${TMPDIR:-/tmp}/gsh-source.XXXXXX")"
  local source_root=""

  if ! curl -fsSL -o "$archive_tmp" "$archive_url" 2>/dev/null; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  if ! tar -xzf "$archive_tmp" -C "$extract_dir"; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  source_root="$(find "$extract_dir" -mindepth 1 -maxdepth 1 -type d -name 'github-shell-helpers-*' | head -n 1)"
  if [ -z "$source_root" ]; then
    rm -rf "$extract_dir" "$archive_tmp"
    return 1
  fi

  if ! copy_packaged_tree "$source_root/build/release-root/$(basename "$source_root" | sed 's/-main$//')"; then
    if ! bash "$source_root/scripts/build-dist.sh" >/dev/null 2>&1; then
      rm -rf "$extract_dir" "$archive_tmp"
      return 1
    fi
    local built_root
    built_root="$(find "$source_root/build/release-root" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    if [ -z "$built_root" ] || ! copy_packaged_tree "$built_root"; then
      rm -rf "$extract_dir" "$archive_tmp"
      return 1
    fi
  fi

  rm -rf "$extract_dir" "$archive_tmp"
  echo "[Git-Shell-Helpers-Installer] Installed packaged tree from source archive fallback"
  return 0
}

fetch_mcp_runtime() {
  local dest_bin_dir="$1"
  local dest_lib_dir="${dest_bin_dir}/lib"
  local mcp_lib=""

  ensure_dir "$dest_lib_dir"

  fetch "$REPO_RAW_BASE/git-shell-helpers-mcp" "$dest_bin_dir/git-shell-helpers-mcp"
  fetch "$REPO_RAW_BASE/git-shell-helpers-mcp.js" "$dest_bin_dir/git-shell-helpers-mcp.js"
  fetch "$REPO_RAW_BASE/git-research-mcp" "$dest_bin_dir/git-research-mcp"
  fetch "$REPO_RAW_BASE/git-research-mcp.js" "$dest_bin_dir/git-research-mcp.js"

  for mcp_lib in \
    mcp-activity-ipc.js \
    mcp-branch-sessions.js \
    mcp-checkpoint.js \
    mcp-git.js \
    mcp-google-headless.js \
    mcp-knowledge-index.js \
    mcp-knowledge-rw.js \
    mcp-language-models.js \
    mcp-pdf-extract.js \
    mcp-research-tools.js \
    mcp-research.js \
    mcp-session-memory.js \
    mcp-strict-lint.js \
    mcp-utils.js \
    mcp-web-search.js \
    mcp-workspace-context.js
  do
    fetch "$REPO_RAW_BASE/lib/${mcp_lib}" "$dest_lib_dir/${mcp_lib}"
  done

  chmod +x "$dest_bin_dir/git-shell-helpers-mcp" "$dest_bin_dir/git-research-mcp"
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
  echo "[Git-Shell-Helpers-Installer] Wrote community cache settings: $COMMUNITY_SETTINGS_FILE"
}

configure_community_cache() {
  local reply=""
  local mode="pull-only"

  printf '[Git-Shell-Helpers-Installer] Enable privacy-safe community cache uploads after successful audits? [y/N]: '
  read -r reply || reply=""
  if [[ "$reply" == "y" || "$reply" == "Y" ]]; then
    mode="pull-and-auto-submit"
  fi

  write_community_settings "$mode"

  if "${BIN_DIR}/git-copilot-devops-audit-community-pull" >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Pulled the latest shared DevOps audit community cache."
  else
    echo "[Git-Shell-Helpers-Installer] WARNING: failed to pull the shared DevOps audit community cache." >&2
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

  printf '[Git-Shell-Helpers-Installer] Install or update VS Code GitHub Copilot extensions now? [Y/n]: '
  read -r reply || reply=""

  if [[ -n "$reply" && "$reply" != "y" && "$reply" != "Y" ]]; then
    echo "[Git-Shell-Helpers-Installer] Skipped VS Code extension install. Install GitHub.copilot and GitHub.copilot-chat later if needed."
    return
  fi

  if "$code_cli" --install-extension GitHub.copilot --force >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Installed or updated VS Code extension: GitHub.copilot"
  else
    echo "[Git-Shell-Helpers-Installer] Failed to install VS Code extension: GitHub.copilot" >&2
  fi

  if "$code_cli" --install-extension GitHub.copilot-chat --force >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Installed or updated VS Code extension: GitHub.copilot-chat"
  else
    echo "[Git-Shell-Helpers-Installer] Failed to install VS Code extension: GitHub.copilot-chat" >&2
  fi
}

maybe_install_gsh_extension() {
  local code_cli="$1"
  local version="$2"
  local reply=""

  echo ""
  echo "[Git-Shell-Helpers-Installer] Git Shell Helpers VS Code Extension (RECOMMENDED)"
  echo "  Provides:"
  echo "    - MCP server auto-registration (no manual mcp.json editing)"
  echo "    - Branch sessions: per-chat isolated git worktrees for parallel work"
  echo "    - AI checkpoint commits with auto-generated messages"
  echo "    - Strict lint bridge: run VS Code diagnostics from Copilot agents"
  echo "    - Community cache sidebar: GitHub sign-in, audit settings, repo whitelist"
  echo "    - Session memory: per-workspace learning across agent conversations"
  echo "    - Model selector: switch Copilot models from the status bar"
  printf '[Git-Shell-Helpers-Installer] Install the Git Shell Helpers extension? [Y/n]: '
  read -r reply || reply=""

  if [[ -n "$reply" && "$reply" != "y" && "$reply" != "Y" ]]; then
    echo "[Git-Shell-Helpers-Installer] Skipped. Install later from a GitHub release .vsix or the extension marketplace."
    return
  fi

  local vsix_url="https://github.com/RockyWearsAHat/github-shell-helpers/releases/download/v${version}/git-shell-helpers-${version}.vsix"
  local vsix_tmp="${TMPDIR:-/tmp}/git-shell-helpers-${version}.vsix"

  if curl -fsSL -o "$vsix_tmp" "$vsix_url" 2>/dev/null; then
    if "$code_cli" --install-extension "$vsix_tmp" --force >/dev/null 2>&1; then
      echo "[Git-Shell-Helpers-Installer] Installed Git Shell Helpers extension v${version}"
    else
      echo "[Git-Shell-Helpers-Installer] Failed to install VSIX. Install manually from: $vsix_url" >&2
    fi
    rm -f "$vsix_tmp"
  else
    echo "[Git-Shell-Helpers-Installer] Could not download VSIX from release. Trying marketplace ID..."
    if "$code_cli" --install-extension RockyWearsAHat.git-shell-helpers --force >/dev/null 2>&1; then
      echo "[Git-Shell-Helpers-Installer] Installed Git Shell Helpers from marketplace."
    else
      echo "[Git-Shell-Helpers-Installer] Extension not yet on marketplace. Download the .vsix from:" >&2
      echo "  https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest" >&2
    fi
  fi
}

setup_vscode_proposed_api() {
  local argv_file="$HOME/.vscode/argv.json"
  local ext_id="RockyWearsAHat.git-shell-helpers"
  local patch_argv_script="${BIN_DIR}/scripts/patch-vscode-argv.js"

  if [ ! -f "$argv_file" ]; then
    echo "[Git-Shell-Helpers-Installer] No ~/.vscode/argv.json found - skipping proposed API setup."
    return
  fi

  if grep -q "$ext_id" "$argv_file" 2>/dev/null; then
    echo "[Git-Shell-Helpers-Installer] Proposed API already enabled for $ext_id."
    return
  fi

  if [ ! -f "$patch_argv_script" ]; then
    echo "[Git-Shell-Helpers-Installer] Proposed API patch helper not installed - skipping argv.json update."
    return
  fi

  if command -v node >/dev/null 2>&1 && node "$patch_argv_script" "$argv_file" "$ext_id" >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Enabled proposed API for $ext_id in argv.json."
    return
  fi

  echo "[Git-Shell-Helpers-Installer] Could not auto-patch argv.json."
  echo "  Add \"$ext_id\" to the enable-proposed-api array in $argv_file manually."
}

apply_vscode_patches() {
  local patch_script="${BIN_DIR}/scripts/patch-vscode-apply-all.js"

  if [ ! -f "$patch_script" ]; then
    echo "[Git-Shell-Helpers-Installer] VS Code patch helpers were not installed - skipping repatch step."
    return
  fi

  if ! command -v node >/dev/null 2>&1; then
    echo "[Git-Shell-Helpers-Installer] Node.js not found - cannot apply VS Code patches."
    return
  fi

  local all_patched
  all_patched="$(node "$patch_script" --all-patched 2>/dev/null)" || {
    echo "[Git-Shell-Helpers-Installer] Could not check patch status."
    return
  }

  if [ "$all_patched" = "true" ]; then
    echo "[Git-Shell-Helpers-Installer] VS Code patches already applied."
    return
  fi

  local missing
  missing="$(node "$patch_script" --missing 2>/dev/null)" || missing="unknown"

  echo ""
  echo "[Git-Shell-Helpers-Installer] VS Code Branch Session Patches"
  echo "  Missing patches: $missing"
  echo ""
  echo "  These patches improve branch-per-chat in Copilot:"
  echo "    - folder-switch: removes confirmation dialog on workspace folder swap"
  echo "    - git-head-display: shows worktree branch name in the status bar"
  echo ""
  echo "  Originals are backed up and can be restored with:"
  echo "    node ~/bin/scripts/patch-vscode-apply-all.js --revert"
  echo ""
  printf '[Git-Shell-Helpers-Installer] Apply VS Code patches now? [Y/n]: '
  read -r patch_reply || patch_reply=""

  if [[ -z "$patch_reply" || "$patch_reply" == "y" || "$patch_reply" == "Y" ]]; then
    if node "$patch_script" 2>&1 | while IFS= read -r line; do
      echo "  $line"
    done; then
      if node "$patch_script" --check >/dev/null 2>&1; then
        echo "[Git-Shell-Helpers-Installer] Patches applied and verified. Quit and restart VS Code (Cmd+Q -> reopen) to activate."
      else
        echo "[Git-Shell-Helpers-Installer] Patch verification failed. Re-run 'node ~/bin/scripts/patch-vscode-apply-all.js --check'."
      fi
    else
      echo "[Git-Shell-Helpers-Installer] Patch application failed. Re-run 'node ~/bin/scripts/patch-vscode-apply-all.js'."
    fi
  else
    echo "[Git-Shell-Helpers-Installer] Skipped. Apply later with: node ~/bin/scripts/patch-vscode-apply-all.js"
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

  python3 - "$mcp_file" "$name" <<'PYEOF'
import json, sys

mcp_path, srv_name = sys.argv[1], sys.argv[2]
try:
  with open(mcp_path, "r", encoding="utf-8") as f:
    data = json.load(f)
except (json.JSONDecodeError, FileNotFoundError):
  sys.exit(0)

if "servers" in data and isinstance(data["servers"], dict):
  if srv_name in data["servers"]:
    del data["servers"][srv_name]
    with open(mcp_path, "w", encoding="utf-8") as f:
      json.dump(data, f, indent=2)
      f.write("\n")
PYEOF
}

remove_legacy_gsh_mcp_servers() {
  remove_mcp_server "gsh"
  remove_mcp_server "git-shell-helpers"
}

configure_mcp_tools() {
  local install_research=true
  local install_vision=true

  echo ""
  echo "[Git-Shell-Helpers-Installer] MCP Tools (global via Git Shell Helpers extension)"
  echo "  Bundled tool modules:"
  echo "    1) git-research-mcp  — web search & knowledge cache for Copilot agents"
  echo "    2) gsh-vision        — screenshot analysis with vision models"
  echo ""
  printf '[Git-Shell-Helpers-Installer] Install MCP tools into VS Code? [Y/n/pick]: '
  read -r mcp_reply || mcp_reply=""

  if [[ "$mcp_reply" == "n" || "$mcp_reply" == "N" ]]; then
    echo "[Git-Shell-Helpers-Installer] Skipped MCP tool installation."
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
  fi

  # Remove legacy static mcp.json entries — the VS Code extension now
  # registers GitHub Shell Helpers automatically.
  remove_legacy_gsh_mcp_servers

  echo "[Git-Shell-Helpers-Installer] Installed MCP server runtime: ${BIN_DIR}/git-shell-helpers-mcp"
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
  echo "[Git-Shell-Helpers-Installer] GitHub Shell Helpers is registered by the Git Shell Helpers VS Code extension."
  echo "[Git-Shell-Helpers-Installer] Reload VS Code after installing the extension to refresh MCP server discovery."
}

install_all() {
  echo "[Git-Shell-Helpers-Installer] Installing git shell helpers..."

  ensure_dir "$BIN_DIR"
  ensure_dir "$MAN_DIR"

  local gsh_version=""
  gsh_version="$(curl -fsSL "$REPO_RAW_BASE/VERSION" 2>/dev/null | tr -d '\n' || echo "")"

  if [ -n "$gsh_version" ] && install_release_archive "$gsh_version"; then
    :
  elif install_source_archive; then
    :
  else
    echo "[Git-Shell-Helpers-Installer] ERROR: failed to install packaged release files from both release and source archives." >&2
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
      printf '[Git-Shell-Helpers-Installer] (HIGHLY RECOMMENDED) GitHub Copilot CLI is installed. Update it now? [Y/n]: ' >&2
      read -r copilot_reply || copilot_reply=""
      if [[ -z "$copilot_reply" || "$copilot_reply" == "y" || "$copilot_reply" == "Y" ]]; then
        gh extension upgrade gh-copilot && \
          echo "[Git-Shell-Helpers-Installer] GitHub Copilot CLI updated." >&2 || \
          echo "[Git-Shell-Helpers-Installer] Update failed — try: gh extension upgrade gh-copilot" >&2
      fi
    else
      printf '[Git-Shell-Helpers-Installer] (HIGHLY RECOMMENDED) Install GitHub Copilot CLI? Enables AI commit messages and better Copilot integration. [Y/n]: ' >&2
      read -r copilot_reply || copilot_reply=""
      if [[ -z "$copilot_reply" || "$copilot_reply" == "y" || "$copilot_reply" == "Y" ]]; then
        gh extension install github/gh-copilot && \
          echo "[Git-Shell-Helpers-Installer] GitHub Copilot CLI installed. Try: gh copilot suggest" >&2 || \
          echo "[Git-Shell-Helpers-Installer] Install failed — try manually: gh extension install github/gh-copilot" >&2
      else
        echo "[Git-Shell-Helpers-Installer] Skipped. Install later with: gh extension install github/gh-copilot" >&2
      fi
    fi
  else
    echo "[Git-Shell-Helpers-Installer] (HIGHLY RECOMMENDED) GitHub CLI (gh) not found." >&2
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

    if [ -n "$gsh_version" ]; then
      maybe_install_gsh_extension "$code_cli" "$gsh_version"
    fi
  elif [ -d "$VSCODE_USER_DIR" ]; then
    should_offer_vscode_setup=true
    echo ""
    echo "[Git-Shell-Helpers-Installer] VS Code user profile found, but no usable 'code' CLI was detected."
    echo "[Git-Shell-Helpers-Installer] Install the GitHub Copilot and GitHub Copilot Chat extensions manually if you want the audit surfaces to be usable in VS Code."
  fi

  if [ "$should_offer_vscode_setup" = true ]; then
    echo ""
    printf '[Git-Shell-Helpers-Installer] Install private Copilot audit agents, natural-language router, and /copilot-devops-audit globally in VS Code? [Y/n]: '
    read -r vscode_reply || vscode_reply=""
    if [[ -z "$vscode_reply" || "$vscode_reply" == "y" || "$vscode_reply" == "Y" ]]; then
      "${BIN_DIR}/git-copilot-devops-audit" --update-agent --force >/dev/null 2>&1 || true
      echo "[Git-Shell-Helpers-Installer] Installed DevOpsAudit agents, the natural-language router instruction, and skills into ~/.copilot, plus the prompt into the VS Code user profile."
      echo "[Git-Shell-Helpers-Installer] Reload VS Code window (Cmd+Shift+P → 'Developer: Reload Window') to activate."
    else
      echo "[Git-Shell-Helpers-Installer] Skipped VS Code global install. Run 'git copilot-devops-audit' in any repo to install later."
    fi

    # MCP tools — always offer when VS Code is available
    configure_mcp_tools

    # Branch-per-chat: proposed API flag + bundle patches
    setup_vscode_proposed_api
    apply_vscode_patches
  fi

  echo "[Git-Shell-Helpers-Installer] Done. Open a new terminal to reload PATH and MANPATH changes."
  echo "Then you can use: git upload, git get, git initialize, git fucked-the-push,"
  echo "  git copilot-devops-audit, and view docs via git help <command>."
}

install_all
