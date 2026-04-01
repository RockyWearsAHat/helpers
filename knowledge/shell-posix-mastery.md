# POSIX Shell Mastery: Parameter Expansion, Here-Documents, and Portability

## Introduction

POSIX shell is the intersection of `/bin/sh`, bash, zsh, dash, ksh, and other shells. Understanding POSIX fundamentals ensures scripts run everywhere: embedded systems, containers, macOS, Linux, BSD, and legacy Unix. While bash adds conveniences, POSIX mastery means writing portable logic that survives shell upgrades and environment migrations. This note covers parameter expansion mechanics, quoting rules, here-documents, file descriptors, and the subtle differences in portability across shells.

The core tension: POSIX is conservative (for portability), but shells extend it liberally. Knowing which extensions are available in which shells (via `type -a command`, `command -V`, or `hash -p`) is the practice of disciplined portability.

## Parameter Expansion: The Core Mechanism

Parameter expansion substitutes a variable's value into a word. The syntax is `$parameter` or `${parameter}`. The braces are required when the next character could be part of the parameter name (e.g., `${var}_suffix` vs. `$var_suffix`), and they enable advanced modifiers.

### Basic Expansion and Removal Patterns

```bash
# Basic: substitute the value
var="hello"
echo "$var"                      # hello

# Remove from end (shortest match)
filename="archive.tar.gz"
echo "${filename%.gz}"           # archive.tar
echo "${filename%.*}"            # archive.tar (matches last dot)

# Remove from end (longest match)
echo "${filename%%.*}"           # archive (matches first dot, greedy)

# Remove from start (shortest match)
path="/home/user/file.txt"
echo "${path#*/}"                # home/user/file.txt

# Remove from start (longest match)
echo "${path##*/}"               # file.txt (greedy: strips all leading path)
```

The four removal operators use prefix `%` (from end) or `#` (from start), doubled `%%` or `##` for greedy:

| Form       | Source         | Pattern     | Result              |
| ---------- | -------------- | ----------- | ------------------- |
| `${var%p}` | `/a/b/c`       | `/*`        | `/a/b` (last `/`)   |
| `${var%%p}`| `/a/b/c`       | `/*`        | `` (first `/`, all) |
| `${var#p}` | `/a/b/c`       | `/*`        | `a/b/c`             |
| `${var##p}`| `/a/b/c`       | `/*`        | `c` (greedy)        |

Patterns support `*`, `?`, and `[...]` glob syntax. The behavior is *substring removal without external processes*, critical for performance in loops.

### Default Values and Substitution

```bash
# Use default if unset or empty
name="${1:-unknown}"             # If $1 is unset/empty, use "unknown"

# Use default only if unset (not empty)
name="${1-unknown}"              # If $1 is unset, use "unknown" (empty OK)

# Assign default if unset
name="${1:=unknown}"             # Set $1 to "unknown" if unset/empty, echo result

# Error if unset or empty
name="${1:?'name is required'}"  # Exit with error message if $1 unset/empty
```

The `:` modifier checks both unset *and* empty. Without `:`, only unset. This distinction matters: `foo=""` is set but empty. POSIX does not specify arithmetic beyond `${...}`, but bash extends to `$((expr))` for portability-critical scripts, use `$(( expr ))` only after feature-detecting bash.

### Substring Extraction (Bash, not POSIX)

Bash adds `${var:offset}` and `${var:offset:length}` for extraction:

```bash
var="hello"
echo "${var:1}"       # ello (offset 1)
echo "${var:1:3}"     # ell (offset 1, length 3)
echo "${var: -2}"     # lo (offset -2 from end; space before minus!)
```

Not portable to strict POSIX shells (sh, dash). Use `cut`, `dd`, or `expr` for portability:

```bash
# Portable equivalents
echo "$var" | cut -c 2-          # Extract from char 2 onward
expr substr "$var" 2 3           # Offset 2, length 3
```

## Command Substitution and Quoting

