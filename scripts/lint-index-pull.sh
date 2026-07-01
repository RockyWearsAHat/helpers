#!/usr/bin/env bash

set -euo pipefail

# lint-index-pull.sh — poll/pull tier of the lint-index tiered resolution.
#
# Role in the tiered design (see lint-index/SCHEMA.md):
#   The packed, committed lint-index/<tool>.json files are the fast path: a
#   project's toolchain version is checksum-matched against them with no fetch.
#   This script is the *poll/pull* tier (step 3): it periodically — and on a
#   fast-path miss — refreshes the local lint-index/ directory from the live
#   Helpers repo so a newer packed index (covering a newer toolchain) becomes
#   the local fast path without anyone re-crawling docs.
#
# It mirrors scripts/community-cache-pull.sh: it can read from a local clone or
# fetch raw files from GitHub, and it only writes files whose content actually
# changed.
#
# Fast "already current" check:
#   Before touching any file, it compares the remote lint-index *tree* against
#   the local one with a single lightweight `git ls-remote` (or, for raw-fetch
#   mode, a directory listing checksum). If the remote tree object id matches
#   what we last pulled, the script exits early — O(one network round-trip),
#   no per-file downloads. Only on a tree mismatch does it fall through to a
#   per-file checksum diff, writing only files whose sha256 differs.

DEFAULT_REPO_SLUG="RockyWearsAHat/helpers"
DEFAULT_BASE_BRANCH="main"
DEFAULT_INDEX_DIR="lint-index"
# Files in lint-index/ that are never tool snapshots (skip on copy).
NON_INDEX_FILES=("SCHEMA.md")

usage() {
  cat <<'EOF'
Usage: lint-index-pull.sh [--repo owner/repo] [--branch main]
                          [--workspace /path/to/repo] [--force] [--dry-run]

Poll the live Helpers repo for updates to the lint-index/ directory and pull
any changed packed index files into the local lint-index/. This is the
poll/pull tier of the lint-index tiered resolution (see lint-index/SCHEMA.md):
it keeps the local fast-path index current without re-crawling docs.

Options:
  --repo owner/repo   Source GitHub repo      (default: RockyWearsAHat/helpers)
  --branch <name>     Source branch           (default: main)
  --workspace <dir>   Repo whose lint-index/ to update (default: current git repo)
  --force             Pull even if the remote tree matches the last pull.
  --dry-run           Report what would change; write nothing.
  -h, --help          Show this help.

Environment overrides (take precedence over flags):
  LINT_INDEX_REPO         Source repo in owner/repo form.
  LINT_INDEX_BASE_BRANCH  Source branch.
  LINT_INDEX_LOCAL_CLONE  Path to a local clone to copy from instead of GitHub.

Fast path: a single `git ls-remote` resolves the remote lint-index tree id; if
it matches the recorded last-pull id the script exits without downloading
anything. Otherwise only files whose sha256 differs from the local copy are
written.
EOF
}

log() { echo "[lint-index-pull] $*" >&2; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    log "Missing required command: $1"
    exit 1
  }
}

# sha256 of a file, hex only (portable across shasum/sha256sum).
file_sha256() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    shasum -a 256 "$file" | awk '{print $1}'
  fi
}

is_non_index_file() {
  local name="$1"
  local skip
  for skip in "${NON_INDEX_FILES[@]}"; do
    [[ "$name" == "$skip" ]] && return 0
  done
  return 1
}

# Resolve the git object id of <branch>:<index_dir> on the remote without a
# full fetch. Returns empty string if it cannot be determined cheaply.
remote_tree_id() {
  local repo="$1" branch="$2" index_dir="$3" local_clone="$4"
  if [[ -n "$local_clone" && -d "$local_clone/.git" ]]; then
    git -C "$local_clone" rev-parse "${branch}:${index_dir}" 2>/dev/null || true
    return 0
  fi
  # ls-remote gives the commit id cheaply; we hash that plus the index path so a
  # change to lint-index/ (which changes the commit) invalidates the cache.
  local commit
  commit="$(git ls-remote "https://github.com/${repo}.git" "refs/heads/${branch}" 2>/dev/null | awk '{print $1}')"
  [[ -n "$commit" ]] && printf '%s:%s\n' "$commit" "$index_dir"
}

