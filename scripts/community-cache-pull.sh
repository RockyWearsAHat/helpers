#!/usr/bin/env bash

set -euo pipefail

DEFAULT_COMMUNITY_REPO="RockyWearsAHat/github-shell-helpers"
DEFAULT_BASE_BRANCH="main"
DEFAULT_MANIFEST_PATH="community-cache/manifest.json"
DEFAULT_INSTALL_DIR="${HOME}/.copilot/devops-audit-community-cache"

usage() {
  cat <<'EOF'
Usage: git-copilot-devops-audit-community-pull [--workspace /path/to/repo]

Optional environment:
  COMMUNITY_CACHE_REPO         Source GitHub repository in owner/repo form
  COMMUNITY_CACHE_BASE_BRANCH  Source branch to pull from (default: main)
  COMMUNITY_CACHE_MANIFEST_PATH Path to top-level manifest (default: community-cache/manifest.json)
  COMMUNITY_CACHE_INSTALL_DIR  Global install directory (default: ~/.copilot/devops-audit-community-cache)

Configuration fallback order:
  1. environment variables
  2. ~/.copilot/devops-audit-community-settings.json
  3. .github/devops-audit-community-settings.json in the target workspace
  4. built-in defaults
EOF
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "[community-cache-pull] Missing required command: $1" >&2
    exit 1
  }
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

fetch_raw_file() {
  local repo="$1"
  local branch="$2"
  local relative_path="$3"
  local destination="$4"
  local url="https://raw.githubusercontent.com/${repo}/${branch}/${relative_path}"

  mkdir -p "$(dirname "$destination")"
  curl -fsSL "$url" -o "$destination"
}

write_workspace_seed() {
  local workspace="$1"
  local manifest_file="$2"
  local snapshot_manifest_file="$3"
  local workspace_cache_dir="$workspace/.github/devops-audit-community-cache"
  local seed_file="$workspace/.github/devops-audit-community-seed.md"
  local snapshot_id
  local snapshot_summary

  mkdir -p "$workspace_cache_dir"
  cp "$manifest_file" "$workspace_cache_dir/manifest.json"
  cp "$snapshot_manifest_file" "$workspace_cache_dir/snapshot-manifest.json"

  while IFS= read -r relative_path; do
    [[ -n "$relative_path" ]] || continue
    cp "${INSTALL_DIR}/${relative_path}" "$workspace_cache_dir/$(basename "$relative_path")"
  done < <(jq -r '.files | to_entries[] | .value' "$snapshot_manifest_file")

  snapshot_id="$(jq -r '.snapshotId' "$snapshot_manifest_file")"
  snapshot_summary="$(jq -r '.summary' "$snapshot_manifest_file")"

  cat > "$seed_file" <<EOF
# DevOps Audit Community Cache Seed

Pulled snapshot: ${snapshot_id}

Summary: ${snapshot_summary}

Rules:
- This seed is bootstrap context only.
- Revalidate important conclusions with live research during each audit.
- Do not treat public examples as normative without checking current official guidance.
- Do not publish repository-specific details back to the public cache.

Files:
- .github/devops-audit-community-cache/manifest.json
- .github/devops-audit-community-cache/snapshot-manifest.json
- .github/devops-audit-community-cache/official-sources.json
- .github/devops-audit-community-cache/public-example-sources.json
- .github/devops-audit-community-cache/prompting-principles.json
- .github/devops-audit-community-cache/application-practices.json
- .github/devops-audit-community-cache/anti-patterns.json
EOF
}

main() {
  local workspace=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --workspace)
        workspace="$2"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "[community-cache-pull] Unknown option: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  require_cmd curl
  require_cmd jq

  local global_settings_file="${HOME}/.copilot/devops-audit-community-settings.json"
  local repo_settings_file=""
  if [[ -n "$workspace" ]]; then
    repo_settings_file="$workspace/.github/devops-audit-community-settings.json"
  elif git rev-parse --show-toplevel >/dev/null 2>&1; then
    workspace="$(git rev-parse --show-toplevel)"
    repo_settings_file="$workspace/.github/devops-audit-community-settings.json"
  fi

  local global_repo="$(read_setting "$global_settings_file" communityRepo)"
  local global_branch="$(read_setting "$global_settings_file" baseBranch)"
  local repo_repo="$(read_setting "$repo_settings_file" communityRepo)"
  local repo_branch="$(read_setting "$repo_settings_file" baseBranch)"

  local community_repo="${COMMUNITY_CACHE_REPO:-${global_repo:-${repo_repo:-$DEFAULT_COMMUNITY_REPO}}}"
  local base_branch="${COMMUNITY_CACHE_BASE_BRANCH:-${global_branch:-${repo_branch:-$DEFAULT_BASE_BRANCH}}}"
  local manifest_path="${COMMUNITY_CACHE_MANIFEST_PATH:-$DEFAULT_MANIFEST_PATH}"

  INSTALL_DIR="${COMMUNITY_CACHE_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
  mkdir -p "$INSTALL_DIR"

  local temp_dir
  temp_dir="$(mktemp -d -t community-cache-pull.XXXXXX)"
  trap 'rm -rf "$temp_dir"' EXIT

  local manifest_file="$temp_dir/manifest.json"
  fetch_raw_file "$community_repo" "$base_branch" "$manifest_path" "$manifest_file"

  local recommended_snapshot
  local snapshot_manifest_path
  recommended_snapshot="$(jq -r '.recommendedSnapshot // ""' "$manifest_file")"
  snapshot_manifest_path="$(jq -r '.snapshotManifest // ""' "$manifest_file")"
  if [[ -z "$snapshot_manifest_path" && -n "$recommended_snapshot" ]]; then
    snapshot_manifest_path="community-cache/snapshots/${recommended_snapshot}/manifest.json"
  fi

  [[ -n "$snapshot_manifest_path" ]] || {
    echo "[community-cache-pull] Manifest did not provide a snapshot manifest path." >&2
    exit 1
  }

  local snapshot_manifest_file="$temp_dir/snapshot-manifest.json"
  fetch_raw_file "$community_repo" "$base_branch" "$snapshot_manifest_path" "$snapshot_manifest_file"

  fetch_raw_file "$community_repo" "$base_branch" "$manifest_path" "$INSTALL_DIR/community-cache/manifest.json"
  fetch_raw_file "$community_repo" "$base_branch" "$snapshot_manifest_path" "$INSTALL_DIR/community-cache/snapshot-manifest.json"

  while IFS= read -r relative_path; do
    [[ -n "$relative_path" ]] || continue
    fetch_raw_file "$community_repo" "$base_branch" "$relative_path" "$INSTALL_DIR/$relative_path"
  done < <(jq -r '.files | to_entries[] | .value' "$snapshot_manifest_file")

  if [[ -n "$workspace" ]]; then
    mkdir -p "$workspace/.github"
    write_workspace_seed "$workspace" "$manifest_file" "$snapshot_manifest_file"
    echo "[community-cache-pull] Refreshed global cache and workspace bootstrap in $workspace/.github" >&2
  else
    echo "[community-cache-pull] Refreshed global cache in $INSTALL_DIR" >&2
  fi
}

main "$@"
