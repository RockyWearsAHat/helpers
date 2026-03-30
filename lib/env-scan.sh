#!/usr/bin/env bash
# lib/env-scan.sh — File scanning and secret detection for git-help-i-pushed-an-env
# Provides: find_matching_files, scan_workspace_files, scan_with_copilot,
#           display_file_info, handle_file_interactively

# Find files matching patterns in current repo
find_matching_files() {
	local repo_root="$1"
	local patterns
	patterns=$(build_pattern_list)

	local found_files=()

	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				found_files+=("(current) $file")
			fi
		done < <(git ls-files "$repo_root" 2>/dev/null | grep -E "(^|/)$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')$" 2>/dev/null || true)

		while IFS= read -r file; do
			if [ -n "$file" ]; then
				found_files+=("(history) $file")
			fi
		done < <(git log --all --diff-filter=D --summary 2>/dev/null | grep -E "delete mode.*$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')" 2>/dev/null | awk '{print $NF}' || true)

	done <<< "$patterns"

	printf '%s\n' "${found_files[@]}" 2>/dev/null | sort -u
}

# Scan workspace for all sensitive files (current + history)
scan_workspace_files() {
	local repo_root="$1"
	local use_search_patterns="${2:-false}"
	local patterns

	if [ "$use_search_patterns" = true ] && [ ${#search_patterns[@]} -gt 0 ]; then
		patterns=$(printf '%s\n' "${search_patterns[@]}")
	else
		patterns=$(build_pattern_list)
	fi

	local results=()

	# Scan current tracked files
	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				if is_example_file "$file"; then continue; fi
				if [ -f "$repo_root/$file" ] && is_file_empty "$repo_root/$file"; then continue; fi
				local file_status="tracked"
				if [ -f "$repo_root/$file" ]; then
					file_status="tracked+exists"
				fi
				results+=("current|$file|$file_status")
			fi
		done < <(git ls-files "$repo_root" 2>/dev/null | grep -E "(^|/)$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')$" 2>/dev/null || true)
	done <<< "$patterns"

	# Scan git history for deleted files
	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				if is_example_file "$file"; then continue; fi
				local already_found=false
				for r in "${results[@]}"; do
					if [[ "$r" == *"|$file|"* ]]; then
						already_found=true
						break
					fi
				done
				if [ "$already_found" = false ]; then
					local hist_content=""
					hist_content=$(git log --all -p -- "$file" 2>/dev/null | /usr/bin/grep -E '^\+[^+]' | /usr/bin/grep -v '^\+\+\+' | /usr/bin/head -5 || true)
					if [ -n "$hist_content" ]; then
						results+=("history|$file|deleted")
					fi
				fi
			fi
		done < <(git log --all --diff-filter=D --summary 2>/dev/null | grep -E "delete mode.*$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')" 2>/dev/null | awk '{print $NF}' || true)
	done <<< "$patterns"

	# Check for files that were ever committed
	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				if is_example_file "$file"; then continue; fi
				local already_found=false
				for r in "${results[@]}"; do
					if [[ "$r" == *"|$file|"* ]]; then
						already_found=true
						break
					fi
				done
				if [ "$already_found" = false ]; then
					local hist_content=""
					hist_content=$(git log --all -p -- "$file" 2>/dev/null | /usr/bin/grep -E '^\+[^+]' | /usr/bin/grep -v '^\+\+\+' | /usr/bin/head -5 || true)
					if [ -n "$hist_content" ]; then
						results+=("history|$file|was-committed")
					fi
				fi
			fi
		done < <(git log --all --name-only --pretty=format: 2>/dev/null | grep -E "(^|/)$(echo "$pattern" | sed 's/\./\\./g; s/\*/.*/g')$" 2>/dev/null | sort -u || true)
	done <<< "$patterns"

	# Scan untracked files NOT in .gitignore
	while IFS= read -r pattern; do
		while IFS= read -r file; do
			if [ -n "$file" ]; then
				if is_example_file "$file"; then continue; fi
				local already_found=false
				for r in "${results[@]}"; do
					if [[ "$r" == *"|$file|"* ]]; then
						already_found=true
						break
					fi
				done
				if [ "$already_found" = false ]; then
					if ! is_file_gitignored "$repo_root" "$file"; then
						results+=("untracked|$file|NOT in .gitignore!")
					fi
				fi
			fi
		done < <(find "$repo_root" -type f -name "$(echo "$pattern" | sed 's/\*/.*/g')" 2>/dev/null | sed "s|$repo_root/||" | grep -v "^\.git/" || true)
	done <<< "$patterns"

	printf '%s\n' "${results[@]}" 2>/dev/null | sort -u
}

# Use Copilot to scan for actual secrets in file contents
scan_with_copilot() {
	local repo_root="$1"

	if ! command -v copilot &>/dev/null; then
		log_warn "GitHub Copilot CLI not found. Install with: gh extension install github/gh-copilot"
		return 1
	fi

	local file_content=""
	local count=0
	local max_content=80

	# PART 1: Scan CURRENT files in working tree
	local files_to_scan=()
	while IFS= read -r -d '' file; do
		local rel_path="${file#$repo_root/}"
		[[ "$rel_path" == .git/* ]] && continue
		[[ "$rel_path" == node_modules/* ]] && continue
		[[ "$rel_path" == vendor/* ]] && continue
		[[ "$rel_path" == .venv/* ]] && continue
		[[ "$rel_path" == __pycache__/* ]] && continue
		[[ "$rel_path" == *.min.js ]] && continue
		[[ "$rel_path" == *.min.css ]] && continue

		if file "$file" 2>/dev/null | grep -qE "binary|executable|image|archive|compressed"; then
			continue
		fi

		typeset size=""
		size=$(stat -f%z "$file" 2>/dev/null || stat --printf="%s" "$file" 2>/dev/null || echo "0")
		[ "$size" -gt 51200 ] && continue

		files_to_scan+=("$file")
	done < <(find "$repo_root" -type f -print0 2>/dev/null)

	for file in "${files_to_scan[@]}"; do
		[ $count -ge $max_content ] && break
		typeset rel_path="${file#$repo_root/}"
		typeset content=""
		content=$(head -150 "$file" 2>/dev/null || true)
		if [ -n "$content" ]; then
			file_content+="
=== CURRENT: $rel_path ===
$content
"
			((count++))
		fi
	done

	# PART 2: Scan GIT HISTORY
	typeset history_files=""
	history_files=$(cd "$repo_root" && git log --all --pretty=format: --name-only --diff-filter=ACDMR 2>/dev/null | sort -u | head -200)

	while IFS= read -r hist_file; do
		[ -z "$hist_file" ] && continue
		[ $count -ge $max_content ] && break

		[[ "$hist_file" == node_modules/* ]] && continue
		[[ "$hist_file" == vendor/* ]] && continue
		[[ "$hist_file" == .venv/* ]] && continue
		[[ "$hist_file" == *.min.js ]] && continue
		[[ "$hist_file" == *.png ]] && continue
		[[ "$hist_file" == *.jpg ]] && continue
		[[ "$hist_file" == *.gif ]] && continue
		[[ "$hist_file" == *.ico ]] && continue
		[[ "$hist_file" == *.woff* ]] && continue
		[[ "$hist_file" == *.ttf ]] && continue

		typeset first_commit=""
		first_commit=$(cd "$repo_root" && git log --all --diff-filter=A --pretty=format:"%H" -- "$hist_file" 2>/dev/null | tail -1)

		if [ -n "$first_commit" ]; then
			typeset hist_content=""
			hist_content=$(cd "$repo_root" && git show "$first_commit:$hist_file" 2>/dev/null | head -100 || true)

			if [ -n "$hist_content" ] && ! echo "$hist_content" | head -5 | grep -qE "^Binary|^\x00"; then
				file_content+="
=== HISTORY (commit ${first_commit:0:8}): $hist_file ===
$hist_content
"
				((count++))
			fi
		fi
	done <<< "$history_files"

	# PART 3: Specifically check for deleted sensitive-looking files
	typeset deleted_sensitive=""
	deleted_sensitive=$(cd "$repo_root" && git log --all --diff-filter=D --name-only --pretty=format: 2>/dev/null | \
		grep -iE '\.(env|pem|key|p12|pfx|jks|keystore|htpasswd|netrc|npmrc|pypirc)$|credentials|secrets?\.(json|ya?ml)|config\.(json|ya?ml)$|password|token|apikey' | \
		sort -u | head -30)

	while IFS= read -r del_file; do
		[ -z "$del_file" ] && continue
		[ $count -ge $max_content ] && break

		typeset last_commit=""
		last_commit=$(cd "$repo_root" && git log --all --diff-filter=D --pretty=format:"%H" -- "$del_file" 2>/dev/null | head -1)

		if [ -n "$last_commit" ]; then
			typeset del_content=""
			del_content=$(cd "$repo_root" && git show "${last_commit}^:$del_file" 2>/dev/null | head -100 || true)

			if [ -n "$del_content" ] && ! echo "$del_content" | head -5 | grep -qE "^Binary|^\x00"; then
				file_content+="
=== DELETED FILE (was in commit ${last_commit:0:8}): $del_file ===
$del_content
"
				((count++))
			fi
		fi
	done <<< "$deleted_sensitive"

	if [ -z "$file_content" ]; then
		echo ""
		return 0
	fi

	# COMPREHENSIVE AI SCAN PROMPT
	typeset prompt="You are an elite security auditor. Scan ALL the following content for ANY sensitive data that should NEVER be in a git repository.

SCAN FOR ALL OF THESE:
1. API Keys & Tokens: AWS (AKIA...), GCP, Azure, OpenAI (sk-...), GitHub (ghp_/gho_/ghs_), Slack (xox...), Stripe (sk_live_/pk_live_), Twilio, SendGrid, any *_API_KEY, *_TOKEN, *_SECRET
2. Passwords & Credentials: Hardcoded passwords, database connection strings with passwords, Basic Auth headers, Bearer tokens
3. Private Keys: RSA/DSA/EC/PGP private keys (-----BEGIN...PRIVATE KEY-----), SSH keys, certificates with private keys
4. Cloud Credentials: AWS secret keys, GCP service account JSON, Azure connection strings, Firebase configs with real keys
5. Database URLs: postgres://user:pass@, mysql://user:pass@, mongodb://user:pass@, redis://user:pass@
6. OAuth Secrets: client_secret, app_secret, consumer_secret with actual values
7. Encryption Keys: AES keys, JWT secrets, signing keys, salt values that look real
8. Personal Data: Email lists, phone numbers in bulk, SSNs, credit card numbers
9. Internal URLs: Internal API endpoints, admin panels, staging/dev server URLs with credentials
10. Webhook URLs/Secrets: Slack webhooks, Discord webhooks, any URL with embedded tokens

CRITICAL RULES:
- Flag REAL values only (not 'your-key-here', 'xxx', 'TODO', '<placeholder>', 'example')
- Check HISTORY files too - secrets in deleted files are STILL EXPOSED in git history!
- Even .example/.sample files should be flagged if they contain real-looking secrets
- When in doubt, FLAG IT - better safe than sorry

Content to scan:
$file_content

OUTPUT FORMAT - one line per finding:
SECRET|<filepath>|<line_number_or_N/A>|<secret_type>|<first_8_chars>****<last_4_chars>

Examples:
SECRET|.env|5|AWS_SECRET_KEY|wJalrXUt****XhMC
SECRET|config/db.yml|12|DATABASE_PASSWORD|super****word
SECRET|DELETED:old/.env|3|OPENAI_API_KEY|sk-proj-****89Qx
SECRET|HISTORY:src/config.js|45|STRIPE_SECRET|sk_live_****7hNm

If nothing found: NO_SECRETS_FOUND

Output ONLY the formatted lines, no explanations."

	local result
	result=$(copilot -s --model gpt-5.1-codex --deny-tool write --deny-tool shell -p "$prompt" 2>/dev/null || echo "COPILOT_ERROR")

	echo "$result" | grep "^SECRET|" | while IFS='|' read -r _ filepath line_num secret_type masked_val; do
		if [ -n "$filepath" ]; then
			local source="copilot"
			if [[ "$filepath" == DELETED:* ]] || [[ "$filepath" == HISTORY:* ]]; then
				source="history-ai"
				filepath="${filepath#*:}"
			fi
			echo "$source|$filepath|$secret_type: $masked_val (line $line_num)"
		fi
	done
}

# Check if a file is in .gitignore
is_file_gitignored() {
	local repo_root="$1"
	local filepath="$2"
	if git -C "$repo_root" check-ignore -q "$filepath" 2>/dev/null; then
		return 0
	fi
	return 1
}

# Display file info for interactive mode
display_file_info() {
	local file_entry="$1"
	local repo_root="$2"

	local location="${file_entry%%|*}"
	local rest="${file_entry#*|}"
	local filepath="${rest%%|*}"
	local file_status="${rest#*|}"

	printf '\n%b━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━%b\n' "$CYAN" "$NC"
	printf '%bFile:%b %s\n' "$BOLD" "$NC" "$filepath"
	printf '%bLocation:%b %s\n' "$BOLD" "$NC" "$location"
	printf '%bStatus:%b %s\n' "$BOLD" "$NC" "$file_status"

	local gitignore="$repo_root/.gitignore"
	if [ -f "$gitignore" ] && grep -qF "$filepath" "$gitignore" 2>/dev/null; then
		printf '%bIn .gitignore:%b Yes\n' "$BOLD" "$NC"
	else
		printf '%bIn .gitignore:%b %bNo%b\n' "$BOLD" "$NC" "$YELLOW" "$NC"
	fi

	if [ -f "$repo_root/$filepath" ]; then
		printf '\n%bFile preview (first 5 lines):%b\n' "$BOLD" "$NC"
		printf '%b' "$CYAN"
		head -n 5 "$repo_root/$filepath" 2>/dev/null | sed 's/^/  │ /'
		printf '%b\n' "$NC"

		if grep -qE '(password|secret|key|token|api_key|apikey|auth|credential)' "$repo_root/$filepath" 2>/dev/null; then
			printf '%b⚠️  This file may contain secrets!%b\n' "$YELLOW" "$NC"
		fi
	fi

	if [ "$location" = "current" ] || [ "$location" = "history" ]; then
		local commit_info
		commit_info=$(git log -1 --pretty=format:"%h %s (%cr)" -- "$filepath" 2>/dev/null || echo "")
		if [ -n "$commit_info" ]; then
			printf '%bLast commit:%b %s\n' "$BOLD" "$NC" "$commit_info"
		fi
	fi
}

# Interactive file handler
handle_file_interactively() {
	local file_entry="$1"
	local repo_root="$2"

	local location="${file_entry%%|*}"
	local rest="${file_entry#*|}"
	local filepath="${rest%%|*}"
	local file_status="${rest#*|}"

	display_file_info "$file_entry" "$repo_root"

	printf '\n%bWhat would you like to do?%b\n' "$BOLD" "$NC"
	printf '  %b[1]%b Delete file & remove from history (recommended if contains secrets)\n' "$GREEN" "$NC"
	printf '  %b[2]%b Keep file, add to .gitignore, remove from history\n' "$YELLOW" "$NC"
	printf '  %b[3]%b Only remove from history (keep file as-is)\n' "$BLUE" "$NC"
	printf '  %b[4]%b Only add to .gitignore (don'\''t touch history)\n' "$CYAN" "$NC"
	printf '  %b[5]%b Skip this file\n' "$MAGENTA" "$NC"
	printf '  %b[q]%b Quit interactive mode\n' "$RED" "$NC"
	printf '\nChoice [1-5, q]: '

	read -r choice

	case "$choice" in
		1)
			log_info "Removing '$filepath' from history and deleting..."
			files_to_remove+=("$filepath")
			if [ -f "$repo_root/$filepath" ]; then
				rm "$repo_root/$filepath"
				log_success "Deleted: $filepath"
			fi
			add_file_to_gitignore "$repo_root" "$filepath"
			return 0
			;;
		2)
			log_info "Will remove '$filepath' from history and add to .gitignore..."
			files_to_remove+=("$filepath")
			add_file_to_gitignore "$repo_root" "$filepath"
			return 0
			;;
		3)
			log_info "Will remove '$filepath' from history only..."
			files_to_remove+=("$filepath")
			return 0
			;;
		4)
			add_file_to_gitignore "$repo_root" "$filepath"
			if [ "$location" = "current" ]; then
				git rm --cached "$filepath" 2>/dev/null || true
				log_success "Untracked: $filepath"
			fi
			return 0
			;;
		5)
			log_info "Skipping: $filepath"
			return 0
			;;
		q|Q)
			log_info "Quitting interactive mode..."
			return 1
			;;
		*)
			log_warn "Invalid choice. Skipping file."
			return 0
			;;
	esac
}
