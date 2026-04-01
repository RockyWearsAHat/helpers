#!/usr/bin/env bash

# scripts/test-node-coverage.sh
# Enforce full coverage for critical Node modules that already have focused
# unit tests. Coverage artifacts are written to a temporary directory so the
# workspace stays clean.

set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

temp_dir="$(mktemp -d -t gsh-node-coverage.XXXXXX)"
trap 'rm -rf "${temp_dir:-}"' EXIT

npx --yes c8 \
	--all \
	--include=lib/mcp-language-models.js \
	--reporter=text-summary \
	--report-dir "$temp_dir/report" \
	--temp-directory "$temp_dir/c8" \
	--check-coverage \
	--lines 100 \
	--functions 100 \
	--branches 100 \
	--statements 100 \
	node ./scripts/test-list-language-models.js