Command substitution captures command output. Two syntaxes exist:

```bash
# Modern (preferred): $( ... )
result=$(echo "hello")

# Legacy: ` ... ` (backticks)
result=`echo "hello"`            # Works but nests poorly
result=`echo "$(echo nested)"`    # Hard to read; need escaping
```

Backticks require escaping inner backticks; `$(...)` nests cleanly. Both are POSIX, but `$(...)` is preferred. Quoting within command substitution is crucial:

```bash
# Unquoted: stdout splitword-splitting and globbing apply
files=$(ls)                      # Becomes individual words
for f in $files; do              # WRONG: already split

# Quoted: preserves literal output
files=$(ls)
for f in "$files"; do            # RIGHT: single argument
```

Variable assignment in subshells does not persist:

```bash
x=1
(x=2)                  # Subshell assignment
echo $x                # Still 1; subshell changes are lost
```

Process substitution (bash/zsh, not POSIX):

```bash
# Bash: <(cmd) creates a file descriptor
diff <(sort file1) <(sort file2)

# POSIX alternative: temporary files
sort file1 > /tmp/f1
sort file2 > /tmp/f2
diff /tmp/f1 /tmp/f2
rm /tmp/f1 /tmp/f2
```

## Here-Documents and Here-Strings

### Here-Documents

A here-document redirects a block of text as stdin to a command. Syntax: `cmd <<DELIMITER`.

```bash
cat <<EOF
This is line one.
This is line two.
The variable $name is substituted.
EOF
```

By default, parameter expansion, command substitution, and arithmetic expansion occur within the here-document. To inhibit expansion, quote the delimiter:

```bash
cat <<'EOF'
The variable $name is NOT substituted.
EOF

# Or escape special characters
cat <<EOF
Literal dollar: \$name
Literal backtick: \`cmd\`
EOF
```

Here-documents are POSIX. Here-strings (bash syntax `cmd <<<string`) are not:

```bash
# Bash/zsh only
cat <<<'hello'

# POSIX alternative: echo piping
echo 'hello' | cat
```

Indenting for readability: prefix `<<-` allows leading tabs (not spaces) in the here-document content and delimiter:

```bash
if true; then
    cat <<-EOF
        Indented for readability.
        Tabs are stripped.
    EOF
fi
```

## File Descriptors and Redirection

### Standard Descriptors and Redirection

Unix provides three standard file descriptors:

| FD  | Name   | Purpose           | Stream |
| --- | ------ | ----------------- | ------ |
| 0   | stdin  | Read from         | input  |
| 1   | stdout | Write to          | output |
| 2   | stderr | Write errors to   | output |

Redirection operators:

```bash
# Redirect stdout to file
cmd > file          # Overwrites file
cmd >> file         # Appends to file

# Redirect stderr to file
cmd 2> file         # Redirect FD 2 (stderr)
cmd 2>> file        # Append stderr

# Redirect both stdout and stderr
cmd > file 2>&1     # Send stderr (2) to where stdout (1) goes
cmd &> file         # Bash shorthand (not POSIX)

# Redirect stdin
cmd < input.txt     # Read stdin from file
cmd <<< 'input'     # Bash here-string (not POSIX)

# Redirect to /dev/null (discard output)
cmd 2>/dev/null     # Suppress error messages
cmd >/dev/null 2>&1 # Discard all output
```

### Duplicating and Closing Descriptors

```bash
# Duplicate FD 2 to FD 1 (send stderr to stdout)
2>&1

# Duplicate FD 1 to FD 3 (make a copy of stdout)
exec 3>&1

# Close FD 2
exec 2>&-           # After this, stderr is closed

# Open a file for reading on FD 3
exec 3< input.txt
read line <&3       # Read from FD 3

# Open a file for writing on FD 4
exec 4> output.txt
echo "data" >&4     # Write to FD 4
```

### Subshells and File Descriptor Leakage

Resource leaks in subshells are common:

