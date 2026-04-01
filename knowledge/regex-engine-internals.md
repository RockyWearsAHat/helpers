# Regex Engine Internals — NFA, DFA, Thompson, Backtracking, ReDoS, Atomic Groups

Regular expression engines convert pattern strings into state machines that match input efficiently. Different engines use fundamentally different strategies with massive performance implications.

## NFA vs DFA: The Core Divide

**NFA (Nondeterministic Finite Automaton)**: From a state, a single input symbol can transition to multiple states simultaneously. Computing union naturally.

```
Pattern: (a|b)*c

State 0 --a--> State 1 \
        --b--> State 2 --> State 3 --c--> State 4 (accept)
```

At each step, track all **possible** states the machine could be in. If any path accepts, pattern matches.

**DFA (Deterministic Finite Automaton)**: From a state, each input symbol transitions to exactly one next state.

Converting NFA to DFA requires **powerset construction**: each DFA state represents a set of NFA states. Trade-off: DFA is larger (exponentially in worst case) but matching is linear.

**Matching complexity**:
- **NFA with backtracking**: O(2^n) worst case (exponential in pattern size)
- **DFA**: O(n) linear time (no backtracking)
- **Practical NFA**: Often O(n) on typical inputs; pathological cases are rare

## Thompson's Construction: NFA from Regex

Algorithm to build NFA directly from regex syntax. Recursive: combine smaller NFAs with epsilon (ε) transitions.

**Rules** (showing transitions as arrows):

1. **Empty string**: ε-transition directly to accept state
   ```
   → ε → ◯
   ```

2. **Single character**: transition on character
   ```
   → —a→ ◯
   ```

3. **Concatenation s·t**: Final state of s becomes initial state of t
   ```
   (NFA s) → (NFA t)
   ```

4. **Alternation s|t**: ε-branch to either s or t, then ε-merge to final
   ```
         → (NFA s) ↘
   → ε →              → ◯
         → (NFA t) ↗
   ```

5. **Kleene star s***: Loop back from final to initial via ε, allow skip
   ```
       ↻ ε
   ↗ (NFA s) ↖
   → ε       ε → ◯
   ```

**Properties**:
- States: O(m) where m = regex length
- Transitions: At most 2 per state (one ε-optionally)
- Matching: O(n·m) with NFA interpreter (n = input length)

**Implementation**: Use explicit transition table or convert to DFA.

## Backtracking: How Most Engines Work

Perl, Python, Ruby, Java, JavaScript, Go's regex: All use **NFA with backtracking**.

```
Pattern: (a+)+b
Input: aaac

State machine explores:
  Try a+ greedily: aaaa
    Try next a: fail, backtrack
    Try b: fail, backtrack to previous a
  Try a+ less greedily: aaa
    Try b: fail, backtrack
  ... exponential exploration in worst case
```

**Simple algorithm** (pseudocode):

```
function match(pattern, input, pos_in_pattern, pos_in_input):
    if pos_in_pattern == END:
        return pos_in_input  # success
    if current_pattern is character:
        if pos_in_input < input.length and input[pos_in_input] == current_pattern:
            return match(pattern, input, pos_in_pattern+1, pos_in_input+1)
        else:
            return FAIL
    if current_pattern is alternation (a|b):
        result = match(pattern, input, pos_in_pattern[a], pos_in_input)
        if result != FAIL:
            return result
        return match(pattern, input, pos_in_pattern[b], pos_in_input)
    # ... etc for *, +, lookahead
```

**Why backtracking**: Supports backreferences (`\1`, `\2`) and lookahead (`(?=...)`). NFAs cannot express these features directly.

**Performance**: Greedy quantifiers try longest match first; backtrack if later parts fail. Second-guessing choices explodes in nested patterns.

## Catastrophic Backtracking (ReDoS)

Pathological patterns cause exponential backtracking on non-matching input.

**Classic example**:
```
Pattern: (a+)+b
Input: aaaaaaaaac     (no 'b' at end)
```

Parser tries: `a* a*` = (1)(8), (2)(7), (3)(6), (4)(5), (5)(4), ... = 2^8 combinations

Each failed attempt backtracks, recombining groupings.

**Real-world examples**:
- Email validation: `([\w\.-]+)*@[\w\.-]+` on non-matching email
- URL parsing: `(https?|ftp)://.*` with complex subpatterns
- Zip codes: `(\d{5})*-` on long numeric strings

**Time complexity**: O(2^n) where n = input length. Input of 20-30 chars can freeze.

**Detection**:
- Nested quantifiers: `(a+)+`, `(a*)*`, `(a+)*`
- Alternation with overlap: `(a|a)*`, `(a|ab)*`
- Groups with backtracking: `(.*)*`

## Possessive Quantifiers & Atomic Groups

**Possessive quantifiers** (PCRE, Java, .NET): `*+` `++` `?+` `{n,m}+`

Once matched, **never backtrack** into the quantifier.

```
Pattern: (a++)b
Input: aaac

a++ matches aaac greedily, then refuses to backtrack.
b fails. Pattern fails immediately.
No exponential exploration.
```

**Atomic groups** `(?>...)`: Similar — once matched, no backtracking into group.

```
Pattern: (?>a+)b
Input: aaac

(?>a+) matches aaa, locks it.
b fails to match c.
No backtracking into group content.
```

