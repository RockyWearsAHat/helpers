#!/usr/bin/env bash
set -euo pipefail
SRC="/Users/alexwaldmann/bin/git-upload"
OUT="/Users/alexwaldmann/bin-refactor-modular/git-upload.new"

{
  # Header: shebang, comments, set, variables, DEFAULT_AI_CMD
  sed -n '1,38p' "$SRC"

  printf '\n'
  printf '# ── Library loading ────────────────────────────────────────────────────\n'
  printf 'SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"\n'
  printf 'source "${SCRIPT_DIR}/lib/upload-spinner.sh"\n'
  printf 'source "${SCRIPT_DIR}/lib/upload-test-detection.sh"\n'
  printf 'source "${SCRIPT_DIR}/lib/upload-test-output.sh"\n'
  printf 'source "${SCRIPT_DIR}/lib/upload-diff-analysis.sh"\n'
  printf 'source "${SCRIPT_DIR}/lib/upload-ai-message.sh"\n'
  printf '\n'
  printf '# Cleanup spinner on exit\n'
  printf "trap 'stop_spinner' EXIT INT TERM\n"
  printf '\n'
  printf '# ── Argument parsing ──────────────────────────────────────────────────\n'

  # Argument parsing block (lines 77-101 in the original)
  sed -n '77,101p' "$SRC"

  printf '\n'
  printf '# ── Main ──────────────────────────────────────────────────────────────\n'

  # main() through end of file
  sed -n '2615,2959p' "$SRC"
} > "$OUT"

echo "Done. New file: $OUT ($(wc -l < "$OUT") lines)"
