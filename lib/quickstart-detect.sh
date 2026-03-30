#!/usr/bin/env bash
# quickstart-detect.sh — Project type detection for git-copilot-quickstart
#
# Provides: detect_project_type
# Sets globals: DETECTED_LANG, DETECTED_FRAMEWORK, DETECTED_BUILD_CMD, DETECTED_TEST_CMD

detect_project_type() {
	local lang=""
	local framework=""
	local build_cmd=""
	local test_cmd=""

	# Check for various project indicators
	if [ -f "Cargo.toml" ]; then
		lang="Rust"
		build_cmd="cargo build"
		test_cmd="cargo test"
	elif [ -f "go.mod" ]; then
		lang="Go"
		build_cmd="go build ./..."
		test_cmd="go test ./..."
	elif [ -f "package.json" ]; then
		lang="JavaScript/TypeScript"
		if grep -q '"react"' package.json 2>/dev/null; then
			framework="React"
		elif grep -q '"vue"' package.json 2>/dev/null; then
			framework="Vue"
		elif grep -q '"next"' package.json 2>/dev/null; then
			framework="Next.js"
		fi
		build_cmd="npm run build"
		test_cmd="npm test"
	elif [ -f "pyproject.toml" ] || [ -f "setup.py" ] || [ -f "requirements.txt" ]; then
		lang="Python"
		test_cmd="pytest"
	elif [ -f "CMakeLists.txt" ]; then
		lang="C/C++"
		build_cmd="cmake --build build"
		test_cmd="ctest --test-dir build"
	elif [ -f "Makefile" ] || [ -f "makefile" ]; then
		lang="Make-based"
		build_cmd="make"
		if grep -qE '^test:' Makefile makefile 2>/dev/null; then
			test_cmd="make test"
		fi
	elif [ -f "pom.xml" ]; then
		lang="Java (Maven)"
		build_cmd="mvn compile"
		test_cmd="mvn test"
	elif [ -f "build.gradle" ] || [ -f "build.gradle.kts" ]; then
		lang="Java/Kotlin (Gradle)"
		build_cmd="./gradlew build"
		test_cmd="./gradlew test"
	elif find . -maxdepth 1 -name '*.sln' -print -quit 2>/dev/null | grep -q . || \
	     find . -maxdepth 2 -name '*.csproj' -print -quit 2>/dev/null | grep -q .; then
		lang="C# (.NET)"
		build_cmd="dotnet build"
		test_cmd="dotnet test"
	elif [ -f "mix.exs" ]; then
		lang="Elixir"
		build_cmd="mix compile"
		test_cmd="mix test"
	elif [ -f "Gemfile" ]; then
		lang="Ruby"
		test_cmd="bundle exec rspec"
	else
		# Check for shell scripts
		if find . -maxdepth 1 -name '*.sh' -print -quit 2>/dev/null | grep -q .; then
			lang="Shell"
		fi
	fi

	# Return values via global variables (zsh-compatible)
	DETECTED_LANG="${lang:-Unknown}"
	DETECTED_FRAMEWORK="${framework}"
	DETECTED_BUILD_CMD="${build_cmd}"
	DETECTED_TEST_CMD="${test_cmd}"
}
