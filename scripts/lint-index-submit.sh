#!/usr/bin/env bash

set -euo pipefail

# lint-index-submit.sh — crawl-on-miss submit-back tier of lint-index resolution.
#
# Role in the tiered design (see lint-index/SCHEMA.md):
#   When the fast path (packed index) and the poll/pull tier both miss — i.e. a
#   toolchain version was never crawled — the shipped crawler/indexer fetches
#   the official docs and (re)generates a lint-index/<tool>.json locally. This
#   script is step 4's *submit-back*: it pushes that freshly-generated index to
#   the live Helpers repo on a dedicated branch and opens a PR, so the crawled
#   result becomes the packed fast path for everyone next time. Crawling is the
#   slow last resort; submitting it back means it only happens once globally.
#
# It mirrors scripts/community-cache-submit.sh: clone the target repo to a temp
# dir, create a branch, commit the single artifact, push the branch, and open a
# PR with `gh`. It never pushes to the base branch directly.

DEFAULT_REPO_SLUG="RockyWearsAHat/helpers"
DEFAULT_BASE_BRANCH="main"
DEFAULT_BRANCH_PREFIX="automation/lint-index"
DEFAULT_INDEX_DIR="lint-index"

usage() {
  cat <<'EOF'
Usage: lint-index-submit.sh [--repo owner/repo] [--branch main]
                            [--dry-run] path/to/lint-index/<tool>.json

Submit a locally-(re)generated packed lint index back to the Helpers repo. The
file is committed on a fresh branch named:

    automation/lint-index-<tool>-<version>

and a pull request is opened with `gh` (if installed). This is the submit-back
tier of the lint-index tiered resolution (see lint-index/SCHEMA.md): a
crawl-on-miss result is contributed so it becomes the packed fast path for
everyone. The base branch is never pushed to directly.

Arguments:
  path/to/<tool>.json   The regenerated lint-index file to submit. Must be a
                        valid lint-index snapshot (tool/toolchainVersion/rules).

Options:
  --repo owner/repo   Target GitHub repo  (default: RockyWearsAHat/helpers)
  --branch <name>     Base branch         (default: main)
  --dry-run           Validate, derive the branch/PR plan, and stop before any
                      clone/push/PR. Nothing is written to the remote.
  -h, --help          Show this help.

Environment overrides (take precedence over flags):
  LINT_INDEX_REPO          Target repo in owner/repo form.
  LINT_INDEX_BASE_BRANCH   Base branch.
  LINT_INDEX_BRANCH_PREFIX Branch prefix (default: automation/lint-index).
EOF
}

log() { echo "[lint-index-submit] $*" >&2; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    log "Missing required command: $1"
    exit 1
  }
}

