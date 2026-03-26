# Compiler Frontend — Lexing, Parsing & Semantic Analysis

## The Compilation Pipeline

Compilers transform source text into executable code through a series of intermediate representations, each lowering the level of abstraction. The frontend handles source-level concerns — understanding what the programmer wrote — while the backend handles target-level concerns — producing efficient machine code. Between them sits an intermediate representation that decouples the two.

A typical pipeline:

```
Source Text → Lexer → Tokens → Parser → AST → Semantic Analysis → Annotated AST → IR
```

Each stage validates and transforms, rejecting malformed input progressively. Lexing rejects illegal characters, parsing rejects structural nonsense, and semantic analysis rejects type errors and unresolved names. This staged rejection concentrates complexity — each phase handles only the errors in its domain.

The frontend-backend split enables the M×N problem reduction: M language frontends and N target backends share a common IR, requiring M+N implementations instead of M×N. In practice, the boundary is fuzzier — language-specific semantics leak into IR design, and target constraints push back into frontend decisions.

## Lexical Analysis

The lexer (scanner, tokenizer) converts a character stream into a stream of tokens — the smallest meaningful units. Whitespace, comments, and other noise are typically discarded or normalized. Each token carries a type (keyword, identifier, literal, operator) and a value.

### Token Structure

| Component       | Role                               | Example                     |
| --------------- | ---------------------------------- | --------------------------- |
| Token type      | Classification for the parser      | `IDENTIFIER`, `INT_LITERAL` |
| Lexeme          | The actual source text             | `"counter"`, `"42"`         |
| Source location | File, line, column for diagnostics | `main.rs:14:8`              |
| Literal value   | Computed value for literals        | `42` (integer)              |

### Theoretical Basis

Regular expressions define the patterns for each token class. These map to deterministic finite automata (DFAs), which can be implemented as table-driven state machines or hardcoded transition logic. The correspondence is exact: regular languages are precisely those recognizable by finite automata.

```
Pattern: [a-zA-Z_][a-zA-Z0-9_]*    →  DFA for identifiers
Pattern: [0-9]+(\.[0-9]+)?          →  DFA for numeric literals
Pattern: "([^"\\]|\\.)*"            →  DFA for string literals
```

Multiple token patterns combine into a single DFA through NFA construction and subset construction (powerset construction). The resulting DFA recognizes the union of all token languages, with disambiguation rules — typically longest match and priority ordering — resolving overlaps.

### Lexer Implementation Strategies

**Generated lexers** use tools that take regular expression specifications and produce DFA tables or code. Advantages include correctness guarantees (the generated automaton matches the spec) and ease of modification. Drawbacks include opaque generated code, difficulty customizing error messages, and build-time dependencies.

**Hand-written lexers** implement the DFA logic directly, often as a switch-on-character structure. Most production compilers choose this approach. Hand-written lexers offer precise control over error recovery, can handle context-sensitive tokens (e.g., `>>` as two `>` tokens in generics contexts), and typically run faster due to specialized logic.

**Hybrid approaches** use generated DFAs for the core but wrap them in hand-written logic for context sensitivity and error handling.

### Lexer Complications

Some language features resist clean lexical analysis:

