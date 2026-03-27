#!/usr/bin/env bash
# lib/upload-test-detection.sh — Test runner detection for git-upload
# Sourced by git-upload. Do not run directly.

detect_vscode_test_task() {
	# Best-effort parser for .vscode/tasks.json (JSON/JSONC) to find a "test" task.
	# Prints two lines on success:
	#  1) command line (shell-quoted where needed)
	#  2) cwd (may be empty)
	local repo_root="$1"
	local tasks_file="$repo_root/.vscode/tasks.json"
	[ -f "$tasks_file" ] || return 1

	# Strip JSONC-style comments (best effort): remove /* ... */ blocks (line-based)
	# and // full-line comments. This stays dependency-free.
	local stripped
	stripped=$(sed -e '/\/\*/,/\*\//d' -e '/^[[:space:]]*\/\//d' "$tasks_file")

	printf '%s\n' "$stripped" | awk -v repo_root="$repo_root" '
		function countch(s, c,    i, n) { n=0; for (i=1; i<=length(s); i++) if (substr(s,i,1)==c) n++; return n }
		function ltrim(s) { sub(/^[[:space:]]+/, "", s); return s }
		function rtrim(s) { sub(/[[:space:]]+$/, "", s); return s }
		function trim(s) { return rtrim(ltrim(s)) }
		function subst_vars(s) {
			gsub(/\$\{workspaceFolder\}/, repo_root, s)
			gsub(/\$\{workspaceRoot\}/, repo_root, s)
			return s
		}
		function extract_string(line, key,    t) {
			t=line
			if (index(t, "\"" key "\"") == 0) return ""
			sub(".*\"" key "\"[[:space:]]*:[[:space:]]*\"", "", t)
			sub("\".*", "", t)
			return t
		}
		BEGIN {
			in_tasks=0; found_tasks=0;
			in_obj=0; depth=0;
			in_args=0;
			label=""; group=""; command=""; cwd=""; args="";
			best_cmd=""; best_cwd=""; best_rank=999;
		}
		{
			line=$0
			if (!in_tasks) {
				if (line ~ /"tasks"[[:space:]]*:/) found_tasks=1
				if (found_tasks && line ~ /\[/) in_tasks=1
			}
			if (!in_tasks) next

			if (!in_obj && line ~ /\{/) {
				in_obj=1; depth=0; in_args=0
				label=""; group=""; command=""; cwd=""; args=""
			}

			if (in_obj) {
				if (label=="") { tmp=extract_string(line, "label"); if (tmp!="") label=tmp }
				if (command=="") { tmp=extract_string(line, "command"); if (tmp!="") command=tmp }
				if (group=="") { tmp=extract_string(line, "group"); if (tmp!="") group=tmp }
				if (group=="" && line ~ /"kind"[[:space:]]*:[[:space:]]*"test"/) group="test"
				if (cwd=="") { tmp=extract_string(line, "cwd"); if (tmp!="") cwd=tmp }

				# args: capture quoted strings inside args array (best-effort)
				if (!in_args && line ~ /"args"[[:space:]]*:[[:space:]]*\[/) {
					in_args=1
					# Drop everything up to the opening '[' to avoid capturing the key name.
					sub(/.*\[/, "", line)
				}
				if (in_args) {
					work=line
					while (index(work, "\"") > 0) {
						i=index(work, "\"")
						work=substr(work, i+1)
						j=index(work, "\"")
						if (j==0) break
						arg=substr(work, 1, j-1)
						arg=subst_vars(arg)
						q=arg
						gsub(/\\/, "\\\\", q)
						gsub(/\"/, "\\\"", q)
						args = args " " "\"" q "\""
						work=substr(work, j+1)
					}
					if (line ~ /\]/) in_args=0
				}

				depth += countch($0, "{") - countch($0, "}")
				if (depth <= 0 && $0 ~ /}/) {
					lbl=tolower(trim(label))
					grp=tolower(trim(group))
					cmd=subst_vars(command)
					cw=subst_vars(cwd)
					rank=999
					if (lbl == "test") rank=0
					else if (grp == "test") rank=1
					if (cmd != "" && rank < best_rank) {
						best_rank=rank
						best_cmd=trim(cmd) args
						best_cwd=trim(cw)
					}
					in_obj=0
				}
			}
		}
		END {
			if (best_cmd != "") {
				print trim(best_cmd)
				print trim(best_cwd)
				exit 0
			}
			exit 1
		}
	'
}

detect_test_cmd() {
	local repo_root="$1"
	if [ -z "${repo_root// /}" ]; then
		repo_root="."
	fi

	repo_has_python_tests() {
		local root="$1"
		find "$root" \
			-type f \
			\( -name 'test_*.py' -o -name '*_test.py' \) \
			-not -path '*/.venv/*' \
			-not -path '*/venv/*' \
			-not -path '*/.tox/*' \
			-not -path '*/.pytest_cache/*' \
			-not -path '*/.mypy_cache/*' \
			-not -path '*/.git/*' \
			-maxdepth 6 \
			-print 2>/dev/null \
			| head -n 1 \
			| grep -q '.'
	}

	repo_has_go_tests() {
		local root="$1"
		find "$root" \
			-type f \
			-name '*_test.go' \
			-not -path '*/.git/*' \
			-maxdepth 6 \
			-print 2>/dev/null \
			| head -n 1 \
			| grep -q '.'
	}

	package_json_test_script() {
		# Best-effort: detect if package.json defines a non-placeholder scripts.test.
		# Returns 0 if present, 1 if missing/placeholder.
		local pkg="$1"
		[ -f "$pkg" ] || return 1

		local script
		script=$(awk '
			function countch(s, c,    i, n) { n=0; for (i=1; i<=length(s); i++) if (substr(s,i,1)==c) n++; return n }
			function ltrim(s) { sub(/^[[:space:]]+/, "", s); return s }
			function rtrim(s) { sub(/[[:space:]]+$/, "", s); return s }
			function trim(s) { return rtrim(ltrim(s)) }
			function extract_test_value(line,    t) {
				t=line
				if (index(t, "\"test\"") == 0) return ""
				sub(".*\"test\"[[:space:]]*:[[:space:]]*\"", "", t)
				sub("\".*", "", t)
				return t
			}
			BEGIN { in_scripts=0; depth=0; found=0; val="" }
			{
				line=$0
				if (!in_scripts) {
					if (line ~ /\"scripts\"[[:space:]]*:/) {
						in_scripts=1
						# allow same-line object start
						depth += countch(line, "{") - countch(line, "}")
						# do not next; scripts/test may be on the same line in minified JSON
					}
					if (!in_scripts) next
				}

				depth += countch(line, "{") - countch(line, "}")
				tmp=extract_test_value(line)
				if (tmp != "") { found=1; val=tmp }
				if (depth <= 0) { in_scripts=0 }
			}
			END {
				if (found) print trim(val)
			}
		' "$pkg" 2>/dev/null || true)

		if [ -z "${script// /}" ]; then
			return 1
		fi
		if printf '%s' "$script" | grep -qi 'no test specified'; then
			return 1
		fi
		return 0
	}

	repo_has_java_tests() {
		# Conservative: require a src/test tree and at least one test-like file.
		local root="$1"
		find "$root" \
			-type f \
			\(
				-path '*/src/test/*' \
				-o -path '*/src/androidTest/*'
			\) \
			\(
				-name '*Test.java' -o -name '*Tests.java' -o -name '*IT.java' \
				-o -name '*Test.kt' -o -name '*Tests.kt' -o -name '*IT.kt' \
				-o -name '*Test.groovy' -o -name '*Tests.groovy' -o -name '*IT.groovy'
			\) \
			-not -path '*/build/*' \
			-not -path '*/target/*' \
			-not -path '*/.git/*' \
			-maxdepth 8 \
			-print 2>/dev/null \
			| head -n 1 \
			| grep -q '.'
	}

	repo_has_rust_tests() {
		# Rust can have inline #[test] modules without a tests/ folder.
		# Use a shallow heuristic to avoid running cargo test in repos with no code.
		local root="$1"
		if [ -d "$root/tests" ]; then
			find "$root/tests" -type f -maxdepth 3 -print 2>/dev/null | head -n 1 | grep -q '.' && return 0
		fi
		find "$root" \
			-type f \
			\( -path '*/src/*.rs' -o -path '*/src/*/*.rs' \) \
			-not -path '*/target/*' \
			-not -path '*/.git/*' \
			-maxdepth 6 \
			-print 2>/dev/null \
			| while read -r f; do
				grep -qE '#\[[[:space:]]*test\]' "$f" 2>/dev/null && { printf '%s\n' "$f"; break; }
			done \
			| head -n 1 \
			| grep -q '.'
	}

	is_dotnet_test_project() {
		local csproj="$1"
		[ -f "$csproj" ] || return 1
		grep -qE 'Microsoft\.NET\.Test\.Sdk|<IsTestProject>[[:space:]]*true[[:space:]]*</IsTestProject>' "$csproj" 2>/dev/null
	}

	# Prints two lines on success:
	#  1) command line
	#  2) cwd (may be empty)
	#
	# Priority:
	#  1) Explicit env var (caller-controlled)
	#  2) Per-repo git config
	#  3) VS Code workspace test task (.vscode/tasks.json group: test)
	#  4) Repo-local test runner / Makefile
	#  5) Simple heuristics for common stacks
	if [ -n "${GIT_UPLOAD_TEST_CMD-}" ]; then
		printf '%s\n' "$GIT_UPLOAD_TEST_CMD"
		printf '%s' ""
		return 0
	fi

	local cfg_cmd
	cfg_cmd=$(git config --get git-upload.testCmd 2>/dev/null || echo "")
	if [ -n "${cfg_cmd// /}" ]; then
		printf '%s\n' "$cfg_cmd"
		printf '%s' ""
		return 0
	fi

	local vs_task
	if vs_task=$(detect_vscode_test_task "$repo_root" 2>/dev/null); then
		# Already prints two lines: cmdline then cwd.
		printf '%s' "$vs_task"
		return 0
	fi

	if [ -x "$repo_root/scripts/test.sh" ]; then
		printf '%s\n' './scripts/test.sh'
		printf '%s' ""
		return 0
	fi

	if { [ -f "$repo_root/Makefile" ] || [ -f "$repo_root/makefile" ]; } && command -v make >/dev/null 2>&1; then
		local mk
		mk=$([ -f "$repo_root/Makefile" ] && echo "$repo_root/Makefile" || echo "$repo_root/makefile")
		if grep -qE '^test:' "$mk" 2>/dev/null; then
			printf '%s\n' 'make test'
			printf '%s' ""
			return 0
		fi
	fi

	if [ -f "$repo_root/package.json" ]; then
		# Only run JS tests if a real scripts.test exists.
		if package_json_test_script "$repo_root/package.json"; then
			if [ -f "$repo_root/pnpm-lock.yaml" ] && command -v pnpm >/dev/null 2>&1; then
				printf '%s\n' 'pnpm test --silent'
				printf '%s' ""
				return 0
			fi
			if [ -f "$repo_root/yarn.lock" ] && command -v yarn >/dev/null 2>&1; then
				printf '%s\n' 'yarn test --silent'
				printf '%s' ""
				return 0
			fi
			if command -v npm >/dev/null 2>&1; then
				printf '%s\n' 'npm test --silent'
				printf '%s' ""
				return 0
			fi
		fi
	fi

	if { [ -f "$repo_root/pyproject.toml" ] || [ -f "$repo_root/pytest.ini" ] || [ -d "$repo_root/tests" ]; } && command -v pytest >/dev/null 2>&1; then
		# Pytest exits non-zero when no tests are collected; require at least one test file.
		if repo_has_python_tests "$repo_root"; then
			printf '%s\n' 'pytest -q'
			printf '%s' ""
			return 0
		fi
	fi

	if [ -f "$repo_root/go.mod" ] && command -v go >/dev/null 2>&1; then
		# Avoid running go test in repos with zero *_test.go files.
		if repo_has_go_tests "$repo_root"; then
			printf '%s\n' 'go test ./...'
			printf '%s' ""
			return 0
		fi
	fi

	if [ -f "$repo_root/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
		# cargo test succeeds with zero tests, but only run when tests are likely present.
		if repo_has_rust_tests "$repo_root"; then
			printf '%s\n' 'cargo test -q'
			printf '%s' ""
			return 0
		fi
	fi

	# .NET (C#)
	if command -v dotnet >/dev/null 2>&1; then
		local sln
		sln=$(find "$repo_root" -maxdepth 2 -name '*.sln' -print 2>/dev/null | head -n 1 || true)
		if [ -n "${sln// /}" ]; then
			# Only run dotnet test when a test project exists (otherwise dotnet returns failure).
			local any_test_proj
			any_test_proj=$(find "$repo_root" -maxdepth 3 -name '*.csproj' -print 2>/dev/null | while read -r p; do
				if is_dotnet_test_project "$p"; then
					printf '%s\n' "$p"
					break
				fi
			done)
			if [ -n "${any_test_proj// /}" ]; then
				printf '%s\n' "dotnet test \"$sln\" --nologo"
				printf '%s' ""
				return 0
			fi
		fi

		local csproj
		csproj=$(find "$repo_root" -maxdepth 3 -name '*.csproj' -print 2>/dev/null | head -n 1 || true)
		if [ -n "${csproj// /}" ] && is_dotnet_test_project "$csproj"; then
			printf '%s\n' "dotnet test \"$csproj\" --nologo"
			printf '%s' ""
			return 0
		fi
	fi

	if [ -f "$repo_root/pom.xml" ] && command -v mvn >/dev/null 2>&1; then
		if repo_has_java_tests "$repo_root"; then
			printf '%s\n' 'mvn test -q'
			printf '%s' ""
			return 0
		fi
	fi

	if [ -x "$repo_root/gradlew" ]; then
		if repo_has_java_tests "$repo_root"; then
			printf '%s\n' './gradlew test -q'
			printf '%s' ""
			return 0
		fi
	fi
	if { [ -f "$repo_root/build.gradle" ] || [ -f "$repo_root/build.gradle.kts" ]; } && command -v gradle >/dev/null 2>&1; then
		if repo_has_java_tests "$repo_root"; then
			printf '%s\n' 'gradle test -q'
			printf '%s' ""
			return 0
		fi
	fi

	return 1
}

