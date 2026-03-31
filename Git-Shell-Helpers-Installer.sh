#!/bin/zsh

# Git-Shell-Helpers-Installer.sh
#
# Standalone installer script that fetches the latest versions of the
# helper commands and man pages from GitHub and installs them into:
#   - ~/bin
#   - ~/man/man1
# then wires PATH and MANPATH in ~/.zshrc.
#
# Usage (one line):
#   curl -fsSL \
#     https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
#     | zsh

set -euo pipefail

REPO_RAW_BASE="https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main"

BIN_DIR="${HOME}/bin"
MAN_DIR="${HOME}/man/man1"
ZSHRC="${HOME}/.zshrc"
COMMUNITY_SETTINGS_DIR="${HOME}/.copilot"
COMMUNITY_SETTINGS_FILE="${COMMUNITY_SETTINGS_DIR}/devops-audit-community-settings.json"
DEFAULT_COMMUNITY_REPO="RockyWearsAHat/github-shell-helpers"
DEFAULT_COMMUNITY_BRANCH="main"

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

fetch() {
  local src="$1"
  local dest="$2"

  echo "[Git-Shell-Helpers-Installer] Fetching $src -> $dest"
  curl -fsSL "$src" -o "$dest"
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

  # Fetch the combined server entry point
  fetch "$REPO_RAW_BASE/git-shell-helpers-mcp" "$BIN_DIR/git-shell-helpers-mcp"

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

  # Scripts
  fetch "$REPO_RAW_BASE/git-upload"     "$BIN_DIR/git-upload"
  fetch "$REPO_RAW_BASE/git-get"        "$BIN_DIR/git-get"
  fetch "$REPO_RAW_BASE/git-initialize" "$BIN_DIR/git-initialize"
  fetch "$REPO_RAW_BASE/git-fucked-the-push" "$BIN_DIR/git-fucked-the-push"
  fetch "$REPO_RAW_BASE/git-copilot-devops-audit" "$BIN_DIR/git-copilot-devops-audit"
  fetch "$REPO_RAW_BASE/git-research-mcp" "$BIN_DIR/git-research-mcp"
  fetch "$REPO_RAW_BASE/scripts/community-cache-submit.sh" "$BIN_DIR/git-copilot-devops-audit-community-submit"
  fetch "$REPO_RAW_BASE/scripts/community-cache-pull.sh" "$BIN_DIR/git-copilot-devops-audit-community-pull"
  fetch "$REPO_RAW_BASE/scripts/community-research-submit.sh" "$BIN_DIR/git-copilot-devops-audit-community-research-submit"
  chmod +x "$BIN_DIR/git-upload" "$BIN_DIR/git-get" "$BIN_DIR/git-initialize" "$BIN_DIR/git-fucked-the-push" "$BIN_DIR/git-copilot-devops-audit" "$BIN_DIR/git-research-mcp" "$BIN_DIR/git-copilot-devops-audit-community-submit" "$BIN_DIR/git-copilot-devops-audit-community-pull" "$BIN_DIR/git-copilot-devops-audit-community-research-submit"

  configure_community_cache

  # Copilot config (product source: agents, instructions, skills, prompts)
  # Needed by git-copilot-devops-audit --update-agent to install globally
  local CC="$BIN_DIR/copilot-config"
  ensure_dir "$CC/agents"
  ensure_dir "$CC/instructions"
  ensure_dir "$CC/prompts"
  ensure_dir "$CC/skills/copilot-research"
  ensure_dir "$CC/skills/devops-audit-community-submit"
  ensure_dir "$CC/skills/devops-audit-context"
  ensure_dir "$CC/skills/devops-audit-evaluation"
  ensure_dir "$CC/skills/devops-audit-fix"
  ensure_dir "$CC/skills/devops-audit-orchestration"

  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAudit.agent.md" "$CC/agents/DevOpsAudit.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAuditCommunitySubmit.agent.md" "$CC/agents/DevOpsAuditCommunitySubmit.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAuditContext.agent.md" "$CC/agents/DevOpsAuditContext.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAuditEvaluate.agent.md" "$CC/agents/DevOpsAuditEvaluate.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAuditImplement.agent.md" "$CC/agents/DevOpsAuditImplement.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/DevOpsAuditResearch.agent.md" "$CC/agents/DevOpsAuditResearch.agent.md"
  fetch "$REPO_RAW_BASE/copilot-config/agents/Explore.agent.md" "$CC/agents/Explore.agent.md"

  fetch "$REPO_RAW_BASE/copilot-config/instructions/devops-audit-router.instructions.md" "$CC/instructions/devops-audit-router.instructions.md"
  fetch "$REPO_RAW_BASE/copilot-config/instructions/gsh-mcp-tools.instructions.md" "$CC/instructions/gsh-mcp-tools.instructions.md"
  fetch "$REPO_RAW_BASE/copilot-config/instructions/git-checkpoint.instructions.md" "$CC/instructions/git-checkpoint.instructions.md"
  fetch "$REPO_RAW_BASE/copilot-config/instructions/shell-scripts.instructions.md" "$CC/instructions/shell-scripts.instructions.md"
  fetch "$REPO_RAW_BASE/copilot-config/instructions/vscode-tool-safety.instructions.md" "$CC/instructions/vscode-tool-safety.instructions.md"

  fetch "$REPO_RAW_BASE/copilot-config/prompts/copilot-devops-audit.prompt.md" "$CC/prompts/copilot-devops-audit.prompt.md"

  fetch "$REPO_RAW_BASE/copilot-config/skills/copilot-research/SKILL.md" "$CC/skills/copilot-research/SKILL.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/copilot-research/studybase.md" "$CC/skills/copilot-research/studybase.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/devops-audit-community-submit/SKILL.md" "$CC/skills/devops-audit-community-submit/SKILL.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/devops-audit-context/SKILL.md" "$CC/skills/devops-audit-context/SKILL.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/devops-audit-evaluation/SKILL.md" "$CC/skills/devops-audit-evaluation/SKILL.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/devops-audit-fix/SKILL.md" "$CC/skills/devops-audit-fix/SKILL.md"
  fetch "$REPO_RAW_BASE/copilot-config/skills/devops-audit-orchestration/SKILL.md" "$CC/skills/devops-audit-orchestration/SKILL.md"

  # Man pages (from repo's man/man1)
  fetch "$REPO_RAW_BASE/man/man1/git-upload.1"     "$MAN_DIR/git-upload.1"
  fetch "$REPO_RAW_BASE/man/man1/git-get.1"        "$MAN_DIR/git-get.1"
  fetch "$REPO_RAW_BASE/man/man1/git-initialize.1" "$MAN_DIR/git-initialize.1"
  fetch "$REPO_RAW_BASE/man/man1/git-fucked-the-push.1" "$MAN_DIR/git-fucked-the-push.1"
  fetch "$REPO_RAW_BASE/man/man1/git-copilot-devops-audit.1" "$MAN_DIR/git-copilot-devops-audit.1"

  # Ensure PATH and MANPATH are wired in ~/.zshrc (idempotent)
  ensure_line_in_file "$ZSHRC" 'export PATH="$HOME/bin:$PATH"'
  ensure_line_in_file "$ZSHRC" 'export MANPATH="$HOME/man:$MANPATH"'

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
  fi

  echo "[Git-Shell-Helpers-Installer] Done. Open a new terminal or run:"
  echo "  source $ZSHRC"
  echo "Then you can use: git upload, git get, git initialize, git fucked-the-push,"
  echo "  git copilot-devops-audit, and view docs via git help <command>."
}

install_all
