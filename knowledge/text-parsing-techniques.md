# Text Parsing Techniques — Recursive Descent, PEG, Combinators, ANTLR, Tree-Sitter

Parsing translates text into structured data. Techniques range from simple hand-written parsers to generated ones, each with distinct trade-offs in expressiveness, performance, error recovery, and ease of writing.

## Parser Taxonomy

All parsers fit into two categories:

**Top-down** (predictive): Start with the start symbol, predict what comes next, match input left-to-right. Simple recursive structure mirrors grammar.

**Bottom-up** (shift-reduce): Accumulate input, reduce to grammar rules when matches appear. More powerful than top-down but requires building parsing tables.

Most production parsers use bottom-up (LR family) for language design but top-down for tool implementation due to simplicity.

## Recursive Descent Parsing

Hand-write one function per grammar rule. Each function returns success/failure, consuming input if successful.

```python
def expression():
    term()
    while current() in ['+', '-']:
        consume(current())
        term()

def term():
    factor()
    while current() in ['*', '/']:
        consume(current())
        factor()

def factor():
    if current() == '(':
        consume('(')
        expression()
        consume(')')
    else:
        consume_number()
```

**Advantages**: Simple to implement, mirrors grammar structure directly, excellent error recovery (easy to add custom error handling), natural to read and modify.

**Disadvantages**: Only works for **LL(k) grammars** — grammars where k tokens of lookahead suffice to decide which production to apply. Cannot handle left recursion directly.

**Left recursion problem**: Rule `E → E '+' T` causes infinite recursion. Solution: rewrite as `E → T ('+' T)*`.

Some variants support limited left recursion through restructuring or memoization (e.g., Pratt parsing for operator precedence).

## Parsing Expression Grammars (PEG)

Formalism introduced by Bryan Ford (2004). Like recursive descent, but expressed as a formal grammar with ordered choice.

```
Expression ← Term (('+' / '-') Term)*
Term ← Factor (('*' / '/') Factor)*
Factor ← Number / '(' Expression ')'
```

**Key difference from context-free grammars**: Choice operator is ordered. First match wins; second alternative is ignored unless first fails.

**Properties**:
- Unambiguous: each input string has exactly one parse tree or none
- No left recursion: order matters; `A ← A 'a' / 'a'` is invalid
- Powerful lookahead: `&expr` (look ahead without consuming), `!expr` (negative lookahead)

**Packrat parsing**: Memoize all parse attempts at each input position. Every nonterminal is called at most once per position. Linear O(n) time but O(n) space.

**Trade-offs**: Clean syntax, guaranteed linear time with packrat parsing, but grammar order affects meaning (adding choices can remove valid parses).

## Parser Combinators

Library functions that combine smaller parsers into larger ones. Functional style; each parser is a function.

```haskell
-- Pseudocode
string :: String -> Parser String
digit :: Parser Char
many :: Parser a -> Parser [a]  -- 0 or more
some :: Parser a -> Parser [a]  -- 1 or more

expr = term `sepBy` (char '+')
term = factor `sepBy` (char '*')
factor = digit `some` <|> (char '(' *> expr <* char ')')
```

**Advantages**: Composable, express grammars as code, reusable parsers, type safety in typed languages.

**Disadvantages**: Performance often lower than generated parsers; error messages less informative; order matters (like PEG).

**Used in**: Haskell (Parsec, Megaparsec), Rust (nom), Python (pyparsing), JavaScript (Jison with combinator variants).

## ANTLR (Another Tool for Language Recognition)

Parser generator: write a grammar, generate lexer and parser in target language.

```antlr
grammar Arithmetic;

program: statement+;
statement: expr SEMICOLON;
expr: term ((PLUS | MINUS) term)*;
term: factor ((MULT | DIV) factor)*;
factor: NUMBER | LPAREN expr RPAREN;

PLUS: '+';
MINUS: '-';
MULT: '*';
DIV: '/';
NUMBER: [0-9]+;
WHITESPACE: [ \t\n]+ -> skip;
```

