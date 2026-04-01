# CLI UX Engineering: Progress, Color, Interactivity, and Machine-Readable Output

## Introduction

CLI user experience spans aesthetics (color, formatting), interactivity (prompts, fuzzy selection), feedback (progress bars, status messages), and output structure (human vs. machine readable). A well-designed CLI feels responsive and transparent; a poorly designed one frustrates with cryptic errors and no feedback. This note covers patterns, ANSI escape sequences, tools like `fzf`, and the delicate balance between interactive ease and scriptability.

## ANSI Escape Codes: Colors and Formatting

ANSI escape sequences control terminal output: colors, bold, underline, cursor movement. Standard `ESC[` introduces a sequence; `m` terminates color codes.

### Basic Colors and Styles

```bash
# Colors (foreground 30–37, background 40–47)
echo -e "\033[31mRed text\033[0m"                       # 31 = red, 0 = reset
echo -e "\033[1;32mBold Green\033[0m"                   # 1 = bold; 32 = green
echo -e "\033[44mBlue background\033[0m"                # 44 = blue background
echo -e "\033[1;37;44mWhite on blue, bold\033[0m"

# Text styling
echo -e "\033[1mBold\033[0m"
echo -e "\033[4mUnderlined\033[0m"
echo -e "\033[7mReverse (invert colors)\033[0m"

# Clear formatting
echo -e "\033[0mReset all\033[0m"
```

### 256-Color and True Color (24-bit)

Modern terminals support 256 colors (xterm) and true color (24-bit RGB):

```bash
# 256-color mode
echo -e "\033[38;5;196mBright red\033[0m"               # Foreground
echo -e "\033[48;5;21mBright blue background\033[0m"    # Background

# True color (24-bit RGB)
echo -e "\033[38;2;255;100;0mOrange\033[0m"             # RGB: 255, 100, 0
echo -e "\033[48;2;0;128;255mBlue background\033[0m"
```

Color value reference: 0–7 (basic), 8–15 (bright), 16–231 (256-color palette), 232–255 (grayscale).

### Cursor Movement and Clearing

```bash
# Move cursor
echo -e "\033[10A"                      # Up 10 lines
echo -e "\033[5B"                       # Down 5 lines
echo -e "\033[C"                        # Right 1 column

# Clear screen
echo -e "\033[2J"                       # Clear entire screen
echo -e "\033[K"                        # Clear from cursor to end of line
echo -e "\033[0J"                       # Clear from cursor to end of screen

# Save and restore cursor position
echo -e "\033[s"                        # Save
echo -e "\033[u"                        # Restore

# Move to absolute position
echo -e "\033[10;5H"                    # Move to row 10, column 5
```

### tput: Terminal Capability Queries

`tput` abstracts terminal differences; use it instead of hardcoding escape sequences for portability.

```bash
# Detect terminal capabilities
tput colors                             # Number of colors supported (8, 256, etc.)
tput cols                               # Terminal width
tput lines                              # Terminal height

# Color output (portable)
red=$(tput setaf 1)                     # Set foreground color 1 (red)
bold=$(tput bold)                       # Bold
reset=$(tput sgr0)                      # Reset all

echo "${red}Error: ${reset}Something failed"

# Cursor control
tput cuu 3                              # Move up 3 lines
tput cup 10 5                           # Move to row 10, column 5
tput ed                                 # Clear to end of display

# Practical: portable progress overwrite (update in place)
for i in {1..100}; do
    echo -ne "\rProgress: $i%"          # \r = carriage return (overwrite)
    sleep 0.01
done
echo                                    # Final newline
```

Portability across terminals: `tput` handles differences between xterm, screen, tmux, and others. Use `tput` when terminal capabilities are needed; direct escape sequences work reliably for standard colors and cursor movement but may misbehave in less common terminal emulators.

## Progress Indicators: Spinners and Progress Bars

### Spinner (Indeterminate Progress)

```bash
# Simple spinner without blocking
spinner() {
    local chars=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
    local i=0
    
    while true; do
        echo -ne "\r${chars[$((i % ${#chars[@]}))]}"
        i=$((i + 1))
        sleep 0.1
    done
}

# Run in background
long_operation &
pid=$!
spinner &
spinner_pid=$!

wait $pid
kill $spinner_pid 2>/dev/null
echo -e "\rDone!    "                   # Overwrite spinner with message
```

### Progress Bar (Determinate Progress)

