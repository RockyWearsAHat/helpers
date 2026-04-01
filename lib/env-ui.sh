#!/usr/bin/env bash
# lib/env-ui.sh — UI helpers, logging, and interactive menu for git-help-i-pushed-an-env
# Provides: term_cols, ui_hr, ui_title, ui_kv, ui_note, ui_clear,
#           log_info, log_warn, log_error, log_success, log_verbose,
#           start_spinner, stop_spinner, read_key, menu_select, interactive_review_menu

# Colors (shared with parent script)
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
NC='\033[0m'

term_cols() {
	local cols=80
	if command -v tput &>/dev/null; then
		cols=$(tput cols 2>/dev/null || echo 80)
	fi
	echo "$cols"
}

ui_hr() {
	local cols
	cols=$(term_cols)
	printf '%s\n' "$(printf '%*s' "$cols" '' | /usr/bin/tr ' ' '─')"
}

ui_title() {
	local title="$1"
	ui_hr
	printf '%b%s%b\n' "$BOLD" "$title" "$NC"
	ui_hr
}

ui_kv() {
	local key="$1"
	local val="$2"
	printf '  %b%-12s%b %s\n' "$BOLD" "$key" "$NC" "$val"
}

ui_note() {
	printf '%b%s%b\n' "$CYAN" "$1" "$NC"
}

ui_clear() {
	printf '\033[2J\033[H'
}

# Logging
log_info() {
	printf '%b[INFO]%b %s\n' "$BLUE" "$NC" "$1" >&2
}

log_warn() {
	printf '%b[WARN]%b %s\n' "$YELLOW" "$NC" "$1" >&2
}

log_error() {
	printf '%b[ERROR]%b %s\n' "$RED" "$NC" "$1" >&2
}

log_success() {
	printf '%b[OK]%b %s\n' "$GREEN" "$NC" "$1" >&2
}

log_verbose() {
	if [ "$verbose" = true ]; then
		printf '%b[DEBUG]%b %s\n' "$CYAN" "$NC" "$1" >&2
	fi
}

# Progress spinner
_spinner_pid=""
start_spinner() {
	local msg="$1"
	(
		local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
		local i=1
		while true; do
			printf '\r\033[K%b[git-help-i-pushed-an-env]%b %s %s' "$MAGENTA" "$NC" "${frames[$i]}" "$msg" >&2
			i=$(( (i % 10) + 1 ))
			sleep 0.1
		done
	) &
	_spinner_pid=$!
	disown $_spinner_pid 2>/dev/null || true
}

stop_spinner() {
	local final_msg="${1:-}"
	if [ -n "$_spinner_pid" ] && kill -0 "$_spinner_pid" 2>/dev/null; then
		kill "$_spinner_pid" 2>/dev/null || true
		wait "$_spinner_pid" 2>/dev/null || true
	fi
	_spinner_pid=""
	if [ -n "$final_msg" ]; then
		printf '\r\033[K%b[git-help-i-pushed-an-env]%b %s\n' "$MAGENTA" "$NC" "$final_msg" >&2
	else
		printf '\r\033[K' >&2
	fi
}

restore_exit_trap() {
	local trap_spec="$1"
	if [ -n "$trap_spec" ]; then
		eval "$trap_spec"
	else
		trap - EXIT
	fi
}

# Read a single keypress from /dev/tty and normalize arrows/enter
read_key() {
	local key=""
	local rest=""
	if ! IFS= read -rsn 1 key </dev/tty; then
		return 1
	fi
	if [[ "$key" == $'\e' ]]; then
		if IFS= read -rsn 2 rest </dev/tty; then
			case "$rest" in
				"[A") echo "up"; return 0 ;;
				"[B") echo "down"; return 0 ;;
				"[C") echo "right"; return 0 ;;
				"[D") echo "left"; return 0 ;;
			esac
		fi
		echo "esc"
		return 0
	fi
	case "$key" in
		$'\r'|$'\n') echo "enter" ;;
		Q|q) echo "q" ;;
		j|J|n|N) echo "down" ;;
		k|K|p|P) echo "up" ;;
		h|H) echo "left" ;;
		l|L) echo "right" ;;
		*) echo "$key" ;;
	esac
}

