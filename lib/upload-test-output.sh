#!/usr/bin/env bash
# lib/upload-test-output.sh — Test output parsing and status computation
# Sourced by git-upload. Do not run directly.
# Requires: lib/upload-spinner.sh, lib/upload-test-detection.sh

summarize_test_output() {
	# Args:
	#  1) test_cmd
	#  2) exit_code
	#  3) path to output file
	local test_cmd="$1"
	local exit_code="$2"
	local output_file="$3"

	local test_status
	if [ "$exit_code" -eq 0 ]; then
		test_status='pass'
	else
		test_status='fail'
	fi

	# If the runner emitted a machine-parseable summary, prefer it.
	# Format:
	#   TEST_SUMMARY: pass 7/7
	#   TEST_SUMMARY: fail 2/7
	#   TEST_FAIL: <name>
	local summary_line
	summary_line=$(grep '^TEST_SUMMARY: ' "$output_file" | tail -n 1 || true)
	if [ -n "${summary_line// /}" ]; then
		local sum_status
		local sum_counts
		sum_status=$(printf '%s\n' "$summary_line" | awk '{print $2}')
		sum_counts=$(printf '%s\n' "$summary_line" | awk '{print $3}')
		if [ -n "${sum_status// /}" ] && [ -n "${sum_counts// /}" ]; then
			local header="Testing: ${sum_status} (${sum_counts})"
			if [ "$sum_status" = "fail" ]; then
				local sum_failures
				sum_failures=$(grep '^TEST_FAIL: ' "$output_file" | sed 's/^TEST_FAIL: //' | head -n 10)
				if [ -n "${sum_failures// /}" ]; then
					printf '%s\n' "$header"
					printf '%s\n' "$sum_failures" | sed 's/^/- /'
					return 0
				fi
			fi
			printf '%s' "$header"
			return 0
		fi
	fi

	local passed=""
	local failed=""
	local total=""
	local count_suffix=""
	local failures=""

	case "$test_cmd" in
		pytest*)
			passed=$(grep -Eo '[0-9]+ passed' "$output_file" | tail -n 1 | awk '{print $1}')
			failed=$(grep -Eo '[0-9]+ failed' "$output_file" | tail -n 1 | awk '{print $1}')
			if [ -n "$passed" ] || [ -n "$failed" ]; then
				local p=${passed:-0}
				local f=${failed:-0}
				total=$((p + f))
				if [ "$total" -gt 0 ]; then
					if [ "$test_status" = "pass" ]; then
						count_suffix=" ($passed/$total)"
					else
						count_suffix=" ($failed/$total)"
					fi
				fi
			fi
			if [ "$test_status" = "fail" ]; then
				failures=$(grep '^FAILED ' "$output_file" | sed 's/^FAILED //' | head -n 10)
			fi
			;;
		"npm test"*|"npm test --silent"*|"yarn test"*|"yarn test --silent"*|"pnpm test"*|"pnpm test --silent"*)
			# Jest and many JS runners print a "Tests:" summary; best-effort parse.
			# Example: "Tests:       2 failed, 3 passed, 5 total"
			local tests_line
			tests_line=$(grep -E '^Tests:' "$output_file" | tail -n 1 || true)
			if [ -n "$tests_line" ]; then
				failed=$(printf '%s\n' "$tests_line" | grep -Eo '[0-9]+ failed' | awk '{print $1}' | tail -n 1)
				passed=$(printf '%s\n' "$tests_line" | grep -Eo '[0-9]+ passed' | awk '{print $1}' | tail -n 1)
				total=$(printf '%s\n' "$tests_line" | grep -Eo '[0-9]+ total' | awk '{print $1}' | tail -n 1)
				if [ -n "$total" ]; then
					if [ "$test_status" = "pass" ] && [ -n "$passed" ]; then
						count_suffix=" ($passed/$total)"
					elif [ "$test_status" = "fail" ] && [ -n "$failed" ]; then
						count_suffix=" ($failed/$total)"
					fi
				fi
			fi
			if [ "$test_status" = "fail" ]; then
				# Jest prints failing suites as: "FAIL  path/to/test"
				failures=$(grep -E '^FAIL\s+' "$output_file" | sed -E 's/^FAIL\s+//' | head -n 10)
			fi
			;;
		dotnet\ test*)
			# Common dotnet summaries:
			#   Total tests: 12. Passed: 11. Failed: 1. Skipped: 0.
			#   Passed!  - Failed: 0, Passed: 12, Skipped: 0, Total: 12, Duration: ...
			local totals
			totals=$(grep -E 'Total tests:[[:space:]]*[0-9]+' "$output_file" | tail -n 1 || true)
			if [ -z "${totals// /}" ]; then
				totals=$(grep -E 'Failed:[[:space:]]*[0-9]+, Passed:[[:space:]]*[0-9]+, Skipped:[[:space:]]*[0-9]+, Total:[[:space:]]*[0-9]+' "$output_file" | tail -n 1 || true)
			fi
			if [ -n "${totals// /}" ]; then
				local t
				local p
				local f
				t=$(printf '%s\n' "$totals" | { grep -Eo 'Total tests:[[:space:]]*[0-9]+' || true; } | grep -Eo '[0-9]+' | tail -n 1 || true)
				if [ -z "${t// /}" ]; then
					t=$(printf '%s\n' "$totals" | { grep -Eo 'Total:[[:space:]]*[0-9]+' || true; } | grep -Eo '[0-9]+' | tail -n 1 || true)
				fi
				p=$(printf '%s\n' "$totals" | { grep -Eo 'Passed:[[:space:]]*[0-9]+' || true; } | grep -Eo '[0-9]+' | tail -n 1 || true)
				f=$(printf '%s\n' "$totals" | { grep -Eo 'Failed:[[:space:]]*[0-9]+' || true; } | grep -Eo '[0-9]+' | tail -n 1 || true)
				if [ -n "${t// /}" ]; then
					if [ "$test_status" = "pass" ] && [ -n "${p// /}" ]; then
						count_suffix=" ($p/$t)"
					elif [ "$test_status" = "fail" ] && [ -n "${f// /}" ]; then
						count_suffix=" ($f/$t)"
					fi
				fi
			fi

			if [ "$test_status" = "fail" ]; then
				# Best-effort: pair "Failed <TestName>" with the first following
				# message line (xUnit commonly prints:
				#   Failed <TestName>
				#   Error Message:
				#     <message>
				# ...). Avoid emitting a blank "Message:" bullet.
				failures=$(awk '
					function trim(s) { sub(/^[[:space:]]+/, "", s); sub(/[[:space:]]+$/, "", s); return s }
					function emit(t, m) {
						if (t == "") return
						if (m == "") { print t; return }
						print t " — " m
					}
					BEGIN { current=""; msg=""; want_msg=0; emitted=0; count=0 }
					/^[[:space:]]*Failed[[:space:]]+/ {
						# Emit previous failure if we never got a message.
						if (current != "" && !emitted) { emit(current, msg); count++; }
						if (count >= 10) exit
						current=$0
						sub(/^[[:space:]]*Failed[[:space:]]+/, "", current)
						current=trim(current)
						msg=""; want_msg=0; emitted=0
						next
					}
					{
						if (current == "") next
						if ($0 ~ /^[[:space:]]*(Error Message:|Message:)[[:space:]]*$/) { want_msg=1; next }
						if (want_msg) {
							if ($0 ~ /^[[:space:]]*$/) next
							msg=$0
							msg=trim(msg)
							emit(current, msg)
							emitted=1
							count++
							if (count >= 10) exit
							current=""; msg=""; want_msg=0
							next
						}
					}
					END {
						if (count < 10 && current != "" && !emitted) emit(current, msg)
					}
				' "$output_file")
			fi
			;;
		*)
			# Unknown runner; we will attempt generic parsing below.
			;;
	esac

	# Generic parsing fallback for wrappers like `make test`.
	# Only compute counts if we don't already have a suffix.
	if [ -z "${count_suffix// /}" ]; then
		# pytest-style summary often appears even when invoked via a wrapper.
		passed=$(grep -Eo '[0-9]+ passed' "$output_file" | tail -n 1 | awk '{print $1}' || true)
		failed=$(grep -Eo '[0-9]+ failed' "$output_file" | tail -n 1 | awk '{print $1}' || true)
		if [ -n "${passed// /}" ] || [ -n "${failed// /}" ]; then
			local p=${passed:-0}
			local f=${failed:-0}
			total=$((p + f))
			if [ "$total" -gt 0 ]; then
				if [ "$test_status" = "pass" ]; then
					count_suffix=" ($p/$total)"
				else
					count_suffix=" ($f/$total)"
				fi
			fi
		fi

		# Python unittest summary:
		#   Ran 48 tests in 0.123s
		#   OK
		# or
		#   FAILED (failures=1, errors=0)
		if [ -z "${count_suffix// /}" ]; then
			total=$(grep -E '^Ran[[:space:]]+[0-9]+[[:space:]]+tests?' "$output_file" | tail -n 1 | grep -Eo '[0-9]+' | tail -n 1 || true)
			if [ -n "${total// /}" ]; then
				if [ "$test_status" = "pass" ]; then
					count_suffix=" ($total/$total)"
				else
					# Best-effort failures count if present.
					local uf
					local ue
					uf=$(grep -Eo 'failures=[0-9]+' "$output_file" | tail -n 1 | grep -Eo '[0-9]+' | tail -n 1 || true)
					ue=$(grep -Eo 'errors=[0-9]+' "$output_file" | tail -n 1 | grep -Eo '[0-9]+' | tail -n 1 || true)
					uf=${uf:-0}
					ue=${ue:-0}
					failed=$((uf + ue))
					count_suffix=" ($failed/$total)"
				fi
			fi
		fi

		# Rust cargo test summary:
		#   test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
		#   test result: FAILED. 47 passed; 1 failed; ...
		if [ -z "${count_suffix// /}" ]; then
			local cargo_line
			cargo_line=$(grep -E 'test result:[[:space:]]*(ok|FAILED)\.' "$output_file" | tail -n 1 || true)
			if [ -n "${cargo_line// /}" ]; then
				local cp
				local cf
				cp=$(printf '%s\n' "$cargo_line" | grep -Eo '[0-9]+ passed' | tail -n 1 | awk '{print $1}' || true)
				cf=$(printf '%s\n' "$cargo_line" | grep -Eo '[0-9]+ failed' | tail -n 1 | awk '{print $1}' || true)
				cp=${cp:-0}
				cf=${cf:-0}
				total=$((cp + cf))
				if [ "$total" -gt 0 ]; then
					if [ "$test_status" = "pass" ]; then
						count_suffix=" ($cp/$total)"
					else
						count_suffix=" ($cf/$total)"
					fi
				fi
			fi
		fi

		# Maven Surefire:
		#   Tests run: 48, Failures: 1, Errors: 0, Skipped: 0
		if [ -z "${count_suffix// /}" ]; then
			local mvn
			mvn=$(grep -E 'Tests run:[[:space:]]*[0-9]+,[[:space:]]*Failures:[[:space:]]*[0-9]+,[[:space:]]*Errors:[[:space:]]*[0-9]+,[[:space:]]*Skipped:[[:space:]]*[0-9]+' "$output_file" | tail -n 1 || true)
			if [ -n "${mvn// /}" ]; then
				local tr
				local tf
				local te
				tr=$(printf '%s\n' "$mvn" | grep -Eo 'Tests run:[[:space:]]*[0-9]+' | grep -Eo '[0-9]+' | tail -n 1 || true)
				tf=$(printf '%s\n' "$mvn" | grep -Eo 'Failures:[[:space:]]*[0-9]+' | grep -Eo '[0-9]+' | tail -n 1 || true)
				te=$(printf '%s\n' "$mvn" | grep -Eo 'Errors:[[:space:]]*[0-9]+' | grep -Eo '[0-9]+' | tail -n 1 || true)
				tr=${tr:-0}
				tf=${tf:-0}
				te=${te:-0}
				failed=$((tf + te))
				if [ "$tr" -gt 0 ]; then
					if [ "$test_status" = "pass" ]; then
						count_suffix=" ($tr/$tr)"
					else
						count_suffix=" ($failed/$tr)"
					fi
				fi
			fi
		fi

		# Gradle:
		#   48 tests completed, 1 failed
		if [ -z "${count_suffix// /}" ]; then
			local gradle
			gradle=$(grep -E '[0-9]+ tests completed, [0-9]+ failed' "$output_file" | tail -n 1 || true)
			if [ -n "${gradle// /}" ]; then
				total=$(printf '%s\n' "$gradle" | awk '{print $1}')
				failed=$(printf '%s\n' "$gradle" | awk '{print $4}')
				if [ -n "${total// /}" ] && [ -n "${failed// /}" ]; then
					if [ "$test_status" = "pass" ]; then
						count_suffix=" ($total/$total)"
					else
						count_suffix=" ($failed/$total)"
					fi
				fi
			fi
		fi
	fi

	local header="Testing: ${test_status}${count_suffix}"
	if [ "$test_status" = "fail" ] && [ -n "${failures// /}" ]; then
		printf '%s\n' "$header"
		printf '%s\n' "$failures" | sed 's/^/- /'
		return 0
	fi

	printf '%s' "$header"
}


