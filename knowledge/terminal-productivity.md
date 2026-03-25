# Terminal Productivity — Shell Tricks & Power Tools

## Shell Fundamentals Everyone Forgets

### Brace Expansion (No spaces!)

```bash
echo {a,b,c}          # a b c
echo file{1..5}.txt    # file1.txt file2.txt file3.txt file4.txt file5.txt
echo {01..10}          # 01 02 03 04 05 06 07 08 09 10
mkdir -p project/{src,test,docs}/{main,utils}  # Creates 6 dirs
cp config.yml{,.bak}   # Copy config.yml to config.yml.bak
mv file.{txt,md}       # Rename file.txt to file.md
```

### Process Substitution

```bash
# Compare outputs of two commands
diff <(ls dir1) <(ls dir2)

# Feed command output as a file
while read -r line; do echo "$line"; done < <(grep -r TODO src/)

# Sort and diff two sorted streams
comm -13 <(sort file1) <(sort file2)  # Lines only in file2
```

### Here Documents & Here Strings

```bash
# Here document (multi-line input)
cat <<EOF > config.json
{
  "key": "$VALUE",
  "debug": true
}
EOF

# Here string (single-line input)
grep "pattern" <<< "$variable"
```

### Parameter Expansion

```bash
file="/path/to/photo.jpg"
echo "${file##*/}"     # photo.jpg  (strip longest prefix up to /)
echo "${file%.*}"      # /path/to/photo  (strip shortest suffix from .)
echo "${file%%.*}"     # /path/to/photo  (strip longest suffix from .)
echo "${file#*/}"      # path/to/photo.jpg  (strip shortest prefix up to /)

name="hello world"
echo "${name^}"        # Hello world  (capitalize first, bash 4+)
echo "${name^^}"       # HELLO WORLD  (uppercase all)
echo "${name,,}"       # hello world  (lowercase all)

# Default values
echo "${UNSET_VAR:-default}"    # Use default if unset/empty
echo "${UNSET_VAR:=default}"    # Set AND use default if unset/empty
echo "${MUST_EXIST:?Error msg}" # Exit with error if unset/empty
```

### Job Control

```bash
command &              # Run in background
jobs                   # List background jobs
fg %1                  # Bring job 1 to foreground
bg %1                  # Resume job 1 in background
Ctrl-Z                 # Suspend foreground process
kill %1                # Kill job 1
disown %1              # Detach job from shell (survives shell exit)
wait                   # Wait for all background jobs
```

## Essential CLI Tools

### fzf — Fuzzy Finder

```bash
# Interactive file finder
vim $(fzf)

# Search command history
history | fzf

# Git branch selector
git checkout $(git branch | fzf)

# Preview files while selecting
fzf --preview 'cat {}'
fzf --preview 'bat --color=always {}'

# Key bindings (add to .bashrc/.zshrc)
# Ctrl-R: fuzzy history search
# Ctrl-T: fuzzy file path completion
# Alt-C: fuzzy cd into directory
```

### ripgrep (rg) — Fast Search

```bash
rg "pattern"                     # Recursive search (respects .gitignore)
rg "pattern" -t py               # Only Python files
rg "pattern" -T js               # Exclude JavaScript files
rg "TODO|FIXME|HACK" -g "*.py"  # Glob filter
rg -l "pattern"                  # List matching files only
rg -c "pattern"                  # Count matches per file
rg -C 3 "pattern"               # 3 lines context
rg --json "pattern"              # Machine-readable output
rg -U "multi\nline"              # Multiline search
```

### fd — Better find

```bash
fd "pattern"                     # Find files matching pattern
fd -e py                         # Find by extension
fd -t d                          # Directories only
fd -t f -x chmod 644             # Execute command on results
fd -H "hidden"                   # Include hidden files
fd "test" --exec wc -l           # Count lines in test files
```

### jq — JSON Swiss Army Knife

```bash
# Pretty print
cat data.json | jq '.'

# Extract field
cat data.json | jq '.name'
cat data.json | jq '.users[0].email'

# Filter array
cat data.json | jq '.items[] | select(.price > 10)'

# Transform
cat data.json | jq '{name: .name, count: (.items | length)}'

# Collect into array
cat data.json | jq '[.items[] | .name]'

# From CSV-like
echo '{"a":1}\n{"a":2}' | jq -s 'map(.a) | add'

# Modify in place
jq '.version = "2.0"' package.json > tmp && mv tmp package.json
```

### bat — Better cat

```bash
bat file.py                  # Syntax highlighting + line numbers
bat -l json                  # Force language detection
bat --diff                   # Show git diff
bat -A file.txt              # Show non-printable characters
bat --range 10:20 file.py   # Show lines 10-20
```

### eza / exa — Better ls

```bash
eza -la                      # Long format, all files
eza --tree --level=2         # Tree view
eza -la --git                # Show git status
eza --icons                  # With file type icons
```

## Text Processing Power

### awk — Column and Pattern Processing

```bash
# Print specific columns
awk '{print $1, $3}' file.txt

# Filter by pattern
awk '/error/ {print $0}' logfile

# Sum a column
awk '{sum += $2} END {print sum}' data.txt

# Custom field separator
awk -F: '{print $1}' /etc/passwd

# Transform CSV
awk -F, '{printf "%-20s %s\n", $1, $2}' data.csv

# Count occurrences
awk '{count[$1]++} END {for (k in count) print k, count[k]}' access.log
```

### sed — Stream Editor