# Arrow-key menu. Returns selected index (1-based) or "q".
menu_select() {
	local prompt="$1"
	shift
	local -a options=("$@")
	local selected=1
	local key=""
	local prompt_lines=0
	local saved_exit_trap=""
	[ -n "$prompt" ] && prompt_lines=1
	local total_lines=$(( ${#options[@]} + prompt_lines ))

	saved_exit_trap="$(trap -p EXIT || true)"
	printf '\033[?25l'
	trap 'printf "\033[?25h"' EXIT

	while true; do
		if [ -n "$prompt" ]; then
			printf '%s\n' "$prompt"
		fi
		local i=1
		for opt in "${options[@]}"; do
			if [ $i -eq $selected ]; then
				printf '\033[2K\r  %b%s%b\n' "$BOLD" "$opt" "$NC"
			else
				printf '\033[2K\r  %s\n' "$opt"
			fi
			((i++)) || true
		done

		key=$(read_key) || {
			printf '\033[%dA' "$total_lines"
			printf '\033[?25h'
			restore_exit_trap "$saved_exit_trap"
			return 1
		}
		case "$key" in
			up)
				((selected--)) || true
				[ $selected -lt 1 ] && selected=${#options[@]}
				;;
			down)
				((selected++)) || true
				[ $selected -gt ${#options[@]} ] && selected=1
				;;
			enter)
				printf '\033[%dA' "$total_lines"
				printf '\033[?25h'
				restore_exit_trap "$saved_exit_trap"
				echo "$selected"
				return 0
				;;
			q)
				printf '\033[%dA' "$total_lines"
				printf '\033[?25h'
				restore_exit_trap "$saved_exit_trap"
				echo "q"
				return 0
				;;
			*)
				;;
		esac
		printf '\033[%dA' "$total_lines"
	done
}

# Interactive review menu (arrow keys)
# Requires: REVIEW_LIST array, github_file_link function, check_ignore function
interactive_review_menu() {
	local read_only="${1:-true}"
	local allow_repair="${2:-false}"
	local total_items=${#REVIEW_LIST[@]}
	
	printf '\n%bFound %d item(s) to review.%b\n' "$YELLOW" "$total_items" "$NC"
	printf 'Open the review UI now? [Y/n] '
	read -r response </dev/tty
	if [[ "$response" =~ ^[Nn]$ ]]; then
		return
	fi
	
	printf '\n%bEntering interactive review mode...%b\n' "$CYAN" "$NC"
	printf 'Use %b↑/↓%b or %bj/k%b and %bEnter%b. Press %bq%b to quit.\n' "$BOLD" "$NC" "$BOLD" "$NC" "$BOLD" "$NC" "$BOLD" "$NC"

	local i=1
	while [ $i -le $total_items ]; do
		local item="${REVIEW_LIST[$i]}"
		local repo="${item%%|*}"
		local rest="${item#*|}"
		local path="${rest%%|*}"
		rest="${rest#*|}"
		local loc="${rest%%|*}"
		local details="${rest#*|}"

		ui_clear
		ui_title "Secret Review"
		ui_kv "Item" "$i/$total_items"
		ui_kv "Repo" "$repo"
		ui_kv "File" "$path"
		ui_kv "Type" "$loc"
		ui_kv "Details" "$details"
		ui_hr
		ui_note "Navigate: ↑/↓ or j/k · Select: Enter · Quit: q"

		local -a actions
		actions=("Next item" "Previous item" "Open in GitHub" "Quit review")
		if [ "$read_only" = false ]; then
			actions=("Next item" "Previous item" "Open in GitHub" "Ignore in scan list" "Quit review")
		fi
		if [ "$allow_repair" = true ]; then
			actions=("Next item" "Previous item" "Open in GitHub" "Repair this repo now" "Quit review")
		fi

		local choice
		choice=$(menu_select "Action" "${actions[@]}") || return 0
		case "$choice" in
			q)
				printf 'Exiting review.\n'
				return
				;;
			1)
				((i++)) || true
				if [ $i -gt $total_items ]; then
					i=$total_items
				fi
				;;
			2)
				((i--)) || true
				if [ $i -lt 1 ]; then
					i=1
				fi
				;;
			3)
				local url
				url=$(github_file_link "$repo" "$path")
				if command -v open &>/dev/null; then
					open "$url"
					printf '  Opened in browser.\n'
				else
					printf '  URL: %s\n' "$url"
				fi
				;;
			4)
				if [ "$read_only" = false ]; then
					add_to_ignore "$repo" "$path"
					printf '%b  ✓ Added to ignore list (will be skipped next scan)%b\n' "$GREEN" "$NC"
				elif [ "$allow_repair" = true ]; then
					printf '\n%bEnter repair mode now? This may rewrite history. [y/N]%b ' "$YELLOW" "$NC"
					local confirm
					read -r confirm </dev/tty
					if [[ "$confirm" =~ ^[Yy]$ ]]; then
						run_interactive_mode
						return
					fi
				fi
				;;
			5)
				printf 'Exiting review.\n'
				return
				;;
		esac
	done

	printf '\n%bReview complete!%b\n' "$GREEN" "$NC"
	if [ "$read_only" = false ]; then
		printf 'Tip: Run the scan again to update the cache with your ignored files.\n'
	fi
}