```bash
# Simple percentage bar
progress_bar() {
    local current=$1
    local total=$2
    local width=50
    
    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    
    echo -ne "\rProgress: ["
    for ((i = 0; i < filled; i++)); do printf "="; done
    for ((i = filled; i < width; i++)); do printf "-"; done
    printf "] %d%%\r" "$percent"
}

# Usage
for i in {1..100}; do
    progress_bar $i 100
    sleep 0.01
done
echo -e "\nDone!"
```

## Interactive Prompts: select, read, and fzf

### Bash select (Built-in Menu)

```bash
# select creates a numbered menu
select option in "Option 1" "Option 2" "Cancel"; do
    case $option in
        "Option 1") echo "Selected 1"; break ;;
        "Option 2") echo "Selected 2"; break ;;
        "Cancel") exit ;;
        *) echo "Invalid" ;;
    esac
done
```

Limitations: limited customization, small menus only. For larger datasets or custom UX, use external tools.

### read for Input Validation

```bash
# Prompt for input with timeout
read -t 5 -p "Enter name (timeout in 5s): " name

# Prompt for password (no echo)
read -s -p "Enter password: " pass
echo                                    # Newline after hidden input

# Multiline input
echo "Enter description (Ctrl+D to end):"
read -d '' description                  # Read until null terminator

# Confirmation prompt
read -p "Continue? (y/n) " -n 1 reply
echo                                    # Newline
[[ "$reply" =~ ^[yY]$ ]] && echo "Continuing..." || echo "Cancelled"
```

### fzf: Fuzzy Finder for Rich Selection

`fzf` provides fuzzy matching and interactive selection on any list. Modern replacement for `select`.

```bash
# Basic usage: select from list
choice=$(echo -e "apple\norange\nbanana\ngrape" | fzf)
echo "You selected: $choice"

# Select multiple items
choices=$(echo -e "file1\nfile2\nfile3" | fzf -m)       # -m = multi-select
echo "You selected: $choices"

# Integration with commands
# Select file from git history
git show $(git log --oneline | fzf | cut -d' ' -f1)

# Search environment variables
eval $(env | fzf)

# Search command history
eval $(history | cut -d' ' -f4- | sort -u | fzf)
```

fzf options:

```bash
--multi                                 # Multi-select mode
--preview 'cat {}'                      # Preview file content when highlighted
--height 50%                            # Use bottom half of terminal
--bind 'ctrl-a:select-all,ctrl-d:deselect-all'  # Custom key bindings
--color 'fg:blue'                       # Customize colors
--reverse                               # Show search box at bottom
--exact                                 # Exact matching only (no fuzzy)
```

## Help Text Formatting and Documentation

### Standard Help Output Structure

```bash
#!/bin/bash

mytool_help() {
    cat <<EOF
Usage: $0 [OPTION]... [FILE]...

Description:
  A brief description of what this tool does.

Options:
  -h, --help                Show this help and exit
  -v, --verbose             Enable verbose output
  -f, --file FILE           Specify input file (required)
  -o, --output FILE         Write output to file (default: stdout)
  -c, --config CONFIG       Configuration file (default: ~/.config/mytool)
  -j, --jobs NUM            Number of parallel jobs (default: 4)

Examples:
  # Process file with verbose output
  $0 -vf input.txt -o output.txt

  # Use configuration and parallel jobs
  $0 -c ~/.config/mytool -j 8 *.txt

Environment:
  MY_DEBUG                  Set to 1 for debug output
  MY_CONFIG                 Override default config path

Exit Codes:
  0     Success
  1     General error
  2     Argument error
  126   File not executable
  127   Command not found

See Also:
  man mytool
  https://example.com/mytool/docs

EOF
}

# Invoke with -h
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    mytool_help
    exit 0
fi
```

### Pager Detection for Long Output

```bash
# Detect if output is going to a pager or terminal
if [[ -t 1 ]]; then
    # Terminal: color and formatting safe
    output_mode="color"
else
    # Pipe or redirect: no colors
    output_mode="plain"
fi

# Conditional formatting
if [[ "$output_mode" == "color" ]]; then
    red='\033[31m'
    reset='\033[0m'
else
    red=''
    reset=''
fi

echo -e "${red}Error${reset}: something failed"
```

## Exit Codes: Signaling Success and Failure

Exit codes communicate program status to calling code:

```bash
# Convention
exit 0                  # Success
exit 1                  # General error
exit 2                  # Misuse of command (bad args)
exit 126                # Command found but not executable
exit 127                # Command not found
exit 128 + N            # Terminated by signal N
exit 130                # Terminated by Ctrl+C (SIGINT=2; 128+2=130)
```

