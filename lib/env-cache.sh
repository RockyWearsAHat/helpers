#!/usr/bin/env bash
# lib/env-cache.sh — Scan cache and ignore-list management for git-help-i-pushed-an-env
# Provides: init_cache, init_issues_cache, init_ignore_file, check_ignore, add_to_ignore,
#           check_cache, update_cache, update_issues_cache, get_cached_issues, get_cache_line

# Cache configuration
CACHE_VERSION="1"
CACHE_FILE="$HOME/.git-secret-scan-cache"
ISSUES_CACHE_FILE="$HOME/.git-secret-scan-issues-cache"
IGNORE_FILE="$HOME/.git-secret-scan-ignore"

# Read latest cache line for a repo (exact match on repo name)
get_cache_line() {
	local repo="$1"
	local file="$2"
	if [ ! -f "$file" ]; then
		return 1
	fi
	/usr/bin/awk -F'|' -v repo="$repo" '$1==repo {line=$0} END{if (line) print line}' "$file" 2>/dev/null
}

init_cache() {
	if [ ! -f "$CACHE_FILE" ]; then
		echo "# git-help-i-pushed-an-env scan cache v$CACHE_VERSION" > "$CACHE_FILE"
		echo "# Format: repo|last_scanned_commit|scan_date|status|can_push" >> "$CACHE_FILE"
	fi
}

init_issues_cache() {
	if [ ! -f "$ISSUES_CACHE_FILE" ]; then
		echo "# git-help-i-pushed-an-env issues cache v$CACHE_VERSION" > "$ISSUES_CACHE_FILE"
		echo "# Format: repo|last_scanned_commit|scan_date|base64_results" >> "$ISSUES_CACHE_FILE"
	fi
}

init_ignore_file() {
	if [ ! -f "$IGNORE_FILE" ]; then
		/usr/bin/touch "$IGNORE_FILE"
	fi
}

check_ignore() {
	local repo="$1"
	local file="$2"
	init_ignore_file
	if grep -qF "${repo}|${file}" "$IGNORE_FILE" 2>/dev/null; then
		return 0
	fi
	return 1
}

add_to_ignore() {
	local repo="$1"
	local file="$2"
	init_ignore_file
	if ! grep -qF "${repo}|${file}" "$IGNORE_FILE"; then
		echo "${repo}|${file}" >> "$IGNORE_FILE"
	fi
}

check_cache() {
	local repo="$1"
	local current_commit="$2"

	if [ "$no_cache" = true ]; then
		return 1
	fi

	init_cache

	local cached_line
	cached_line=$(get_cache_line "$repo" "$CACHE_FILE" 2>/dev/null) || true

	if [ -z "$cached_line" ]; then
		return 1
	fi

	local cached_commit
	cached_commit=$(echo "$cached_line" | /usr/bin/cut -d'|' -f2)
	local cached_status
	cached_status=$(echo "$cached_line" | /usr/bin/cut -d'|' -f4)

	if [ "$cached_commit" = "$current_commit" ] && [ "$cached_status" = "clean" ]; then
		return 0
	fi

	return 1
}

update_cache() {
	local repo="$1"
	local commit="$2"
	local scan_status="$3"
	local can_push="$4"

	init_cache

	local temp_file
	temp_file=$(/usr/bin/mktemp)
	/usr/bin/awk -F'|' -v repo="$repo" '$1!=repo {print $0}' "$CACHE_FILE" > "$temp_file" 2>/dev/null || true
	/bin/mv "$temp_file" "$CACHE_FILE"

	local scan_date
	scan_date=$(/bin/date -u +"%Y-%m-%dT%H:%M:%SZ")
	echo "${repo}|${commit}|${scan_date}|${scan_status}|${can_push}" >> "$CACHE_FILE"
}

update_issues_cache() {
	local repo="$1"
	local commit="$2"
	local results="$3"

	init_issues_cache

	local temp_file
	temp_file=$(/usr/bin/mktemp)
	/usr/bin/awk -F'|' -v repo="$repo" '$1!=repo {print $0}' "$ISSUES_CACHE_FILE" > "$temp_file" 2>/dev/null || true
	/bin/mv "$temp_file" "$ISSUES_CACHE_FILE"

	local scan_date
	scan_date=$(/bin/date -u +"%Y-%m-%dT%H:%M:%SZ")
	local encoded
	encoded=$(printf '%s' "$results" | /usr/bin/base64 | /usr/bin/tr -d '\n')
	echo "${repo}|${commit}|${scan_date}|${encoded}" >> "$ISSUES_CACHE_FILE"
}

get_cached_issues() {
	local repo="$1"
	local commit="$2"

	if [ "$no_cache" = true ]; then
		return 1
	fi

	init_issues_cache

	local cached_line
	cached_line=$(get_cache_line "$repo" "$ISSUES_CACHE_FILE" 2>/dev/null) || true
	if [ -z "$cached_line" ]; then
		return 1
	fi

	local cached_commit
	cached_commit=$(echo "$cached_line" | /usr/bin/cut -d'|' -f2)
	if [ "$cached_commit" != "$commit" ]; then
		return 1
	fi

	local encoded
	encoded=$(echo "$cached_line" | /usr/bin/cut -d'|' -f4-)
	if [ -z "$encoded" ]; then
		return 1
	fi

	printf '%s' "$encoded" | /usr/bin/base64 -D 2>/dev/null
}
