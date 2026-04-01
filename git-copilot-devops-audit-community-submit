#!/usr/bin/env bash

set -euo pipefail

DEFAULT_COMMUNITY_REPO="RockyWearsAHat/github-shell-helpers"

usage() {
  cat <<'EOF'
Usage: COMMUNITY_CACHE_REPO=owner/repo git-copilot-devops-audit-community-submit path/to/packet.json

Required environment:
  COMMUNITY_CACHE_REPO         Target GitHub repository in owner/repo form

Optional environment:
  COMMUNITY_CACHE_BASE_BRANCH  Base branch to target (default: main)
  COMMUNITY_CACHE_BRANCH_PREFIX Branch prefix for submission PRs

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
    echo "[community-cache-submit] Missing required command: $1" >&2
    exit 1
  }
}

trimmed_statement() {
  jq -r '.statement | gsub("\\s+"; " ") | sub("^ "; "") | sub(" $"; "")' "$1"
}

validate_packet() {
  local packet_file="$1"

  jq -e '
    .schemaVersion == 1 and
    (.kind | IN("principle", "anti-pattern", "example", "warning")) and
    (.topic | IN("prompts", "instructions", "agents", "skills", "routing", "tooling", "workflow", "other")) and
    (.recommendationStrength | IN("required", "recommended", "optional", "illustrative")) and
    (.applicability | IN("general", "prompt-design", "instruction-design", "agent-design", "skill-design", "routing", "tool-use", "workflow-general")) and
    (.statement | type == "string" and length >= 10) and
    (.evidenceRefs | type == "array" and length >= 1) and
    (.auditCompletedAt | type == "string") and
    (.freshnessCheckedAt | type == "string") and
    (.authoritativeSupport | IN("none", "weak", "medium", "strong")) and
    (.liveRevalidated | type == "boolean") and
    (.contradictionCount | type == "number") and
    (.clientBehaviorVersion | type == "number")
  ' "$packet_file" >/dev/null

  jq -e '
    has("repository") or
    has("repo") or
    has("workspace") or
    has("projectName") or
    has("filePath") or
    has("localPath") or
    has("directoryStructure") or
    has("repoTypeSignals")
    | not
  ' "$packet_file" >/dev/null

  if grep -Eq '(/Users/|/home/|[A-Za-z]:\\\\|this repository|my workspace|our repo)' "$packet_file"; then
    echo "[community-cache-submit] Packet appears to contain private or repository-specific context." >&2
    exit 1
  fi
}

