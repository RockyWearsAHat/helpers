#!/usr/bin/env bash
# lib/env-patterns.sh — Default secret patterns and file classification for git-help-i-pushed-an-env
# Provides: DEFAULT_PATTERNS, build_pattern_list, is_example_file, is_file_empty

# Default patterns to scan/remove
DEFAULT_PATTERNS=(
	'.env'
	'.env.*'
	'*.env'
	'*.log'
	'logs/'
	'*.pem'
	'*.key'
	'*.p12'
	'*.pfx'
	'id_rsa'
	'id_dsa'
	'id_ecdsa'
	'id_ed25519'
	'credentials.json'
	'secrets.json'
	'secrets.yml'
	'secrets.yaml'
	'*-secret*.json'
	'application.properties'
	'application.yml'
	'application-*.properties'
	'application-*.yml'
	'appsettings.json'
	'appsettings.*.json'
	'web.config'
	'wp-config.php'
	'LocalSettings.php'
	'.aws/credentials'
	'.aws/config'
	'gcloud-service-key.json'
	'firebase-adminsdk*.json'
	'*.dump'
	'*.tfstate'
	'*.tfstate.*'
	'.terraform/'
)

# Build list of patterns to remove (includes user-supplied extras)
# Requires: extra_extensions, extra_files arrays from parent
build_pattern_list() {
	local patterns=("${DEFAULT_PATTERNS[@]}")

	for ext in "${extra_extensions[@]}"; do
		patterns+=("*$ext")
	done

	for file in "${extra_files[@]}"; do
		patterns+=("$file")
	done

	printf '%s\n' "${patterns[@]}"
}

# Check if a file is an example/sample/template file (usually safe to commit)
is_example_file() {
	local filepath="$1"
	if [[ "$filepath" == */test/* ]] || \
	   [[ "$filepath" == */tests/* ]] || \
	   [[ "$filepath" == */fixtures/* ]] || \
	   [[ "$filepath" == */fixture/* ]] || \
	   [[ "$filepath" == */simulation/* ]] || \
	   [[ "$filepath" == */__fixtures__/* ]] || \
	   [[ "$filepath" == */__mocks__/* ]] || \
	   [[ "$filepath" == */testdata/* ]] || \
	   [[ "$filepath" == */test-data/* ]]; then
		return 0
	fi
	if [[ "$filepath" == *.example ]] || \
	   [[ "$filepath" == *.example.* ]] || \
	   [[ "$filepath" == *.sample ]] || \
	   [[ "$filepath" == *.sample.* ]] || \
	   [[ "$filepath" == *.template ]] || \
	   [[ "$filepath" == *.template.* ]] || \
	   [[ "$filepath" == *-example.* ]] || \
	   [[ "$filepath" == *-sample.* ]] || \
	   [[ "$filepath" == *-template.* ]] || \
	   [[ "$filepath" == *_example.* ]] || \
	   [[ "$filepath" == *_sample.* ]] || \
	   [[ "$filepath" == *_template.* ]] || \
	   [[ "$filepath" == *.dist ]] || \
	   [[ "$filepath" == *.dist.* ]]; then
		return 0
	fi
	return 1
}

# Check if a file is empty or only whitespace (not a real concern)
is_file_empty() {
	local filepath="$1"
	if [ ! -f "$filepath" ]; then
		return 0
	fi
	if [ ! -s "$filepath" ]; then
		return 0
	fi
	local content
	content=$(grep -v '^\s*$' "$filepath" 2>/dev/null | grep -v '^\s*#' | head -1) || content=""
	if [ -z "$content" ]; then
		return 0
	fi
	return 1
}
