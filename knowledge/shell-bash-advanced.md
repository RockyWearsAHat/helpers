# Bash Advanced Features: Arrays, Process Substitution, Completion, and Debugging

## Introduction

Bash extends POSIX shell with powerful abstractions: indexed and associative arrays, process substitution, coprocesses, and an extensive completion API. These features enable patterns impossible in POSIX shells but come with trade-offs in portability, performance, and complexity. This note covers when and how to use them effectively, with emphasis on when bash-specific features are worth the portability cost and when POSIX alternatives are more robust.

The central question: is the bash feature solving a real problem, or is it syntactic sugar over string manipulation? Often, the latter suggests sticking with portable alternatives.

## Arrays: Indexed and Associative

### Indexed Arrays

Bash indexed arrays are ordered collections. Elements are accessed by zero-based index.

```bash
# Declare and initialize
numbers=(1 2 3 4 5)

# Elements can be assigned after declaration
numbers[0]=1
numbers[5]=10           # Sparse; indices 1-4 are unset

# Append element
numbers+=(6 7)

# Access single element
echo "${numbers[0]}"                            # 1

# Access all elements (maintains order)
echo "${numbers[@]}"                            # 1 2 3 4 5 10 6 7

# Expansion with quotes: preserves array structure
for n in "${numbers[@]}"; do                    # Each element as separate arg
    echo "$n"
done

# Expansion without quotes: treated as single string (word splitting applies)
for n in ${numbers[@]}; do                      # Same result if elements have no spaces
    echo "$n"
done

# Array length
echo "${#numbers[@]}"                           # 8 elements

# Get keys (indices)
echo "${!numbers[@]}"                           # 0 1 2 3 4 5 6 7
```

Pitfalls with arrays:

```bash
# Cannot pass arrays to functions easily (bash 4.3+ supports nameref, earlier versions don't)
add_element() {
    local -n arr=$1                             # nameref: reference to array
    arr+=(new_element)
}
myarr=(a b c)
add_element myarr                               # Modifies myarr
echo "${myarr[@]}"                              # a b c new_element

# Without nameref: manual workaround using eval (security risk; avoid)
# Arrays pass-by-value in functions; changes inside functions don't persist
array_func() {
    local arr=($@)
    arr+=(modified)
    echo "${arr[@]}"                            # Shows change
}
myarr=(a b c)
array_func "${myarr[@]}"                        # Shows: a b c modified
echo "${myarr[@]}"                              # Still: a b c (unchanged)
```

### Associative Arrays

Associative arrays (hash maps, dictionaries) map keys to values. Require `declare -A`:

```bash
# Declare
declare -A config

# Assign elements
config[host]="localhost"
config[port]="8080"
config[db]="myapp"

# Access by key
echo "${config[host]}"                          # localhost

# Iterate over keys
for key in "${!config[@]}"; do
    echo "$key => ${config[$key]}"
done

# Access all values (order undefined)
echo "${config[@]}"                             # Values in arbitrary order

# Delete key
unset config[port]

# Check if key exists
if [[ -v config[host] ]]; then                  # -v: check variable (or key) exists
    echo "Key exists"
fi
```

Limitations and trade-offs:

- **Portability**: Associative arrays are bash 4+ only. Dash, POSIX sh, and bash 3 don't have them.
- **Performance**: Hash map lookups are O(1) but slower than indexed arrays for small sets.
- **Serialization**: Difficult to pass to functions or store in environment variables without tricks.
- **Alternative pattern**: For small, fixed sets, `case` statements or indexed arrays with parallel indices are more portable.

Example: portable config without associative arrays:

```bash
# Portable: parallel indexed arrays
config_keys=(host port db)
config_vals=(localhost 8080 myapp)

# Access
for i in "${!config_keys[@]}"; do
    key="${config_keys[$i]}"
    val="${config_vals[$i]}"
    echo "$key => $val"
done

# Or use case for fixed keys (most portable)
get_config() {
    local key="$1"
    case "$key" in
        host) echo "localhost" ;;
        port) echo "8080" ;;
        db)   echo "myapp" ;;
    esac
}
```

## Process Substitution: <() and >()

Process substitution connects a process's output to a file descriptor, appearing as a path that can be read from or written to.

