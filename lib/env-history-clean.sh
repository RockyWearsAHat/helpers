#!/usr/bin/env bash
# lib/env-history-clean.sh — Git history rewriting for git-help-i-pushed-an-env
# Provides: backup_existing_files, restore_files, add_to_gitignore, untrack_sensitive_files,
#           add_file_to_gitignore, remove_files_from_history,
#           clean_with_filter_repo, clean_with_filter_branch, clean_repository

repo_git() {
	local repo_root="$1"
	shift
	git -C "$repo_root" "$@"
}

tracked_files_matching_pattern() {
	local repo_root="$1"
	local pattern="$2"
	local regex
	regex=$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')
	if [[ "$pattern" == */ ]]; then
		repo_git "$repo_root" ls-files 2>/dev/null | grep -E "(^|/)${regex}.*$" 2>/dev/null || true
		return 0
	fi
	repo_git "$repo_root" ls-files 2>/dev/null | grep -E "(^|/)${regex}$" 2>/dev/null || true
}

backup_existing_files() {
	local repo_root="$1"
	local backup_dir="$repo_root/.git-env-backup-$$"
	local patterns
	patterns=$(build_pattern_list)

	mkdir -p "$backup_dir"

	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ] && [ -f "$repo_root/$file" ]; then
				local dir=$(dirname "$file")
				mkdir -p "$backup_dir/$dir"
				cp "$repo_root/$file" "$backup_dir/$file"
			fi
		done < <(tracked_files_matching_pattern "$repo_root" "$pattern")
	done <<< "$patterns"

	echo "$backup_dir"
}

restore_files() {
	local backup_dir="$1"
	local repo_root="$2"

	if [ -d "$backup_dir" ]; then
		find "$backup_dir" -type f | while read -r file; do
			local rel_path="${file#$backup_dir/}"
			local dest="$repo_root/$rel_path"
			local dest_dir=$(dirname "$dest")
			mkdir -p "$dest_dir"
			cp "$file" "$dest"
		done
		rm -rf "$backup_dir"
	fi
}

add_to_gitignore() {
	local repo_root="$1"
	local gitignore="$repo_root/.gitignore"
	local patterns
	patterns=$(build_pattern_list)

	touch "$gitignore"

	local added=0
	while IFS= read -r pattern; do
		if ! grep -qxF "$pattern" "$gitignore" 2>/dev/null; then
			echo "$pattern" >> "$gitignore"
			((added++))
		fi
	done <<< "$patterns"

	if [ $added -gt 0 ]; then
		repo_git "$repo_root" add .gitignore 2>/dev/null || true
		repo_git "$repo_root" commit -m "Add sensitive file patterns to .gitignore" 2>/dev/null || true
	fi
}

untrack_sensitive_files() {
	local repo_root="$1"
	local patterns
	patterns=$(build_pattern_list)

	log_verbose "Untracking sensitive files from git index..."

	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				log_verbose "Untracking: $file"
				repo_git "$repo_root" rm --cached "$file" 2>/dev/null || true
			fi
		done < <(tracked_files_matching_pattern "$repo_root" "$pattern")
	done <<< "$patterns"

	if ! repo_git "$repo_root" diff --cached --quiet 2>/dev/null; then
		repo_git "$repo_root" commit -m "Remove sensitive files from tracking (files preserved locally)" 2>/dev/null || true
		log_success "Sensitive files removed from git tracking"
	fi
}

add_file_to_gitignore() {
	local repo_root="$1"
	local file_path="$2"
	local gitignore="$repo_root/.gitignore"

	if [ ! -f "$gitignore" ]; then
		touch "$gitignore"
		log_info "Created .gitignore"
	fi

	if ! grep -qxF "$file_path" "$gitignore" 2>/dev/null; then
		echo "$file_path" >> "$gitignore"
		log_success "Added '$file_path' to .gitignore"
		return 0
	else
		log_info "'$file_path' already in .gitignore"
		return 1
	fi
}