# List the index files (basenames) available on the source.
list_remote_index_files() {
  local repo="$1" branch="$2" index_dir="$3" local_clone="$4"
  if [[ -n "$local_clone" && -d "$local_clone/$index_dir" ]]; then
    find "$local_clone/$index_dir" -maxdepth 1 -type f -name '*.json' -exec basename {} \; 2>/dev/null || true
    return 0
  fi
  # GitHub contents API: list *.json names in the index dir.
  local api="https://api.github.com/repos/${repo}/contents/${index_dir}?ref=${branch}"
  curl -fsSL "$api" 2>/dev/null \
    | jq -r '.[]? | select(.type == "file") | .name | select(endswith(".json"))' \
    || true
}

fetch_one_file() {
  local repo="$1" branch="$2" index_dir="$3" name="$4" dest="$5" local_clone="$6"
  if [[ -n "$local_clone" && -f "$local_clone/$index_dir/$name" ]]; then
    cp "$local_clone/$index_dir/$name" "$dest"
    return 0
  fi
  local url="https://raw.githubusercontent.com/${repo}/${branch}/${index_dir}/${name}"
  curl -fsSL "$url" -o "$dest"
}

main() {
  local repo="" branch="" workspace="" force=false dry_run=false

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --repo) repo="$2"; shift 2 ;;
      --branch) branch="$2"; shift 2 ;;
      --workspace) workspace="$2"; shift 2 ;;
      --force) force=true; shift ;;
      --dry-run) dry_run=true; shift ;;
      -h|--help) usage; exit 0 ;;
      *) log "Unknown option: $1"; usage; exit 1 ;;
    esac
  done

  require_cmd git
  require_cmd curl
  require_cmd jq

  repo="${LINT_INDEX_REPO:-${repo:-$DEFAULT_REPO_SLUG}}"
  branch="${LINT_INDEX_BASE_BRANCH:-${branch:-$DEFAULT_BASE_BRANCH}}"
  local local_clone="${LINT_INDEX_LOCAL_CLONE:-}"

  if [[ -z "$workspace" ]]; then
    workspace="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
  fi
  local index_dir_abs="$workspace/$DEFAULT_INDEX_DIR"
  mkdir -p "$index_dir_abs"

  # --- Fast "already current" check -------------------------------------------
  local state_file="$index_dir_abs/.pull-state"
  local last_tree_id=""
  [[ -f "$state_file" ]] && last_tree_id="$(cat "$state_file" 2>/dev/null || true)"
  local current_tree_id=""
  current_tree_id="$(remote_tree_id "$repo" "$branch" "$DEFAULT_INDEX_DIR" "$local_clone")"

  if [[ "$force" != true && -n "$current_tree_id" && "$current_tree_id" == "$last_tree_id" ]]; then
    log "Already current (lint-index tree ${current_tree_id} unchanged since last pull)."
    exit 0
  fi

  # --- Per-file checksum diff -------------------------------------------------
  local temp_dir
  temp_dir="$(mktemp -d -t lint-index-pull.XXXXXX)"
  trap 'rm -rf "${temp_dir:-}"' EXIT

  local updated=0 checked=0
  local name
  while IFS= read -r name; do
    [[ -n "$name" ]] || continue
    is_non_index_file "$name" && continue
    checked=$((checked + 1))

    local staged="$temp_dir/$name"
    if ! fetch_one_file "$repo" "$branch" "$DEFAULT_INDEX_DIR" "$name" "$staged" "$local_clone"; then
      log "Warning: could not fetch $name; skipping."
      continue
    fi

    local dest="$index_dir_abs/$name"
    local remote_sum local_sum=""
    remote_sum="$(file_sha256 "$staged")"
    [[ -f "$dest" ]] && local_sum="$(file_sha256 "$dest")"

    if [[ "$remote_sum" == "$local_sum" ]]; then
      continue
    fi

    if [[ "$dry_run" == true ]]; then
      log "Would update: $name (sha ${local_sum:-<absent>} -> ${remote_sum})"
    else
      mv "$staged" "$dest"
      log "Updated: $name"
    fi
    updated=$((updated + 1))
  done < <(list_remote_index_files "$repo" "$branch" "$DEFAULT_INDEX_DIR" "$local_clone")

  if [[ "$checked" -eq 0 ]]; then
    log "No index files found on $repo@$branch:$DEFAULT_INDEX_DIR/."
  fi

  if [[ "$dry_run" == true ]]; then
    log "Dry run: $updated of $checked file(s) would change. State unchanged."
    exit 0
  fi

  # Record the tree id only after a successful real pull, so a future fast-path
  # check can short-circuit.
  if [[ -n "$current_tree_id" ]]; then
    printf '%s\n' "$current_tree_id" > "$state_file"
  fi
  log "Pull complete: $updated of $checked file(s) updated."
}

main "$@"