Script patterns for clean exit codes:

```bash
#!/bin/bash
set -euo pipefail

main() {
    # Do work
    echo "Success"
}

main "$@"
exit $?                 # Propagate exit code
```

Using exit codes in conditions:

```bash
if command; then
    echo "Command succeeded"
else
    code=$?
    case $code in
        1) echo "General error" ;;
        2) echo "Misuse" ;;
        *) echo "Unknown error: $code" ;;
    esac
    exit "$code"
fi
```

## Signal Handling and Graceful Cleanup

CLIs receive signals during execution. Proper handling ensures cleanup:

```bash
#!/bin/bash

# Cleanup function
cleanup() {
    local exit_code=$?
    echo "Cleaning up..."
    
    # Kill any background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    # Remove temp files
    rm -f /tmp/mytool_*
    
    # Restore terminal state if modified
    stty echo                           # Re-enable terminal echo if disabled
    
    exit $exit_code
}

# Register trap
trap cleanup EXIT
trap 'echo "Interrupted"; exit 130' INT    # Ctrl+C

# Your program below
echo "Processing..."
sleep 100
```

Common signals and default behavior:

| Signal | Default    | Cause              | Handle For              |
| ------ | ---------- | ------------------ | ----------------------- |
| SIGINT | Terminate  | Ctrl+C             | Cleanup temp files      |
| SIGTERM| Terminate  | System shutdown    | Graceful shutdown       |
| SIGHUP | Terminate  | Terminal closed    | Save state              |
| SIGPIPE| Terminate  | Pipe closed        | Prevent crash on closed pipe |

## Machine-Readable Output: JSON, CSV, and Structured Formats

Balance human-readable output with machine-parseable options:

```bash
#!/bin/bash

# Detect output format from flag
format="human"
case "$1" in
    --json) format="json" ;;
    --csv) format="csv" ;;
esac

# Example: list files with metadata
files=(file1.txt file2.txt file3.txt)

case "$format" in
    human)
        for f in "${files[@]}"; do
            # Colored, pretty output for humans
            printf "%-20s %10d bytes\n" "$f" "$(wc -c < "$f")"
        done
        ;;
    json)
        # JSON output for programmatic consumption
        jq -n --arg files "$(for f in "${files[@]}"; do echo "$f"; done)" \
            '[inputs] | map({name: ., size: 0})'
        # Real implementation would compute sizes
        ;;
    csv)
        # CSV header and rows
        echo "name,size"
        for f in "${files[@]}"; do
            echo "$f,$(wc -c < "$f")"
        done
        ;;
esac
```

Standard formats:

- **JSON**: Best for rich data structures; widely supported in programming languages
- **CSV**: Simple tabular data; easily imported to spreadsheets
- **TSV**: Tab-separated; copy-paste friendly in terminals
- **YAML**: Human-readable; common in configuration
- **Plain text**: Suitable when structure is implicit (line-based)

## Streaming Output and Piping

Design CLIs to work efficiently in pipelines:

```bash
# Good: streaming output suitable for piping
while read -r line; do
    process "$line"
done < input.txt

# Bad: buffered output; delays availability to downstream processes
cat input.txt | process_all_at_once

# Unbuffered output for real-time feedback
python -u script.py             # -u = unbuffered
stdbuf -oL command              # Line-buffered stdout (with stdbuf)
```

## Silent Mode and Verbosity Levels

Support users who want less or more output:

```bash
#!/bin/bash

verbose=0       # 0 = normal, 1 = verbose, -1 = quiet

log() {
    if [[ $verbose -ge 0 ]]; then
        echo "INFO: $*" >&2
    fi
}

debug() {
    if [[ $verbose -ge 1 ]]; then
        echo "DEBUG: $*" >&2
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose) verbose=$((verbose + 1)); shift ;;
        -q|--quiet) verbose=-1; shift ;;
        *) break ;;
    esac
done

log "Starting..."
debug "Config loaded"
```

## Cross-References

See also: [cli-design-patterns.md](cli-design-patterns.md) (argument parsing, exit codes), [shell-zsh-power.md](shell-zsh-power.md) (fzf integration, completion), [shell-testing-quality.md](shell-testing-quality.md) (testing interactive commands), [terminal-productivity.md](terminal-productivity.md) (awk/sed for output formatting).