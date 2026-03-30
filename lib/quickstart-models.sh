#!/usr/bin/env bash
# quickstart-models.sh — Model selection for git-copilot-quickstart
#
# Provides: fetch_copilot_models, parse_models, interactive_select, select_model
# Sets globals: SELECTED_MODEL, SELECTED_MODEL_DISPLAY, SELECTED_INDEX
#              MODEL_IDS, MODEL_NAMES, MODEL_VENDORS, MODEL_CAPABILITIES

# Fallback models if API fetch fails
FALLBACK_MODELS=(
	"claude-sonnet-4|Claude Sonnet 4"
	"gpt-4.1|GPT-4.1"
	"gpt-4o|GPT-4o"
	"o3-mini|o3-mini"
)

# Fetch available Copilot models from GitHub API
fetch_copilot_models() {
	local token=""
	local models_json=""

	# Try to get GitHub token from gh CLI
	if command -v gh &>/dev/null; then
		token=$(gh auth token 2>/dev/null || echo "")
	fi

	# Try environment variables if gh CLI didn't work
	if [ -z "$token" ]; then
		token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
	fi

	if [ -z "$token" ]; then
		echo "[git-copilot-quickstart] Warning: No GitHub token found. Using fallback model list." >&2
		echo "[git-copilot-quickstart] Run 'gh auth login' for live model list." >&2
		return 1
	fi

	# Fetch models from GitHub Copilot API
	models_json=$(curl -s -f \
		-H "Authorization: Bearer $token" \
		-H "Accept: application/json" \
		-H "Copilot-Integration-Id: vscode-chat" \
		-H "Editor-Version: vscode/1.96.0" \
		"https://api.githubcopilot.com/models" 2>/dev/null) || {
		echo "[git-copilot-quickstart] Warning: Failed to fetch models from API. Using fallback list." >&2
		return 1
	}

	# Check if we got valid JSON with models
	if ! echo "$models_json" | grep -q '"models"'; then
		echo "[git-copilot-quickstart] Warning: Invalid API response. Using fallback list." >&2
		return 1
	fi

	echo "$models_json"
	return 0
}

# Parse models JSON and populate arrays
parse_models() {
	local json="$1"

	# Clear arrays
	MODEL_IDS=()
	MODEL_NAMES=()
	MODEL_VENDORS=()
	MODEL_CAPABILITIES=()

	# Use Python/jq to parse JSON if available, otherwise basic parsing
	if command -v python3 &>/dev/null; then
		# Python parsing for reliable JSON handling
		# Pass JSON via stdin to avoid shell injection from untrusted API responses
		eval "$(printf '%s' "$json" | python3 -c '
import json
import sys

try:
    data = json.loads(sys.stdin.read())
    models = data.get("models", [])

    # Filter for chat-capable models
    chat_models = [m for m in models if "chat" in m.get("capabilities", [])]

    # Sort: recommended first, then by name
    def sort_key(m):
        is_default = m.get("is_default", False)
        name = m.get("name", "")
        return (not is_default, name.lower())

    chat_models.sort(key=sort_key)

    ids = []
    names = []
    vendors = []
    caps = []

    for m in chat_models:
        ids.append(m.get("id", ""))
        names.append(m.get("name", m.get("id", "")))
        vendors.append(m.get("vendor", ""))
        caps.append(",".join(m.get("capabilities", [])))

    # Output as zsh array assignments with proper shell escaping
    def shell_escape(s):
        """Escape string for safe use in shell double quotes"""
        return s.replace("\\\\", "\\\\\\\\").replace("\"", "\\\\\"").replace("$", "\\\\$").replace("`", "\\\\`")

    print("MODEL_IDS=(" + " ".join(f"\"{shell_escape(i)}\"" for i in ids) + ")")
    print("MODEL_NAMES=(" + " ".join(f"\"{shell_escape(n)}\"" for n in names) + ")")
    print("MODEL_VENDORS=(" + " ".join(f"\"{shell_escape(v)}\"" for v in vendors) + ")")
    print("MODEL_CAPABILITIES=(" + " ".join(f"\"{shell_escape(c)}\"" for c in caps) + ")")
except Exception as e:
    print(f"# Parse error: {e}", file=sys.stderr)
    sys.exit(1)
' 2>/dev/null)"
		return $?
	elif command -v jq &>/dev/null; then
		# jq parsing as fallback
		local model_data
		model_data=$(echo "$json" | jq -r '
			.models
			| map(select(.capabilities | contains(["chat"])))
			| sort_by(if .is_default then 0 else 1 end, .name)
			| .[]
			| "\(.id)|\(.name)|\(.vendor)|\(.capabilities | join(","))"
		' 2>/dev/null) || return 1

		while IFS='|' read -r id name vendor caps; do
			MODEL_IDS+=("$id")
			MODEL_NAMES+=("$name")
			MODEL_VENDORS+=("$vendor")
			MODEL_CAPABILITIES+=("$caps")
		done <<< "$model_data"
		return 0
	fi

	return 1
}

# Interactive arrow-key selector
# Usage: interactive_select "prompt" array_of_options
# Sets global: SELECTED_INDEX
interactive_select() {
	local prompt="$1"
	shift
	local options=("$@")
	local num_options=${#options[@]}
	local selected=1
	local default_idx=1

	# Find default/recommended option (first one, or one marked with ⭐)
	for i in {1..$num_options}; do
		if [[ "${options[$i]}" == *"⭐"* ]] || [[ "${options[$i]}" == *"(default)"* ]]; then
			selected=$i
			default_idx=$i
			break
		fi
	done

	# Hide cursor
	tput civis 2>/dev/null || true

	# Trap to restore cursor on exit
	trap 'tput cnorm 2>/dev/null || true; trap - INT TERM EXIT' INT TERM EXIT

	# Print header
	echo ""
	echo "$prompt"
	echo ""

	# Function to draw options
	draw_options() {
		# Move cursor up to redraw
		for ((i = 1; i <= num_options; i++)); do
			tput cuu1 2>/dev/null || printf '\033[1A'
		done
		tput cr 2>/dev/null || printf '\r'

		for i in {1..$num_options}; do
			# Clear line
			tput el 2>/dev/null || printf '\033[K'

			if [ "$i" -eq "$selected" ]; then
				# Highlighted option
				printf "  \033[7m → %s \033[0m\n" "${options[$i]}"
			else
				printf "    %s\n" "${options[$i]}"
			fi
		done
	}

	# Initial draw
	for i in {1..$num_options}; do
		if [ "$i" -eq "$selected" ]; then
			printf "  \033[7m → %s \033[0m\n" "${options[$i]}"
		else
			printf "    %s\n" "${options[$i]}"
		fi
	done

	# Read input
	while true; do
		# Read single character (including escape sequences)
		local key
		IFS= read -rsn1 key

		case "$key" in
			$'\x1b')
				# Escape sequence - read more
				read -rsn2 -t 0.1 key2 || true
				case "$key2" in
					'[A') # Up arrow
						if [ "$selected" -gt 1 ]; then
							((selected--))
							draw_options
						fi
						;;
					'[B') # Down arrow
						if [ "$selected" -lt "$num_options" ]; then
							((selected++))
							draw_options
						fi
						;;
				esac
				;;
			'k'|'K') # Vim up
				if [ "$selected" -gt 1 ]; then
					((selected--))
					draw_options
				fi
				;;
			'j'|'J') # Vim down
				if [ "$selected" -lt "$num_options" ]; then
					((selected++))
					draw_options
				fi
				;;
			'') # Enter key
				break
				;;
			'q'|'Q') # Quit - use default
				selected=$default_idx
				break
				;;
			[1-9]) # Number key for quick selection
				if [ "$key" -le "$num_options" ]; then
					selected=$key
					draw_options
				fi
				;;
		esac
	done

	# Restore cursor
	tput cnorm 2>/dev/null || true
	trap - INT TERM EXIT

	SELECTED_INDEX=$selected
}