Generates:
- **Lexer**: Tokenizes input (lexical analysis)
- **Parser**: Builds parse tree from tokens (bottom-up LALR or top-down LL)
- **Listener/Visitor**: Walk parse tree to extract meaning

**Advantages**: No hand-coding complexity; grammar reuse across targets; rich error recovery; widely used in industry (Java, Python, C++, Go generators).

**Disadvantages**: Overkill for small languages; adds build step; generated code can be verbose.

**Error recovery**: ANTLR uses **automatic error recovery** — inserts/deletes tokens to recover and continue parsing, reporting errors without crashing.

## Tree-Sitter (Incremental Parsing)

Incremental parser generator optimized for interactive editors. Reuse previous parse results when text changes.

**Key innovation**: After edit, re-parse only affected portions. Track **byte ranges** precisely.

```
Initial parse:   [ function foo() { body } ]
Edit line 5:     Only re-parse body, reuse function/name regions
```

**Properties**:
- Resilient parsing: syntax errors don't block entire parse
- Precise error reporting: marks invalid regions
- Streaming: parse as text arrives
- Language-agnostic: use tree-sitter grammars cross-language

**Used in**: Neovim, Helix, zed editor, VS Code (tree-sitter extensions). Node-based parse tree; fast queries via S-expression-like syntax.

## Error Recovery Strategies

**Panic mode**: Skip tokens until synchronization point (next semicolon, dedent, keyword). Simple, robust but loses context.

```
Statement* <- Statement (';' Statement)*
On error, resync to next ';'
```

**Error tokens**: Allow invalid constructs in grammar; mark and continue.

```
Statement <- ValidStatement | ERROR BadTokens
```

**Insertion/deletion**: Assume missing or extra tokens; ANTLR's default. Works well when errors are minor typos.

**Phrase-level recovery**: Track parser state stack; backtrack to safe point. More complex but recovers better in nested structures.

## Language Server Protocol (LSP) Integration

LSP decouples editors from language tools. Servers provide:

- **Parsing**: Background parse as user types
- **Diagnostics**: Error/warning feedback
- **Completion**: Suggest based on partial parse
- **Go to definition**: AST navigation
- **Hover**: Type/documentation lookup

**Architecture**:
```
Editor  ←→  LSP Client ←→ JSON-RPC ←→ LSP Server ←→ Parser
                                                        ↓
                                                      AST
```

Server maintains incremental parse tree; sends diagnostics on change.

**Parser choice for LSP**:
- **Incremental parsers** (tree-sitter): Fast updates, resilient to errors
- **Packrat parsers** (PEG): Linear time, predictable
- **Handwritten recursive descent**: Full control, custom error recovery

## Comparing Techniques

| Technique          | Expressiveness | Ease       | Error Recovery | Performance | Maintenance |
| ------------------ | -------------- | ---------- | -------------- | ----------- | ----------- |
| Recursive descent  | LL(k)          | Simple     | Excellent      | Fast        | High        |
| PEG packrat        | Powerful       | Clean      | Good           | Linear O(n) | Medium      |
| Parser combinators | Powerful       | Elegant    | Varies         | Medium      | Medium      |
| ANTLR              | LR/LL          | Generated  | Good (auto)    | Fast        | Low         |
| Tree-sitter        | Flexible       | Generated  | Resilient      | Incremental | Low         |

## When to Use What

**Recursive descent**: Building a language from scratch, full control wanted, team comfortable with compiler theory.

**PEG**: Cleanest syntax for medium-complexity languages, parsing as a library.

**Parser combinators**: Functional language fans, heavy reuse of parser pieces.

**ANTLR**: Mature production languages, multiple target languages needed, large team.

**Tree-sitter**: Editors, LSP servers, any tool needing fast incremental updates.

## See Also

- [compiler-design-frontend.md](compiler-design-frontend.md) — Lexing and AST construction
- [math-automata-computability.md](math-automata-computability.md) — Finite automata theory
- [regex-patterns.md](regex-patterns.md) — Regular expressions and pattern matching
- [architecture-state-machines.md](architecture-state-machines.md) — State machine design patterns