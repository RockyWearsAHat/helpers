# Shell Scripting Best Practices

## The Foundation: Safe Defaults

Every shell script should start with:

```bash
#!/usr/bin/env bash
set -euo pipefail
```

| Flag | Effect |
|------|--------|
| `set -e` (errexit) | Exit immediately on non-zero return |
| `set -u` (nounset) | Error on undefined variables |
| `set -o pipefail` | Pipeline fails if ANY command fails (not just the last) |

**Without `pipefail`:**
```bash
curl http://bad-url | grep pattern  # grep returns 0 even if curl fails!
```

## Quoting — The #1 Source of Shell Bugs

```bash
# ✅ ALWAYS quote variables
echo "$name"
cp "$source" "$dest"
if [[ "$var" == "value" ]]; then

# ❌ NEVER leave variables unquoted
echo $name          # Word splitting + glob expansion
cp $source $dest    # Breaks on spaces in filenames
if [ $var == value ] # Fails if var is empty

# Quote arrays properly
files=("file one.txt" "file two.txt")
for f in "${files[@]}"; do   # ✅ Preserves elements
    echo "$f"
done

# When to NOT quote:
# - Glob patterns: for f in *.txt (intentional expansion)
# - Arithmetic: $(( count + 1 ))
# - Inside [[ ]] for regex: [[ $var =~ ^[0-9]+$ ]]
```

## Use [[ ]] Not [ ]

```bash
# [[ ]] is a bash builtin — safer and more powerful than [ ]
[[ -f "$file" ]]           # File exists and is regular
[[ -d "$dir" ]]            # Directory exists
[[ -z "$var" ]]            # String is empty
[[ -n "$var" ]]            # String is non-empty
[[ "$a" == "$b" ]]         # String equality
[[ "$var" =~ ^[0-9]+$ ]]  # Regex matching
[[ "$a" < "$b" ]]          # String comparison (no escaping needed)
[[ -f "$f" && -r "$f" ]]  # Logical AND (safe inside [[ ]])
```

## Functions

```bash
# Prefer this form (POSIX + bash compatible)
my_function() {
    local name="$1"
    local count="${2:-0}"  # Default value

    # Use local for ALL function variables
    local result
    result=$(compute "$name")

    echo "$result"
}

# Return values: echo output (captured with $()) or return codes
get_name() {
    echo "Alice"
}
name=$(get_name)

# Error signaling: return non-zero
validate_port() {
    local port="$1"
    if [[ "$port" -lt 1 || "$port" -gt 65535 ]]; then
        return 1
    fi
}

if validate_port "$port"; then
    echo "Valid"
fi
```

## Variable Best Practices

```bash
# Use lowercase for local variables, UPPERCASE for exported/environment
local file_path="/tmp/data.txt"
export DATABASE_URL="postgres://..."

# Use descriptive names
# ❌ f, d, x
# ✅ file_path, dir_name, exit_code

# Default values
name="${1:-default}"         # Use default if $1 is unset or empty
name="${1:?'name required'}" # Error if $1 is unset or empty

# Readonly for constants
readonly MAX_RETRIES=3
readonly CONFIG_DIR="${HOME}/.config/myapp"

# Arrays
declare -a files=()
files+=("one.txt")
files+=("two.txt")
echo "${#files[@]}"  # Length
echo "${files[0]}"   # First element
```

## Command Substitution & Process Substitution

```bash
# Command substitution: $() (never backticks)
count=$(wc -l < "$file")
today=$(date +%Y-%m-%d)

# Process substitution: <() — treat command output as a file
diff <(sort file1.txt) <(sort file2.txt)
while IFS= read -r line; do
    process "$line"
done < <(find . -name "*.txt")

# Here-strings
grep "pattern" <<< "$variable"

# Here-documents
cat << 'EOF'
This is a literal heredoc.
Variables like $HOME are NOT expanded.
EOF

cat << EOF
This heredoc DOES expand variables: $HOME
EOF
```

## Robust File & Path Handling

```bash
# Use mktemp for temporary files
tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT  # Clean up on exit

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

# Resolve real paths
real_path=$(realpath "$file")
script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

# Safe file reading
while IFS= read -r line; do
    echo "$line"
done < "$file"

# Find + safe iteration (handles spaces, newlines in filenames)
find . -name "*.sh" -print0 | while IFS= read -r -d '' file; do
    shellcheck "$file"
done
```

## Error Handling

```bash
# Trap for cleanup
cleanup() {
    rm -f "$tmpfile"
    # ... other cleanup
}
trap cleanup EXIT      # Runs on any exit
trap cleanup ERR       # Runs on error (with set -e)
trap cleanup INT TERM  # Runs on Ctrl-C or kill

# Die function
die() {
    echo "ERROR: $*" >&2
    exit 1
}

[[ -f "$config" ]] || die "Config file not found: $config"

# Error messages to stderr
echo "Processing..." >&1    # stdout — normal output
echo "Warning: slow" >&2    # stderr — diagnostics

# Check command availability
command -v jq > /dev/null 2>&1 || die "jq is required but not installed"
```

## ShellCheck

**Run ShellCheck on every script.** It catches:
- Unquoted variables (SC2086)
- Useless use of cat (SC2002)
- Bash-specific features in sh scripts (SC2039)
- Word splitting issues (SC2046)
- Globbing issues (SC2035)
- And hundreds more

```bash
# Install
brew install shellcheck   # macOS
apt install shellcheck    # Debian/Ubuntu

# Run
shellcheck script.sh

# Inline suppressions (when you know better)
# shellcheck disable=SC2086
echo $intentionally_unquoted
```

## POSIX Portability

If the script must run on non-bash shells (dash, sh, ash):

```bash
#!/bin/sh  # Not bash — POSIX only

# No [[ ]], use [ ]
# No arrays
# No (( )) arithmetic, use $(( ))
# No process substitution <()
# No local keyword in functions (use it anyway — widely supported)
# No ${var,,} lowercase or ${var^^} uppercase
# No =~ regex matching
```

## Common Pitfalls

1. **Unquoted `$@`**: `"$@"` preserves arguments. `$@` splits on spaces.
2. **`cd` without checking**: `cd /nonexistent && rm -rf *` — `rm` runs in wrong directory.
3. **Parsing `ls` output**: Use `find` or globs instead. `ls` output is for humans.
4. **`read` without `-r`**: Without `-r`, backslashes are treated as escape characters.
5. **`echo` with variables**: `echo "$var"` can misinterpret flags. Use `printf '%s\n' "$var"` for safety.
6. **Forgetting `--` to end options**: `rm -- "$file"` handles filenames starting with `-`.
7. **Testing with `==` in `[`**: POSIX `[` uses `=` for equality, not `==`.

## Google Shell Style Guide (Key Points)

- Use `bash` explicitly. Don't write for `/bin/sh` unless you must.
- Source file names: lowercase, underscores (no dashes in the Google Guide, but dashes are common in executables).
- Functions: lowercase with underscores.
- Constants/environment: UPPERCASE.
- Use `$(command)` not backticks.
- Use `[[ ]]` not `[ ]`.
- `; then` and `; do` on the same line as `if`/`while`/`for`.

---

*Sources: Google Shell Style Guide, Bash Pitfalls (Greg's Wiki), ShellCheck documentation, POSIX Shell specification, Advanced Bash-Scripting Guide, Wooledge BashFAQ*
