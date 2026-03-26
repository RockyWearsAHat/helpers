# CLI Design Patterns — Argument Parsing, Help Text, I/O & User Experience

## Overview

Command-line interfaces (CLIs) are how developers interact with tools directly. Unlike graphical UIs, CLIs require explicit design for usability: argument structure, error communication, progress feedback, and configuration handling all shape the user experience. Patterns emerged from decades of Unix tool design and are now codified across ecosystems (npm, cargo, Go, Python).

## Argument Parsing Architecture

### Positional Arguments

Positional arguments are values matched by their position in the command line. They convey the primary subject of the command.

```bash
git commit "message"       # message is positional arg 1
cp source.txt dest.txt     # source, dest are positional args 1, 2
docker run image:tag       # image:tag is positional arg 1
```

**Usage:**
- Primary subject of the command (what to operate on)
- Limited to 2–3 arguments before readability breaks
- Order matters; users must remember sequence
- Advantage: concise, unambiguous, familiar to Unix conventions

**Trade-offs:**
- Scalability: 5+ positional args become hard to remember
- Discoverability: `cmd arg1 arg2` reveals nothing about what arg1/arg2 mean
- Flexibility: reordering arguments requires the user to rethink syntax

### Flags (Options)

Flags are named parameters introduced by `-` (short form) or `--` (long form). They modify behavior but are not required to operate.

```bash
git clone --depth 1 https://repo.git    # --depth modifies clone behavior
npm install --save-dev package-name     # --save-dev flags behavior
ls -la                                  # -l and -a are short-form flags
```

**Short flags:**
- Single dash + single character: `-v`, `-o`, `-x`
- Can be grouped: `-la` = `-l -a`
- Memorable for frequent users
- Convention: `-v` for verbose, `-o` for output, `-h` for help

**Long flags:**
- Double dash + descriptive name: `--verbose`, `--output`, `--recursive`
- Self-documenting; easier for new users
- Help text can attach directly: `--output=file.txt` or `--output file.txt`
- Convention: full English words, kebab-case

**Flag patterns:**
- Boolean: `--verbose` (present = true, absent = false)
- With value: `--output file.txt` or `--output=file.txt`
- Repeatable: `--include pattern1 --include pattern2`
- Variadic: `--file a.txt --file b.txt --file c.txt`

### Subcommands

Subcommands organize a tool's functionality into logical groups. They split a single large command into a hierarchy.

```bash
git config --list          # config is a subcommand
git config user.name       # subcommand + positional args
npm install package-name   # install is a subcommand
npm audit fix              # audit is a subcommand; fix is nested
kubectl apply -f file.yaml # apply is a subcommand
```

**Grammar:**
```
command subcommand [args] [flags]
kubectl apply -f deployment.yaml
^       ^      ^  ^
|       |      |  positional/flag combo
|       |      flag
|       subcommand
command
```

**Design principles:**
- Use when 5+ major actions exist (git: config, init, clone, commit, push, pull, etc.)
- Nest to max 2 levels (git has config + config subkeys, not git config user name)
- Keep subcommand names consistent: `list`, `add`, `delete` across the tool
- Reuse verbs: `npm install`, `npm update`, `cargo build`, `cargo test`

## Help Text and Documentation

### The help (−h, −−help) Convention

Help must be immediate, accurate, and discoverable. Unix convention:

```bash
command -h              # short form, often works in older tools
command --help          # long form, standard
command help            # sometimes a subcommand (git help commit)
```

**Output format:**
```
USAGE:
  command [OPTIONS] [ARGS]

DESCRIPTION:
  Explain what this command does in one sentence.
  Optionally add context: when to use it, what it requires.

OPTIONS:
  -v, --verbose        Increase logging verbosity
  -o, --output <FILE>  Write output to FILE (default: stdout)
  -h, --help           Show this message

EXAMPLES:
  command --verbose input.txt
  command -o output.txt data.csv
```

**Principles:**
- Fit help in one terminal height (60–80 lines max)
- Start with a one-liner description
- List options in alphabetical order or by frequency
- Use aligned columns for readability
- Always include examples
- Never hide help behind a GUI

### Man Pages

