#!/usr/bin/env bash

# lint-intake-workflow.sh
#
# Complete workflow for handling lint feedback:
# 1. Pull latest lint-index updates
# 2. Update errors.dx queue from feedback
# 3. Display status
#
# This wires the helpers lint report intake into SARA's worked errors queue.

set -euo pipefail

DEFAULT_REPO_SLUG="RockyWearsAHat/helpers"
DEFAULT_BASE_BRANCH="main"

usage() {
  cat <<'EOF'
Usage: lint-intake-workflow.sh [options]

Complete workflow for handling lint feedback intake:
  1. Pull latest lint-index updates from Helpers repo
  2. Regenerate errors.dx queue from .helpers/lint-feedback.jsonl
  3. Display status

Options:
  --repo owner/repo     Source GitHub repo (default: RockyWearsAHat/helpers)
  --branch <name>       Source branch (default: main)
  --skip-pull           Skip pulling lint-index updates
  --skip-queue          Skip regenerating errors.dx queue
  -h, --help            Show this help
EOF
}

log() { echo "[lint-intake] $*" >&2; }

main() {
  local repo="" branch="" skip_pull=false skip_queue=false
  local script_dir workspace root

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --repo) repo="$2"; shift 2 ;;
      --branch) branch="$2"; shift 2 ;;
      --skip-pull) skip_pull=true; shift ;;
      --skip-queue) skip_queue=true; shift ;;
      -h|--help) usage; exit 0 ;;
      *) log "Unknown option: $1"; usage; exit 1 ;;
    esac
  done

  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  root="$(cd "$script_dir/.." && pwd)"
  repo="${repo:-$DEFAULT_REPO_SLUG}"
  branch="${branch:-$DEFAULT_BASE_BRANCH}"

  log "Lint intake workflow started"
  log "  workspace: $root"
  log "  repo: $repo@$branch"

  # Step 1: Pull lint-index updates
  if [[ "$skip_pull" != true ]]; then
    log "Step 1: Pulling lint-index updates..."
    if [[ -f "$script_dir/lint-index-pull.sh" ]]; then
      bash "$script_dir/lint-index-pull.sh" \
        --repo "$repo" \
        --branch "$branch" \
        --workspace "$root" \
        || log "Warning: lint-index pull failed (optional)"
    else
      log "Warning: lint-index-pull.sh not found"
    fi
  fi

  # Step 2: Update errors.dx queue from feedback
  if [[ "$skip_queue" != true ]]; then
    log "Step 2: Updating errors.dx queue from feedback..."
    if command -v node >/dev/null 2>&1 && [[ -f "$script_dir/lint-feedback-to-errors.mjs" ]]; then
      node "$script_dir/lint-feedback-to-errors.mjs" \
        --output "$root/errors.dx" \
        || log "Error: could not update errors.dx"
    else
      log "Error: Node.js or lint-feedback-to-errors.mjs not found"
      exit 1
    fi
  fi

  # Step 3: Display status
  if [[ -f "$root/errors.dx" ]]; then
    local violation_count
    violation_count=$(grep -c "^~ coverage:" "$root/errors.dx" || echo "unknown")
    if [[ "$violation_count" != "unknown" ]]; then
      violation_count=$(grep "^~ coverage:" "$root/errors.dx" | sed 's/.*: \([0-9]*\) .*/\1/')
    fi
    log "Status: errors.dx queue ready ($violation_count violations)"
  fi

  log "Lint intake workflow complete"
}

main "$@"
