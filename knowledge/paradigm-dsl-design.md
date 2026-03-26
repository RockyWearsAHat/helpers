# Domain-Specific Languages (DSLs) — Internal, External & Design Patterns

## The DSL Spectrum

A domain-specific language customizes syntax and semantics for a specific problem domain, trading generality for expressiveness in that domain. DSLs range from light configurations (YAML, JSON) to embedded mini-languages (Terraform, Gradle) to full languages (Vimscript, Emacs Lisp).

**Internal DSLs** embed in a host language, reusing its execution model. "Fluent" APIs chain method calls to build configurations or queries. Low implementation cost; limited syntactic freedom.

**External DSLs** define a new syntax, requiring parsing and compilation. Full control over syntax but higher implementation cost and tool support is weaker.

## Internal DSLs: Fluent APIs and Method Chaining

**The pattern.** Return `this` or a builder object from methods, enabling call chains:

```kotlin
request()
    .header("Authorization", "Bearer token")
    .body("{}").json()
    .post("/users")
    .timeout(5000)
    .execute()
```

Each method transforms the builder state, accumulating configuration. Modern IDEs autocomplete chain options.

**Advantage:** Leverages host language syntax, parser, and type checker. Developers already know the execution model. IDE support is free.

**Limitation:** Syntax is constrained by the host language. Can't use `<>` angles or custom operators (without heavy metaprogramming).

**Type safety via builders.** Typed interfaces ensure invalid chains fail at compile-time:

```kotlin
// Post requires a body; compile error if omitted
request().post("/empty").execute() // ✗ Type error

// Optional steps are optional
request().get("/data").execute() // ✓ OK, no body needed
```

Kotlin DSL builders use receiver scope and lambda syntax to create near-declarative syntax:

```kotlin
html {
    body {
        div(id = "main") {
            p { +"Hello, World" }
        }
    }
}
```

The `html { }` lambda receives a receiver (`HtmlBuilder`), so `body` is a method on that receiver, not a global function.

**Ruby and Python alternatives.** Ruby's method-missing metaprogramming and Python's context managers achieve similar fluency with lower type safety:

```ruby
# Ruby: method_missing intercepts undefined methods
ActiveRecord::Relation.where(age: 25).order(:name)

# Python: context managers and operator overloading
with db.transaction() as t:
    t.insert("users", name="Alice")
```

## External DSLs: Parsing and Semantics

**Architecture.** External DSLs require four components:

1. **Lexer** — converts input text into tokens (identifiers, keywords, operators, literals)
2. **Parser** — arranges tokens into an abstract syntax tree (AST)
3. **Semantic analyzer** — type checking, scope resolution, validation
4. **Evaluator/Compiler** — interprets or compiles the AST

**Grammar design.** Use a formal grammar (EBNF, Yacc, or parser generators like ANTLR) to specify syntax unambiguously. Prioritize readability for domain users, not parser implementers.

```ebnf
RuleSet := Rule+
Rule := Condition "->" Action
Condition := Predicate ("AND" | "OR" Predicate)*
Predicate := Field Operator Value
```

**Parser generators.** Tools like ANTLR, Yacc, and Bison automate parser construction from grammar. They handle ambiguity resolution (precedence, associativity) and error recovery. Learning curve is steep; mistakes in grammar cause cryptic parser errors.

**Semantic validation.** After parsing, validate the AST:
- Type consistency (rule operating on numeric field with string operation)
- Undefined references (using a variable not in scope)
- Dead code (unreachable rules)

Validation errors should be collected and reported together, not fail-fast on the first error.

## Design Patterns for DSL Usability

**Literal syntax for domain values.** Support domain-native literals. A query language should write `date "2024-03-25"`, not `parse_date("2024-03-25")`. A YAML-based pipeline should use proper lists and maps, not quoted strings.

**Error messages with location.** "Parse error at line 12, column 3: unexpected token 'END'. Did you mean 'END RUN'?" beats "unexpected token".

**Incremental parsing.** For interactive tools (editors, REPLs), re-parse only affected regions when users edit. Full re-parse on every keystroke is slow.