- **Context-sensitive tokenization**: Python's significant whitespace requires tracking indentation levels. Template strings in many languages require nested lexing modes.
- **Keyword-identifier overlap**: Languages where keywords are unreserved (e.g., contextual keywords in C#) need parser feedback.
- **String interpolation**: `"Hello, ${name}!"` requires the lexer to recursively enter expression-lexing mode inside a string.
- **Encoding**: Source files may be UTF-8, UTF-16, or other encodings. Identifier rules vary — many languages allow Unicode identifiers, requiring Unicode category awareness.

## Parsing

The parser consumes a token stream and produces a structured representation — typically an abstract syntax tree (AST). Parsing validates that the token sequence conforms to the language's grammar, which is typically context-free (expressible as a CFG).

### Grammar Formalism

Context-free grammars define the legal structures:

```
expression → expression '+' term | term
term       → term '*' factor | factor
factor     → '(' expression ')' | NUMBER | IDENTIFIER
```

Ambiguity arises when a token sequence has multiple parse trees. The classic example: `if a then if b then s1 else s2` — does `else` bind to the inner or outer `if`? Grammars are rewritten or augmented with disambiguation rules to resolve such cases.

### Parsing Strategies

| Strategy            | Direction | Grammar Class | Typical Use                  |
| ------------------- | --------- | ------------- | ---------------------------- |
| Recursive descent   | Top-down  | LL(k)         | Hand-written parsers         |
| LL parser (table)   | Top-down  | LL(1)         | Generated parsers            |
| Operator precedence | Bottom-up | Expressions   | Expression sub-parsers       |
| LR / SLR / LALR     | Bottom-up | LR(1)/LALR(1) | Parser generators            |
| GLR                 | Bottom-up | All CFGs      | Ambiguous grammars           |
| PEG / Packrat       | Top-down  | PEG           | Ordered choice, no ambiguity |
| Earley              | —         | All CFGs      | General parsing, O(n³) worst |

**Recursive descent (LL)** parsers map each grammar rule to a function. They are intuitive, debuggable, and allow arbitrary computation during parsing. Left recursion must be eliminated (or handled via Pratt parsing for expressions). Most hand-written production parsers use this approach — its flexibility in error recovery and incremental parsing outweighs the manual effort.

**Pratt parsing (operator precedence)** handles expression grammars elegantly by associating binding powers with operators. It naturally handles precedence and associativity without grammar transformations:

```
func parseExpression(minBP):
    left = parsePrefix()
    while nextToken.bp > minBP:
        op = consume()
        right = parseExpression(op.rightBP)
        left = BinaryExpr(op, left, right)
    return left
```

**LR/LALR parsing** builds the parse tree bottom-up using a shift-reduce automaton. These parsers handle a broader class of grammars than LL parsers and are typically generated from grammar specifications. The resulting tables are opaque, making debugging and error recovery more challenging.

**PEG (Parsing Expression Grammars)** use ordered choice (`/`) instead of unordered alternation (`|`), eliminating ambiguity by definition. Packrat parsing memoizes intermediate results for guaranteed linear time. The trade-off: ordered choice can produce surprising results if rules are ordered incorrectly, and left recursion requires special handling.

### Parser Generators vs Hand-Written Parsers

| Dimension         | Generated                         | Hand-written                         |
| ----------------- | --------------------------------- | ------------------------------------ |
| Error messages    | Generic, requires customization   | Precisely tailored to context        |
| Error recovery    | Limited to generator's strategies | Arbitrary recovery logic             |
| Incremental parse | Difficult to retrofit             | Can be designed in from the start    |
| Maintenance       | Change grammar spec, regenerate   | Manual updates, risk of drift        |
| Correctness       | Grammar = specification           | Grammar implicit in code             |
| Performance       | Table-driven, predictable         | Can exploit language-specific tricks |
| Build complexity  | Requires generator toolchain      | No external dependencies             |

The industry trend has been toward hand-written parsers for major language implementations. The control over error messages and recovery — critical for developer experience — tends to justify the additional implementation effort.

### Concrete Syntax Trees vs Abstract Syntax Trees

A **concrete syntax tree** (CST, parse tree) mirrors the grammar exactly — every production, every token, including parentheses, semicolons, and syntactic sugar. An **abstract syntax tree** (AST) discards syntactic noise and represents only semantically meaningful structure.

```
Expression: (2 + 3) * 4

CST:                          AST:
  factor                        Multiply
  ├─ '('                        ├─ Add
  ├─ expression                 │  ├─ 2
  │  ├─ term: 2                 │  └─ 3
  │  ├─ '+'                     └─ 4
  │  └─ term: 3
  ├─ ')'
  └─ '*'
  └─ factor: 4
```

Some tools (formatters, linters, refactoring engines) need the CST to preserve all source details — whitespace, comments, parenthesization. Compilers typically work with ASTs. Modern approaches sometimes use a **lossless syntax tree** that stores everything but provides AST-like query interfaces.

## Semantic Analysis

After parsing produces a syntactically valid AST, semantic analysis determines whether the program is meaningful. This phase catches errors that grammars cannot express — type mismatches, undeclared variables, invalid overloads.

### Name Resolution

Name resolution determines what each identifier refers to. This requires understanding scoping rules, which vary significantly:

- **Lexical (static) scoping**: Names resolve to the nearest enclosing definition. The binding is determined by program text, not execution path.
- **Dynamic scoping**: Names resolve based on the runtime call stack. Rare in modern languages but present in some (Emacs Lisp, Bash variables).
- **Module scoping**: Names may resolve through import graphs, requiring cross-file analysis.

**Forward references** complicate single-pass resolution. Languages allowing definitions to reference later declarations (mutual recursion, class members) require either multi-pass resolution or deferred checking.

### Symbol Tables

The symbol table maps names to their declarations, carrying type information, scope level, mutability, visibility, and other attributes.

| Approach        | Access   | Scope handling                        |
| --------------- | -------- | ------------------------------------- |
| Hash map stack  | O(1) avg | Push/pop maps per scope               |
| Persistent map  | O(log n) | Structural sharing between scopes     |
| Arena + indices | O(1)     | Flat allocation, scope encoded in IDs |

Symbol table design interacts with incremental compilation — persistent or immutable structures allow sharing between compilation runs, rebuilding only what changed.

### Type Checking

Type checking verifies that operations receive values of compatible types. Systems vary along several axes:

- **Static vs dynamic**: Checked at compile time vs runtime. Most compiled languages use static checking; some support gradual typing.
- **Nominal vs structural**: Types match by declared name or by shape. A `Point { x: int, y: int }` is distinct from `Coord { x: int, y: int }` under nominal typing but equivalent under structural.
- **Inference**: The type checker deduces types from usage. Hindley-Milner inference (as in ML-family languages) can infer most types without annotations. More complex type systems (dependent types, higher-kinded types) require more annotations.
- **Bidirectional type checking**: Combines inference (synthesizing types bottom-up) with checking (propagating expected types top-down). This approach scales to richer type systems while keeping inference practical.

Type checking traverses the AST, computing the type of each expression from its components and checking compatibility at each operation. When type errors are found, the checker typically assigns an "error type" to the failing node and continues, avoiding cascading errors.

### Attribute Grammars

Attribute grammars formalize the decoration of AST nodes with computed information (attributes). **Synthesized attributes** flow bottom-up (a node's type is computed from its children). **Inherited attributes** flow top-down (an expected type propagates from parent to child).

This framework provides a declarative way to specify semantic computations. While pure attribute grammar evaluators are rarely used directly in production compilers, the concepts — synthesized vs inherited, dependency ordering — inform how semantic passes are structured.

### Scope Analysis Complications

- **Closures** capture variables from enclosing scopes, requiring lifetime analysis to determine whether captured variables need heap allocation.
- **Imports and exports** create inter-module name spaces with visibility rules (public, private, package-private).
- **Overloading** means a single name resolves to multiple candidates; the type checker selects the appropriate one based on argument types and context.
- **Shadowing** allows inner scopes to redefine outer names. Whether this is permitted, warned, or errors varies by language.

## Error Recovery

Good error recovery is critical for usability. A compiler that stops at the first error forces edit-compile cycles for each mistake. Modern compilers aim to report as many independent errors as possible in a single pass.

### Parser Error Recovery Strategies

| Strategy          | Mechanism                                 | Quality          |
| ----------------- | ----------------------------------------- | ---------------- |
| Panic mode        | Skip tokens until a synchronizing token   | Low — loses info |
| Phrase-level      | Insert/delete specific tokens to continue | Medium           |
| Error productions | Grammar rules matching common mistakes    | High but brittle |
| Context-aware     | Use parser state to predict likely intent | Highest          |

**Synchronization points** are tokens where the parser can reliably resume — semicolons, closing braces, keywords that start statements. After an error, the parser skips to the next synchronization point, reports the error, and continues.

**Error productions** anticipate common mistakes. A grammar rule matching `if expression block` (missing parentheses) can produce a specific "did you forget parentheses?" message. This requires anticipating mistakes, which limits scalability.

### Error Message Quality

Quality error messages include:

- **Location**: Precise source span — not just line, but the exact range of the problematic construct.
- **What's wrong**: Clear description of the mismatch between expected and found.
- **Why it's wrong**: Context about what the compiler was trying to parse or check.
- **Suggestions**: Potential fixes — "did you mean X?", "add a semicolon here", "this function expects 3 arguments but got 2".

Computing good suggestions requires heuristics — edit distance for misspelled names, type compatibility for wrong-type arguments, scope analysis for "did you forget to import?" suggestions.

## Source Locations

Source location information originates in the lexer and must propagate through every transformation. Each AST node carries a **span** — the byte range (or line/column range) in the original source it corresponds to.

### Location Challenges

- **Macro expansion**: A macro-expanded node exists in multiple locations — the macro definition and the invocation site. Location information becomes a stack of expansion contexts.
- **Desugaring**: When syntactic sugar is lowered (e.g., `for` loops to `while` loops), the generated nodes need locations pointing back to the original sugar.
- **String interning**: Source text for identifiers and literals is typically interned (stored once, referenced by ID) to save memory.
- **Incremental reparsing**: When source changes, locations of unchanged nodes shift. Incremental parsers must efficiently update spans without reparsing everything.

### Diagnostic Rendering

The diagnostic system consumes locations to produce readable output:

```
error[E0308]: mismatched types
  --> src/main.rs:14:9
   |
14 |     let x: i32 = "hello";
   |            ---   ^^^^^^^ expected `i32`, found `&str`
   |            |
   |            expected due to this
```

This rendering requires loading the source line, computing highlight ranges, and potentially showing multiple related locations (the type annotation, the assignment, the conflicting value). Multi-span diagnostics are considerably more complex to render correctly, especially when spans cross line boundaries.

## Pipeline Integration Considerations

### Single-Pass vs Multi-Pass

Some compilers interleave phases — parsing and type checking happen simultaneously, with the parser calling the type checker as AST nodes are produced. This reduces memory usage (no full AST in memory) but couples the phases tightly.

Multi-pass designs build the full AST, then run semantic passes in sequence. This is cleaner, easier to debug, and more amenable to parallelism, at the cost of higher memory usage.

### Incremental Compilation

Modern language servers (providing IDE features) must re-analyze code on every keystroke. Full re-parsing and re-checking is too slow for large projects. Incremental approaches include:

- **Incremental lexing**: Re-lex only the changed region, splicing new tokens into the existing stream.
- **Incremental parsing**: Tree-sitter and similar systems reuse unchanged subtrees, reparsing only affected branches.
- **Demand-driven analysis**: Only analyze functions/modules that are actually queried, caching results for unchanged inputs.

### The Frontend-Backend Contract

The frontend's output — typically an annotated AST or high-level IR — must capture all information the backend needs without exposing language-specific surface syntax. This includes:

- Type information for every expression
- Resolved names (references point to declarations, not strings)
- Implicit operations made explicit (coercions, default arguments, method dispatch)
- Source locations for debug information generation

The quality of this contract determines how cleanly language frontends can share a backend, and how effectively the backend can optimize without language-specific knowledge.