sanitize_packet() {
  local packet_file="$1"
  local submitted_at="$2"
  local submission_id="$3"
  local statement_hash="$4"

  jq \
    --arg submittedAt "$submitted_at" \
    --arg submissionId "$submission_id" \
    --arg statementHash "$statement_hash" '
      {
        schemaVersion: 1,
        kind,
        topic,
        statement: (.statement | gsub("\\s+"; " ") | sub("^ "; "") | sub(" $"; "")),
        recommendationStrength,
        applicability,
        evidenceRefs,
        auditCompletedAt,
        freshnessCheckedAt,
        authoritativeSupport,
        liveRevalidated,
        contradictionCount,
        clientBehaviorVersion,
        publicSummary: (.publicSummary // ""),
        cautionNotes: (.cautionNotes // ""),
        nonGeneralizationNotes: (.nonGeneralizationNotes // ""),
        submittedAt: $submittedAt,
        submissionId: $submissionId,
        statementHash: $statementHash
      }
    ' "$packet_file"
}

main() {
  if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || $# -ne 1 ]]; then
    usage
    exit $(( $# == 1 ? 0 : 1 ))
  fi

  local packet_file="$1"
  [[ -f "$packet_file" ]] || {
    echo "[community-cache-submit] Packet file not found: $packet_file" >&2
    exit 1
  }

  require_cmd gh
  require_cmd git
  require_cmd jq
  require_cmd shasum

  local repo_root
  repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
  local global_settings_file="$HOME/.copilot/devops-audit-community-settings.json"
  local repo_settings_file="$repo_root/.github/devops-audit-community-settings.json"
  local repo_settings_dir
  repo_settings_dir="$(dirname "$repo_settings_file")"

  local global_mode=""
  global_mode="$(read_setting "$global_settings_file" mode)"
  local repo_mode=""
  repo_mode="$(read_setting "$repo_settings_file" mode)"
  local global_repo=""
  global_repo="$(read_setting "$global_settings_file" communityRepo)"
  local repo_repo=""
  repo_repo="$(read_setting "$repo_settings_file" communityRepo)"
  local global_branch=""
  global_branch="$(read_setting "$global_settings_file" baseBranch)"
  local repo_branch=""
  repo_branch="$(read_setting "$repo_settings_file" baseBranch)"
  local global_local_clone_raw=""
  global_local_clone_raw="$(read_setting "$global_settings_file" localClone)"
  local repo_local_clone_raw=""
  repo_local_clone_raw="$(read_setting "$repo_settings_file" localClone)"
  local repo_local_clone=""
  repo_local_clone="$(expand_path "$repo_local_clone_raw" "$repo_root")"
  local global_local_clone=""
  global_local_clone="$(expand_path "$global_local_clone_raw" "$HOME")"
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

  local configured_mode="${global_mode:-${repo_mode:-}}"

  local community_repo="${COMMUNITY_CACHE_REPO:-${repo_repo:-${global_repo:-${manifest_default_repo:-${inferred_repo:-$DEFAULT_COMMUNITY_REPO}}}}}"
  local base_branch="${COMMUNITY_CACHE_BASE_BRANCH:-${repo_branch:-${global_branch:-main}}}"
  local branch_prefix="${COMMUNITY_CACHE_BRANCH_PREFIX:-automation/community-cache-submission}"

  if [[ -z "$community_repo" ]]; then
    echo "[community-cache-submit] COMMUNITY_CACHE_REPO is not set and no community repo is configured" >&2
    exit 1
  fi

  # Check mode allows submission
  local submit_allowed=false
  case "${configured_mode}" in
    pull-and-auto-submit)
      submit_allowed=true
      ;;
    auto-submit-only-public)
      # Check if the current repo is public via gh
      local current_repo_visibility
      current_repo_visibility="$(gh repo view --json visibility --jq '.visibility' 2>/dev/null || echo "")"
      if [[ "$current_repo_visibility" == "PUBLIC" ]]; then
        submit_allowed=true
      else
        echo "[community-cache-submit] Mode is 'auto-submit-only-public' but current repo is not public (visibility: ${current_repo_visibility:-unknown}). Skipping." >&2
        exit 1
      fi
      ;;
    auto-submit-whitelist)
      # Check if current repo is in the whitelist
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
        echo "[community-cache-submit] Mode is 'auto-submit-whitelist' but '${current_nwo:-unknown}' is not in the whitelist. Skipping." >&2
        exit 1
      fi
      ;;
    pull-only|disabled|"")
      if [[ -z "${COMMUNITY_CACHE_REPO:-}" ]]; then
        echo "[community-cache-submit] Community participation mode is '${configured_mode:-not set}'; automatic submission is disabled." >&2
        exit 1
      fi
      # If COMMUNITY_CACHE_REPO is explicitly set via env, allow override
      submit_allowed=true
      ;;
    *)
      echo "[community-cache-submit] Unknown mode: '$configured_mode'" >&2
      exit 1
      ;;
  esac

  validate_packet "$packet_file"

  local statement
  statement="$(trimmed_statement "$packet_file")"
  local statement_hash
  statement_hash="$(printf '%s' "$statement" | shasum -a 256 | awk '{print $1}')"
  local submitted_at
  submitted_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  local submission_id
  submission_id="$(date -u +"%Y%m%dT%H%M%SZ")-$(LC_ALL=C tr -dc 'a-z0-9' </dev/urandom | head -c 8 || true)"

  local temp_dir
  temp_dir="$(mktemp -d -t community-cache-submit.XXXXXX)"
  trap 'rm -rf "${temp_dir:-}"' EXIT

  gh repo clone "$community_repo" "$temp_dir/repo" -- --quiet
  cd "$temp_dir/repo"

  local branch_name="${branch_prefix}-${submission_id}"
  git checkout -b "$branch_name"

  local dated_dir=""
  dated_dir="community-cache/candidates/$(date -u +"%Y-%m-%d")"
  mkdir -p "$dated_dir"
  local target_file="$dated_dir/${submission_id}-${statement_hash:0:12}.json"

  sanitize_packet "$packet_file" "$submitted_at" "$submission_id" "$statement_hash" > "$target_file"

  git config user.name "community-cache-bot"
  git config user.email "community-cache-bot@users.noreply.github.com"
  git add "$target_file"
  git commit -m "Add community cache conclusion $submission_id"
  git push --set-upstream origin "$branch_name"

  gh pr create \
    --repo "$community_repo" \
    --base "$base_branch" \
    --head "$branch_name" \
    --title "Add community cache conclusion $submission_id" \
    --body "Automated community cache submission.

- Submission ID: $submission_id
- Statement hash: $statement_hash
- Source: opted-in audit conclusion packet
- Privacy: sanitized to generalized Copilot guidance only"

  echo "[community-cache-submit] Submitted $target_file to $community_repo" >&2
}

main "$@"