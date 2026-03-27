#!/usr/bin/env bash
# lib/upload-diff-analysis.sh — Diff analysis, syntax checks, and risk scoring
# Sourced by git-upload. Do not run directly.

parse_diff_analysis() {
	local __input="$1"
	local __line __key __value
	
	# Initialize all variables to safe defaults
	diff_empty=0
	files_changed=0
	test_files_changed=0
	config_files_changed=0
	core_files_changed=0
	total_additions=0
	total_deletions=0
	code_additions=0
	code_deletions=0
	whitespace_changes=0
	comment_changes=0
	syntax_error_count=0
	api_removals=0
	signature_changes=0
	breaking_hints=""
	
	while IFS= read -r __line; do
		# Skip empty lines
		[ -z "$__line" ] && continue
		
		# Only process lines with = sign
		[[ "$__line" != *"="* ]] && continue
		
		# Split on first =
		__key="${__line%%=*}"
		__value="${__line#*=}"
		
		# Only allow known safe variable names
		case "$__key" in
			diff_empty|files_changed|test_files_changed|config_files_changed|\
core_files_changed|total_additions|total_deletions|code_additions|\
code_deletions|whitespace_changes|comment_changes|syntax_error_count|\
api_removals|signature_changes|breaking_hints)
				# Safely assign the value without executing it
				printf -v "$__key" '%s' "$__value"
				;;
			# Ignore syntax_errors and api_removal_details - they contain
			# arbitrary diff content that we don't need for risk scoring
			*)
				;;
		esac
	done <<< "$__input"
}

run_with_timeout_capture() {
	# Capture command output with a hard timeout, dependency-free.
	# Usage: run_with_timeout_capture <seconds> <cmd> [args...]
	local timeout_secs="$1"
	shift

	local tmp_out
	tmp_out=$(mktemp -t git-upload-cmd.XXXXXX)

	"$@" >"$tmp_out" 2>/dev/null &
	local cmd_pid=$!
	local ticks=0
	local max_ticks=$((timeout_secs * 10))

	while kill -0 "$cmd_pid" 2>/dev/null; do
		sleep 0.1
		ticks=$((ticks + 1))
		if [ "$ticks" -ge "$max_ticks" ]; then
			kill "$cmd_pid" 2>/dev/null || true
			sleep 0.2
			kill -9 "$cmd_pid" 2>/dev/null || true
			# Do not wait here: if the child is stuck in an uninterruptible state,
			# wait can block indefinitely and defeat the timeout safeguard.
			rm -f "$tmp_out" >/dev/null 2>&1 || true
			return 124
		fi
	done

	wait "$cmd_pid" 2>/dev/null || true
	cat "$tmp_out"
	rm -f "$tmp_out" >/dev/null 2>&1 || true
	return 0
}

get_staged_diff_for_analysis() {
	# Safe staged-diff capture for analysis: disable external diff/textconv
	# and enforce a timeout to avoid lockups on large repos or custom drivers.
	# Usage: get_staged_diff_for_analysis [diff flags and pathspec...]
	local diff_output=""
	if diff_output=$(run_with_timeout_capture 15 git --no-pager diff --cached --no-ext-diff --no-textconv "$@"); then
		printf '%s' "$diff_output"
		return 0
	fi

	echo "[git-upload] ⚠️  Staged diff analysis timed out after 15s; continuing with reduced signals." >&2
	printf '%s' ""
	return 0
}

run_function_with_timeout_capture() {
	# Run a shell function with a hard timeout and capture stdout.
	# Usage: run_function_with_timeout_capture <seconds> <function_name>
	local timeout_secs="$1"
	local func_name="$2"

	local tmp_out
	tmp_out=$(mktemp -t git-upload-func.XXXXXX)

	(
		"$func_name"
	) >"$tmp_out" 2>/dev/null &
	local func_pid=$!
	local ticks=0
	local max_ticks=$((timeout_secs * 10))

	while kill -0 "$func_pid" 2>/dev/null; do
		sleep 0.1
		ticks=$((ticks + 1))
		if [ "$ticks" -ge "$max_ticks" ]; then
			kill "$func_pid" 2>/dev/null || true
			sleep 0.2
			kill -9 "$func_pid" 2>/dev/null || true
			rm -f "$tmp_out" >/dev/null 2>&1 || true
			return 124
		fi
	done

	wait "$func_pid" 2>/dev/null || true
	cat "$tmp_out"
	rm -f "$tmp_out" >/dev/null 2>&1 || true
	return 0
}