```bash
# Replace (first per line vs global)
sed 's/old/new/' file     # First occurrence per line
sed 's/old/new/g' file    # All occurrences

# Delete lines
sed '/pattern/d' file     # Lines matching pattern
sed '5d' file             # Line 5
sed '5,10d' file          # Lines 5-10

# Insert/append
sed '3i\New line' file    # Insert before line 3
sed '3a\New line' file    # Append after line 3

# Multiple operations
sed -e 's/a/b/' -e 's/c/d/' file
```

### cut, sort, uniq — The Unix Pipeline Staples

```bash
# Extract columns
cut -d: -f1,3 /etc/passwd        # Fields 1 and 3, colon-delimited
cut -c1-10 file.txt               # Characters 1-10

# Sort
sort file.txt                     # Alphabetical
sort -n file.txt                  # Numeric
sort -k2 -t, file.csv            # By column 2, comma-separated
sort -u file.txt                  # Unique lines (sort + uniq)
sort -h file.txt                  # Human-readable numbers (1K, 2M)

# Count occurrences
sort file.txt | uniq -c | sort -rn    # Most frequent lines first
```

### xargs — Build Commands from Input

```bash
# Delete all .pyc files
find . -name "*.pyc" | xargs rm

# With spaces in filenames (null-delimited)
find . -name "*.log" -print0 | xargs -0 rm

# Parallel execution
find . -name "*.png" | xargs -P4 -I{} convert {} -resize 50% {}

# Limit arguments per invocation
echo {1..100} | xargs -n 10 echo    # 10 args per line
```

## tmux — Terminal Multiplexer

### Essential Commands

```
PREFIX = Ctrl-b (default)

# Sessions
tmux new -s name          Create named session
tmux ls                   List sessions
tmux attach -t name       Attach to session
PREFIX d                  Detach from session
tmux kill-session -t name Kill session

# Windows (tabs)
PREFIX c     Create window
PREFIX n/p   Next/previous window
PREFIX 0-9   Switch to window by number
PREFIX ,     Rename window
PREFIX &     Close window

# Panes (splits)
PREFIX %     Vertical split
PREFIX "     Horizontal split
PREFIX o     Cycle through panes
PREFIX z     Toggle pane zoom (fullscreen)
PREFIX x     Close pane
PREFIX {/}   Move pane left/right
PREFIX space Cycle layouts
```

### Recommended .tmux.conf

```bash
# Prefix: Ctrl-a (easier than Ctrl-b)
set -g prefix C-a
unbind C-b

# Mouse support
set -g mouse on

# Start windows at 1 (not 0)
set -g base-index 1
setw -g pane-base-index 1

# Intuitive splits
bind | split-window -h -c "#{pane_current_path}"
bind - split-window -v -c "#{pane_current_path}"

# Vim-style pane navigation
bind h select-pane -L
bind j select-pane -D
bind k select-pane -U
bind l select-pane -R

# Faster escape
set -sg escape-time 0

# 256 color support
set -g default-terminal "screen-256color"
```

## Shell Aliases & Functions Worth Having

```bash
# Navigation
alias ..='cd ..'
alias ...='cd ../..'
alias -- -='cd -'

# Safety
alias rm='rm -i'
alias cp='cp -i'
alias mv='mv -i'

# Quick looks
alias ll='ls -la'
alias lt='ls -ltr'         # Sort by time, most recent last

# Git shortcuts
alias g='git'
alias gs='git status -sb'
alias gl='git log --oneline -20'
alias gd='git diff'
alias ga='git add'
alias gc='git commit'
alias gp='git push'

# Docker
alias d='docker'
alias dc='docker compose'
alias dps='docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"'

# Useful functions
mkcd() { mkdir -p "$1" && cd "$1"; }

# HTTP server in current directory
serve() { python3 -m http.server "${1:-8000}"; }

# Quick backup
bak() { cp "$1" "$1.bak.$(date +%Y%m%d_%H%M%S)"; }

# Extract any archive
extract() {
    case "$1" in
        *.tar.gz|*.tgz)  tar xzf "$1" ;;
        *.tar.bz2|*.tbz2) tar xjf "$1" ;;
        *.tar.xz)        tar xJf "$1" ;;
        *.zip)            unzip "$1" ;;
        *.gz)             gunzip "$1" ;;
        *.bz2)            bunzip2 "$1" ;;
        *.7z)             7z x "$1" ;;
        *)                echo "Unknown format: $1" ;;
    esac
}
```

## Performance Debugging from the Terminal

```bash
# Who's eating CPU/memory
top -o cpu                   # Sort by CPU (macOS)
htop                         # Interactive process viewer

# Disk usage
du -sh */                    # Directory sizes
du -sh * | sort -h           # Sorted by size
df -h                        # Filesystem usage
ncdu                         # Interactive disk usage (ncurses)

# Network
lsof -i :8080               # What's on port 8080
ss -tlnp                    # Listening TCP sockets (Linux)
netstat -an | grep LISTEN   # Listening sockets (macOS)

# File watching
watch -n 1 'ls -la file'    # Re-run command every second
fswatch .                    # File change events (macOS)
inotifywait -m dir/          # File change events (Linux)

# Benchmarking
time command                 # Wall/user/sys time
hyperfine 'command1' 'command2'  # Statistical benchmarking
```

---

_Philosophy: The terminal is a composable IDE. Small tools, piped together, solve problems no single tool can. Learn the Unix pipeline and you'll never need to leave the shell._
