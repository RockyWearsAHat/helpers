# Compiler Internals — How Code Becomes Executable

## The Pipeline

```
Source Code
    ↓
[1. Lexing / Tokenization]     "if (x > 5)"  →  [IF, LPAREN, IDENT("x"), GT, INT(5), RPAREN]
    ↓
[2. Parsing]                   Tokens → Abstract Syntax Tree (AST)
    ↓
[3. Semantic Analysis]         Type checking, name resolution, scope analysis
    ↓
[4. IR Generation]             AST → Intermediate Representation (SSA, three-address code)
    ↓
[5. Optimization]              Constant folding, dead code elimination, inlining, loop opts
    ↓
[6. Code Generation]           IR → Machine code / bytecode / WebAssembly
    ↓
[7. Linking]                   Combine object files, resolve symbols
    ↓
Executable
```

## Stage 1: Lexing (Tokenization)

The lexer converts a stream of characters into a stream of tokens.

```python
# Input
"let x = 42 + y;"

# Output tokens
[
    Token(LET, "let"),
    Token(IDENT, "x"),
    Token(EQUALS, "="),
    Token(INT_LIT, "42"),
    Token(PLUS, "+"),
    Token(IDENT, "y"),
    Token(SEMICOLON, ";"),
]
```

### Key Decisions
- **Maximal munch**: `>=` is one token (GTE), not `>` then `=`
- **Whitespace**: Usually discarded (except Python, where indentation is syntactic)
- **Comments**: Stripped (but preserved for doc generators / linters)
- **String interpolation**: Tricky — lexer must handle nested expressions: `"Hello ${name}!"`

### Implementation
```python
# Simple hand-written lexer
class Lexer:
    def __init__(self, source):
        self.source = source
        self.pos = 0
    
    def next_token(self):
        self.skip_whitespace()
        if self.pos >= len(self.source):
            return Token(EOF, "")
        
        c = self.source[self.pos]
        if c.isdigit():
            return self.read_number()
        if c.isalpha() or c == '_':
            return self.read_identifier()
        if c == '+':
            self.pos += 1
            return Token(PLUS, "+")
        # ... more cases
```

## Stage 2: Parsing

The parser converts tokens into an AST (Abstract Syntax Tree).

### BNF Grammar (Backus-Naur Form)
```
expression   ::= term (('+' | '-') term)*
term         ::= factor (('*' | '/') factor)*
factor       ::= NUMBER | IDENT | '(' expression ')'
```

### Recursive Descent (Top-Down) — Most Common for Hand-Written Parsers
```python
def parse_expression(tokens):
    left = parse_term(tokens)
    while tokens.peek() in (PLUS, MINUS):
        op = tokens.next()
        right = parse_term(tokens)
        left = BinaryExpr(op, left, right)
    return left

def parse_term(tokens):
    left = parse_factor(tokens)
    while tokens.peek() in (STAR, SLASH):
        op = tokens.next()
        right = parse_factor(tokens)
        left = BinaryExpr(op, left, right)
    return left

def parse_factor(tokens):
    if tokens.peek() == NUMBER:
        return NumberLit(tokens.next().value)
    if tokens.peek() == LPAREN:
        tokens.expect(LPAREN)
        expr = parse_expression(tokens)
        tokens.expect(RPAREN)
        return expr
```

### Pratt Parsing (Top-Down Operator Precedence)
Elegant approach for expressions with infix operators:
```python
def parse_expr(tokens, min_precedence=0):
    left = parse_prefix(tokens)  # number, identifier, unary op, grouped expr
    
    while precedence(tokens.peek()) >= min_precedence:
        op = tokens.next()
        right = parse_expr(tokens, precedence(op) + (1 if left_assoc(op) else 0))
        left = BinaryExpr(op, left, right)
    
    return left
```

### Parser Generators
| Tool | Grammar Type | Language |
|------|-------------|---------|
| ANTLR | LL(*) | Java, Python, JS, C++, many |
| Bison/Yacc | LALR(1) | C/C++ |
| PEG.js / Ohm | PEG | JavaScript |
| tree-sitter | GLR-like | C (used by editors for syntax highlighting) |
| nom | Parser combinators | Rust |
| pest | PEG | Rust |

### The AST

```
Source: "x + 2 * 3"

AST (respecting precedence):
    BinaryExpr(+)
    ├── Ident("x")
    └── BinaryExpr(*)
        ├── IntLit(2)
        └── IntLit(3)
```

## Stage 3: Semantic Analysis

### Name Resolution
```python
# Which 'x' does this refer to?
x = 10
def foo():
    x = 20          # Different x (local scope)
    def bar():
        print(x)    # Captures foo's x (closure)
```
Build a scope chain: each scope knows its parent. Look up names by walking the chain.

### Type Checking
```typescript
// Type inference: deduce types from context
let x = 42;           // x: number (inferred)
let y = x + "hello";  // Error: can't add number and string (in strict languages)

// Unification: solving type equations
// Given: f(a, b) = a + b
// If a: int and b: int → f returns int
// If a: string and b: string → f returns string (in some languages)
// If a: int and b: string → type error
```

### Type Inference Algorithms
- **Hindley-Milner (Algorithm W)**: Used by Haskell, ML, Rust (partially). Complete inference — you rarely write type annotations.
- **Local type inference**: Used by TypeScript, Kotlin, Swift. Infers within expressions but requires annotations on function signatures.
- **Bidirectional type checking**: Combines inference (synthesize type from expression) with checking (verify expression against expected type).