```bash
# Resource leak: FD 3 stays open across subshells
exec 3< file.txt
( read x <&3; echo "$x" ) &
# Background subshell keeps FD 3 open; file cannot be deleted on Windows

# Fix: close FD in parent after background job completes
exec 3>&-           # Close explicitly
```

## Traps and Signal Handling

Traps respond to signals and shell events. POSIX defines trap for `EXIT`, `INT`, `TERM`, and error conditions:

```bash
# Set trap: run cleanup on EXIT
cleanup() {
    rm -f /tmp/temp_file
}
trap cleanup EXIT

# Run code on Ctrl+C (SIGINT)
trap 'echo "Interrupted"; exit 130' INT

# Use trap to debug: print each line executing
set -x          # Enable debugging
trap 'echo "Line $LINENO"' DEBUG    # Bash extension

# Common: disable EXIT trap for a subshell
(
    trap - EXIT     # Disable EXIT trap
    some_command
)
```

Signals and their typical numbers:

| Signal | Number | Cause                       | Default       |
| ------ | ------ | --------------------------- | ------------- |
| SIGINT | 2      | Ctrl+C                      | Terminate     |
| SIGTERM| 15     | Termination request         | Terminate     |
| SIGHUP | 1      | Terminal closed             | Terminate     |
| SIGKILL| 9      | Kill (cannot be caught)     | Terminate     |
| SIGSTOP| 19     | Pause (cannot be caught)    | Pause         |

Trap precedence: specific signal > general `EXIT`. Always use `trap - SIGNAL` to *remove* a trap.

## Portability Across Shells

POSIX defines `/bin/sh` as the minimum standard. In practice:

- **bash**: Full superset of POSIX; widely used. May not be available on embedded systems or plan9.
- **dash**: Minimal, POSIX-compliant; used as `/bin/sh` on Debian/Ubuntu. No arrays, no `set -o pipefail`.
- **zsh**: POSIX-compatible but with extensions enabled by default; requires `emulate sh` for strict POSIX behavior.
- **ksh (AT&T)**: Proprietary; POSIX compliant + extensions. Rarely available on modern systems.
- **busybox sh**: Embedded variant; minimal feature set.

Feature detection in portable scripts:

```bash
# Check if array syntax is available
if (eval 'arr=(1 2 3)' 2>/dev/null); then
    arr=(1 2 3)
else
    # Fall back to string-based data
    arr="1 2 3"
fi

# Check for pipefail (bash/zsh only)
if set -o pipefail 2>/dev/null; then
    set -o pipefail
fi

# Use which/command instead of type for portability
if command -v shuf >/dev/null 2>&1; then
    random_line=$(shuf -n1 file)
else
    random_line=$(sort -R file | head -1)
fi
```

## Exit Codes and Error Handling

Exit codes range from 0–255. By convention:

- **0**: Success
- **1**: General error
- **2**: Misuse of shell command
- **126**: Command found but not executable
- **127**: Command not found
- **128+N**: Command terminated by signal N (e.g., 128+15 = 143 for TERM)
- **130**: Script terminated by Ctrl+C (SIGINT=2, so 128+2)

The pipeline exit code (without `pipefail`) is only the last command:

```bash
false | true        # Exit code: 0 (true succeeded)
echo $?             # 0

set -o pipefail
false | true        # Exit code: 1 (one command failed)
echo $?             # 1
```

Without `pipefail`, errors in pipe segments are silent unless checked explicitly:

```bash
cmd1 | cmd2
status={ { $? } }   # Only status of cmd2 is captured
```

Modern practice: `set -euo pipefail` early in scripts to catch errors aggressively (though this is bash/zsh specific; dash does not support `pipefail`).

## Cross-References

See also: [shell-bash-advanced.md](shell-bash-advanced.md) (arrays, coproc, getopts), [shell-zsh-power.md](shell-zsh-power.md) (glob qualifiers, completion), [shell-testing-quality.md](shell-testing-quality.md) (ShellCheck for portability validation).