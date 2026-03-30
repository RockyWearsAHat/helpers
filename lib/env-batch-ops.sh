#!/usr/bin/env bash
# lib/env-batch-ops.sh — Multi-repo scan and clean operations for git-help-i-pushed-an-env
# Provides: run_interactive_mode, run_scan_only, scan_multiple_repos,
#           clean_selected_repos, clean_multiple_repos

run_interactive_mode() {
	local repo_root
	repo_root=$(get_repo_root)

	printf '\n%b🔍 Interactive Sensitive File Scanner%b\n\n' "$BOLD" "$NC"

	printf 'Would you like to:\n'
	printf '  %b[1]%b Scan for all known sensitive file patterns (default)\n' "$GREEN" "$NC"
	printf '  %b[2]%b Search for specific file patterns\n' "$YELLOW" "$NC"
	printf '  %b[3]%b Specify exact files to examine\n' "$BLUE" "$NC"
	printf '\nChoice [1-3, default=1]: '

	read -r scan_choice
	scan_choice="${scan_choice:-1}"

	local use_search=false

	case "$scan_choice" in
		2)
			printf '\nEnter patterns to search for (comma-separated, e.g., ".env,secrets.json,*.key"):\n> '
			read -r pattern_input
			IFS=',' read -ra search_patterns <<< "$pattern_input"
			use_search=true
			;;
		3)
			printf '\nEnter exact file paths (comma-separated, relative to repo root):\n> '
			read -r file_input
			IFS=',' read -ra search_patterns <<< "$file_input"
			use_search=true
			;;
	esac

	start_spinner "Scanning for suspicious filenames..."
	local scan_results
	scan_results=$(scan_workspace_files "$repo_root" "$use_search")
	stop_spinner "Filename scan complete"

	start_spinner "Running AI-powered secret detection (Copilot)..."
	local copilot_results
	copilot_results=$(scan_with_copilot "$repo_root" 2>/dev/null) || copilot_results=""
	stop_spinner "AI scan complete"

	if [ -n "$copilot_results" ]; then
		if [ -n "$scan_results" ]; then
			scan_results="$scan_results"$'\n'"$copilot_results"
		else
			scan_results="$copilot_results"
		fi
	fi

	if [ -z "$scan_results" ]; then
		log_success "No sensitive files or secrets found in workspace or history!"
		return 0
	fi

	local file_count
	file_count=$(echo "$scan_results" | grep -c '^' || echo "0")

	printf '\n%bFound %d potentially sensitive file(s):%b\n\n' "$YELLOW" "$file_count" "$NC"

	echo "$scan_results" | while IFS='|' read -r loc path stat; do
		printf '  %b[%s]%b %s (%s)\n' "$CYAN" "$loc" "$NC" "$path" "$stat"
	done

	printf '\n%bOptions:%b\n' "$BOLD" "$NC"
	printf '  %b[1]%b Review each file individually (recommended)\n' "$GREEN" "$NC"
	printf '  %b[2]%b Remove ALL found files from history\n' "$RED" "$NC"
	printf '  %b[3]%b Add ALL found files to .gitignore only\n' "$YELLOW" "$NC"
	printf '  %b[4]%b Cancel\n' "$BLUE" "$NC"
	printf '\nChoice [1-4]: '

	read -r action_choice

	typeset -a files_to_remove
	files_to_remove=()

	case "$action_choice" in
		1)
			echo "$scan_results" | while IFS= read -r entry; do
				if [ -n "$entry" ]; then
					if ! handle_file_interactively "$entry" "$repo_root"; then
						break
					fi
				fi
			done
			;;
		2)
			echo "$scan_results" | while IFS='|' read -r loc filepath stat; do
				files_to_remove+=("$filepath")
				add_file_to_gitignore "$repo_root" "$filepath"
			done
			;;
		3)
			echo "$scan_results" | while IFS='|' read -r loc filepath stat; do
				add_file_to_gitignore "$repo_root" "$filepath"
				if [ "$loc" = "current" ]; then
					git rm --cached "$filepath" 2>/dev/null || true
				fi
			done
			git add .gitignore 2>/dev/null || true
			git commit -m "Add sensitive file patterns to .gitignore" 2>/dev/null || true
			log_success "Files added to .gitignore"
			return 0
			;;
		4|*)
			log_info "Cancelled."
			return 0
			;;
	esac

	if [ ${#files_to_remove[@]} -gt 0 ]; then
		printf '\n%b⚠️  About to remove %d file(s) from git history%b\n' "$YELLOW" "${#files_to_remove[@]}" "$NC"
		printf 'This will rewrite history. Continue? (yes/no): '
		read -r confirm
		if [ "$confirm" = "yes" ]; then
			remove_files_from_history "$repo_root" "${files_to_remove[@]}"

			git add .gitignore 2>/dev/null || true
			git commit -m "Add removed sensitive files to .gitignore" 2>/dev/null || true

			printf '\n%bNext steps:%b\n' "$BOLD" "$NC"
			printf '  git push --force --all\n'
			printf '  git push --force --tags\n'
			printf '\n%b⚠️  ROTATE ALL EXPOSED CREDENTIALS!%b\n\n' "$RED" "$NC"
		else
			log_info "Operation cancelled."
		fi
	fi
}

run_scan_only() {
	local repo_root
	repo_root=$(get_repo_root)

	printf '\n%b🔍 Scanning for sensitive files and secrets...%b\n\n' "$BOLD" "$NC"

	start_spinner "Running AI-powered secret detection (Copilot)..."
	local scan_results=""
	scan_results=$(scan_with_copilot "$repo_root" 2>/dev/null) || scan_results=""
	stop_spinner "AI scan complete"

	if [ -z "$scan_results" ]; then
		log_success "No sensitive files or secrets found!"
		return 0
	fi

	local file_count
	file_count=$(echo "$scan_results" | grep -c '^' || echo "0")

	printf '%bFound %d potentially sensitive item(s):%b\n\n' "$YELLOW" "$file_count" "$NC"

	printf '%b%-10s %-40s %s%b\n' "$BOLD" "SOURCE" "FILE" "DETAILS" "$NC"
	printf '%s\n' "────────── ──────────────────────────────────────── ──────────────"

	typeset repo_name
	repo_name=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) || repo_name=$(basename "$repo_root")

	typeset -a REVIEW_LIST=()
	while IFS='|' read -r loc path details; do
		[[ -z "$loc" ]] && continue
		if check_ignore "$repo_name" "$path"; then
			continue
		fi
		REVIEW_LIST+=("$repo_name|$path|$loc|$details")
		printf '%b%-10s%b %-40s %s\n' "$CYAN" "$loc" "$NC" "$path" "$details"
	done <<< "$scan_results"

	if [ ${#REVIEW_LIST[@]} -gt 0 ]; then
		interactive_review_menu "true" "true"
	fi
}

scan_multiple_repos() {
	local visibility="$1"
	local temp_dir
	temp_dir=$(mktemp -d)

	init_cache

	log_info "Fetching repository list from GitHub..."

	local repos
	repos=$(get_github_repos "$visibility")

	if [ -z "$repos" ]; then
		log_warn "No repositories found."
		command rm -rf "$temp_dir" 2>/dev/null || true
		return 0
	fi

	local repo_count
	repo_count=$(echo "$repos" | wc -l | tr -d ' ')

	typeset -a REVIEW_LIST=()

	printf '\n%bScanning repositories for sensitive files:%b\n' "$BOLD" "$NC"
	if [ "$no_cache" = true ]; then
		printf '%b(cache disabled - scanning all repos)%b\n\n' "$YELLOW" "$NC"
	else
		printf '%b(using cache - use --no-cache to rescan all)%b\n\n' "$CYAN" "$NC"
	fi

	local total_issues=0
	local repos_with_issues=0
	local clean_repos=0
	local cached_repos=0
	local warning_repos=0
	local current_repo_num=0

	local orig_dir
	orig_dir=$(pwd)

	local repo_array
	repo_array=("${(@f)repos}")
	for repo in "${repo_array[@]}"; do
		[ -z "$repo" ] && continue
		((current_repo_num++)) || true

		typeset access_level=""
		access_level=$(check_push_access "$repo")
		typeset can_modify=false
		if [ "$access_level" = "owner" ] || [ "$access_level" = "collaborator" ]; then
			can_modify=true
		fi

		typeset latest_commit=""
		latest_commit=$(gh api "repos/$repo/commits?per_page=1" --jq '.[0].sha' 2>/dev/null) || latest_commit=""

		local cached_commit_line=""
		local cached_commit=""
		local cached_status=""
		if [ -z "$latest_commit" ]; then
			cached_commit_line=$(get_cache_line "$repo" "$CACHE_FILE" 2>/dev/null) || true
			if [ -n "$cached_commit_line" ]; then
				cached_commit=$(echo "$cached_commit_line" | /usr/bin/cut -d'|' -f2)
				cached_status=$(echo "$cached_commit_line" | /usr/bin/cut -d'|' -f4)
				if [ -n "$cached_commit" ]; then
					latest_commit="$cached_commit"
				fi
			fi
		fi

		if [ -n "$latest_commit" ] && check_cache "$repo" "$latest_commit"; then
			((cached_repos++)) || true
			((clean_repos++)) || true
			continue
		fi

		local cached_results=""
		if [ -n "$latest_commit" ]; then
			cached_results=$(get_cached_issues "$repo" "$latest_commit" 2>/dev/null || true)
		fi
		if [ -n "$cached_results" ]; then
			local issue_count
			issue_count=$(echo "$cached_results" | wc -l | tr -d ' ')
			printf '%b[%d/%d]%b %b→ %s%b ' "$CYAN" "$current_repo_num" "$repo_count" "$NC" "$CYAN" "$repo" "$NC"
			if [ "$can_modify" = true ]; then
				printf '%b⚠ %d ISSUE(s)%b [cached] [%s]\n' "$RED" "$issue_count" "$NC" "$access_level"
				((repos_with_issues++)) || true
			else
				printf '%b⚠ %d WARNING(s)%b [cached] [%s - cannot modify]\n' "$YELLOW" "$issue_count" "$NC" "$access_level"
				((warning_repos++)) || true
			fi
			((total_issues += issue_count)) || true
			((cached_repos++)) || true

			printf '\n%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n' "$CYAN" "$NC"
			if [ "$can_modify" = true ]; then
				printf '%b🔧 Issues found (YOU CAN FIX):%b\n' "$BOLD" "$NC"
			else
				printf '%b👀 Warnings found (not your repo):%b\n' "$BOLD" "$NC"
			fi
			while IFS='|' read -r loc path details; do
				[[ -z "$loc" ]] && continue
				if check_ignore "$repo" "$path"; then
					continue
				fi
				REVIEW_LIST+=("$repo|$path|$loc|$details")
				typeset color="$YELLOW"
				typeset icon="⚠"
				case "$loc" in
					copilot|history-ai)
						color="$RED"
						icon="🔑"
						;;
					current)
						color="$RED"
						icon="📄"
						;;
					history)
						color="$YELLOW"
						icon="📜"
						;;
				esac
				printf '  %b%s [%s]%b %s\n' "$color" "$icon" "$loc" "$NC" "$path"
				if [[ -n "$details" && "$details" != "deleted" && "$details" != "was-committed" ]]; then
					printf '     └─ %b%s%b\n' "$CYAN" "$details" "$NC"
				fi
				printf '     └─ %b%s%b\n' "$BLUE" "$(github_file_link "$repo" "$path")" "$NC"
			done <<< "$cached_results"
			printf '%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n\n' "$CYAN" "$NC"
			continue
		fi

		if [ $current_repo_num -gt 1 ]; then
			printf '%b[%d/%d]%b %b→ %s%b %b[cloning]%b ' "$CYAN" "$current_repo_num" "$repo_count" "$NC" "$CYAN" "$repo" "$NC" "$CYAN" "$NC"
		else
			printf '%b[%d/%d]%b %b→ %s%b ' "$CYAN" "$current_repo_num" "$repo_count" "$NC" "$CYAN" "$repo" "$NC"
		fi

		local repo_name="${repo##*/}"
		local repo_owner="${repo%%/*}"
		local repo_dir="$temp_dir/${repo_owner}--${repo_name}"

		[ -d "$repo_dir" ] && rm -rf "$repo_dir" 2>/dev/null || true

		local clone_success=false
		local clone_attempts=0
		local max_attempts=3
		local clone_error_file="/tmp/clone_error_$$"
		while [ $clone_attempts -lt $max_attempts ] && [ "$clone_success" = false ]; do
			if PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:$PATH" /opt/homebrew/bin/gh repo clone "$repo" "$repo_dir" -- --depth=1 >/dev/null 2>"$clone_error_file"; then
				clone_success=true
			else
				((clone_attempts++)) || true
				if [ $clone_attempts -lt $max_attempts ]; then
					printf '%b(retry %d/%d)%b ' "$YELLOW" "$clone_attempts" "$((max_attempts-1))" "$NC"
					/bin/sleep 3
				fi
			fi
		done

		if [ "$clone_success" = false ]; then
			local clone_error=""
			[ -f "$clone_error_file" ] && clone_error=$(cat "$clone_error_file" 2>/dev/null) || true
			local error_reason="unknown"
			if echo "$clone_error" | grep -qi "not found\|404" 2>/dev/null; then
				error_reason="repo not found"
			elif echo "$clone_error" | grep -qi "permission\|403\|access" 2>/dev/null; then
				error_reason="no access"
			elif echo "$clone_error" | grep -qi "timeout" 2>/dev/null; then
				error_reason="timeout"
			elif echo "$clone_error" | grep -qi "already exists" 2>/dev/null; then
				error_reason="dir exists"
			fi
			rm -f "$clone_error_file" 2>/dev/null || true
			printf '%b✗ Failed (%s)%b\n' "$RED" "$error_reason" "$NC"
			continue
		fi
		rm -f "$clone_error_file" 2>/dev/null || true

		/bin/sleep 1

		(
			cd "$repo_dir" 2>/dev/null || exit 1
			git fetch --unshallow >/dev/null 2>&1 || true
		)

		local repo_commit=""
		repo_commit=$(cd "$repo_dir" 2>/dev/null && git rev-parse HEAD 2>/dev/null) || repo_commit=""
		if [ -z "$repo_commit" ]; then
			repo_commit=$(cd "$repo_dir" 2>/dev/null && git ls-remote origin HEAD 2>/dev/null | /usr/bin/awk '{print $1}') || repo_commit=""
		fi
		if [ -z "$repo_commit" ]; then
			repo_commit="unknown"
		fi
		if [ -z "$repo_commit" ]; then
			repo_commit="$latest_commit"
		fi
		if [ -z "$latest_commit" ] && [ -n "$repo_commit" ]; then
			latest_commit="$repo_commit"
		fi

		local use_search=false
		if [ ${#search_patterns[@]} -gt 0 ]; then
			use_search=true
		fi

		local scan_results=""
		local copilot_results=""
		if cd "$repo_dir" 2>/dev/null; then
			printf '%b[AI scanning]%b ' "$MAGENTA" "$NC"
			copilot_results=$(scan_with_copilot "$repo_dir" 2>/dev/null) || copilot_results=""
			scan_results="$copilot_results"
			cd "$orig_dir" 2>/dev/null || true
		fi

		if [ -z "$repo_commit" ]; then
			repo_commit=$(cd "$repo_dir" 2>/dev/null && git rev-parse HEAD 2>/dev/null) || repo_commit="$latest_commit"
		fi

		if [ -z "$scan_results" ]; then
			printf '%b✓ Clean | Added to cache%b\n' "$GREEN" "$NC"
			((clean_repos++)) || true
			update_cache "$repo" "$repo_commit" "clean" "$can_modify"
			if [ -d "$repo_dir" ]; then
				command rm -rf "$repo_dir" 2>/dev/null || true
			fi
		else
			local issue_count
			issue_count=$(echo "$scan_results" | wc -l | tr -d ' ')

			if [ "$can_modify" = true ]; then
				printf '%b⚠ %d ISSUE(s)%b [%s]\n' "$RED" "$issue_count" "$NC" "$access_level"
				((repos_with_issues++)) || true
			else
				printf '%b⚠ %d WARNING(s)%b [%s - cannot modify]\n' "$YELLOW" "$issue_count" "$NC" "$access_level"
				((warning_repos++)) || true
			fi
			((total_issues += issue_count)) || true

			update_cache "$repo" "$repo_commit" "issues" "$can_modify"
			update_issues_cache "$repo" "$repo_commit" "$scan_results"

			printf '\n%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n' "$CYAN" "$NC"
			if [ "$can_modify" = true ]; then
				printf '%b🔧 Issues found (YOU CAN FIX):%b\n' "$BOLD" "$NC"
			else
				printf '%b👀 Warnings found (not your repo):%b\n' "$BOLD" "$NC"
			fi

			while IFS='|' read -r loc path details; do
				[[ -z "$loc" ]] && continue
				if check_ignore "$repo" "$path"; then
					continue
				fi
				REVIEW_LIST+=("$repo|$path|$loc|$details")
				typeset color="$YELLOW"
				typeset icon="⚠"
				case "$loc" in
					copilot|history-ai)
						color="$RED"
						icon="🔑"
						;;
					current)
						color="$RED"
						icon="📄"
						;;
					history)
						color="$YELLOW"
						icon="📜"
						;;
				esac
				printf '  %b%s [%s]%b %s\n' "$color" "$icon" "$loc" "$NC" "$path"
				if [[ -n "$details" && "$details" != "deleted" && "$details" != "was-committed" ]]; then
					printf '     └─ %b%s%b\n' "$CYAN" "$details" "$NC"
				fi
				printf '     └─ %b%s%b\n' "$BLUE" "$(github_file_link "$repo" "$path")" "$NC"
			done <<< "$scan_results"
			printf '%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n\n' "$CYAN" "$NC"
		fi

		if [ -d "$repo_dir" ]; then
			command rm -rf "$repo_dir" 2>/dev/null || true
		fi
	done

	printf '\n%b[Cleaning up temporary repositories...]%b\n' "$CYAN" "$NC"
	if [ -d "$temp_dir" ]; then
		local repos_to_delete=0
		repos_to_delete=$(/usr/bin/find "$temp_dir" -maxdepth 1 -type d ! -name "$(/usr/bin/basename "$temp_dir")" 2>/dev/null | /usr/bin/wc -l)

		if [ "$repos_to_delete" -gt 0 ]; then
			printf 'Removing %d temporary repository/repositories...' "$repos_to_delete"
			command rm -rf "$temp_dir"/* 2>/dev/null || true
			printf ' ✓\n'
		fi
		command rm -rf "$temp_dir" 2>/dev/null || true
	fi

	cd "$orig_dir" || true

	printf '\n%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n' "$CYAN" "$NC"
	printf '%bScan Complete!%b\n' "$BOLD" "$NC"
	printf '  Repositories scanned: %d\n' "$repo_count"
	if [ "$cached_repos" -gt 0 ]; then
		printf '  Skipped (cached clean): %b%d%b\n' "$CYAN" "$cached_repos" "$NC"
	fi
	printf '  Clean repositories: %b%d%b\n' "$GREEN" "$clean_repos" "$NC"
	if [ "$repos_with_issues" -gt 0 ]; then
		printf '  %bRepos with issues you can fix: %d%b\n' "$RED" "$repos_with_issues" "$NC"
	fi
	if [ "$warning_repos" -gt 0 ]; then
		printf '  %bRepos with warnings (no access): %d%b\n' "$YELLOW" "$warning_repos" "$NC"
	fi
	printf '  Findings (files/secrets): %b%d%b\n' "$RED" "$total_issues" "$NC"
	printf '%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n\n' "$CYAN" "$NC"

	if [ ${#REVIEW_LIST[@]} -gt 0 ]; then
		interactive_review_menu
	fi

	if [ "$repos_with_issues" -gt 0 ]; then
		log_warn "You have $repos_with_issues repo(s) with issues YOU can fix!"
	fi
	if [ "$warning_repos" -gt 0 ]; then
		log_info "$warning_repos repo(s) have issues but you don't have push access (warnings only)."
	fi
	if [ "$repos_with_issues" -eq 0 ] && [ "$warning_repos" -eq 0 ]; then
		log_success "All your repositories are clean!"
	fi

	printf '\n%bCache stored at: %s%b\n' "$CYAN" "$CACHE_FILE" "$NC"
	printf '%bNext scan will skip clean repos (use --no-cache to rescan all)%b\n\n' "$CYAN" "$NC"
}

clean_selected_repos() {
	local repos=("$@")
	local temp_dir
	temp_dir=$(mktemp -d)

	printf '\n%bCleaning %d repositories...%b\n\n' "$BOLD" "${#repos[@]}" "$NC"

	local success_count=0
	local fail_count=0

	for repo in "${repos[@]}"; do
		printf '%b→ %s%b\n' "$CYAN" "$repo" "$NC"

		local repo_dir="$temp_dir/$(basename "$repo")"

		if gh repo clone "$repo" "$repo_dir" 2>/dev/null; then
			cd "$repo_dir" || continue

			if clean_repository; then
				if git push --force --all 2>/dev/null && git push --force --tags 2>/dev/null; then
					log_success "Cleaned and pushed: $repo"
					((success_count++))
				else
					log_error "Cleaned but failed to push: $repo"
					((fail_count++))
				fi
			else
				log_error "Failed to clean: $repo"
				((fail_count++))
			fi

			cd - > /dev/null
		else
			log_error "Failed to clone: $repo"
			((fail_count++))
		fi

		rm -rf "$repo_dir"
	done

	rm -rf "$temp_dir"

	printf '\n%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n' "$CYAN" "$NC"
	printf '%bCleaning Summary:%b\n' "$BOLD" "$NC"
	printf '  Successfully cleaned: %b%d%b\n' "$GREEN" "$success_count" "$NC"
	printf '  Failed: %b%d%b\n' "$RED" "$fail_count" "$NC"
	printf '%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n\n' "$CYAN" "$NC"

	printf '%b⚠️  REMEMBER TO ROTATE ALL EXPOSED CREDENTIALS!%b\n\n' "$RED" "$NC"
}

clean_multiple_repos() {
	local visibility="$1"
	local temp_dir
	temp_dir=$(mktemp -d)

	log_info "Fetching repository list from GitHub..."

	local repos
	repos=$(get_github_repos "$visibility")

	if [ -z "$repos" ]; then
		log_warn "No repositories found."
		return 0
	fi

	local repo_count
	repo_count=$(echo "$repos" | wc -l | tr -d ' ')

	printf '\n%bRepositories to clean (%d):%b\n' "$BOLD" "$repo_count" "$NC"

	echo "$repos" | while read -r repo; do
		printf '  • %s\n' "$repo"
	done

	printf '\n'

	if [ "$force" = false ]; then
		printf '%b⚠️  WARNING: This will rewrite history for %d repositories!%b\n' "$YELLOW" "$repo_count" "$NC"
		printf 'Are you sure you want to continue? (yes/no): '
		read -r confirm
		if [ "$confirm" != "yes" ]; then
			log_info "Operation cancelled."
			rm -rf "$temp_dir"
			return 1
		fi
	fi

	local success_count=0
	local fail_count=0

	echo "$repos" | while read -r repo; do
		printf '\n%b→ %s%b\n' "$BOLD" "$repo" "$NC"

		local repo_dir="$temp_dir/$(basename "$repo")"

		if gh repo clone "$repo" "$repo_dir" -- --mirror 2>/dev/null; then
			cd "$repo_dir"

			if clean_repository; then
				if [ "$dry_run" = false ]; then
					if git push --force --all 2>/dev/null && git push --force --tags 2>/dev/null; then
						log_success "Successfully cleaned: $repo"
						((success_count++))
					else
						log_error "Failed to push changes: $repo"
						((fail_count++))
					fi
				else
					log_info "[DRY RUN] Would clean: $repo"
					((success_count++))
				fi
			else
				((fail_count++))
			fi

			cd - > /dev/null
		else
			log_error "Failed to clone: $repo"
			((fail_count++))
		fi

		rm -rf "$repo_dir"
	done

	rm -rf "$temp_dir"

	printf '\n%bComplete:%b %d succeeded, %d failed\n\n' "$BOLD" "$NC" "$success_count" "$fail_count"
}