## Stage 4: Intermediate Representation (IR)

### Three-Address Code
```
Source: x = a + b * c

IR:
  t1 = b * c
  t2 = a + t1
  x = t2
```

### SSA (Static Single Assignment)
Each variable is assigned exactly once. Multiple assignments create new "versions":
```
Source:                SSA form:
x = 1                 x₁ = 1
x = x + 1             x₂ = x₁ + 1
if (cond)              if (cond)
  x = x + 2             x₃ = x₂ + 2
else                   else
  x = x + 3             x₄ = x₂ + 3
y = x                  x₅ = φ(x₃, x₄)    ← phi function: "which version?"
                       y₁ = x₅
```

SSA makes optimizations easier because each "variable" has exactly one definition point.

### LLVM IR
```llvm
define i32 @add(i32 %a, i32 %b) {
  %result = add i32 %a, %b
  ret i32 %result
}
```
LLVM IR is the lingua franca of compiler backends. Languages target LLVM IR → LLVM handles optimization and code generation for every CPU architecture.

## Stage 5: Optimization

### Classic Optimizations
| Optimization | What it does | Example |
|-------------|-------------|---------|
| Constant folding | Evaluate at compile time | `3 + 4` → `7` |
| Constant propagation | Replace variable with known value | `x = 5; y = x + 1` → `y = 6` |
| Dead code elimination | Remove unreachable/unused code | Remove code after `return` |
| Common subexpression elimination | Reuse computed values | `a*b + a*b` → `t = a*b; t + t` |
| Function inlining | Replace call with function body | Small functions get "pasted in" |
| Loop-invariant code motion | Move unchanging code out of loop | `for (i) { x = heavy(); use(x, i); }` → `x = heavy(); for (i) { use(x, i); }` |
| Strength reduction | Replace expensive ops with cheaper ones | `x * 2` → `x << 1` |
| Tail call optimization | Reuse stack frame for tail calls | Recursive → iterative |
| Vectorization (SIMD) | Process multiple values per instruction | Loop over array → SIMD instructions |
| Loop unrolling | Reduce loop overhead | One iteration → four iterations per loop step |

### Optimization Levels
```bash
gcc -O0 file.c    # No optimization (fast compile, debuggable)
gcc -O1 file.c    # Basic optimizations
gcc -O2 file.c    # Standard optimizations (recommended for production)
gcc -O3 file.c    # Aggressive (may increase code size, occasionally slower)
gcc -Os file.c    # Optimize for size
gcc -Og file.c    # Optimize for debugging
```

## Stage 6: Code Generation

### Register Allocation
CPU has limited registers (x86-64: 16 general-purpose). The compiler must decide which variables live in registers vs. memory (spilling).

**Graph coloring**: Build an interference graph (variables that are "live" at the same time can't share a register), then color the graph with K colors (K = number of registers).

### Instruction Selection
Map IR operations to actual CPU instructions. Different CPUs have different instruction sets (x86, ARM, RISC-V).

```
IR: %result = add i32 %a, %b
x86: add eax, ebx
ARM: ADD R0, R1, R2
```

## JIT vs AOT Compilation

| Aspect | AOT (Ahead of Time) | JIT (Just in Time) |
|--------|---------------------|-------------------|
| When | Before execution | During execution |
| Startup | Fast (already compiled) | Slow (compiles at start) |
| Peak perf | Good (static analysis) | Potentially better (runtime profiling) |
| Examples | C, C++, Rust, Go | Java HotSpot, V8 (JS), .NET RyuJIT |
| Optimization | Based on static analysis | Based on actual runtime behavior |

### JIT Tricks
- **Tiered compilation**: Interpret first (fast startup), compile hot functions (peak performance)
- **Speculative optimization**: Assume this branch is always taken → optimize for it → deoptimize if wrong
- **Inline caching**: Monomorphic call sites → skip virtual dispatch
- **On-stack replacement (OSR)**: Replace a running interpreted function with compiled version mid-execution

## Interpreters

### Tree-Walking Interpreter
```python
def evaluate(node):
    if isinstance(node, NumberLit):
        return node.value
    if isinstance(node, BinaryExpr):
        left = evaluate(node.left)
        right = evaluate(node.right)
        if node.op == '+': return left + right
        if node.op == '*': return left * right
    if isinstance(node, IfExpr):
        if evaluate(node.condition):
            return evaluate(node.then_branch)
        return evaluate(node.else_branch)
```
Simple but slow (pointer-chasing through AST nodes).

### Bytecode Interpreter
Compile AST → bytecode, then execute bytecode in a virtual machine:
```
Source: 1 + 2 * 3

Bytecode:
  PUSH 1
  PUSH 2
  PUSH 3
  MUL         # 2 * 3 = 6
  ADD         # 1 + 6 = 7
```
Stack-based VM (Python, Java, Ruby) or register-based VM (Lua, Dalvik).

## Resources for Learning Compilers

- **"Crafting Interpreters"** by Robert Nystrom — Free online. Build two complete interpreters (tree-walking + bytecode). Best practical introduction.
- **"Writing An Interpreter In Go"** by Thorsten Ball — Build a complete interpreter. Very practical.
- **"Engineering a Compiler"** by Cooper & Torczon — Academic but readable. Good on optimization.
- **LLVM Tutorial**: "My First Language Frontend" — Build a language targeting LLVM IR.

---

*"A compiler is just a program that reads a program written in one language (the source) and translates it into another language (the target)." Simple definition. Infinite depth.*