extract_test_failure_count() {
	# Best-effort extraction of failure count from test output
	# Returns a number (0 if can't determine)
	local output_file="$1"
	local count=0
	
	# Helper: sanitize a captured number to ensure it's a clean integer
	_sanitize_int() {
		local val="$1"
		# Strip whitespace, newlines, and non-digit characters
		val=$(printf '%s' "$val" | tr -cd '0-9' | head -c 10)
		if [ -z "$val" ]; then
			printf '0'
			return
		fi
		printf '%s' "$val"
	}
	
	# Try various patterns for different test runners
	
	# Jest/Vitest: "Tests: X failed"
	local jest_fail
	jest_fail=$(grep -oE 'Tests:[[:space:]]+[0-9]+[[:space:]]+failed' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$jest_fail" ]; then
		_sanitize_int "$jest_fail"
		return 0
	fi
	
	# dotnet: "Failed!  - Failed:     X" or "Failed:  X" (handles multiple spaces)
	# Must check before generic pytest pattern since both have "X failed"
	local dotnet_fail
	dotnet_fail=$(grep -oE 'Failed:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$dotnet_fail" ]; then
		_sanitize_int "$dotnet_fail"
		return 0
	fi
	
	# dotnet alternate: count individual test failure lines "X [Xms]" after "Failed!"
	local dotnet_fail_count
	dotnet_fail_count=$(grep -cE '^[[:space:]]*[A-Za-z_][A-Za-z0-9_]*[[:space:]]+\[[0-9]+[[:space:]]*m?s\][[:space:]]*—' "$output_file" 2>/dev/null || echo "0")
	if [ "$dotnet_fail_count" -gt 0 ] 2>/dev/null; then
		_sanitize_int "$dotnet_fail_count"
		return 0
	fi
	
	# Pytest: "X failed" or "failed: X"
	local pytest_fail
	pytest_fail=$(grep -oE '[0-9]+[[:space:]]+failed' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$pytest_fail" ]; then
		_sanitize_int "$pytest_fail"
		return 0
	fi
	
	# Go: "FAIL" lines (count them)
	local go_fail
	go_fail=$(grep -c '^--- FAIL:' "$output_file" 2>/dev/null || echo "0")
	if [ "$go_fail" -gt 0 ] 2>/dev/null; then
		_sanitize_int "$go_fail"
		return 0
	fi
	
	# Maven: "Failures: X, Errors: Y"
	local mvn_fail
	mvn_fail=$(grep -oE 'Failures:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | tail -n 1 || true)
	local mvn_err
	mvn_err=$(grep -oE 'Errors:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | tail -n 1 || true)
	if [ -n "$mvn_fail" ] || [ -n "$mvn_err" ]; then
		mvn_fail=$(_sanitize_int "${mvn_fail:-0}")
		mvn_err=$(_sanitize_int "${mvn_err:-0}")
		printf '%s' "$((mvn_fail + mvn_err))"
		return 0
	fi
	
	# Cargo/Rust: "X failed"
	local cargo_fail
	cargo_fail=$(grep -oE '[0-9]+[[:space:]]+failed' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$cargo_fail" ]; then
		_sanitize_int "$cargo_fail"
		return 0
	fi
	
	# Fallback: count lines with common failure indicators
	local generic_fail
	generic_fail=$(grep -ciE '(FAIL|FAILED|ERROR|BROKEN)' "$output_file" 2>/dev/null || echo "0")
	generic_fail=$(_sanitize_int "$generic_fail")
	if [ "$generic_fail" -gt 0 ] 2>/dev/null; then
		printf '%s' "$generic_fail"
		return 0
	fi
	
	# Can't determine - return 1 as a fallback (at least 1 failure since tests failed)
	printf '1'
}

extract_test_total_count() {
	# Best-effort extraction of total test count from test output
	# Returns a number (0 if can't determine)
	local output_file="$1"
	
	# Helper: sanitize a captured number to ensure it's a clean integer
	_sanitize_int() {
		local val="$1"
		val=$(printf '%s' "$val" | tr -cd '0-9' | head -c 10)
		if [ -z "$val" ]; then
			printf '0'
			return
		fi
		printf '%s' "$val"
	}
	
	# Jest/Vitest: "Tests: X total"
	local jest_total
	jest_total=$(grep -oE 'Tests:[[:space:]]+.*[0-9]+[[:space:]]+total' "$output_file" 2>/dev/null | grep -oE '[0-9]+[[:space:]]+total' | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$jest_total" ]; then
		_sanitize_int "$jest_total"
		return 0
	fi
	
	# dotnet: "Total: X" or "Total tests: X"
	local dotnet_total
	dotnet_total=$(grep -oE 'Total:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | tail -n 1 || true)
	if [ -z "$dotnet_total" ]; then
		dotnet_total=$(grep -oE 'Total tests:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	fi
	if [ -n "$dotnet_total" ]; then
		_sanitize_int "$dotnet_total"
		return 0
	fi
	
	# Pytest: "X passed" + "X failed" = total, or "collected X items"
	local pytest_collected
	pytest_collected=$(grep -oE 'collected[[:space:]]+[0-9]+[[:space:]]+items?' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$pytest_collected" ]; then
		_sanitize_int "$pytest_collected"
		return 0
	fi
	
	# Python unittest: "Ran X tests"
	local unittest_total
	unittest_total=$(grep -oE 'Ran[[:space:]]+[0-9]+[[:space:]]+tests?' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | head -n 1 || true)
	if [ -n "$unittest_total" ]; then
		_sanitize_int "$unittest_total"
		return 0
	fi
	
	# Go: count all test function results (PASS + FAIL)
	local go_total
	go_total=$(grep -cE '^--- (PASS|FAIL):' "$output_file" 2>/dev/null || echo "0")
	if [ "$go_total" -gt 0 ] 2>/dev/null; then
		_sanitize_int "$go_total"
		return 0
	fi
	
	# Maven: "Tests run: X"
	local mvn_total
	mvn_total=$(grep -oE 'Tests run:[[:space:]]*[0-9]+' "$output_file" 2>/dev/null | grep -oE '[0-9]+' | tail -n 1 || true)
	if [ -n "$mvn_total" ]; then
		_sanitize_int "$mvn_total"
		return 0
	fi
	
	# Fallback: can't determine
	printf '0'
}


compute_testing_status() {
	# Returns testing status with comparison against previous commit.
	# Format: "Testing: <status> [(<details>)]"
	# Where status can be:
	#   - "pass" - all tests pass
	#   - "fail (new)" - tests that were passing before now fail (THIS COMMIT BROKE THEM)
	#   - "fail (pre-existing)" - tests failing but they were already failing before this commit
	#   - "not configured" - no test runner detected
	
	local repo_root
	repo_root=$(git rev-parse --show-toplevel 2>/dev/null || echo "")
	if [ -z "${repo_root// /}" ]; then
		printf '%s' 'Testing: not configured'
		return 0
	fi

	local test_spec
	if ! test_spec=$(detect_test_cmd "$repo_root"); then
		printf '%s' 'Testing: not configured'
		return 0
	fi
	local test_cmd
	local test_cwd
	IFS=$'\n' read -r test_cmd test_cwd <<EOF
$test_spec
EOF
	if [ -z "${test_cmd// /}" ]; then
		printf '%s' 'Testing: not configured'
		return 0
	fi
	if [ -z "${test_cwd// /}" ]; then
		test_cwd="$repo_root"
	fi

	# Step 1: Run tests on current state (with staged changes)
	echo "" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "[git-upload] 📋 TEST VERIFICATION" >&2
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "[git-upload] Step 1/2: Testing your changes..." >&2
	start_spinner "Running tests on current changes..."

	local tmp_current
	tmp_current=$(mktemp -t git-upload-tests-current.XXXXXX)
	local current_exit_code=0

	if (cd "$test_cwd" && eval "$test_cmd") >"$tmp_current" 2>&1; then
		current_exit_code=0
	else
		current_exit_code=$?
	fi
	
	stop_spinner

	# Check for "no tests" conditions first
	if [ "$current_exit_code" -ne 0 ]; then
		# Pytest: exit 5, plus common text (works even if invoked via wrappers).
		if grep -qiE 'collected[[:space:]]+0[[:space:]]+items|no[[:space:]]+tests[[:space:]]+ran' "$tmp_current" 2>/dev/null; then
			rm -f "$tmp_current" >/dev/null 2>&1 || true
			echo "[git-upload] Testing: not configured" >&2
			printf '%s' 'Testing: not configured'
			return 0
		fi

		# JS runners (Jest/Vitest/Mocha wrappers): "No tests found" variants.
		if grep -qiE 'no[[:space:]]+tests[[:space:]]+found|no[[:space:]]+test[[:space:]]+files[[:space:]]+found|no[[:space:]]+tests[[:space:]]+to[[:space:]]+run' "$tmp_current" 2>/dev/null; then
			rm -f "$tmp_current" >/dev/null 2>&1 || true
			echo "[git-upload] Testing: not configured" >&2
			printf '%s' 'Testing: not configured'
			return 0
		fi

		# Maven/Surefire can be configured to fail when no tests match.
		if grep -qiE 'No tests were executed|There are no tests to run|No tests to run' "$tmp_current" 2>/dev/null; then
			rm -f "$tmp_current" >/dev/null 2>&1 || true
			echo "[git-upload] Testing: not configured" >&2
			printf '%s' 'Testing: not configured'
			return 0
		fi

		# dotnet test can fail when it discovers zero tests.
		if grep -qiE 'No test is available|No tests are available|No test files were found|No test matches the given testcase filter' "$tmp_current" 2>/dev/null; then
			rm -f "$tmp_current" >/dev/null 2>&1 || true
			echo "[git-upload] Testing: not configured" >&2
			printf '%s' 'Testing: not configured'
			return 0
		fi
	fi

	# Treat "no tests collected" as not configured (common for pytest).
	if printf '%s' "$test_cmd" | grep -q '^pytest' && [ "$current_exit_code" -eq 5 ]; then
		rm -f "$tmp_current" >/dev/null 2>&1 || true
		echo "[git-upload] Testing: not configured" >&2
		printf '%s' 'Testing: not configured'
		return 0
	fi

	# Treat JS package managers' default placeholder tests as not configured.
	if printf '%s' "$test_cmd" | grep -Eq '^(npm|yarn|pnpm) test' && grep -qi 'no test specified' "$tmp_current" 2>/dev/null; then
		rm -f "$tmp_current" >/dev/null 2>&1 || true
		echo "[git-upload] Testing: not configured" >&2
		printf '%s' 'Testing: not configured'
		return 0
	fi

	# If current tests pass, we're done - no need to check baseline
	if [ "$current_exit_code" -eq 0 ]; then
		local summary
		if summary=$(summarize_test_output "$test_cmd" "$current_exit_code" "$tmp_current" 2>/dev/null); then
			:
		else
			summary='Testing: pass'
		fi
		rm -f "$tmp_current" >/dev/null 2>&1 || true
		echo "[git-upload] ✅ $summary" >&2
		echo "[git-upload] Step 2/2: Skipped (tests pass, no baseline comparison needed)" >&2
		echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
		echo "" >&2
		printf '%s' "$summary"
		return 0
	fi

	# Tests are failing - now we need to compare against baseline
	# to determine if this commit made things BETTER, WORSE, or SAME
	
	# Extract failure count from current test output
	local current_fail_count
	current_fail_count=$(extract_test_failure_count "$tmp_current")
	echo "[git-upload] ⚠️  Current tests: $current_fail_count failure(s) detected" >&2
	
	# Check if we have a previous commit to compare against
	local has_head=0
	if git rev-parse HEAD >/dev/null 2>&1; then
		has_head=1
	fi
	
	local test_delta="unknown"  # worse, same, better, or unknown
	local baseline_fail_count=0
	
	if [ "$has_head" -eq 1 ]; then
		echo "[git-upload] Step 2/2: Comparing against previous commit (HEAD)..." >&2
		
		# Stash current changes (including staged), run tests on HEAD, then restore
		local stash_result
		stash_result=$(git stash push -u -m "git-upload-baseline-test" 2>&1 || echo "stash_failed")
		
		if ! printf '%s' "$stash_result" | grep -q "stash_failed"; then
			local tmp_baseline
			tmp_baseline=$(mktemp -t git-upload-tests-baseline.XXXXXX)
			
			start_spinner "Running tests on HEAD (baseline)..."
			
			local baseline_exit_code=0
			if (cd "$test_cwd" && eval "$test_cmd") >"$tmp_baseline" 2>&1; then
				baseline_exit_code=0
			else
				baseline_exit_code=$?
			fi
			
			stop_spinner
			
			# Restore the stashed changes
			echo "[git-upload] Restoring your changes..." >&2
			if ! git stash pop >/dev/null 2>&1; then
				echo "[git-upload] ⚠️  WARNING: Failed to re-apply stashed changes after baseline tests." >&2
				echo "[git-upload]    Please inspect your working tree (e.g., 'git status') and resolve manually." >&2
			fi
			# Re-stage files after stash pop (stash pop leaves files unstaged)
			git add -A 2>/dev/null || true
			
			# Extract baseline failure count and total test count
			local baseline_total_count=0
			if [ "$baseline_exit_code" -eq 0 ]; then
				baseline_fail_count=0
				baseline_total_count=$(extract_test_total_count "$tmp_baseline")
			else
				baseline_fail_count=$(extract_test_failure_count "$tmp_baseline")
				baseline_total_count=$(extract_test_total_count "$tmp_baseline")
			fi
			
			# Also get current total for comparison
			local current_total_count
			current_total_count=$(extract_test_total_count "$tmp_current")
			
			# Compare: did this commit make things better, worse, or same?
			# Key insight: if total tests increased and that accounts for the new failures,
			# these are likely NEW TESTS (progression), not broken old tests (regression)
			local new_test_count=0
			if [ "$current_total_count" -gt "$baseline_total_count" ] 2>/dev/null; then
				new_test_count=$((current_total_count - baseline_total_count))
			fi
			
			if [ "$baseline_fail_count" -eq 0 ] && [ "$current_fail_count" -gt 0 ]; then
				# Tests WERE passing, now failing
				if [ "$new_test_count" -ge "$current_fail_count" ] 2>/dev/null && [ "$new_test_count" -gt 0 ]; then
					# All failures are from new tests = PROGRESSION (new tests added)
					test_delta="newTests"
					echo "[git-upload] 🟣 Baseline: 0 failures ($baseline_total_count tests) → Current: $current_fail_count failures ($current_total_count tests)" >&2
					echo "[git-upload]    Result: NEW TESTS added (+$new_test_count), failures are from new tests (progression, not regression)" >&2
				else
					test_delta="worse"
					echo "[git-upload] 🔴 Baseline: 0 failures → Current: $current_fail_count failures" >&2
					echo "[git-upload]    Result: This commit BROKE previously passing tests" >&2
				fi
			elif [ "$current_fail_count" -gt "$baseline_fail_count" ]; then
				# More failures now
				local additional_failures=$((current_fail_count - baseline_fail_count))
				if [ "$new_test_count" -ge "$additional_failures" ] 2>/dev/null && [ "$new_test_count" -gt 0 ]; then
					# Additional failures explained by new tests = PROGRESSION
					test_delta="newTests"
					echo "[git-upload] 🟣 Baseline: $baseline_fail_count failures ($baseline_total_count tests) → Current: $current_fail_count failures ($current_total_count tests)" >&2
					echo "[git-upload]    Result: NEW TESTS added (+$new_test_count), additional failures are from new tests" >&2
				else
					test_delta="worse"
					echo "[git-upload] 🔴 Baseline: $baseline_fail_count failures → Current: $current_fail_count failures" >&2
					echo "[git-upload]    Result: This commit made things WORSE" >&2
				fi
			elif [ "$current_fail_count" -lt "$baseline_fail_count" ]; then
				# Fewer failures now = BETTER (commit is fixing things!)
				test_delta="better"
				echo "[git-upload] 🟢 Baseline: $baseline_fail_count failures → Current: $current_fail_count failures" >&2
				echo "[git-upload]    Result: This commit is IMPROVING test health" >&2
			else
				# Same number of failures
				test_delta="same"
				echo "[git-upload] 🟡 Baseline: $baseline_fail_count failures → Current: $current_fail_count failures" >&2
				echo "[git-upload]    Result: No change in test health" >&2
			fi
			
			rm -f "$tmp_baseline" >/dev/null 2>&1 || true
		else
			# Couldn't stash - assume neutral to avoid false positives
			test_delta="unknown"
			echo "[git-upload] ⚠️  Could not stash changes - baseline comparison skipped" >&2
		fi
	else
		# No previous commit - this is initial commit with failing tests
		test_delta="worse"
		echo "[git-upload] 📝 Initial commit - no baseline to compare against" >&2
		echo "[git-upload] 🔴 $current_fail_count failing test(s) in initial commit" >&2
	fi
	
	echo "[git-upload] ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" >&2
	echo "" >&2

	# Build the summary with delta information
	local summary
	if summary=$(summarize_test_output "$test_cmd" "$current_exit_code" "$tmp_current" 2>/dev/null); then
		:
	else
		summary='Testing: fail'
	fi
	rm -f "$tmp_current" >/dev/null 2>&1 || true

	# Modify the summary to indicate the impact of this commit on test health
	case "$test_delta" in
		worse)
			summary=$(printf '%s' "$summary" | sed 's/^Testing: fail/Testing: fail (degraded)/')
			;;
		better)
			summary=$(printf '%s' "$summary" | sed 's/^Testing: fail/Testing: fail (improving)/')
			;;
		same)
			summary=$(printf '%s' "$summary" | sed 's/^Testing: fail/Testing: fail (unchanged)/')
			;;
		newTests)
			# New tests were added - failures are from progression, not regression
			summary=$(printf '%s' "$summary" | sed 's/^Testing: fail/Testing: fail (new tests)/')
			;;
		*)
			summary=$(printf '%s' "$summary" | sed 's/^Testing: fail/Testing: fail (unknown baseline)/')
			;;
	esac

	if [ -z "${summary// /}" ]; then
		summary='Testing: not configured'
	fi

	echo "[git-upload] $summary" >&2
	printf '%s' "$summary"
}