**Trade-off**: Lose some matches but prevent ReDoS. Careful pattern design required.

## Lookahead & Lookbehind

Regex supports **lookahead** and **lookbehind** predicates without consuming input.

```
Positive lookahead (?=...):
    Match 'a' only if followed by 'b', but don't consume 'b'
    Pattern: a(?=b) matches "ab" but only consumes 'a'

Negative lookahead (?!...):
    Match 'a' only if NOT followed by 'b'
    Pattern: a(?!b) matches "ac" but not "ab"

Positive lookbehind (?<=...):
    Match 'a' only if preceded by 'b' (not consuming 'b')
    Pattern: (?<=b)a matches "ba" but only consumes 'a'

Negative lookbehind (?<!...):
    Match 'a' only if NOT preceded by 'b'
    Pattern: (?<!b)a matches "ca" but not "ba"
```

**Implementation complexity**: Backtracking engines implement these via explicit lookahead/lookbehind checks. DFA engines cannot support them (context-free extension).

**Performance issue**: Lookahead/lookbehind with quantifiers can amplify backtracking:
```
Pattern: (?=a+)a+b  (look for a+ followed by a+b)
Nested lookahead + quantifiers = bad news
```

## RE2: The Linear-Time Engine

Google's RE2 library (Go, C++) uses **DFA only** — guarantees linear time, no backtracking.

**Constraints** (vs PCRE):
- No backreferences: `\1`, `\2` disabled
- No lookahead/lookbehind: `(?=...)`, `(?<=...)` disabled
- No alternation inside groups that matter: `(a|b){n,m}` works, but backtracking prevented
- No atomic groups or possessive quantifiers: unnecessary

**Implementation**: Thompson's construction → NFA → DFA, with smart state minimization.

**Guarantee**: Matching is O(n) where n = input length. No surprise hangs.

**Trade-off**: Simpler patterns, faster, safer (no ReDoS), but less powerful.

## Optimization Techniques

**Compiled patterns**: Engines ahead-of-time compile regex to bytecode or machine code, cached across uses.

Every `re.match()` call parses pattern again if not cached. Production code should compile once:

```python
# Bad
for line in lines:
    if re.search(r'\d{3}-\d{4}', line):  # Recompile every iteration
        ...

# Good
pattern = re.compile(r'\d{3}-\d{4}')
for line in lines:
    if pattern.search(line):  # Reuse compiled
        ...
```

**Anchors & prefixes**: Patterns starting with `^` or literal string can skip false positions.

```
Pattern: ^const\s+\w+
Engine note: No match possible before 'c' in input
Early termination if first chars don't match
```

**Finite lookahead**: Engines with bounded lookahead avoid worst-case backtracking.

**Memoization**: Track attempted (State, InputPos) pairs; avoid redundant computation.

## Unicode & Regex

**Character class complexity**:
- ASCII: `[a-z]` = 26 states
- Unicode: `\p{Letter}` = 100,000+ code points

**Grapheme clusters**: User-perceived character (e.g., 👨‍👩‍👧‍👦) = multiple code points. Most regex engines don't understand; match individual code points, not clusters.

**Normalization**: `café` = U+00E9 or U+0065 U+0301?

```
\p{Letter}+ matches differently based on normalization form

Most engines do byte-wise matching:
  NFC (single codepoint): Matches as 1
  NFD (decomposed): Matches as 2 separate codepoints
```

## Regex Engine Comparison

| Engine         | Type      | Backrefs | Lookahead | ReDoS Risk | Use Case                |
| -------------- | --------- | -------- | --------- | ---------- | ----------------------- |
| Perl           | NFA       | Yes      | Yes       | High       | Text processing, scripting |
| Python         | NFA       | Yes      | Yes       | High       | Data wrangling           |
| PCRE           | NFA       | Yes      | Yes       | High (but possessive fix) | PHP, R, many tools |
| Go regexp      | DFA       | No       | No        | None       | Safe servers            |
| RE2 (C++/Go)   | DFA       | No       | No        | None       | Denial-of-service safety |
| JavaScript     | NFA       | Yes      | Yes       | High       | Browsers, Node.js       |
| Java           | NFA       | Yes      | Yes       | High       | Enterprise systems      |

## Writing Safe Regexes

1. Flatten nested quantifiers: `(a+)+` → `a+`, `(a*)*` → `a*`
2. Avoid alternation with overlap: `(cat|ca)` → `ca(?:t|$)`
3. Be specific on character classes: `.*` → `[^\n]*` (exclude newline)
4. Use atomic groups on risky sections: `a++` in place of `a+` if backtracking unwanted
5. Test on long non-matching inputs: `timeit.timeit(lambda: re.search(...), number=1000)`
6. Prefer exact matches over regex: `if 'needle' in haystack:` beats `re.search(r'needle', haystack)`

## See Also

- [regex-patterns.md](regex-patterns.md) — Syntax, patterns, common pitfalls
- [algorithms-string.md](algorithms-string.md) — String matching algorithms (KMP, Rabin-Karp, suffix trees)
- [math-automata-computability.md](math-automata-computability.md) — NFA/DFA theory, language hierarchy
- [compiler-design-frontend.md](compiler-design-frontend.md) — Lexical analysis uses NFAs/DFAs