```bash
# Output process substitution: <() creates a readable FD
diff <(sort file1) <(sort file2)               # Compares sorted outputs

# Simultaneously run two commands and diff
diff <(ps aux) <(ps aux --forest)          # Compare different views

# Input process substitution: >() creates a writable FD
tee >(gzip > file.gz) >(bzip2 > file.bz2) < input
                                                # Tee input to two compression filters

# Real-world: logging to multiple outputs
{
    echo "stdout line"
    echo "stderr line" >&2
} > >(cat -t)  2> >(cat -t | sed 's/^/[ERR] /')
                                                # Decorate stdout and stderr differently
```

Process substitution is syntactic sugar over file descriptor management:

```bash
# This: diff <(sort file1) <(sort file2)
# Roughly equivalent to:
exec 3< <(sort file1)
exec 4< <(sort file2)
diff /dev/fd/3 /dev/fd/4
```

Advantages:
- Cleaner syntax than manual FD management
- Avoids temporary files
- Parallel execution of multiple processes

Disadvantages:
- Not POSIX; bash/zsh only
- Performance: spawns subshells; slower than equivalent POSIX code for tight loops
- Debugging: harder to inspect file handles

When to use process substitution:
- **Yes**: One-off comparisons, complex pipelines requiring parallel execution
- **No**: Performance-critical loops; use traditional pipes or temp files
- **No**: Embedded systems without bash; use POSIX file descriptor management

## Here-Strings: <<< (Bash Only)

A here-string is a shorthand for sending a string as stdin. Bash only; not POSIX.

```bash
# Bash: here-string
grep "pattern" <<< "$text"

# POSIX equivalent
echo "$text" | grep "pattern"

# Advantage: no subshell overhead (minor in most cases)
# Disadvantage: reader must know bash syntax; less portable
```

Here-strings matter when combined with complex input redirection:

```bash
# Bash: read from variable with here-string
IFS=: read -r user pass <<< "admin:secret123"

# POSIX: echo piping (requires subshell)
echo "admin:secret123" | IFS=: read -r user pass
# Note: in POSIX, read in pipe runs in subshell; vars don't persist
```

In POSIX, use direct assignment or `printf`:

```bash
# POSIX: avoid subshell with printf and IFS manipulation
IFS=: set -- admin secret123           # Parse with set
user="$1" pass="$2"
```

## Coprocesses: coproc

A coprocess is a bidirectional pipe to a subprocess. Bash only; not POSIX.

```bash
# Start coprocess
coproc bc << EOF         # Start bc (calculator) as coprocess
scale=2
EOF

# Write to coprocess's stdin
echo "2 + 3" >&${COPROC[1]}

# Read result from coprocess's stdout
read result <&${COPROC[0]}

echo "$result"           # 5
```

Coprocess automation: useful for long-lived subprocess communication without manual FD management. Rarely needed in modern bash; `expect` or Python's `subprocess.Popen` are better for interactive subprocess control.

```bash
# Manual approach (more control, more verbose)
myprog <&3 >&4 &
PID=$!
exec 3> input
exec 4< output
echo "cmd" >&3
read result <&4
```

## mapfile and readarray: Efficient Line Reading

Traditional line-by-line reading:

```bash
# Slow: each `read` invocation parses input; O(n) calls
while IFS= read -r line; do
    process "$line"
done < file.txt

# Faster: mapfile reads entire file into array in one pass
mapfile -t lines < file.txt             # -t: trim trailing newlines
for line in "${lines[@]}"; do
    process "$line"
done
```

`mapfile` (or its synonym `readarray`) is O(1) relative to line count in modern bash, because it uses internal buffering. For files with millions of lines, the speedup is substantial.

Variations:

```bash
# mapfile with custom IFS and callback
mapfile -d: -t fields < words.txt       # Delimiter ':'
mapfile -C 'callback' < file.txt        # Call function for each line read

# readarray is identical
readarray -t < file.txt                 # Alternative name
```

Trade-off: memory. `mapfile` loads entire file into memory; for gigabyte-scale files, the while loop wins.

## getopts: Argument Parsing

`getopts` is the POSIX-compliant argument parser (bash, dash, zsh all support it). It handles flags like `-f file`, `-v`, `--` (end of flags), and operands.