Man pages are the formal documentation for CLI tools. Format: name(1) for commands, name(5) for file formats.

```bash
man git       # displays git.1 (the main page)
man 5 passwd  # displays passwd.5 (file format documentation)
```

**Sections:**
- NAME: one-line description
- SYNOPSIS: usage grammar
- DESCRIPTION: detailed explanation
- OPTIONS: flag reference
- EXAMPLES: realistic use cases
- ENVIRONMENT: env var dependencies
- EXIT STATUS: what error codes mean
- SEE ALSO: related commands

**Format:** Groff/mandoc syntax. Most modern tools generate man pages from help text (using tooling like `clap`, `docopt`, `cobra`).

## Streams and Redirection

CLIs operate on three standard streams:

| Stream     | FD | Purpose | Convention |
|------------|----|---------|----|
| stdin      | 0  | Input from user/pipe | `-` often means "read stdin" |
| stdout     | 1  | Normal output | Output data, results, progress |
| stderr     | 2  | Errors and diagnostics | Warnings, errors, debug logs |

**Pattern:**
```bash
# Chain commands via pipes (stdout → stdin)
cat data.txt | grep pattern | sort | uniq

# Redirect output
command > file.txt         # stdout → file
command 2> errors.txt      # stderr → file
command &> combined.txt    # both → file

# Read from stdin
cat < input.txt            # equivalent to cat input.txt
command -o - data.txt      # write to stdout explicitly
```

**Design choices:**
- If no arguments, read from stdin (following Unix philosophy)
- Use `-` as a flag to explicitly request stdin/stdout: `tar -czf - file.tar.gz` writes compressed output to stdout
- Separate signal/progress output (stderr) from results (stdout) so pipes work

## Exit Codes (Status Codes)

Exit codes communicate success or failure to the shell and parent processes:

```bash
command
echo $?      # prints the exit code (0–255)
```

**Standard conventions:**

| Code | Meaning | Usage |
|------|---------|-------|
| 0    | Success | Program completed without error |
| 1    | General error | Catch-all for unspecified failure |
| 2    | Misuse of shell command | Bad arguments, invalid flags |
| 126  | Permission denied | File exists but not executable |
| 127  | Command not found | Shell couldn't find the binary |
| 128  | Invalid exit code | Code outside 0–255 range |
| 128+N| Signal termination | Process killed by signal N (128+9=137 for SIGKILL) |

**Design:**
- Always exit 0 on success
- Use 1 for expected errors (file not found, validation failed)
- Reserved codes (126, 127, 128+N) for system-level issues; avoid them
- Don't use codes > 1 unless you need to distinguish multiple failure types (rare; use logging instead)
- Document your exit codes in the man page

**Scripting pattern:**
```bash
if command --validate; then
  command --execute
  exit $?
else
  echo "Validation failed" >&2
  exit 1
fi
```

## Output and Interactivity

### ANSI Escape Codes (Color, Formatting)

ANSI escape sequences add color and styling to terminal output. Format: `\033[<code>m` or `\x1b[<code>m`.

| Code | Effect | Example |
|------|--------|---------|
| 0    | Reset (turn off all styles) | `\033[0m` |
| 1    | Bold | `\033[1m` |
| 31   | Red foreground | `\033[31m` |
| 32   | Green foreground | `\033[32m` |
| 33   | Yellow foreground | `\033[33m` |
| 34   | Blue foreground | `\033[34m` |
| 90–97| Bright colors | `\033[90m` (bright black) |

**Pattern:**
```bash
# Bash: use $'...' string syntax or echo -e
echo -e "\033[1;32mSUCCESS\033[0m"    # bold green
echo $'Output: \033[33mwarning\033[0m'

# Programmatic in JS, Python, etc.
console.log('\x1b[1;31mERROR\x1b[0m')
print(f'\033[32m{message}\033[0m')
```

**When to use:**
- Errors in red: `\033[31m`
- Success in green: `\033[32m`
- Warnings in yellow: `\033[33m`
- Muted/debug in dim gray: `\033[90m`

**Important:** Always detect if stdout is a terminal (isatty). Color in redirected output is noise:

