#!/usr/bin/env bash
# lib/upload-spinner.sh — Progress spinner for git-upload long-running operations
# Sourced by git-upload. Do not run directly.

_spinner_pid=""
start_spinner() {
	local msg="$1"
	# Only show spinner if stderr is a real terminal
	if [ ! -t 2 ]; then
		return 0
	fi
	(
		trap 'exit 0' TERM
		local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
		local i=0
		while true; do
			printf '\033[2K\r[git-upload] %s %s' "${frames[$i]}" "$msg" >&2
			i=$(( (i + 1) % 10 ))
			sleep 0.3
		done
	) &
	_spinner_pid=$!
}

stop_spinner() {
	local final_msg="${1:-}"
	if [ -n "$_spinner_pid" ]; then
		kill "$_spinner_pid" 2>/dev/null || true
		wait "$_spinner_pid" 2>/dev/null || true
	fi
	_spinner_pid=""
	printf '\033[2K\r' >&2
	if [ -n "$final_msg" ]; then
		printf '[git-upload] %s\n' "$final_msg" >&2
	fi
}
