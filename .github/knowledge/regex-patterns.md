# Regular Expressions - Patterns & Pitfalls

## Engine Types

- **NFA (backtracking)**: Perl, Python, Ruby, Java, JavaScript, .NET, Go. Most common. Supports backreferences, lookaround.
- **DFA (Thompson)**: grep, awk, RE2 (Go's regexp). Linear time guarantee. No backreferences.
- **PCRE**: "Perl Compatible Regular Expressions" — the de facto standard. Used by PHP, R, many tools.

## Syntax Quick Reference

### Character Classes

```
.        Any character (except newline, unless DOTALL/s flag)
\d       Digit [0-9] (most engines; Unicode-aware in Python 3, Java)
\D       Non-digit
\w       Word char [a-zA-Z0-9_] (Unicode-aware in some engines)
\W       Non-word char
\s       Whitespace [ \t\n\r\f\v] (plus Unicode spaces in some engines)
\S       Non-whitespace
[abc]    Character class — a, b, or c
[^abc]   Negated class — not a, b, or c
[a-z]    Range
[a-zA-Z] Multiple ranges
[\s\S]   Any character INCLUDING newline (useful when . doesn't match \n)
```

### Anchors

```
^        Start of string (or line with MULTILINE/m flag)
$        End of string (or line with MULTILINE/m flag)
\b       Word boundary
\B       Non-word boundary
\A       Start of string (always, ignores MULTILINE)
\Z       End of string (always, ignores MULTILINE)
```

### Quantifiers

```
*        0 or more (greedy)
+        1 or more (greedy)
?        0 or 1 (greedy)
{n}      Exactly n
{n,}     n or more
{n,m}    Between n and m (inclusive)
*?       0 or more (lazy/non-greedy)
+?       1 or more (lazy)
??       0 or 1 (lazy)
*+       0 or more (possessive — no backtracking, PCRE/Java)
```

### Groups & Backreferences

```
(abc)       Capturing group
(?:abc)     Non-capturing group
(?<name>x)  Named group (Python: (?P<name>x))
\1          Backreference to group 1
(?=abc)     Positive lookahead
(?!abc)     Negative lookahead
(?<=abc)    Positive lookbehind
(?<!abc)    Negative lookbehind
(?>abc)     Atomic group (no backtracking into group)
```

## Common Patterns Cookbook

### Email (Simplified, RFC 5322 compliant is 6000+ chars)

```regex
^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$
```

**Reality check:** True RFC 5322 email validation is nearly impossible with regex. Validate format loosely, then send a confirmation email.

### URL

```regex
https?://[^\s/$.?#].[^\s]*
```

### IPv4 Address

```regex
\b(?:(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]?\d)\b
```

### Date (YYYY-MM-DD)

```regex
\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])
```

### Phone Number (North American)

```regex
(?:\+?1[-.\s]?)?\(?[2-9]\d{2}\)?[-.\s]?\d{3}[-.\s]?\d{4}
```

### Password Strength (min 8, upper, lower, digit, special)

```regex
^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$
```

### Trim Whitespace

```regex
^\s+|\s+$
```

### Duplicate Words

```regex
\b(\w+)\s+\1\b
```

### HTML Tag (don't parse HTML with regex — but for simple extraction)

```regex
<([a-zA-Z][a-zA-Z0-9]*)\b[^>]*>(.*?)</\1>
```

### UUID

```regex
[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}
```

### Semantic Version

```regex
(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[\da-zA-Z-]+(?:\.[\da-zA-Z-]+)*)?(?:\+[\da-zA-Z-]+(?:\.[\da-zA-Z-]+)*)?
```

## Dangerous Patterns — ReDoS (Catastrophic Backtracking)

### The Problem

NFA engines backtrack exponentially on certain patterns with ambiguous quantifiers:

```regex
# DANGEROUS — exponential backtracking on failure
(a+)+$           # Nested quantifiers
(a|aa)+$         # Overlapping alternatives
(.*a){10}        # Greedy .* with repetition
```

**Attack:** Input `aaaaaaaaaaaaaaaaaaaaaaaaaaa!` — the engine tries every possible way to split `a`s between the inner and outer `+` before failing at `!`.

### How to Avoid

1. **No nested quantifiers** on overlapping patterns: `(a+)+` → `a+`
2. **Use possessive quantifiers**: `(a++)` (PCRE/Java) — no backtracking
3. **Use atomic groups**: `(?>a+)` — same effect
4. **Use RE2/DFA engines** for untrusted input (Go, Rust `regex` crate)
5. **Set timeouts** on regex execution (Python `regex` module, .NET `RegexOptions.Timeout`)
6. **Test with tools**: [regex101.com](https://regex101.com) shows step count

### Safe Alternatives

```regex
# Dangerous          # Safe equivalent
(a+)+$              → a+$
(a|b)*              → [ab]*
(x+x+)+y           → x{2,}y
(\w+\.)*\w+@       → [\w.]+@ (if precision allows)
```

## Engine-Specific Gotchas

### JavaScript

- No lookbehind before ES2018 (`(?<=...)` and `(?<!...)`)
- No possessive quantifiers or atomic groups
- `/u` flag required for proper Unicode matching
- `/s` flag (dotAll) added in ES2018 — before that, `.` never matched `\n`
- `\d` matches only `[0-9]`, never Unicode digits
- Named groups: `(?<name>...)`, backreference `\k<name>`

### Python

- `re` module: backtracking NFA, no possessive/atomic
- `regex` module (pip): possessive quantifiers, atomic groups, fuzzy matching
- `(?P<name>...)` for named groups (non-standard syntax)
- `re.VERBOSE` / `re.X` for comments and whitespace
- `re.compile()` caches, but Python also internally caches recent patterns

### Go

- Uses RE2 (linear time, no backtracking)
- No backreferences, no lookahead/lookbehind
- `(?i)` inline flag for case-insensitive
- Returns `[]byte` by default; use `FindString` variants for strings

### Rust

- `regex` crate: RE2-like, linear time, no backreferences
- `fancy-regex` crate: adds backreferences and lookaround (with backtracking)
- Unicode-aware by default; `(?-u)` to restrict to ASCII

### Java

- `\b` is Unicode-aware
- Possessive quantifiers: `a++`, `a*+`, `a?+`
- Named groups: `(?<name>...)`, backreference `\k<name>`
- `Pattern.UNICODE_CHARACTER_CLASS` for Unicode \w, \d, etc.

## Performance Tips

1. **Anchor when possible**: `^pattern` lets the engine skip positions
2. **Prefer character classes over alternation**: `[aeiou]` not `a|e|i|o|u`
3. **Put common alternatives first**: `(common|rare)` not `(rare|common)`
4. **Use non-capturing groups** when you don't need the capture: `(?:...)`
5. **Avoid `.* ` at the start** — it forces the engine to try every position
6. **Compile and reuse** — don't recompile in loops
7. **Use lazy quantifiers** when you want the shortest match: `.*?`

## Regex in Practice — One-Liners

### sed (stream editor)

```bash
# Replace first occurrence per line
sed 's/old/new/' file.txt

# Replace all occurrences (global)
sed 's/old/new/g' file.txt

# In-place edit (macOS needs -i '')
sed -i '' 's/old/new/g' file.txt
```

### grep

```bash
# Extended regex (-E, no need to escape +, ?, etc.)
grep -E 'pattern1|pattern2' file.txt

# Perl-compatible regex (-P, Linux only, not macOS grep)
grep -P '(?<=prefix)\w+' file.txt

# Count matches
grep -c 'pattern' file.txt

# Only matching part
grep -o 'pattern' file.txt
```

### Common Shell Regex Tasks

```bash
# Extract IPs from a log
grep -oE '\b([0-9]{1,3}\.){3}[0-9]{1,3}\b' access.log

# Find TODO/FIXME comments
grep -rn 'TODO\|FIXME\|HACK\|XXX' src/

# Validate that a variable looks like a version
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] && echo "valid"
```

---

_Tip: Always use https://regex101.com for testing — it shows the regex engine's step count, which reveals catastrophic backtracking before it hits production._