# A lint-index snapshot must carry its identity and a non-empty rules array so
# the PR is a real, usable index — not a stub. Mirrors community-cache-submit's
# validate_packet guard.
validate_index() {
  local file="$1"
  jq -e '
    (.tool | type == "string" and length >= 1) and
    (.language | type == "string" and length >= 1) and
    (.toolchainVersion | type == "string" and length >= 1) and
    (.rules | type == "array" and length >= 1) and
    ((.ruleCount // (.rules | length)) | type == "number")
  ' "$file" >/dev/null 2>&1 || {
    log "Not a valid lint-index snapshot: $file"
    log "Requires string tool/language/toolchainVersion and a non-empty rules[]."
    exit 1
  }
}

# Slugify a tool/version into a branch-safe token.
slugify() {
  printf '%s' "$1" | LC_ALL=C tr '[:upper:]' '[:lower:]' \
    | LC_ALL=C sed -E 's/[^a-z0-9._-]+/-/g; s/^-+//; s/-+$//'
}

main() {
  local repo="" branch="" dry_run=false packet_file=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --repo) repo="$2"; shift 2 ;;
      --branch) branch="$2"; shift 2 ;;
      --dry-run) dry_run=true; shift ;;
      -h|--help) usage; exit 0 ;;
      -*) log "Unknown option: $1"; usage; exit 1 ;;
      *)
        if [[ -n "$packet_file" ]]; then
          log "Only one index file may be submitted at a time."
          exit 1
        fi
        packet_file="$1"; shift ;;
    esac
  done

  if [[ -z "$packet_file" ]]; then
    log "No lint-index file given."
    usage
    exit 1
  fi
  [[ -f "$packet_file" ]] || {
    log "Index file not found: $packet_file"
    exit 1
  }

  require_cmd git
  require_cmd jq

  repo="${LINT_INDEX_REPO:-${repo:-$DEFAULT_REPO_SLUG}}"
  branch="${LINT_INDEX_BASE_BRANCH:-${branch:-$DEFAULT_BASE_BRANCH}}"
  local branch_prefix="${LINT_INDEX_BRANCH_PREFIX:-$DEFAULT_BRANCH_PREFIX}"

  validate_index "$packet_file"

  # Derive identity for branch/PR naming. The committed filename keeps the
  # submitter's stem (e.g. rust-clippy.json) so it lands as the packed file.
  local tool version dest_name
  tool="$(slugify "$(jq -r '.tool' "$packet_file")")"
  version="$(slugify "$(jq -r '.toolchainVersion' "$packet_file")")"
  dest_name="$(basename "$packet_file")"
  case "$dest_name" in
    *.json) ;;
    *) log "Index file must be a .json file: $dest_name"; exit 1 ;;
  esac

  local branch_name="${branch_prefix}-${tool}-${version}"
  local target_rel="$DEFAULT_INDEX_DIR/$dest_name"
  local rule_count
  rule_count="$(jq -r '.ruleCount // (.rules | length)' "$packet_file")"

  log "Plan:"
  log "  repo        : $repo"
  log "  base branch : $branch"
  log "  new branch  : $branch_name"
  log "  target file : $target_rel"
  log "  tool/version: $tool / $version  (${rule_count} rules)"

  if [[ "$dry_run" == true ]]; then
    log "Dry run: validated and planned only. No clone, push, or PR performed."
    exit 0
  fi

  # Real submission needs gh to clone + open the PR.
  require_cmd gh

  local temp_dir
  temp_dir="$(mktemp -d -t lint-index-submit.XXXXXX)"
  trap 'rm -rf "${temp_dir:-}"' EXIT

  gh repo clone "$repo" "$temp_dir/repo" -- --quiet
  cd "$temp_dir/repo"

  # Guard: never operate on the base branch directly.
  if [[ "$branch_name" == "$branch" ]]; then
    log "Refusing to submit: computed branch equals base branch ($branch)."
    exit 1
  fi
  git checkout -b "$branch_name"

  mkdir -p "$DEFAULT_INDEX_DIR"
  cp "$OLDPWD/$packet_file" "$target_rel" 2>/dev/null || cp "$packet_file" "$target_rel"

  if git diff --quiet -- "$target_rel" && git diff --cached --quiet -- "$target_rel" \
     && [[ -z "$(git status --porcelain -- "$target_rel")" ]]; then
    log "No change versus the packed index already on $repo@$branch; nothing to submit."
    exit 0
  fi

  git config user.name "lint-index-bot"
  git config user.email "lint-index-bot@users.noreply.github.com"
  git add "$target_rel"
  git commit -m "lint-index: add ${tool} ${version} (${rule_count} rules)"
  git push --set-upstream origin "$branch_name"

  gh pr create \
    --repo "$repo" \
    --base "$branch" \
    --head "$branch_name" \
    --title "lint-index: ${tool} ${version}" \
    --body "Automated lint-index submission (crawl-on-miss submit-back).

- Tool: ${tool}
- Toolchain version: ${version}
- Rules: ${rule_count}
- File: ${target_rel}
- Source: shipped crawler/indexer output (see lint-index/SCHEMA.md)

This is the submit-back tier of the lint-index tiered resolution: a
crawled-on-miss index is contributed so it becomes the packed fast path."

  log "Submitted $target_rel to $repo on branch $branch_name."
}

main "$@"
