# Compilers — Lexical Analysis & Tokenization

## Overview

**Lexical analysis** (or "scanning" / "lexing") is the first phase of compilation. The lexer reads source text as a sequence of characters and produces a stream of **tokens** — the atomic units of meaning. Tokens are the vocabulary of a language: keywords, identifiers, literals (strings, numbers), operators, punctuation.

The lexer's role is to:
- Group characters into meaningful units
- Discard whitespace and comments (usually)
- Identify token types and values
- Report error locations with line/column information
- Separate concerns: parsing works with tokens, not raw character streams

Without lexing, a parser would need to handle character-level details. By tokenizing first, parsing logic stays cleaner.

## Regular Expressions to Finite Automata

Lexers recognize tokens using **regular expressions (regex)** — patterns that describe sets of strings. A regex like `[0-9]+` matches one or more digits.

Since computers can't "understand" regex directly, they must be compiled into **finite automata**:

### NFA: Nondeterministic Finite Automaton
- Multiple transitions on the same input symbol
- Epsilon (empty) transitions allowed
- Easier to construct from regex

### DFA: Deterministic Finite Automaton
- Single transition per state/symbol
- No epsilon transitions
- Must simulate one state at a time
- Efficient to execute

**Thompson's Construction** converts a regex to an NFA:
- Each regex operator (concatenation, alternation `|`, Kleene star `*`) maps to NFA fragment rules
- Fragments are composed incrementally
- Result: an NFA with O(n) states for O(n)-length regex

**Subset Construction** converts NFA to DFA:
- Each DFA state represents a set of NFA states
- Reachable NFA state sets become DFA states
- Can cause exponential blowup: regex → NFA → DFA can go from n states to 2^n
- In practice, subset of reachable states is usually manageable

**Hopcroft-Karp Minimization** reduces a DFA:
- Merges equivalent states (states with identical transition behavior)
- Further state reduction for smaller tables

## Maximal Munch (Longest Match)

A critical lexer decision: which token matches? Multiple regexes may match a prefix of the input.

**Maximal munch rule**: consume the longest possible match. Example:
```
Token: NUMBER [0-9]+
Token: IDENTIFIER [a-zA-Z_[a-zA-Z0-9_]*
Input: if

Partial match at 'i': IDENTIFIER matches 'i'
Partial match at 'if': IDENTIFIER matches 'if'
Maximal munch: take 'if' → keyword IF (if keywords are checked after ID)
```

If no token matches at a position, the lexer is stuck. Typically, a default rule (catch-all) or error token handles this.

**Tie-breaking**: when multiple regexes match the same length:
- Earlier rule wins (by definition order in the lexer spec)
- Common approach: keywords listed as a table, not regexes, checked *after* identifier match

## Keywords vs Identifiers

A naive approach: define `KEYWORD` as `if | else | while | ...` in the regex, and `IDENTIFIER` as `[a-zA-Z_][a-zA-Z0-9_]*`. But keywords form a long list, and new keywords might be added. This is inefficient and error-prone.

**Better approach**: match all word-like tokens as `IDENTIFIER`, then check against a keyword table:

```
Token pattern: [a-zA-Z_][a-zA-Z0-9_]*
  → Lexer produces IDENTIFIER("if")
  → Post-lexing check: is "if" a reserved word?
  → If yes, return IF token. If no, return IDENTIFIER.
```

This decouples the lexer spec from keyword list and makes adding language features easier.

## Whitespace & Comment Handling

**Whitespace** (spaces, tabs, newlines) typically has no meaning (ignoring significant indentation languages like Python). The lexer:
- Matches whitespace with a regex like `[ \t\n]+`
- Discards the matched text (no token produced)
- Tracks line/column for error reporting

**Comments** are similar: matched and discarded. Multi-line comments require care:
```
Comment: /* ... */
  NFA: / followed by * followed by ... followed by * followed by /
  
But */ can overlap across different comments if naively designed.
Solution: explicit multi-char lookahead or a simple state machine.
```

Some languages track newlines inside comments for error location reporting (though the token itself is discarded).

## String Literal Scanning

Strings are trickier than simple regexes:

```
String: "..." or '...'
  Naively: \"[^\"]*\"
  But what about escaped quotes? \" inside a string.
  Need: \"(*raw character OR escape sequence)*\"
  
Escape sequences: \n, \t, \\, \", \'
  Can be `\x`, where x is any char (C-like), or specific recognized escapes.
```

A regex like `"(\\.|[^"\\])*"` handles escapes:
- `\\.` matches a backslash followed by any character (escape sequence)
- `[^"\\]` matches any non-quote, non-backslash character
- Together they match valid strings with embedded escapes

**Raw strings** (Python, Rust, C++) complicate matters: `r"..."` ignores escapes. Lexer must distinguish token type based on prefix.

## Error Recovery

When the lexer encounters invalid input (no token matches):
- **Error token**: produce a special token (ERROR) and resume; let parser handle it
- **Skip & retry**: discard problematic character, try again
- **Scan to delimiter**: consume until a safe delimiter (whitespace, semicolon, newline)
- **Report & stop**: halt immediately with diagnostic

Most production lexers use smart recovery: produce error tokens but keep scanning to report multiple errors per pass.

## Hand-Written Lexers vs. Lex/Flex

### Generated Lexers (lex, flex)
- **Input**: regex-based lexer specification (`.l` file)
- **Output**: C or C++ lexer code
- **Advantages**: concise spec, automata generation is correct if tool is correct, easy to modify
- **Disadvantages**: generated code is hard to debug, tool dependency, harder to extend with custom logic

### Hand-Written Lexers
- **Advantages**: explicit control, custom error handling, easy to integrate, no tool dependency, faster development for simple cases
- **Disadvantages**: manual state machine management can be error-prone, harder to maintain if complex
- **Common in**: modern compilers (Rust's lexer, Go's lexer, V8) prefer hand-written for control and diagnostics

**Modern trend**: hand-written scanners (clear code patterns in Go, Rust, TypeScript) over generated lexers. Explicit state machines are debuggable.

## Lexer Implementation Pattern

Typical hand-written lexer loop:
```
position = 0
tokens = []
while position < input.length:
  if input[position] is space/newline:
    position += skip_whitespace()
  else if input[position:position+2] == "//":
    position += skip_comment()
  else if input[position] is digit:
    (value, len) = scan_number()
    tokens.append(NUMBER(value))
    position += len
  else if input[position] is alpha or '_':
    (word, len) = scan_identifier()
    token_type = lookup_keyword(word) or IDENTIFIER
    tokens.append(token_type(word))
    position += len
  else:
    handle_error_or_operator()
```

Single-pass, greedy, maximal-munch by construction.

## Lexer Tables & DFA Simulation

For large lexers with many tokens, table-driven approaches are used:

1. **Transition table**: 2D array `table[state][input_char]` → next_state
2. **Accepting states**: mark which states are token boundaries
3. **Execution**: for each input char, look up next state, update line/column, check accept

This is more compact than if-chains for large token sets, though modern branch prediction can sometimes offset the advantage.

## See Also

- `compiler-design-frontend.md` — parsing after lexing
- `text-parsing-techniques.md` — general parsing patterns
- `math-automata-computability.md` — formal theory of finite automata