compute_diff_analysis() {
	# Comprehensive diff analysis that determines ACTUAL risk based on:
	# 1. What type of changes are being made (code vs formatting vs comments)
	# 2. The semantic impact of changes (new vs modified vs deleted functionality)
	# 3. Whether changes introduce potential issues (dead code, syntax errors)
	# 4. Test results and syntax validation
	#
	# Returns a structured analysis that can be used by AI and risk scoring.
	#
	# NOTE: Output is key=value format. Callers MUST use parse_diff_analysis()
	# instead of eval to avoid shell injection from diff content.
	
	local diff
	diff=$(get_staged_diff_for_analysis --unified=0)
	if [ -z "${diff// /}" ]; then
		printf 'diff_empty=1\n'
		return 0
	fi

	# Count actual code changes vs whitespace/formatting
	local total_additions=0
	local total_deletions=0
	local whitespace_only_changes=0
	local comment_only_changes=0
	local code_additions=0
	local code_deletions=0
	local files_changed=0
	local test_files_changed=0
	local config_files_changed=0
	local core_files_changed=0
	
	# Get list of changed files
	local changed_files
	changed_files=$(git diff --cached --name-only 2>/dev/null || echo "")
	files_changed=$(printf '%s\n' "$changed_files" | grep -c . || echo 0)
	
	# Categorize changed files
	while IFS= read -r file; do
		[ -z "$file" ] && continue
		case "$file" in
			*test*|*Test*|*spec*|*Spec*|*_test.*|*.test.*)
				test_files_changed=$((test_files_changed + 1))
				;;
			*.json|*.yaml|*.yml|*.toml|*.ini|*.cfg|*.conf|*config*|*.env*)
				config_files_changed=$((config_files_changed + 1))
				;;
			*)
				core_files_changed=$((core_files_changed + 1))
				;;
		esac
	done <<< "$changed_files"
	
	# Analyze the actual diff content in a single awk pass (fast for large diffs)
	local awk_result
	awk_result=$(printf '%s\n' "$diff" | awk '
		/^\+[^+]/ {
			add++
			content = substr($0, 2)
			# whitespace-only: nothing but spaces/tabs
			gsub(/[ \t]/, "", content)
			if (content == "") { ws++; next }
			# comment line
			if (content ~ /^(\/\/|#|\/\*|\*|<!--)/) { cmt++; next }
			code_add++
			next
		}
		/^-[^-]/ {
			del++
			content = substr($0, 2)
			gsub(/[ \t]/, "", content)
			if (content == "") { ws++; next }
			if (content ~ /^(\/\/|#|\/\*|\*|<!--)/) { cmt++; next }
			code_del++
			next
		}
		END {
			printf "%d %d %d %d %d %d\n", add, del, ws, cmt, code_add, code_del
		}
	')
	read -r total_additions total_deletions whitespace_only_changes comment_only_changes \
		code_additions code_deletions <<< "$awk_result"
	
	# Check for potential dead code patterns in additions
	local dead_code_signals=0
	local dead_code_details=""
	# Unreachable code after return/exit
	if printf '%s\n' "$diff" | grep -qE '^\+.*return[[:space:]]*;' && \
	   printf '%s\n' "$diff" | grep -qE '^\+[^}]*[^/].*[^/]$' 2>/dev/null; then
		# Very rough heuristic - look for code after returns that isn't closing braces
		:
	fi
	# Unused imports (added imports that might not be used)
	local added_imports
	added_imports=$(printf '%s\n' "$diff" | grep -E '^\+.*(import |from .* import |require\(|#include)' | wc -l | tr -d ' ')
	
	# Check for syntax errors in changed files (language-specific)
	# Run checks in parallel for speed, collecting results via temp files
	local syntax_errors=""
	local syntax_error_count=0
	local syntax_tmpdir
	syntax_tmpdir=$(mktemp -d -t git-upload-syntax.XXXXXX)
	local syntax_pids=()
	local tsc_checked=false
	
	while IFS= read -r file; do
		[ -z "$file" ] && continue
		[ ! -f "$file" ] && continue
		
		case "$file" in
			*.py)
				if command -v python3 >/dev/null 2>&1; then
					(
						if ! python3 -m py_compile "$file" >/dev/null 2>&1; then
							printf '%s\n' "- Python syntax error in $file" >> "$syntax_tmpdir/errors"
						fi
					) &
					syntax_pids+=($!)
				fi
				;;
			*.js|*.jsx)
				# Only check plain JavaScript files with node --check
				if command -v node >/dev/null 2>&1; then
					(
						if ! node --check "$file" >/dev/null 2>&1; then
							printf '%s\n' "- JavaScript syntax error in $file" >> "$syntax_tmpdir/errors"
						fi
					) &
					syntax_pids+=($!)
				fi
				;;
			*.ts|*.tsx)
				# TypeScript: run tsc once for entire project (not per-file)
				# Use timeout to prevent blocking on large projects
				if [ "$tsc_checked" = false ] && command -v tsc >/dev/null 2>&1; then
					tsc_checked=true
					local repo_root
					repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
					if [ -f "$repo_root/tsconfig.json" ]; then
						(
							# 10-second timeout for TypeScript checking
							if command -v timeout >/dev/null 2>&1; then
								if ! timeout 10 bash -c "cd '$repo_root' && tsc --noEmit" >/dev/null 2>&1; then
									printf '%s\n' "- TypeScript syntax/compile error (see tsc output)" >> "$syntax_tmpdir/errors"
								fi
							elif command -v gtimeout >/dev/null 2>&1; then
								# macOS with coreutils
								if ! gtimeout 10 bash -c "cd '$repo_root' && tsc --noEmit" >/dev/null 2>&1; then
									printf '%s\n' "- TypeScript syntax/compile error (see tsc output)" >> "$syntax_tmpdir/errors"
								fi
							else
								# No timeout command available - skip tsc to avoid blocking
								:
							fi
						) &
						syntax_pids+=($!)
					fi
				fi
				;;
			*.sh|*.bash|*.zsh)
				if command -v bash >/dev/null 2>&1; then
					(
						if ! bash -n "$file" >/dev/null 2>&1; then
							printf '%s\n' "- Shell syntax error in $file" >> "$syntax_tmpdir/errors"
						fi
					) &
					syntax_pids+=($!)
				fi
				;;
			*.json)
				if command -v python3 >/dev/null 2>&1; then
					(
						if ! python3 -m json.tool -- "$file" >/dev/null 2>&1; then
							printf '%s\n' "- JSON syntax error in $file" >> "$syntax_tmpdir/errors"
						fi
					) &
					syntax_pids+=($!)
				fi
				;;
		esac
	done <<< "$changed_files"
	
	# Wait for all syntax checks to complete
	for pid in "${syntax_pids[@]}"; do
		wait "$pid" 2>/dev/null || true
	done
	
	# Collect syntax errors from temp file
	if [ -f "$syntax_tmpdir/errors" ]; then
		syntax_errors=$(cat "$syntax_tmpdir/errors")
		syntax_error_count=$(wc -l < "$syntax_tmpdir/errors" | tr -d ' ')
	fi
	rm -rf "$syntax_tmpdir" >/dev/null 2>&1 || true
	
	# Check for removed public APIs/exports (actual breaking changes)
	local api_removals=0
	local api_removal_details=""
	# Look for removed function/class/export declarations
	local removed_exports
	removed_exports=$(printf '%s\n' "$diff" | grep -E '^-[[:space:]]*(export |public |def |function |class |module )' | head -n 5 || true)
	if [ -n "${removed_exports// /}" ]; then
		api_removals=$(printf '%s\n' "$removed_exports" | wc -l | tr -d ' ')
		api_removal_details="$removed_exports"
	fi
	
	# Check for changed function signatures (potential breaking)
	local signature_changes=0
	# This is complex - for now, just flag if function definitions changed
	local modified_signatures
	modified_signatures=$(printf '%s\n' "$diff" | grep -E '^[-+].*(def |function |func |fn )[a-zA-Z_][a-zA-Z0-9_]*\(' | wc -l | tr -d ' ')
	if [ "$modified_signatures" -gt 0 ]; then
		signature_changes=$((modified_signatures / 2))  # Roughly - a change has both - and +
	fi
	
	# ── Breaking-change hints (formerly a separate pass) ──
	local breaking_hints_text=""
	local has_breaking=0
	local has_potential=0

	# Removed exports already detected above in api_removals / removed_exports
	if [ -n "${removed_exports// /}" ]; then
		has_breaking=1
		breaking_hints_text="${breaking_hints_text}Likely breaking: Public API/export removed: $(printf '%s' "$removed_exports" | head -c 200). "
	fi

	# Changed function definitions (signature changes)
	local removed_funcs
	local added_funcs
	removed_funcs=$(printf '%s\n' "$diff" | grep -E '^-[[:space:]]*(def|function|func|fn|public|private|protected)[[:space:]]+[a-zA-Z_]' | head -n 10 || true)
	added_funcs=$(printf '%s\n' "$diff" | grep -E '^\+[[:space:]]*(def|function|func|fn|public|private|protected)[[:space:]]+[a-zA-Z_]' | head -n 10 || true)
	if [ -n "${removed_funcs// /}" ] && [ -n "${added_funcs// /}" ]; then
		has_potential=1
		breaking_hints_text="${breaking_hints_text}Potential breaking: Function definitions modified (check for signature changes). "
	fi

	# Removed error handling without replacement
	local removed_validation
	removed_validation=$(printf '%s\n' "$diff" | grep -E '^-.*\b(throw|raise|assert|panic|error\(|Error\(|Exception)\b' | grep -Ev '^-\s*(//|#|/\*|\*)' | head -n 3 || true)
	local added_validation
	added_validation=$(printf '%s\n' "$diff" | grep -E '^\+.*\b(throw|raise|assert|panic|error\(|Error\(|Exception)\b' | grep -Ev '^\+\s*(//|#|/\*|\*)' | head -n 3 || true)
	if [ -n "${removed_validation// /}" ] && [ -z "${added_validation// /}" ]; then
		has_potential=1
		breaking_hints_text="${breaking_hints_text}Potential breaking: Error handling/validation removed. "
	fi

	# Changed default values
	local changed_defaults
	changed_defaults=$(printf '%s\n' "$diff" | grep -E "^[-+].*=[[:space:]]*(true|false|null|nil|None|0|1|\"\"|'')" | wc -l | tr -d ' ')
	if [ "$changed_defaults" -gt 2 ]; then
		has_potential=1
		breaking_hints_text="${breaking_hints_text}Potential: Default values modified. "
	fi

	local breaking_hints_summary
	if [ "$has_breaking" -eq 1 ]; then
		breaking_hints_summary="LIKELY BREAKING changes detected: ${breaking_hints_text}"
	elif [ "$has_potential" -eq 1 ]; then
		breaking_hints_summary="Potential behavior changes detected (review recommended): ${breaking_hints_text}"
	else
		breaking_hints_summary="No obvious breaking-change patterns detected."
	fi

	# Output structured analysis
	printf 'diff_empty=0\n'
	printf 'files_changed=%d\n' "$files_changed"
	printf 'test_files_changed=%d\n' "$test_files_changed"
	printf 'config_files_changed=%d\n' "$config_files_changed"
	printf 'core_files_changed=%d\n' "$core_files_changed"
	printf 'total_additions=%d\n' "$total_additions"
	printf 'total_deletions=%d\n' "$total_deletions"
	printf 'code_additions=%d\n' "$code_additions"
	printf 'code_deletions=%d\n' "$code_deletions"
	printf 'whitespace_changes=%d\n' "$whitespace_only_changes"
	printf 'comment_changes=%d\n' "$comment_only_changes"
	printf 'syntax_error_count=%d\n' "$syntax_error_count"
	printf 'api_removals=%d\n' "$api_removals"
	printf 'signature_changes=%d\n' "$signature_changes"
	printf 'breaking_hints=%s\n' "$breaking_hints_summary"
	if [ -n "$syntax_errors" ]; then
		printf 'syntax_errors=%s\n' "$syntax_errors"
	fi
	if [ -n "$api_removal_details" ]; then
		printf 'api_removal_details=%s\n' "$api_removal_details"
	fi
}

