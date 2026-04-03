#!/usr/bin/env bash
# lib/upload-ai-message.sh — AI commit message generation and normalization
# Sourced by git-upload. Do not run directly.
# Requires: lib/upload-spinner.sh, lib/upload-diff-analysis.sh, lib/upload-test-output.sh

normalize_ai_commit_message() {
	# Args:
	#  1) commit message (may be multi-line)
	#  2) authoritative testing line (single line)
	#  3) breaking hints (multi-line; optional)
	#  4) pre-computed risk level (low|medium|high)
	#  5) pre-computed risk reason
	local msg="$1"
	local testing_line="$2"
	local breaking_hints="$3"
	local computed_risk="${4:-}"
	local risk_reason="${5:-}"

	local tmp_bullets
	tmp_bullets=$(mktemp -t git-upload-testing-bullets.XXXXXX)
	# Extract bullet lines from the authoritative testing section.
	printf '%s\n' "$testing_line" | grep '^- ' >"$tmp_bullets" 2>/dev/null || true

	local has_likely_breaking=0
	if printf '%s\n' "$breaking_hints" | grep -qi 'LIKELY BREAKING'; then
		has_likely_breaking=1
	fi
	
	# Determine test impact for breaking change logic
	local testing_degraded=0
	if printf '%s\n' "$testing_line" | grep -q '^Testing: fail (degraded)'; then
		testing_degraded=1
	fi

	# 1) Drop any AI-provided Testing lines.
	# 2) Drop duplicated failure bullets outside Testing.
	# 3) Normalize Breaking changes yes/no.
	local normalized
	normalized=$(printf '%s\n' "$msg" \
		| grep -v '^Testing:' \
		| grep -vFf "$tmp_bullets" 2>/dev/null \
		| awk -v has_likely="$has_likely_breaking" -v testing_degraded="$testing_degraded" '
			function ltrim(s) { sub(/^[[:space:]]+/, "", s); return s }
			function rtrim(s) { sub(/[[:space:]]+$/, "", s); return s }
			function trim(s) { return rtrim(ltrim(s)) }
			BEGIN { rewrote=0; in_breaking=0 }
			{
				line=$0

				if (line == "Breaking changes:") {
					in_breaking=1
					print line
					next
				}
				# If we leave the breaking section (blank line or new section header), stop de-hedging.
				if (in_breaking==1 && (line ~ /^[[:space:]]*$/ || line ~ /^[A-Z][A-Za-z ]+:[[:space:]]*$/)) {
					in_breaking=0
				}

				if (match(line, /^Breaking changes:[[:space:]]*/)) {
					rest=line
					sub(/^Breaking changes:[[:space:]]*/, "", rest)
					rest=trim(rest)

					low=tolower(rest)
					# yes/no normalization
					split(rest, parts, /[[:space:]]+/)
					yn=tolower(parts[1])
					if (!rewrote && (yn=="yes" || yn=="no")) {
						rewrote=1
						body=rest
						# drop the first word (yes/no) without relying on non-portable regex flags
						sub(/^[^[:space:]]+[[:space:]]*/, "", body)
						body=trim(body)
						sub(/^[-–—:]+[[:space:]]*/, "", body)
						if (yn=="no") {
							print "Breaking changes: none"
							next
						}
						print "Breaking changes:"
						if (length(body) > 0) print "- " body
						else print "- (details missing; review staged diff)"
						next
					}

					# Only force breaking changes if we have STRONG evidence (API removal, test degradation)
					if (low=="none" && (has_likely==1 || testing_degraded==1)) {
						print "Breaking changes:"
						print "- Likely breaking change detected; review staged diff"
						next
					}
				}
				# De-hedge bullets only when we have strong signals (test degradation)
				if (in_breaking==1 && (has_likely==1 || testing_degraded==1) && line ~ /^- /) {
					gsub(/ may now /, " will now ", line)
					gsub(/ might now /, " will now ", line)
					gsub(/ may no longer /, " will no longer ", line)
					gsub(/ might no longer /, " will no longer ", line)
					gsub(/ may be /, " is ", line)
					gsub(/ might be /, " is ", line)
					gsub(/ may /, " will ", line)
					gsub(/ might /, " will ", line)
				}
				print line
			}
		')

	rm -f "$tmp_bullets" >/dev/null 2>&1 || true

	# Append authoritative Testing section.
	if [ -n "${testing_line// /}" ]; then
		normalized="$normalized

$testing_line"
	fi

	# Determine whether the final message indicates breaking changes (bullet list).
	local breaking_kind
	breaking_kind=$(printf '%s\n' "$normalized" | awk '
		BEGIN{kind="none"}
		/^Breaking changes:[[:space:]]*none[[:space:]]*$/ { kind="none" }
		/^Breaking changes:[[:space:]]*$/ { kind="bullets" }
		END{ print kind }
	')

	# Use pre-computed risk - it's based on contextualized impact analysis
	# Only override if we detect breaking API changes with bullet list
	local final_risk="$computed_risk"
	local final_reason="$risk_reason"
	
	# Override: explicit breaking changes with likely API removal -> high
	if [ "$breaking_kind" = "bullets" ] && [ "$has_likely_breaking" -eq 1 ]; then
		final_risk="high"
		final_reason="breaking API changes detected"
	fi
	
	# Default fallback
	if [ -z "$final_risk" ]; then
		final_risk="medium"
		final_reason="review recommended"
	fi

	# Replace or append risk line with the computed risk
	if printf '%s\n' "$normalized" | grep -q '^Risk:'; then
		normalized=$(printf '%s\n' "$normalized" | awk -v risk="$final_risk" -v reason="$final_reason" '
			BEGIN{done=0}
			/^Risk:/ {
				if (!done) {
					print "Risk: " risk " (" reason ")"
					done=1
					next
				}
			}
			{ print }
		')
	else
		normalized="$normalized

Risk: $final_risk ($final_reason)"
	fi

	# Trim trailing blank lines.
	normalized=$(printf '%s\n' "$normalized" | awk '{ lines[NR]=$0 } $0 !~ /^[[:space:]]*$/ { last=NR } END { for (i=1; i<=last; i++) print lines[i] }')
	printf '%s' "$normalized"
}

# Safe parser for diff_analysis output.
# Parses key=value lines without using eval, preventing shell injection.
# Sets the following variables in the caller's scope:
#   diff_empty, files_changed, test_files_changed, config_files_changed,
#   core_files_changed, total_additions, total_deletions, code_additions,
#   code_deletions, whitespace_changes, comment_changes, syntax_error_count,
#   api_removals, signature_changes
#
# IMPORTANT: This is safe because we only assign to a whitelist of variable
# names and use printf -v to assign values as literal strings.

collect_repo_ai_guidance() {
	local repo_root="$1"
	[ -n "$repo_root" ] || return 0

	local max_total_chars=8000
	local total_chars=0
	local guidance=""
	local entry=""
	local relative_path=""
	local guidance_label=""
	local max_lines=""
	local absolute_path=""
	local snippet=""
	local original_length=0
	local remaining_chars=0
	local block=""
	declare -a guidance_candidates
	guidance_candidates=(
		".github/COMMIT_GUIDELINES.md:Project commit guidelines:120"
		".github/commit_guidelines.md:Project commit guidelines:120"
		".github/COMMIT_MESSAGE.md:Project commit message guide:120"
		".github/commit_message.md:Project commit message guide:120"
		".github/UPLOAD_GUIDELINES.md:Project upload workflow guidelines:120"
		".github/copilot-instructions.md:Repository Copilot instructions:120"
		"AGENTS.md:Repository agent instructions:100"
		"CLAUDE.md:Repository Claude instructions:100"
		"CONTRIBUTING.md:Contribution guidelines:120"
		"README.md:Repository overview:120"
	)

	for entry in "${guidance_candidates[@]}"; do
		IFS=':' read -r relative_path guidance_label max_lines <<< "$entry"
		absolute_path="$repo_root/$relative_path"
		[ -f "$absolute_path" ] || continue

		snippet=$(sed -n "1,${max_lines}p" "$absolute_path" 2>/dev/null || true)
		[ -n "$snippet" ] || continue

		original_length=${#snippet}
		remaining_chars=$((max_total_chars - total_chars))
		[ "$remaining_chars" -gt 0 ] || break

		if [ "$original_length" -gt "$remaining_chars" ]; then
			snippet=$(printf '%s' "$snippet" | head -c "$remaining_chars")
		fi

		block="$guidance_label ($relative_path):
$snippet"
		if [ ${#snippet} -lt "$original_length" ]; then
			block="$block
[truncated to fit prompt budget]"
		fi

		if [ -n "$guidance" ]; then
			guidance="$guidance

---

$block"
		else
			guidance="$block"
		fi

		total_chars=$((total_chars + ${#block} + 6))
		[ "$total_chars" -lt "$max_total_chars" ] || break
	done

	printf '%s' "$guidance"
}

resolve_ai_cmd() {
	# Full command override takes top priority (manual env var).
	if [ -n "${GIT_UPLOAD_AI_CMD-}" ]; then
		printf '%s' "$GIT_UPLOAD_AI_CMD"
		return
	fi

	# Read model preference written by the VS Code sidebar (via syncCheckpointSettings).
	# Falls back to DEFAULT_AI_CMD (no --model flag) when unset, letting the CLI auto-select.
	local model
	model="$(git config --get checkpoint.model 2>/dev/null || true)"
	model="${model//[[:space:]]/}"  # strip whitespace

	if [ -n "$model" ]; then
		printf 'copilot -s --model %s --deny-tool write --deny-tool shell -p "$GIT_UPLOAD_AI_PROMPT"' "$model"
	else
		printf '%s' "$DEFAULT_AI_CMD"
	fi
}

extract_marked_block() {
	local start_marker="$1"
	local end_marker="$2"
	local text="$3"

	printf '%s\n' "$text" | awk -v start="$start_marker" -v end="$end_marker" '
		$0 == start { capture = 1; next }
		$0 == end { capture = 0; exit }
		capture { print }
	'
}

ensure_version_bump_release_notes() {
	local repo_root="$1"
	[ -n "$repo_root" ] || return 0
	[ -f "$repo_root/VERSION" ] || return 0

	if ! git diff --cached --name-only 2>/dev/null | grep -qx 'VERSION'; then
		return 0
	fi

	local new_version
	new_version=$(tr -d '\n' < "$repo_root/VERSION" | xargs 2>/dev/null || echo "")
	[ -n "$new_version" ] || return 0

	local old_version=""
	old_version=$(git show HEAD:VERSION 2>/dev/null | tr -d '\n' | xargs || echo "")
	if [ -n "$old_version" ] && [ "$old_version" = "$new_version" ]; then
		return 0
	fi

	local notes_rel_path="release-notes/v${new_version}.md"
	local notes_path="$repo_root/$notes_rel_path"
	if [ -f "$notes_path" ]; then
		git add "$notes_path" >/dev/null 2>&1 || true
		return 0
	fi

	if [ "$use_ai" != true ]; then
		echo "[git-upload] VERSION changed to $new_version but $notes_rel_path is missing." >&2
		echo "[git-upload] Create that release notes file before running git-upload." >&2
		return 1
	fi

	local ai_cmd
	ai_cmd=$(resolve_ai_cmd)
	local ai_binary
	ai_binary=${ai_cmd%% *}
	if ! command -v "$ai_binary" >/dev/null 2>&1; then
		echo "[git-upload] VERSION changed to $new_version but $notes_rel_path is missing." >&2
		echo "[git-upload] AI command '$ai_binary' is unavailable, so release notes cannot be generated automatically." >&2
		return 1
	fi

	local repo_guidance=""
	repo_guidance=$(collect_repo_ai_guidance "$repo_root")
	local changed_files=""
	changed_files=$(git diff --cached --name-only -- . ':(exclude)release-notes/*' 2>/dev/null | head -40 || echo "")
	local file_list_display=""
	if [ -n "$changed_files" ]; then
		file_list_display=$(printf '%s\n' "$changed_files" | sed 's/^/- /')
	fi

	local stat_summary=""
	stat_summary=$(git diff --cached --stat -- . ':(exclude)release-notes/*' 2>/dev/null | tail -1 || echo "")
	local actual_diff=""
	actual_diff=$(get_staged_diff_for_analysis --unified=1 -- . ':(exclude)release-notes/*')
	local diff_line_count
	diff_line_count=$(printf '%s\n' "$actual_diff" | wc -l | tr -d ' ')
	local diff_truncated=""
	if [ "$diff_line_count" -gt 1200 ]; then
		actual_diff=$(printf '%s\n' "$actual_diff" | head -1200)
		diff_truncated="(truncated - showing first 1200 of $diff_line_count lines)"
	fi

	local recent_commits=""
	if [ -n "$old_version" ] && git rev-parse -q --verify "refs/tags/v$old_version" >/dev/null 2>&1; then
		recent_commits=$(git log --oneline "v$old_version"..HEAD -- . ':(exclude)release-notes/*' 2>/dev/null | head -20 || echo "")
	fi

	local prompt="You are generating release notes for Git Shell Helpers v${new_version}.

OPERATING CONSTRAINTS:
- Output ONLY markdown bullet lines between NOTES_BEGIN and NOTES_END.
- Do NOT include code fences, headings, or explanatory text.
- Write 3 to 6 bullets.
- Keep each bullet concise, usually about 5 to 10 words.
- Focus on shipped changes and workflow improvements.
- Use ONLY the context provided here.
- Do NOT mention the release-notes file itself.
- Do NOT mention that notes were generated by AI.

FORMAT:
NOTES_BEGIN
- Bullet one
- Bullet two
NOTES_END

RELEASE CONTEXT:
- New version: ${new_version}
- Previous version: ${old_version:-unknown}

REPOSITORY GUIDANCE:
${repo_guidance:-No extra repository guidance found.}

CHANGED FILES:
${file_list_display:-- No changed files detected.}

STAGED DIFF SUMMARY:
${stat_summary:-No diff stat summary available.}

RECENT COMMITS SINCE THE PREVIOUS RELEASE TAG:
${recent_commits:-No previous release tag history available locally.}

STAGED DIFF ${diff_truncated}:
\`\`\`diff
${actual_diff}
\`\`\`
"

	echo "[git-upload] VERSION changed to $new_version and $notes_rel_path is missing." >&2
	echo "[git-upload] Generating release notes before commit..." >&2

	local ai_output_file
	ai_output_file=$(mktemp -t git-upload-release-notes.XXXXXX)
	start_spinner "Generating release notes for v${new_version}..."
	local ai_exit_code=0
	if ! GIT_UPLOAD_AI_PROMPT="$prompt" eval "$ai_cmd" > "$ai_output_file" 2>&1; then
		ai_exit_code=$?
	fi
	stop_spinner

	local ai_output=""
	ai_output=$(cat "$ai_output_file" 2>/dev/null || true)
	rm -f "$ai_output_file"

	if [ "$ai_exit_code" -ne 0 ]; then
		echo "[git-upload] Failed to auto-generate $notes_rel_path." >&2
		echo "[git-upload] AI command '$ai_cmd' exited with status $ai_exit_code." >&2
		return 1
	fi

	local highlights=""
	highlights=$(extract_marked_block "NOTES_BEGIN" "NOTES_END" "$ai_output")
	if [ -z "${highlights// /}" ]; then
		highlights=$(printf '%s\n' "$ai_output" | grep '^-' || true)
	fi
	if [ -z "${highlights// /}" ]; then
		echo "[git-upload] Failed to parse generated release notes for $notes_rel_path." >&2
		echo "[git-upload] Expected NOTES_BEGIN/NOTES_END markers or bullet lines from AI output." >&2
		return 1
	fi

	mkdir -p "$repo_root/release-notes"
	cat > "$notes_path" <<EOF
# Git Shell Helpers v${new_version}

## Highlights

${highlights}

## Installer Assets

- \`github-shell-helpers-${new_version}.pkg\` installs the commands and man pages into \`/usr/local\` on macOS.
- \`Git-Shell-Helpers-Installer-${new_version}.sh\` is the versioned standalone installer asset for manual download or mirroring.
EOF

	git add "$notes_path"
	echo "[git-upload] Added generated release notes: $notes_rel_path" >&2
	return 0
}

generate_ai_message() {
	local ai_cmd
	ai_cmd=$(resolve_ai_cmd)

	local ai_binary
	ai_binary=${ai_cmd%% *}
	if ! command -v "$ai_binary" >/dev/null 2>&1; then
		echo "[git-upload] AI command '$ai_binary' not found. " \
			"Install GitHub Copilot CLI (e.g. 'brew install copilot-cli' on macOS) " \
			"and ensure it is on your PATH." >&2
		return 1
	fi

	echo "" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "[git-upload] 🔍 ANALYZING CHANGES" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2

	local testing_status
	testing_status=$(compute_testing_status)
	
	# Combined diff analysis: syntax checks, risk signals, and breaking-change hints in one pass
	start_spinner "Running syntax checks & analyzing diff..."
	local diff_analysis
	if ! diff_analysis=$(run_function_with_timeout_capture 15 compute_diff_analysis); then
		diff_analysis='diff_empty=1'
		echo "[git-upload] ⚠️  Diff analysis timed out after 15s; using conservative defaults." >&2
	fi
	stop_spinner "✅ Diff analysis complete"
	
	# Compute actual risk based on meaningful criteria
	local risk_result
	risk_result=$(compute_risk_score "$testing_status" "$diff_analysis")
	local computed_risk="${risk_result%%|*}"
	local risk_reason="${risk_result#*|}"
	
	# Build diff summary for AI context
	local diff_summary=""
	# Safely parse diff_analysis without using eval to prevent shell injection
	local breaking_hints=""
	parse_diff_analysis "$diff_analysis"
	# breaking_hints is now set by parse_diff_analysis
	if [ -z "$breaking_hints" ]; then
		breaking_hints='Breaking-change hints: No obvious breaking-change patterns detected.'
	else
		breaking_hints="Breaking-change hints: $breaking_hints"
	fi
	
	# Get the actual file names for context
	local changed_file_list
	changed_file_list=$(git diff --cached --name-only 2>/dev/null | head -20 || echo "")
	local file_list_display=""
	if [ -n "$changed_file_list" ]; then
		file_list_display="
Changed files:
$(printf '%s\n' "$changed_file_list" | sed 's/^/  - /')"
	fi
	
	# Get a compact stat summary
	local stat_summary
	stat_summary=$(git diff --cached --stat 2>/dev/null | tail -1 || echo "")
	
	# Get the actual diff - full content for complete context
	local actual_diff
	actual_diff=$(get_staged_diff_for_analysis --unified=3)
	local diff_line_count
	diff_line_count=$(printf '%s\n' "$actual_diff" | wc -l | tr -d ' ')
	local diff_truncated=""
	# For very large diffs (>2000 lines), truncate with notice to keep prompt manageable
	if [ "$diff_line_count" -gt 2000 ]; then
		actual_diff=$(printf '%s\n' "$actual_diff" | head -2000)
		diff_truncated="(truncated - showing first 2000 of $diff_line_count lines; review full diff with 'git diff --cached')"
	fi
	
	# Load repository-authored commit and workflow guidance if it exists
	local repo_root
	repo_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
	local repo_guidance=""
	if [ -n "$repo_root" ]; then
		repo_guidance=$(collect_repo_ai_guidance "$repo_root")
	fi
	
	# Get recent commit history for context (last 5 commits with summary)
	local recent_history=""
	recent_history=$(git log --oneline -10 --no-decorate 2>/dev/null | head -10 || echo "")
	
	# Get more detailed recent commits (last 3) to understand project momentum
	local detailed_recent=""
	detailed_recent=$(git log -3 --pretty=format:"--- Commit %h (%ar) ---
%s

%b
" 2>/dev/null || echo "")
	
	diff_summary="Diff analysis:
- Files changed: $files_changed (core: $core_files_changed, tests: $test_files_changed, config: $config_files_changed)
- Code changes: +$code_additions/-$code_deletions lines
- Formatting/whitespace changes: $whitespace_changes lines
- Comment changes: $comment_changes lines
- Syntax errors detected: $syntax_error_count
- API/export removals: $api_removals
- Function signature changes: $signature_changes
$file_list_display

Git stat: $stat_summary

Pre-computed risk assessment: $computed_risk ($risk_reason)

ACTUAL DIFF ($diff_line_count lines)$diff_truncated:
\`\`\`diff
$actual_diff
\`\`\`"

	# Add repository guidance to context if available
	if [ -n "$repo_guidance" ]; then
		diff_summary="REPOSITORY GUIDANCE (repo-authored commit, workflow, and instruction files):
$repo_guidance

---

$diff_summary"
	fi

	# Add recent history context so AI understands project momentum
	if [ -n "$recent_history" ]; then
		diff_summary="RECENT COMMIT HISTORY (for context - understand what the project has been working on):
$recent_history

DETAILED RECENT COMMITS (last 3 - use this to understand WHY current changes might be happening):
$detailed_recent

---

$diff_summary"
	fi

	local default_prompt="Write a commit message for the staged changes below.

If you have read tools available, use them to understand the codebase
beyond just the diff — read related files, understand the project
structure, and get the full picture. The better you understand what
the project does and where these changes fit, the better the message.
Do not invent changes that are not in the diff.

CONTEXT MATTERS MOST:
Read the recent commit history provided below. Understand what the project
has been doing. Each commit is part of an ongoing thread of work — frame
yours as the next step in that story. If the last few commits were fixing
upload retries, and now there'\''s another tweak to the same area, say so:
'\''Still seeing timeout issues on slow connections — bump retry delay from
2s to 5s and add a log line so we can actually see what'\''s happening'\''

That'\''s what a useful commit message is. It tells the NEXT person what
the situation was and what you did about it.

SUBJECT LINE:
One line, <= 72 chars, that tells someone skimming git log what happened.
Write it the way you'\''d describe the change to a coworker. Say what the
commit DOES or FIXES, not what files it touches.

Good:
  '\''Fix install button showing up even after app is already installed'\''
  '\''Stop blocking payouts until Stripe says so'\''
  '\''Add loading text to clash fetch so users stop retrying'\''
  '\''Local storage fixed so logged-out users can add items to cart'\''
  '\''Image scroller, mobile fixes, queue system, basic filtering'\''
  '\''Remove automatic var() injection — causes issues with dashed idents'\''

Bad:
  '\''Update git-upload default-value regex quoting'\'' — robotic, says nothing useful
  '\''Fix bug in compute_diff_analysis'\'' — which bug? what was broken?
  '\''Refactor implementation details'\'' — meaningless
  '\''Support cancelling MCP tool calls in server'\'' — too jargony
  '\''Auto-detect checkpoint cwd via MCP roots'\'' — insider terminology

BODY:
Describe the situation, what you did, and why. Someone reading git blame
should understand the reasoning without opening the diff.

DO NOT use section headers like '\''What changed:'\'', '\''Why this matters:'\'',
or any rigid template. Those scream '\''AI wrote this'\''. Just write naturally.

For a tiny fix: one sentence or no body at all.

For a small fix: a short paragraph about the problem and the solution.
  '\''The regex for matching empty-string defaults wasn'\''t quoted, so ""
  and '\'''\'' patterns slipped through breaking-change detection silently.'\''

For a medium change: a sentence of context, then bullets.
  '\''Verification threads were getting auto-archived by Discord'\''s
  inactivity timer, which broke the onboarding flow.

  - Touch open threads periodically to reset the archive timer
  - Clean up threads for members who left
  - Add /admin-unlink for force-unlinking stolen tags'\''

For a large change: group under short topic-specific headers.
  '\''Secret scanning:
  - Add git-scan-for-leaked-envs with 40+ secret patterns
  - Wire up pre-commit hook to block pushes with matches

  Installer:
  - Bundle the new command into both installers
  - Add man page'\''

The body should make it obvious WHY this change was made, not just
WHAT code was touched. Name the specific things — the flag, the file,
the function, the value. If there'\''s a number (speed, count, size),
include it. Don'\''t pad a one-line fix with five bullets.

Never anthropomorphize code — scripts don'\''t '\''learn'\'' or '\''understand'\''.
Never restate the subject in different words.
Never use corporate filler like '\''formalize'\'', '\''artifacts'\'', '\''subsystem'\'',
'\''implement'\'', '\''enhance'\'', '\''leverage'\''. Write like a human developer talking
to another developer. If a non-programmer couldn'\''t roughly understand
the subject line, it'\''s too jargony.

METADATA FOOTER — always include at the end, after a blank line:

Breaking changes: none
  (or list specific breakage: '\''--verbose now defaults to true; quiet scripts break'\'')

Risk: <low|medium|high> (<short reason>)
  Use the pre-computed risk unless you have strong evidence otherwise.

Testing: <copy the provided testing status verbatim>

OUTPUT FORMAT — output ONLY the commit message between these markers.
Each marker MUST be on its own line with nothing else on it.
Do NOT put COMMIT_END on the same line as message text.

COMMIT_BEGIN
<commit message>
COMMIT_END
"
	local effective_prompt

	effective_prompt="$default_prompt

$diff_summary

Authoritative testing line to copy verbatim into the commit message:
$testing_status

$breaking_hints"
	if [ -n "$ai_extra_context" ]; then
		effective_prompt="$effective_prompt

Additional context from user: $ai_extra_context"
	fi

	echo "" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "[git-upload] 🤖 AI COMMIT MESSAGE GENERATION" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "[git-upload] Pre-computed risk: $computed_risk ($risk_reason)" >&2
	echo "" >&2

	# Call Copilot CLI with streaming output display
	local ai_output_file
	ai_output_file=$(mktemp -t git-upload-ai-output.XXXXXX)
	local ai_exit_code=0
	
	# Start the AI command in background, capturing output
	(GIT_UPLOAD_AI_PROMPT="$effective_prompt" eval "$ai_cmd" > "$ai_output_file" 2>&1) &
	local ai_pid=$!
	
	# Display streaming "thinking" output with spinner - show all thoughts
	local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
	local frame_idx=0
	local last_line_count=0
	local printed_lines=0
	local elapsed=0
	local elapsed_sec=0
	
	while kill -0 "$ai_pid" 2>/dev/null; do
		# Read latest output and display any new non-marker lines as thoughts
		if [ -f "$ai_output_file" ]; then
			# Note: Combine local declaration with assignment to avoid zsh printing
			# variable names to stdout when inside command substitution context
			local current_line_count=$(wc -l < "$ai_output_file" 2>/dev/null | tr -d ' ' || echo "0")
			
			# Display new lines as they arrive
			if [ "$current_line_count" -gt "$printed_lines" ]; then
				# Clear the spinner line first
				printf '\033[2K\r' >&2
				
				# Get and display all new non-empty, non-marker lines
				# Use head to cap at current_line_count to avoid printing
				# an incomplete trailing line that wc -l hasn't counted yet.
				local new_lines=$(head -n "$current_line_count" "$ai_output_file" 2>/dev/null | \
					tail -n +$((printed_lines + 1)) | \
					grep -v '^$' | \
					grep -v '^[[:space:]]*COMMIT_BEGIN[[:space:]]*$' | \
					grep -v '^[[:space:]]*COMMIT_END[[:space:]]*$' | \
					grep -v '^COMMIT: ' || echo "")
				
				if [ -n "$new_lines" ]; then
					# Print each thought line (no truncation)
					printf '%s\n' "$new_lines" | while IFS= read -r thought_line; do
						[ -n "$thought_line" ] && printf '\033[2m[git-upload] 💭 %s\033[0m\n' "$thought_line" >&2
					done
				fi
				
				printed_lines=$current_line_count
			fi
		fi
		
		# Update spinner on current line
		elapsed=$((elapsed + 1))
		elapsed_sec=$((elapsed / 3))  # roughly seconds (sleep 0.3 * 3 ≈ 1s)
		printf '\033[2K\r[git-upload] %s Generating commit message... (%ds)' "${frames[$frame_idx]}" "$elapsed_sec" >&2
		frame_idx=$(( (frame_idx + 1) % 10 ))
		sleep 0.3
	done
	
	# Get exit code
	wait "$ai_pid" 2>/dev/null
	ai_exit_code=$?
	
	# Clear spinner line
	printf '\033[2K\r' >&2
	
	local ai_output
	ai_output=$(cat "$ai_output_file" 2>/dev/null || echo "")
	rm -f "$ai_output_file" >/dev/null 2>&1 || true
	
	if [ "$ai_exit_code" -ne 0 ] || [ -z "${ai_output// /}" ]; then
		echo "[git-upload] ❌ AI generation failed" >&2
		echo "[git-upload] AI command '$ai_cmd' failed. " \
			"Make sure GitHub Copilot CLI is installed and you have run 'copilot' " \
			"at least once to authenticate." >&2
		return 1
	fi
	
	echo "[git-upload] ✅ Commit message generated" >&2

	# Prefer COMMIT_BEGIN/COMMIT_END block; fall back to legacy single-line COMMIT:.
	local commit_block
	commit_block=$(printf '%s\n' "$ai_output" | awk '
		# Trim leading/trailing whitespace for marker detection
		{ trimmed = $0; gsub(/^[[:space:]]+|[[:space:]]+$/, "", trimmed) }
		trimmed == "COMMIT_BEGIN" { inside=1; next }
		trimmed == "COMMIT_END" { inside=0 }
		inside { print }
	')

	# Strip inline COMMIT_END from message (AI sometimes appends it to last line)
	commit_block=$(printf '%s\n' "$commit_block" | sed 's/[[:space:]]*COMMIT_END[[:space:]]*$//')

	if [ -n "${commit_block// /}" ]; then
		# Trim trailing blank lines.
		commit_block=$(printf '%s\n' "$commit_block" | awk '{ lines[NR]=$0 } $0 !~ /^[[:space:]]*$/ { last=NR } END { for (i=1; i<=last; i++) print lines[i] }')
		# Trim leading blank lines.
		commit_block=$(printf '%s\n' "$commit_block" | awk 'BEGIN{found=0} { if (!found && $0 ~ /^[[:space:]]*$/) next; found=1; print }')
		if [ -n "${commit_block// /}" ]; then
			normalize_ai_commit_message "$commit_block" "$testing_status" "$breaking_hints" "$computed_risk" "$risk_reason"
			return 0
		fi
	fi

	local commit_line
	commit_line=$(printf '%s\n' "$ai_output" | grep -E '^[[:space:]]*COMMIT: ' | tail -n 1 | sed 's/^[[:space:]]*COMMIT: //')

	if [ -z "${commit_line// /}" ]; then
		echo "[git-upload] AI did not produce a COMMIT_BEGIN/COMMIT_END block (or legacy COMMIT: line); skipping AI" >&2
		# Debug: show first/last few lines of AI output to help diagnose parsing issues
		echo "[git-upload] DEBUG: AI output preview (first 5 lines):" >&2
		printf '%s\n' "$ai_output" | head -n 5 | sed 's/^/    /' >&2
		echo "[git-upload] DEBUG: AI output preview (last 5 lines):" >&2
		printf '%s\n' "$ai_output" | tail -n 5 | sed 's/^/    /' >&2
		return 1
	fi

	normalize_ai_commit_message "$commit_line" "$testing_status" "$breaking_hints" "$computed_risk" "$risk_reason"
}