select_model() {
	echo ""
	echo "┌───────────────────────────────────────────────────────────────────┐"
	echo "│              Select Copilot Model for Agents                      │"
	echo "├───────────────────────────────────────────────────────────────────┤"
	echo "│  Use ↑/↓ arrows (or j/k) to navigate, Enter to select            │"
	echo "└───────────────────────────────────────────────────────────────────┘"

	# Try to fetch models from API
	local models_json
	local use_api=false

	echo ""
	echo "  Fetching available models from GitHub Copilot..."

	if models_json=$(fetch_copilot_models); then
		if parse_models "$models_json"; then
			use_api=true
			echo "  ✓ Found ${#MODEL_IDS[@]} models"
		fi
	fi

	# Use fallback if API failed
	if [ "$use_api" = false ]; then
		echo "  Using fallback model list"
		MODEL_IDS=()
		MODEL_NAMES=()
		MODEL_VENDORS=()
		for entry in "${FALLBACK_MODELS[@]}"; do
			MODEL_IDS+=("${entry%%|*}")
			MODEL_NAMES+=("${entry#*|}")
			MODEL_VENDORS+=("")
		done
	fi

	# Build display options
	local display_options=()
	for i in {1..${#MODEL_IDS[@]}}; do
		local id="${MODEL_IDS[$i]}"
		local name="${MODEL_NAMES[$i]}"
		local vendor="${MODEL_VENDORS[$i]}"

		local display="$name"
		if [ -n "$vendor" ]; then
			display="$name ($vendor)"
		fi

		# Mark first option as default/recommended
		if [ "$i" -eq 1 ] && [ "$use_api" = true ]; then
			display="⭐ $display (recommended)"
		fi

		display_options+=("$display")
	done

	# Check if we're in an interactive terminal
	if [ -t 0 ] && [ -t 1 ]; then
		# Interactive mode - use arrow key selector
		interactive_select "  Available models:" "${display_options[@]}"
		local selection=$SELECTED_INDEX
	else
		# Non-interactive - just use first option
		echo ""
		echo "  Non-interactive mode, using default: ${MODEL_NAMES[1]}"
		local selection=1
	fi

	# Set selected model
	SELECTED_MODEL="${MODEL_IDS[$selection]}"
	SELECTED_MODEL_DISPLAY="${MODEL_NAMES[$selection]}"

	echo ""
	echo "  ✓ Selected: $SELECTED_MODEL_DISPLAY"
	echo ""
}