compute_risk_score() {
	# Risk assessment based on CONTEXTUALIZED IMPACT of the changes:
	#
	# LOW RISK: Changes that improve the codebase or have minimal impact
	#   - Fewer test failures than before (fixing things)
	#   - All tests pass
	#   - Changes that don't introduce new problems
	#
	# MEDIUM RISK: Changes with uncertain impact that need review/testing
	#   - Same number of test failures (neutral)
	#   - Unknown baseline comparison
	#   - New functionality that hasn't been tested yet
	#
	# HIGH RISK: Changes that make things worse or introduce vulnerabilities
	#   - More test failures than before (breaking things)
	#   - Tests that were passing now fail
	#   - Syntax errors introduced
	#   - Removal of public APIs (could break consumers)
	#
	# Returns: low, medium, or high with a reason
	
	local testing_status="$1"
	local diff_analysis="$2"
	
	# Parse diff analysis
	local diff_empty=0
	local files_changed=0
	local test_files_changed=0
	local config_files_changed=0
	local core_files_changed=0
	local total_additions=0
	local total_deletions=0
	local code_additions=0
	local code_deletions=0
	local whitespace_changes=0
	local comment_changes=0
	local syntax_error_count=0
	local api_removals=0
	local signature_changes=0
	local breaking_hints=""
	
	# Safely parse diff_analysis without using eval to prevent shell injection
	parse_diff_analysis "$diff_analysis"
	
	local risk_level="medium"  # Default to medium (unknown/uncertain)
	local risk_reasons=""
	
	# =================================================================
	# HIGH RISK: Changes that actively make things WORSE
	# =================================================================
	
	# 1. Tests degraded (more failures, or tests broke that were passing)
	#    NOTE: "fail (new tests)" is NOT degraded - it's progression (new failing tests added)
	if printf '%s\n' "$testing_status" | grep -q '^Testing: fail (degraded)'; then
		risk_level="high"
		risk_reasons="commit degraded test health"
	fi
	
	# 2. Syntax errors introduced
	if [ "$syntax_error_count" -gt 0 ]; then
		risk_level="high"
		if [ -n "$risk_reasons" ]; then
			risk_reasons="$risk_reasons, syntax errors detected"
		else
			risk_reasons="syntax errors detected"
		fi
	fi
	
	# 3. Public API/exports removed (could break downstream consumers)
	if [ "$api_removals" -gt 0 ]; then
		risk_level="high"
		if [ -n "$risk_reasons" ]; then
			risk_reasons="$risk_reasons, API exports removed"
		else
			risk_reasons="API exports removed (potential breaking change)"
		fi
	fi
	
	# =================================================================
	# LOW RISK: Changes that IMPROVE things or have minimal impact
	# =================================================================
	
	# 4. Tests improving (fewer failures than before) - this commit is FIXING things
	if printf '%s\n' "$testing_status" | grep -q '^Testing: fail (improving)'; then
		if [ "$risk_level" != "high" ]; then
			risk_level="low"
			risk_reasons="commit is fixing test failures"
		fi
	fi
	
	# 5. New tests added with failures - this is PROGRESSION, not regression
	if printf '%s\n' "$testing_status" | grep -q '^Testing: fail (new tests)'; then
		if [ "$risk_level" != "high" ]; then
			risk_level="low"
			risk_reasons="new tests added (progression, not regression)"
		fi
	fi
	
	# 6. All tests pass - codebase is healthy
	if printf '%s\n' "$testing_status" | grep -q '^Testing: pass'; then
		if [ "$risk_level" != "high" ]; then
			risk_level="low"
			risk_reasons="all tests pass"
		fi
	fi
	
	# 7. No tests configured but no syntax errors or API removals
	if printf '%s\n' "$testing_status" | grep -q '^Testing: not configured'; then
		if [ "$risk_level" != "high" ]; then
			# Without tests, we rely on other signals - default to medium for untested changes
			if [ "$risk_level" != "low" ]; then
				risk_level="medium"
				risk_reasons="changes not tested (no test suite)"
			fi
		fi
	fi
	
	# =================================================================
	# MEDIUM RISK: Uncertain impact, needs review
	# =================================================================
	
	# 8. Same number of failures (neutral impact)
	if printf '%s\n' "$testing_status" | grep -q '^Testing: fail (unchanged)'; then
		if [ "$risk_level" != "high" ] && [ "$risk_level" != "low" ]; then
			risk_level="medium"
			risk_reasons="test failures unchanged (neutral impact)"
		fi
	fi
	
	# 8. Unknown baseline (couldn't compare)
	if printf '%s\n' "$testing_status" | grep -q '^Testing: fail (unknown baseline)'; then
		if [ "$risk_level" != "high" ] && [ "$risk_level" != "low" ]; then
			risk_level="medium"
			risk_reasons="could not compare against baseline"
		fi
	fi
	
	# 9. Function signatures changed (could affect callers, but not certain)
	if [ "$risk_level" = "low" ] && [ "$signature_changes" -gt 0 ]; then
		risk_level="medium"
		risk_reasons="function signatures modified (review callers)"
	fi
	
	# Default reason if none set
	if [ -z "$risk_reasons" ]; then
		case "$risk_level" in
			low)
				risk_reasons="routine changes"
				;;
			medium)
				risk_reasons="review recommended"
				;;
			high)
				risk_reasons="significant impact detected"
				;;
		esac
	fi
	
	printf '%s|%s' "$risk_level" "$risk_reasons"
}