```bash
if [ -t 1 ]; then   # is stdout a terminal?
  echo -e "\033[32mGREEN\033[0m"
else
  echo "GREEN"      # no ANSI codes to files
fi
```

### Progress Indicators

Long-running operations need user feedback. Patterns:

**Spinner:** Indicates work in progress; no quantifiable progress.
```
⠋ Fetching...
⠙ Fetching...
⠹ Fetching...
```

**Progress bar:** Shows percentage complete.
```
[████████░░] 80%
[████────────] 40%
```

**Percent + ETA:** Useful for network operations.
```
Downloading: 45% (2.1 MB / 4.7 MB) [ETA: 3s]
```

**Logging steps:** For discrete operations, log each step:
```
Building...
  ✓ Compiling source
  ✓ Running tests
  ✓ Generating docs
```

**Design principle:** Update in-place (carriage return `\r`) rather than spamming the terminal with new lines.

### Interactive Prompts

When input is required, prompt the user:

```bash
# Simple prompt
read -p "Continue? [y/N]: " response
if [[ "$response" =~ ^[Yy]$ ]]; then
  execute_action
fi

# Password input (no echo)
read -sp "Password: " password

# Selection prompt (checklist)
# Use libraries like fzf, select, or whiptail
select option in "Option A" "Option B" "Abort"; do
  break
done
```

**Principles:**
- Default to "No" for destructive operations: `[y/N]`
- Default to "Yes" for safe operations: `[Y/n]`
- Show defaults in brackets
- Never prompt in a pipeline (violates Unix philosophy)
- If automated (non-interactive), use flags: `--force`, `--assume-yes`

## Configuration Files

CLIs accept persistent configuration via files. Conventions:

**Locations (by precedence):**
1. Explicit flag: `--config file.yaml`
2. Current directory: `.mytoollrc`, `myconfig.toml`
3. User home: `~/.mytoolvrc`, `~/.config/mytool/config.yaml`
4. System-wide: `/etc/mytool/config.yaml`

**Format:**
- YAML: human-readable, common in tools like aws-cli, Kubernetes
- TOML: simple, increasingly popular (Rust ecosystem)
- JSON: universal but verbose
- INI: legacy but lightweight
- Declarative: only config, no logic

**Pattern:**
```yaml
# ~/.config/mytool/config.yaml
verbose: true
output_format: json
timeout: 30
plugins:
  - name: plugin1
    enabled: true
```

**Merging strategies:**
- System defaults → system config → user config → project config → flags
- Flags always override config files
- Document the merge order in help text

## Summary of Conventions

| Aspect | Convention | Example |
|--------|-----------|---------|
| Boolean flags | `--flag` (present = true) | `--verbose`, `--force` |
| Flags with values | `--flag value` or `--flag=value` | `--output file.txt` |
| Short flags | `-v`, `-x`, group as `-vx` | `ls -la` |
| Long flags | `--very-descriptive-name` | `--recursive`, `--dry-run` |
| Help | `--help`, `-h` | `git --help` |
| Version | `--version`, `-V` | `npm --version` |
| Exit codes | 0 = success, 1 = error | `echo $?` |
| Stdin | `-` as filename or no file arg | `cat -`, `git apply -` |
| Stderr | Diagnose messages only | Errors, warnings, debug logs |
| Colors | Detect terminal; default off in pipes | `\033[31m` for red |
| Config | Home dir + project dir + flags | `~/.config/app/config.yaml` |

## Trade-offs and anti-patterns

**Too many subcommands:** If a tool grows beyond 10–15 subcommands, it's time to split into separate commands or reconsider UX.

**Inconsistent naming:** Mixing `--verbose`, `--debug`, `-D` for similar purposes creates confusion. Pick a pattern and stick to it.

**Help that's too brief:** Empty descriptions like `-f flag f` teach nothing. Every option should have a one-sentence explanation.

**Ignoring the default:** If the common case is `--force`, make it the default and add `--no-force` for the override.

**Piping and prompts:** Prompts block pipes. Use `--assume-yes` / `--force` flags to make tools scriptable.

**Undocumented exit codes:** If your tool uses exit code 42 for a specific error, document it. Grep scripts and integrations depend on predictable codes.