**Extensibility points.** Allow users to define functions, custom operators, or plug-in rules. Good DSLs grow with their domain.

## Configuration Languages as External DSLs

Several general-purpose configuration languages blur the DSL line:

**HCL (HashiCorp Configuration Language).** Terraform's DSL. Relatively simple syntax (assignments, blocks, expressions), minimal grammar, JSON-compatible. Easy to parse but less expressive than full languages.

**CUE (Configure, Unify, Execute).** Multi-pass language with unification and constraints. Supports schema validation, defaults, and validation rules. Higher learning curve but powerful for infra-as-code.

**Dhall.** Strongly typed, functional configuration language. Every Dhall program terminates. Good for preventing invalid configurations but harder to learn than imperative styles.

**Jsonnet.** Superset of JSON with functions, variables, inheritance. Feels familiar to JSON users but adds enough power for complex templating.

**Trade-off:** More restrictive languages (CUE, Dhall) prevent mistakes early; more permissive ones (HCL, Jsonnet) are easier to learn but allow footguns.

## Macro-Based DSLs

Languages with macro systems (Lisp, Scheme, Rust) allow DSL extension via compile-time metaprogramming.

**Lisp macros.** Lisp code is data (homoiconicity), so macros receive and produce code directly:

```lisp
(defmacro when (condition body)
  `(if ,condition ,body nil))

(when (> x 5) (print "large"))
```

The macro transforms `(when ...)` into `(if ...)` before evaluation. Lisp-based DSLs are common because macro power is built-in.

**Rust procedural macros.** Rust's macro system uses syntax trees (not s-expressions), allowing precise control:

```rust
#[derive(serde::Serialize)]
struct User { name: String }
```

The `derive` macro generates boilerplate serialization code at compile time.

**Limitations:** Macros are powerful but hard to debug (generated code may be complex) and require language-level support. Not all languages support this.

## Real-World Examples

**SQL as a DSL.** Specialized for relational queries. Simple, declarative syntax; weak for imperative logic (stored procedures are ugly). Most modern code avoids writing SQL directly, using query builders instead.

**Terraform/HCL.** Configuration DSL for cloud infrastructure. Prioritizes readability (blocks, nested structure) over terseness. Extensible via providers.

**regex.** Tiny external DSL for pattern matching. Extremely expressive in small syntax; notoriously hard to read once patterns become complex.

**Gradle and Maven build scripts.** Gradle is Groovy-based (internal DSL); Maven is XML with plugin configurations (declarative). Gradle is more flexible, Maven more predictable.

## Implementation Considerations

**Start with a library (internal DSL) before external DSL.** Internal DSLs are faster to prototype and easier to integrate with host-language tools.

**Test the grammar thoroughly.** Ambiguities and shift/reduce conflicts cause subtle parser errors. Use parser generators' built-in conflict diagnostics.

**Version the DSL syntax.** If DSL documents are user-authored (not generated), plan for versions. "DSL 2.0 no longer supports X; use Y instead" requires migration tooling.

**Provide good parse error messages.** Most DSL users are not compiler experts. Error messages must guess intent: "Expected expression, got EOF" is too terse; "Expected a ) after the date argument, but the line ended" is better.

**Avoid Turing completeness unless necessary.** Simpler DSLs are easier to reason about, optimize, and sandbox (runtime resource limits, no infinite loops). Add features only when users request them repeatedly.

## Common Pitfalls

**Syntax without semantics.** Pretty syntax is useless if the meaning is unclear or unpredictable.

**Feature creep.** Starting as a focused DSL, then adding variables, functions, imports—soon you've reinvented Python.

**Ignoring context switching.** Developers context-switch between host language and DSL. Minimize cognitive burden by reusing familiar constructs.

**Poor error recovery.** One syntax error stops parsing; user can't get feedback on other issues in the document.

**Underestimating adoption friction.** Convincing teams to learn new syntax is hard. Internal DSLs (reusing host language syntax) have lower adoption barriers.