remove_files_from_history() {
	local repo_root="$1"
	shift
	local files=("$@")

	if [ ${#files[@]} -eq 0 ]; then
		log_info "No files to remove from history."
		return 0
	fi

	log_info "Removing ${#files[@]} file(s) from git history..."

	if [ "$dry_run" = true ]; then
		log_info "[DRY RUN] Would remove files from history:"
		printf '%s\n' "${files[@]}" | sed 's/^/  - /'
		return 0
	fi

	if [ "$create_backup" = true ]; then
		local backup_branch="backup-$(date +%Y%m%d-%H%M%S)"
		repo_git "$repo_root" branch "$backup_branch"
		log_success "Backup branch: $backup_branch"
	fi

	if command -v git-filter-repo &>/dev/null; then
		local path_args=()
		for f in "${files[@]}"; do
			path_args+=(--path "$f" --invert-paths)
		done
		git-filter-repo "${path_args[@]}" --force
	else
		local rm_commands=""
		for f in "${files[@]}"; do
			rm_commands+="git rm --cached --ignore-unmatch '$f' 2>/dev/null || true; "
		done
		repo_git "$repo_root" filter-branch --force --index-filter "$rm_commands" \
			--prune-empty --tag-name-filter cat -- --all

		rm -rf "$repo_root/.git/refs/original/"
		repo_git "$repo_root" reflog expire --expire=now --all
		repo_git "$repo_root" gc --prune=now --aggressive
	fi

	log_success "Files removed from history."
}

clean_with_filter_repo() {
	local repo_root="$1"
	local patterns
	patterns=$(build_pattern_list)

	log_verbose "Using git-filter-repo for history cleaning"

	local path_args=()
	while IFS= read -r pattern; do
		path_args+=(--path-glob "$pattern" --invert-paths)
	done <<< "$patterns"

	if [ "$dry_run" = true ]; then
		log_info "[DRY RUN] Would remove patterns from history"
		return 0
	fi

	local backup_dir
	backup_dir=$(backup_existing_files "$repo_root")

	if [ "$create_backup" = true ]; then
		local backup_branch="backup-$(date +%Y%m%d-%H%M%S)"
		repo_git "$repo_root" branch "$backup_branch"
		log_success "Backup branch: $backup_branch"
	fi

	(
		cd "$repo_root"
		git-filter-repo "${path_args[@]}" --force
	)

	restore_files "$backup_dir" "$repo_root"
	add_to_gitignore "$repo_root"
	untrack_sensitive_files "$repo_root"
}

clean_with_filter_branch() {
	local repo_root="$1"
	local patterns
	patterns=$(build_pattern_list)

	log_verbose "Using git-filter-branch (slower method)"

	local rm_commands=""
	while IFS= read -r pattern; do
		rm_commands+="git rm --cached --ignore-unmatch '$pattern' 2>/dev/null || true; "
	done <<< "$patterns"

	if [ "$dry_run" = true ]; then
		log_info "[DRY RUN] Would remove patterns from history"
		return 0
	fi

	local backup_dir
	backup_dir=$(backup_existing_files "$repo_root")

	if [ "$create_backup" = true ]; then
		local backup_branch="backup-$(date +%Y%m%d-%H%M%S)"
		repo_git "$repo_root" branch "$backup_branch"
		log_success "Backup branch: $backup_branch"
	fi

	repo_git "$repo_root" filter-branch --force --index-filter "$rm_commands" \
		--prune-empty --tag-name-filter cat -- --all

	rm -rf "$repo_root/.git/refs/original/"
	repo_git "$repo_root" reflog expire --expire=now --all
	repo_git "$repo_root" gc --prune=now --aggressive

	restore_files "$backup_dir" "$repo_root"
	add_to_gitignore "$repo_root"
	untrack_sensitive_files "$repo_root"
}

clean_repository() {
	local repo_root
	repo_root=$(get_repo_root)

	log_info "Scanning repository: $repo_root"

	start_spinner "Finding sensitive files..."
	local matching_files
	matching_files=$(find_matching_files "$repo_root")
	stop_spinner "Scan complete"

	if [ -z "$matching_files" ]; then
		log_success "No sensitive files found in repository or history!"
		return 0
	fi

	printf '%bFiles to remove from history:%b\n' "$BOLD" "$NC"

	echo "$matching_files" | while read -r file; do
		if [ -n "$file" ]; then
			printf '  %s\n' "$file"
		fi
	done

	printf '\n'

	if [ "$force" = false ] && [ "$dry_run" = false ]; then
		printf '%b⚠️  WARNING: This will permanently rewrite git history!%b\n' "$YELLOW" "$NC"
		printf '%bAll collaborators will need to re-clone or force-pull.%b\n\n' "$YELLOW" "$NC"
		printf 'Are you sure you want to continue? (yes/no): '
		read -r confirm
		if [ "$confirm" != "yes" ]; then
			log_info "Operation cancelled."
			return 1
		fi
	fi

	start_spinner "Cleaning git history (this may take a while)..."

	if command -v git-filter-repo &>/dev/null; then
		clean_with_filter_repo "$repo_root"
	else
		clean_with_filter_branch "$repo_root"
	fi

	stop_spinner "History cleaning complete"

	if [ "$dry_run" = false ]; then
		log_success "History cleaned. Files preserved in working directory."
		log_success "Patterns added to .gitignore"

		printf '\n%bNext steps:%b\n' "$BOLD" "$NC"
		printf '  git push --force --all\n'
		printf '  git push --force --tags\n'
		printf '\n%bRotate exposed credentials.%b\n\n' "$YELLOW" "$NC"
	fi

	if [ "$run_review" = true ]; then
		log_info "Running post-cleanup security scan..."
		printf '\n'
		if command -v git-scan-for-leaked-envs &>/dev/null; then
			git-scan-for-leaked-envs --verbose
		else
			local script_dir
			script_dir="$(dirname "$(realpath "$0")")"
			"$script_dir/git-scan-for-leaked-envs" --verbose
		fi
	fi

	return 0
}