```bash
#!/bin/bash
verbose=0
file=""

while getopts "vf:" opt; do
    case "$opt" in
        v) verbose=1 ;;
        f) file="$OPTARG" ;;   # OPTARG contains the flag's value
        *) echo "Usage: $0 [-v] [-f file]" >&2; exit 1 ;;
    esac
done

shift "$((OPTIND - 1))"         # Shift out parsed flags; remaining args are operands

echo "Verbose: $verbose, File: $file"
echo "Operands: $@"
```

Syntax: string after `getopts` lists valid flags. `:` after a flag means it requires an argument. `?` means unknown option (handled by `case *`).

Limitations:
- POSIX `getopts` only handles short flags (`-f`), not long flags (`--file`).
- Multi-character flags require manual parsing or third-party tools (e.g., `getopt` from GNU coreutils; not POSIX).

For long flags in portable scripts, use `case` on `$1`:

```bash
while [[ $# -gt 0 ]]; do
    case "$1" in
        -v|--verbose) verbose=1; shift ;;
        -f|--file) file="$2"; shift 2 ;;
        --) shift; break ;;             # End of options
        -*) echo "Unknown option: $1" >&2; exit 1 ;;
        *) break ;;                     # End of flags; start of operands
    esac
done
```

## Bash Completion API

Bash provides a programmable completion system triggered by Tab. Commands can register custom completions.

```bash
# Simple completion: suggest files and directories
complete -f mycommand               # Complete files
complete -d mycommand               # Complete directories
complete -c mycommand               # Complete only commands

# Custom completion function
_myfunc_complete() {
    local cur prev words cword
    COMPREPLY=()
    
    # Current word being completed
    cur="${COMP_WORDS[COMP_CWORD]}"
    
    # Previous word (useful for context: `--flag <TAB>`)
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Generate completions matching cur
    local opts="--verbose --file --help"
    COMPREPLY=( $(compgen -W "$opts" -- "$cur") )
}

# Register the completion function
complete -o bashdefault -o default -o nospace -F _myfunc_complete mycommand
```

Completion debugging:

```bash
# Trigger completion and see what function is called
set -x COMP_DEBUG=1
# Then Tab in interactive bash
```

Completion is intensely interactive and difficult to test in scripts. Typically configured in `.bashrc`:

```bash
# Place completion registration in ~/.bashrc or /etc/bash_completion.d/
source /path/to/completion_functions.sh
```

## Debugging: set -x, PS4, BASH_SOURCE

Interactive debugging vs. non-interactive logging:

```bash
# Enable execution tracing: print each command before execution
set -x

# Customize trace prompt (default is '+ ')
export PS4='[${BASH_SOURCE}:${LINENO}] '

# Execution
echo "hello"           # Prints trace, then command output

# Disable tracing
set +x

# Per-function debugging
debug_func() {
    (set -x; actual_logic)     # Subshell preserves outer set -x state
}
```

Examining call stack:

```bash
# BASH_SOURCE: array of source files for each call frame
# FUNCNAME: array of function names
for i in "${!FUNCNAME[@]}"; do
    echo "Frame $i: ${FUNCNAME[$i]} in ${BASH_SOURCE[$i]}:${BASH_LINENO[$i]}"
done
```

Inspecting variable state at a breakpoint (manual technique):

```bash
# Add a `read` to pause script
echo "Debug: var=$var, arr=${arr[@]}"
read -p "Press Enter to continue: "
```

For automated debugging, use `bash --debugger` (if available in your bash build) or pipe script into a debugger like `bashdb`.

## Trade-Offs: When to Use Bash Features

| Feature              | Use Case                        | Cost             |
| -------------------- | ------------------------------- | ---------------- |
| Arrays               | Data aggregation, small N       | Portability      |
| Assoc arrays         | Configuration, name→value maps  | Bash 4+ only     |
| Process substitution | Parallel streams, no temp files | Complexity       |
| coproc               | Long-lived subprocess interact  | Rarely needed    |
| mapfile              | Million-line files              | Memory per file  |
| getopts              | Argument parsing                | Single-letter    |
| Completion           | Interactive helpers             | Not scriptable   |

## Cross-References

See also: [shell-posix-mastery.md](shell-posix-mastery.md) (parameter expansion, file descriptors), [shell-zsh-power.md](shell-zsh-power.md) (zsh equivalents), [shell-testing-quality.md](shell-testing-quality.md) (testing array-heavy scripts), [cli-ux-engineering.md](cli-ux-engineering.md) (completion integration).