# Shell Script Testing & Quality: ShellCheck, BATS, and Common Pitfalls

## Introduction

Shell scripts are notoriously error-prone due to quoting ambiguities, implicit word splitting, and globbing behavior. Quality practices—static analysis via ShellCheck, automated testing via BATS or shunit2, and disciplined quoting—transform shell from "script language people avoid" to "reliable scripting platform." This note covers tools, testing frameworks, CI integration, and the most common pitfalls that ship into production.

## Static Analysis: ShellCheck

### ShellCheck Fundamentals

ShellCheck is a static analyzer for shell scripts. It detects syntax errors, portability issues, quoting mistakes, and logical problems before execution.

```bash
# Basic usage
shellcheck script.sh

# Check all shell scripts
shellcheck *.sh

# Quiet mode: only exit code
shellcheck -f quiet script.sh
echo $?     # 0 if no issues, 1 if problems found

# Output formats
shellcheck -f gcc script.sh             # GCC format (IDE integration)
shellcheck -f json script.sh            # JSON output (programmatic)
shellcheck -f tty script.sh             # Terminal format (colored)
```

### Common ShellCheck Rules

ShellCheck assigns codes to issues. Major categories:

```bash
# SC2086: Double quote to prevent globbing and word splitting
# WRONG: for f in $(ls); do   (expands unquoted)
for f in $(ls)
do
    rm $f      # WRONG: if filename has spaces, rm fails
done

# RIGHT
for f in $(ls)
do
    rm "$f"    # Quote the variable
done

# SC2181: Check exit code of command immediately
# WRONG: pipeline result lost
mysqldump db > dump.sql
if [ $? -ne 0 ]; then   # Only checks if test command succeeded

# RIGHT
if mysqldump db > dump.sql; then
    echo "Backup succeeded"
fi

# SC2046: Quote this expansion
# WRONG: unquoted command substitution expands unpredictably
echo $( echo "hello world" )             # Becomes: echo hello world

# RIGHT
echo "$( echo "hello world" )"          # Preserves spaces

# SC2143: grep without -q (use if [ -z $(grep) ] check)
# INEFFICIENT: grep returns all matching lines, even if you only need existence
if [ -n "$(grep 'pattern' file.txt)" ]; then

# EFFICIENT: grep -q (quiet) returns only exit code
if grep -q 'pattern' file.txt; then

# SC2090: Shells are space-sensitive, word splitting applies here
# WRONG
my_var="hello world"
echo ${my_var}  # Becomes: echo hello world

# RIGHT
echo "${my_var}"

# SC3000–3999: Portability issues (features not in POSIX)
# Warning if script uses bash arrays with #!/bin/sh
declare -A config    # ERROR if /bin/sh; OK if #!/bin/bash

# SC1072: Check file syntax before running
syntax_error
do    # Not properly closed if
```

### Suppressing ShellCheck Warnings

Sometimes warnings are false positives or accepted trade-offs. Suppress locally:

```bash
# Suppress single line
# shellcheck disable=SC2086
for f in $(ls); do
    rm "$f"
done

# Suppress entire script (top of file)
# shellcheck disable=SC2046,SC2181
set -euo pipefail
echo $( echo "hello" )
mysqldump db > dump.sql

# Disable for specific function
# shellcheck disable=SC2214
unsafe_parse() {
    getopts ":" opt
}
```

Configure in `.shellcheckrc` for repository-wide rules:

```bash
# .shellcheckrc
disable=SC2086,SC2046          # Suppress for all files
enable=require-variable-braces # Enable a disabled-by-default rule
```

### ShellCheck in CI

Integrate ShellCheck into your CI pipeline:

```bash
#!/bin/bash
# scripts/lint-shell.sh

exit_code=0

for script in scripts/*.sh; do
    if ! shellcheck -f gcc "$script"; then
        exit_code=1
    fi
done

exit $exit_code
```

GitHub Actions example:

```yaml
# .github/workflows/shellcheck.yml
name: ShellCheck
on: [push, pull_request]

jobs:
  shellcheck:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: ludeeus/action-shellcheck@master
        with:
          scandir: './scripts'
          format: gcc
```

## Automated Testing: BATS (Bash Automated Testing System)

BATS is a test framework for shell scripts. Tests are written in Bash; assertions are functions.

### Basic BATS Test Structure

```bash
#!/usr/bin/env bats
# tests/myapp.bats

# Setup runs before each test
setup() {
    source ./myapp.sh
    export TEST_DIR=$(mktemp -d)
}

# Teardown runs after each test
teardown() {
    rm -rf "$TEST_DIR"
}

# Test case: function name must start with test_
@test "should add two numbers" {
    result=$(add 2 3)
    [ "$result" = "5" ]
}

@test "should exit with error on invalid input" {
    run add "not_a_number" 5
    [ $status -eq 1 ]
}

@test "should create output file" {
    run myapp --output "$TEST_DIR/out.txt"
    [ -f "$TEST_DIR/out.txt" ]
}
```

Running BATS tests:

```bash
# Run all tests
bats tests/*.bats

# Run specific file
bats tests/myapp.bats

# Run specific test
bats tests/myapp.bats --filter "should add"

# Verbose output
bats -v tests/myapp.bats

# Tap format (for CI)
bats --tap tests/myapp.bats
```

### Assertions: bats-assert and bats-support

BATS core provides only `[ condition ]` syntax. Extensions provide richer assertions:

```bash
# Install
git clone https://github.com/bats-core/bats-support tests/test_helper/bats-support
git clone https://github.com/bats-core/bats-assert tests/test_helper/bats-assert

# In your test file
load test_helper/bats-support/load
load test_helper/bats-assert/load

@test "output includes expected text" {
    run myapp
    assert_output --partial "success"    # Output contains "success"
}

@test "file exists and is readable" {
    run myapp --output /tmp/out.txt
    assert_file_exist /tmp/out.txt
}

@test "exit status is zero" {
    run myapp
    assert_success                       # exit status = 0
}

@test "exit status is not zero" {
    run myapp --invalid-flag
    assert_failure 1                     # exit status = 1
}

@test "output matches pattern" {
    run myapp
    assert_output --regexp 'User: \w+'
}

@test "stderr contains error" {
    run myapp 2>&1
    assert_output "Error: invalid"
}
```

### BATS Test Patterns

```bash
# Test cleanup and signal handling
@test "should cleanup on interruption" {
    run bash -c 'source ./myapp.sh; trap_cleanup; cleanup'
    assert_success
}

# Test with fixtures (pre-created test data)
setup() {
    export TEST_DIR=$(mktemp -d)
    echo "test data" > "$TEST_DIR/input.txt"
}

@test "should process input file" {
    run myapp "$TEST_DIR/input.txt"
    assert_success
}

# Test environment variable handling
@test "should use env var if set" {
    export MY_VAR="custom"
    run myapp
    assert_output "MY_VAR=custom"
}

# Test subshell isolation
@test "function should not modify parent environment" {
    x=before
    run bash -c 'source ./myapp.sh; modify_var; echo $x'
    assert_output "before"              # Changed in subshell only
}
```

## Alternative: shunit2 (xUnit for Shell)

shunit2 is lighter-weight than BATS; uses xUnit-style assertions. Good for simpler scripts or embedded testing.

```bash
#!/bin/bash
# tests/test_myapp.sh

# Source the library
. /path/to/shunit2

# Source the app
. ./myapp.sh

# Test function
testAdd() {
    result=$(add 2 3)
    assertEquals "5" "$result"
}

testInvalidInput() {
    add "not_a_number" 5
    assertEquals 1 $?
}

testFileCreation() {
    output=$(mktemp)
    myapp --output "$output"
    assertTrue "[ -f $output ]"
    rm "$output"
}

# Run tests
. shunit2
```

Common shunit2 assertions:

```bash
assertEquals "expected" "$actual"       # String equality
assertNotEquals "not" "$actual"
assertTrue "[ condition ]"              # Condition
assertFalse "[ condition ]"
assertNull "$var"                       # Variable is empty
assertNotNull "$var"
assertSame "value" "$var"               # Same
assertFileExists "/path/to/file"
assertFileNotExists "/path/to/file"
```

## Common Shell Pitfalls and How to Avoid Them

### 1. Word Splitting: Unquoted Variables Expand Unexpectedly

```bash
# WRONG: unquoted variable
files="file with spaces.txt another.txt"
for f in $files; do             # Splits on spaces → 4 iterations
    echo "Processing: $f"
done
# Output: Processing: file
#         Processing: with
#         Processing: spaces.txt
#         Processing: another.txt

# RIGHT: quote the variable
for f in "$files"; do           # Single iteration
    echo "Processing: $f"
done
# Output: Processing: file with spaces.txt another.txt

# RIGHT: use array
files=("file with spaces.txt" "another.txt")
for f in "${files[@]}"; do
    echo "Processing: $f"
done
```

Rule: **Always quote variables** (`"$var"`) unless you explicitly want word splitting.

### 2. Globbing: Unquoted Expansion Matches Filenames

```bash
# WRONG: unquoted variable with glob chars
pattern="*.txt"
rm $pattern         # Expands to all txt files in current directory
                    # If no matches, rm . (deletes current dir!)

# RIGHT: quote to prevent globbing
rm "$pattern"       # Treats as literal string (likely wrong too; better: rm -f *.txt)

# RIGHT: use glob for intended expansion
rm *.txt            # Glob expansion: expands to matching files
```

Rule: **Quote variables to prevent globbing**; use unquoted globs only when intentional expansion is desired.

### 3. Command Substitution Exit Code Loss (without pipefail)

```bash
# WRONG: error in middle of pipeline is silent
result=$(curl "http://bad-url" | grep pattern)

# RIGHT: detect each failure
set -o pipefail         # Bash/zsh only
result=$(curl "http://bad-url" | grep pattern)

# RIGHT: check each step separately for portability
if ! output=$(curl "http://bad-url"); then
    echo "Error: curl failed" >&2
    exit 1
fi

if ! result=$(echo "$output" | grep pattern); then
    echo "Error: pattern not found" >&2
    exit 1
fi
```

