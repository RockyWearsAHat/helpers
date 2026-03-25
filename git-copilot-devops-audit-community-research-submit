#!/usr/bin/env bash

set -euo pipefail

DEFAULT_COMMUNITY_REPO="RockyWearsAHat/github-shell-helpers"

usage() {
  cat <<'EOF'
Usage: community-research-submit.sh path/to/knowledge-note.md

Submits a knowledge note to the community repository as a PR.

Required: The file must be a .md file located under .github/knowledge/.

Optional environment:
  COMMUNITY_CACHE_REPO          Target GitHub repository in owner/repo form
  COMMUNITY_CACHE_BASE_BRANCH   Base branch to target (default: main)

If environment variables are not set, the script will also look for:
  ~/.copilot/devops-audit-community-settings.json
  .github/devops-audit-community-settings.json
EOF
}

read_setting() {
  local file="$1"
  local key="$2"

  if [[ -f "$file" ]]; then
    jq -r --arg key "$key" '.[$key] // ""' "$file"
  else
    printf '\n'
  fi
}

expand_path() {
  local raw_path="$1"
  local anchor_dir="$2"

  [[ -n "$raw_path" ]] || return 0

  case "$raw_path" in
    ~)
      printf '%s\n' "$HOME"
      ;;
    ~/*)
      printf '%s\n' "$HOME/${raw_path#~/}"
      ;;
    /*)
      printf '%s\n' "$raw_path"
      ;;
    ./*|../*|*)
      if [[ -n "$anchor_dir" ]]; then
        (cd "$anchor_dir" && cd "$raw_path" 2>/dev/null && pwd) || true
      else
        printf '%s\n' "$raw_path"
      fi
      ;;
  esac
}

default_repo_from_manifest() {
  local manifest_file="$1"

  if [[ -f "$manifest_file" ]]; then
    jq -r '.defaultCommunityRepo // ""' "$manifest_file"
  else
    printf '\n'
  fi
}

repo_from_git_remote() {
  local clone_dir="$1"
  local remote_url=""

  [[ -d "$clone_dir/.git" ]] || return 0
  remote_url="$(git -C "$clone_dir" remote get-url origin 2>/dev/null || true)"
  [[ -n "$remote_url" ]] || return 0

  printf '%s\n' "$remote_url" | sed -E 's#^[^:]+:##; s#^https?://[^/]+/##; s#\.git$##'
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "[community-research-submit] Missing required command: $1" >&2
    exit 1
  }
}

sanitize_content() {
  local file="$1"

  # Reject content that embeds private paths or repo-specific context
  if grep -Eq '(/Users/|/home/|[A-Za-z]:\\\\|this repository|my workspace|our repo)' "$file"; then
    echo "[community-research-submit] File appears to contain private or repository-specific context." >&2
    exit 1
  fi
}

main() {
  if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || $# -ne 1 ]]; then
    usage
    exit $(( $# == 1 ? 0 : 1 ))
  fi

  local note_file="$1"
  [[ -f "$note_file" ]] || {
    echo "[community-research-submit] File not found: $note_file" >&2
    exit 1
  }

  # Must be a .md file
  [[ "$note_file" == *.md ]] || {
    echo "[community-research-submit] Only .md files are accepted." >&2
    exit 1
  }

  require_cmd gh
  require_cmd git
  require_cmd jq

  local repo_root
  repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
  local global_settings_file="$HOME/.copilot/devops-audit-community-settings.json"
  local repo_settings_file="$repo_root/.github/devops-audit-community-settings.json"

  # Check shareResearch flag
  local global_share_research="$(read_setting "$global_settings_file" shareResearch)"
  local repo_share_research="$(read_setting "$repo_settings_file" shareResearch)"
  local share_research="${repo_share_research:-${global_share_research:-}}"

  if [[ "$share_research" != "true" ]]; then
    echo "[community-research-submit] Research sharing is not enabled. Set shareResearch: true in community settings." >&2
    exit 1
  fi

  # Check community mode allows submissions
  local global_mode="$(read_setting "$global_settings_file" mode)"
  local repo_mode="$(read_setting "$repo_settings_file" mode)"
  local configured_mode="${global_mode:-${repo_mode:-}}"

  local submit_allowed=false
  case "${configured_mode}" in
    pull-and-auto-submit)
      submit_allowed=true
      ;;
    auto-submit-only-public)
      local current_repo_visibility
      current_repo_visibility="$(gh repo view --json visibility --jq '.visibility' 2>/dev/null || echo "")"
      if [[ "$current_repo_visibility" == "PUBLIC" ]]; then
        submit_allowed=true
      else
        echo "[community-research-submit] Mode is 'auto-submit-only-public' but current repo is not public (visibility: ${current_repo_visibility:-unknown}). Skipping." >&2
        exit 1
      fi
      ;;
    auto-submit-whitelist)
      local current_nwo
      current_nwo="$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || echo "")"
      local global_whitelist
      global_whitelist="$(jq -r '.whitelistedRepos[]? // empty' "$global_settings_file" 2>/dev/null || true)"
      local repo_whitelist
      repo_whitelist="$(jq -r '.whitelistedRepos[]? // empty' "$repo_settings_file" 2>/dev/null || true)"
      local all_whitelist
      all_whitelist="$(printf '%s\n%s' "$global_whitelist" "$repo_whitelist" | sort -u | grep -v '^$')"
      if echo "$all_whitelist" | grep -qxF "$current_nwo"; then
        submit_allowed=true
      else
        echo "[community-research-submit] Mode is 'auto-submit-whitelist' but '${current_nwo:-unknown}' is not in the whitelist. Skipping." >&2
        exit 1
      fi
      ;;
    disabled|"")
      echo "[community-research-submit] Community submissions are disabled." >&2
      exit 1
      ;;
    *)
      echo "[community-research-submit] Unknown mode: '$configured_mode'" >&2
      exit 1
      ;;
  esac

  # Resolve community repo
  local global_repo="$(read_setting "$global_settings_file" communityRepo)"
  local repo_repo="$(read_setting "$repo_settings_file" communityRepo)"
  local global_local_clone_raw="$(read_setting "$global_settings_file" localClone)"
  local repo_local_clone_raw="$(read_setting "$repo_settings_file" localClone)"
  local repo_local_clone="$(expand_path "$repo_local_clone_raw" "$repo_root")"
  local global_local_clone="$(expand_path "$global_local_clone_raw" "$HOME")"
  local local_clone="${COMMUNITY_CACHE_LOCAL_CLONE:-${repo_local_clone:-${global_local_clone:-}}}"
  local manifest_default_repo=""
  local inferred_repo=""

  if [[ -n "$local_clone" && -f "$local_clone/community-cache/manifest.json" ]]; then
    manifest_default_repo="$(default_repo_from_manifest "$local_clone/community-cache/manifest.json")"
    inferred_repo="$(repo_from_git_remote "$local_clone")"
  elif [[ -f "$repo_root/community-cache/manifest.json" ]]; then
    manifest_default_repo="$(default_repo_from_manifest "$repo_root/community-cache/manifest.json")"
    inferred_repo="$(repo_from_git_remote "$repo_root")"
  fi

  local community_repo="${COMMUNITY_CACHE_REPO:-${repo_repo:-${global_repo:-${manifest_default_repo:-${inferred_repo:-$DEFAULT_COMMUNITY_REPO}}}}}"
  local global_branch="$(read_setting "$global_settings_file" baseBranch)"
  local repo_branch="$(read_setting "$repo_settings_file" baseBranch)"
  local base_branch="${COMMUNITY_CACHE_BASE_BRANCH:-${repo_branch:-${global_branch:-main}}}"
  local branch_prefix="automation/community-research"

  if [[ -z "$community_repo" ]]; then
    echo "[community-research-submit] No community repo configured." >&2
    exit 1
  fi

  sanitize_content "$note_file"

  # Derive the target path inside the community repo
  local basename
  basename="$(basename "$note_file")"
  local target_path=".github/knowledge/$basename"

  local submission_id
  submission_id="$(date -u +"%Y%m%dT%H%M%SZ")-$(LC_ALL=C tr -dc 'a-z0-9' </dev/urandom | head -c 8 || true)"

  local temp_dir
  temp_dir="$(mktemp -d -t community-research-submit.XXXXXX)"
  trap 'rm -rf "${temp_dir:-}"' EXIT

  gh repo clone "$community_repo" "$temp_dir/repo" -- --quiet
  cd "$temp_dir/repo"

  # Check if file already exists in the community repo
  local action="add"
  if [[ -f "$target_path" ]]; then
    # File exists — check if content differs
    if diff -q "$note_file" "$target_path" >/dev/null 2>&1; then
      echo "[community-research-submit] $basename is already up to date in the community repo. Nothing to submit." >&2
      exit 0
    fi
    action="update"
  fi

  local branch_name="${branch_prefix}-${submission_id}"
  git checkout -b "$branch_name"

  mkdir -p "$(dirname "$target_path")"
  cp "$note_file" "$target_path"

  git config user.name "community-research-bot"
  git config user.email "community-research-bot@users.noreply.github.com"
  git add "$target_path"
  git commit -m "${action^} knowledge note: $basename"
  git push --set-upstream origin "$branch_name"

  gh pr create \
    --repo "$community_repo" \
    --base "$base_branch" \
    --head "$branch_name" \
    --title "${action^} knowledge note: $basename" \
    --body "Automated community research contribution.

- Submission ID: $submission_id
- Action: $action
- File: $target_path
- Source: knowledge research contribution
- Privacy: content validated (no private paths or repo-specific context)"

  echo "[community-research-submit] Submitted $target_path ($action) to $community_repo" >&2
}

main "$@"