Rule: **Use `set -o pipefail`** in bash/zsh; in POSIX scripts, check exit codes explicitly or use separate commands.

### 4. Subshell Variable Scope: Changes Don't Persist

```bash
# WRONG: assignment in subshell
count=0
{
    count=$((count + 1))        # Subshell assignment
} | while read line; do         # Pipe creates subshell
    count=$((count + 1))
done
echo $count                     # Still 0 (changes lost)

# RIGHT: use process substitution or avoid subshells
count=0
while read line; do
    count=$((count + 1))
done < <(command)               # Process substitution (bash/zsh)
echo $count                     # Correct

# RIGHT: use temporary file for cross-subshell communication
count=0
command | while read line; do
    count=$((count + 1))
done
echo "$count" > /tmp/result
count=$(cat /tmp/result)
```

Rule: **Avoid pipes with variable assignment**; use process substitution or separate concerns.

### 5. Quoting and Special Characters

```bash
# WRONG: unquoted dollar signs in single quotes (no expansion happens anyway)
var="hello"
echo '$var'         # Literal: $var (not expanded)

# RIGHT: double quotes for expansion
echo "$var"         # hello

# WRONG: escaped dollar in double quotes
echo "\$var is $var"
echo '$var is $var'

# RIGHT: understand escape semantics
eval "result=$var"  # DANGEROUS: allows code injection
                    # Only use if you control the source

# WRONG: unescaped glob in assignment
filename="*.txt"    # Literal string, not glob

# RIGHT: expansion requires unquoted context
filename=*.txt      # ERROR if multiple files or no match
filename="$(ls *.txt | head -1)"    # Capture first match
```

Rule: **Master quoting rules**: single quotes preserve literally; double quotes allow expansion; bare variables trigger word splitting and globbing.

### 6. Pipefail Edge Cases

```bash
# Pitfall: `set -o pipefail` affects ALL pipelines
set -o pipefail

# This fails if any step fails (intended)
data=$(fetch_api | parse_json | filter)

# This also fails if `true` fails (unintended)
status=$(echo "ok" | true)      # Fails if echo fails
                                 # Silly but affects real pipelines

# Workaround: use local scope or disable temporarily
( set +o pipefail; result=$(command | filter_that_might_fail) )

# Workaround: structure differently
data=$(fetch_api) || exit 1
data=$(echo "$data" | parse_json) || exit 1
data=$(echo "$data" | filter) || exit 1
```

Rule: **Understand `pipefail` semantics**: it makes ALL pipes fail if any command fails, which can have unintended consequences.

## CI Integration: Example GitHub Actions Workflow

```yaml
# .github/workflows/test-shell.yml
name: Shell Tests
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install ShellCheck
        run: apt-get update && apt-get install -y shellcheck
      
      - name: ShellCheck
        run: shellcheck scripts/*.sh
  
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install BATS
        run: |
          git clone https://github.com/bats-core/bats-core /tmp/bats
          cd /tmp/bats && ./install.sh /usr/local
          git clone https://github.com/bats-core/bats-support /tmp/bats-support
          git clone https://github.com/bats-core/bats-assert /tmp/bats-assert
      
      - name: Run tests
        run: |
          mkdir -p tests/test_helper
          cp -r /tmp/bats-support tests/test_helper/
          cp -r /tmp/bats-assert tests/test_helper/
          bats --tap tests/*.bats | tee test-results.tap
      
      - name: Publish results
        if: always()
        uses: dorny/test-reporter@v1
        with:
          name: Shell Tests
          path: test-results.tap
          reporter: 'tap'
```

## Practical Testing Strategy

Prioritize testing by risk:

1. **Core logic**: Test function outcomes (add, parse, transform)
2. **Error paths**: Invalid input, missing files, permission denied
3. **Integration**: End-to-end with real data (but in sandbox)
4. **Cleanup**: Verify temp files are removed, processes exit cleanly

Sample test distribution:

```bash
# 50% of tests: happy path and basic variations
@test "add positive integers" { ... }
@test "add zero" { ... }
@test "add negative integers" { ... }

# 30% of tests: error cases
@test "add non-numeric input" { ... }
@test "add with missing argument" { ... }
@test "add with too many arguments" { ... }

# 15% of tests: edge cases
@test "add maximum values" { ... }
@test "add in subshell" { ... }

# 5% of tests: integration
@test "end-to-end workflow" { ... }
```

## Cross-References

See also: [shell-posix-mastery.md](shell-posix-mastery.md) (exit codes, quoting rules), [shell-bash-advanced.md](shell-bash-advanced.md) (advanced features to test), [shell-zsh-power.md](shell-zsh-power.md) (zsh-specific testing), [cli-ux-engineering.md](cli-ux-engineering.md) (testing interactive CLIs).