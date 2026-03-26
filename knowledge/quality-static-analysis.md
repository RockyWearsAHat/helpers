# Static Analysis — SAST, Tools & Techniques for Automated Code Quality

## Static Analysis Fundamentals

Static analysis examines source code without executing it, answering: "What properties does this code have?" without running it. Contrasts with dynamic analysis (execute code, observe behavior) and manual review (human judgment).

Static analysis spans:

- **Linting**: Catch style violations, suspicious patterns (unused variables, unreachable code)
- **Type checking**: Verify type consistency across expressions and function calls
- **Control flow analysis**: Track variables, function calls, data flow through paths
- **Taint analysis**: Trace untrusted data from source (user input, network) to sink (SQL query, OS command)
- **Abstract interpretation**: Compute abstract properties (ranges, nullability) over all possible executions
- **SMT solver techniques**: Reduce programs to logical formulas verifiable by satisfiability solvers

## Linting

Entry-level static analysis: flag obvious mistakes.

- **Dead code**: Variables assigned but never read; unreachable catch blocks
- **Suspicious patterns**: Comparing strings with `==` instead of `.equals()` (Java), missing awaits on Promises (JS)
- **Style violations**: Naming conventions, indentation, imports
- **Logic bugs**: Conditions always true/false, index out of bounds

**Strengths:** Fast, intuitive, catches typos and oversight.

**Weaknesses:** High false positive rate (flagging valid code); limited semantic understanding. Not suitable for correctness properties requiring deep analysis.

Tools: ESLint (JavaScript), Pylint (Python), Clippy (Rust), Checkstyle (Java).

## Type Checking as Static Analysis

Type systems prove subset of correctness properties: "values flowing into expressions have compatible types."

Strong, static type systems (TypeScript, mypy in strict mode, Rust) catch broad classes of bugs: passing wrong argument types, null dereferences, type confusion. Weak or unsound type systems (some TypeScript configurations) provide less assurance.

**Type checking strengths:**
- Catches whole categories of bugs (type mismatches)
- IDE integration enables real-time feedback
- Refactoring becomes safer (type checker verifies consistency)

**Limitations:**
- Doesn't catch logical errors (adding 1+1 type-checks, produces wrong answer)
- Doesn't enforce invariants (type system doesn't know "balance >= 0")
- Can block valid code ("false positives" from type system perspective, even if semantically sound)

## Dataflow Analysis

Tracks how data flows through a program: variables defined, reassigned, used, and where.

**Reachability analysis**: Which statements can possibly execute? Identifies unreachable code.

**Def-use analysis**: For each variable use, which definitions can reach it? Helps detect uninitialized variables or stale values.

**Live variable analysis**: At each program point, which variables will be used before being redefined? Improves optimizer heuristics.

Dataflow forms the basis for taint analysis (next section) and abstract interpretation.

## Taint Analysis

Specialization of dataflow: tracks untrusted data flowing into dangerous operations.

**Sources** (input): User input, network data, file reads.  
**Sinks** (dangerous use): SQL queries, OS commands, logging secrets.  
**Sanitizers**: Functions that cleanse untrusted data (SQL parameterized queries, shell escaping).

Algorithm:
1. Mark sources as tainted
2. Propagate taint through operations (assignment, function calls)
3. If tainted data reaches sink without sanitization, flag

Example: SQL injection detection.

```
input = user_request.get("id")          # source: tainted
query = f"SELECT * FROM users WHERE id={input}"  # sink: SQL query
                       ↑ Error: tainted data reaching SQL sink
```

**Contextual challenge**: Sanitization functions (escaping, parameterization) nullify taint in specific contexts.

```
input = user_request.get("id")          # tainted
escaped = escape_sql(input)             # sanitizer: taint removed
query = f"SELECT * FROM users WHERE id={escaped}"  # OK
```

Tools: Semgrep (rules-based), CodeQL (query language), Kiuwan, Fortify.

## Abstract Interpretation

Computes abstract properties of all possible executions without running the program.

**Example**: Value range analysis. For each variable, compute the possible range of values it can hold.

```
x = 5                    # Range: [5, 5]
y = read_input()         # Range: [-∞, +∞] (unbounded)
z = x + y                # Range: [-∞, +∞]
if z > 100:
  divide_by(z - 101)     # Range of denominator: [-1, +∞]
                         # Possible division by zero!
```

Abstract interpretation discards concrete values, retains properties (ranges, nullability, aliasing). This allows analyzing all paths without exponential explosion of concrete states.

**Strengths:**
- Finds subtle bugs (off-by-one, null dereferences, buffer overflows)
- Sound (no false negatives, though may have false positives)

**Weaknesses:**
- Complex to implement correctly; tooling expensive
- Over-approximation creates false positives (reports bugs that can't actually occur)
- Scales poorly to large codebases

## SMT Solvers in Verification

Satisfiability Modulo Theories (SMT) solvers answer: "Is there an assignment of variables making this formula true or false?"

Application: verify properties by reducing to SMT formulas.

```
Property: Precondition P → Post-condition Q over function f
Query: "Can we find inputs where P is true but Q is false?"
If SAT solver returns UNSAT (unsatisfiable): property holds
If SAT solver returns SAT + counterexample: property violates
```

Used by formal verification tools (SLAM, Z3) to find bugs or prove correctness.

**Strengths:** Sound, automated, can discover subtle bugs.

**Weaknesses:** Scales poorly; timeouts on complex programs; requires formalization of properties.

Rarely used in daily development; more common in safety-critical domains (OS kernels, crypto).

## Static Analysis Tools Landscape

### Semgrep

**Rules-based taint and pattern matching.** Write rules in YAML/Semgrep DSL to match code patterns.

```yaml
rules:
  - id: no-sql-injection
    patterns:
      - pattern-either:
          - patterns:
              - pattern: $SQL = f"SELECT * WHERE id={$INPUT}"
              - metavariable-pattern:
                  metavariable: $INPUT
                  patterns:
                    - pattern-either:
                        - pattern: request.args
                        - pattern: request.form

message: "SQL injection risk"
severity: WARNING
```

**Strengths:** Easy to write custom rules; fast scanning; integrates into CI/CD.  
**Weaknesses:** Rules are pattern-matching, not semantic analysis; deep bugs require complex rules.

### CodeQL

**Query language for static analysis.** Write queries in CodeQL QL to explore code structure (AST, dataflow graphs).

Treats code as a database; queries retrieve matching patterns. Powerful for complex analysis.

```
method m = any(Method m | m.getReturnType instanceof VoidType)
where f.getSource().hasName("execute") and 
      f gets value from (Expr e | e instanceof Call)
```

**Strengths:** Expressive, can capture nuanced properties; proven for real bugs (GitHub uses CodeQL for security analysis).  
**Weaknesses:** Steep learning curve; query compilation slow; smaller ecosystem than Semgrep.

### SonarQube

**Integrated code quality platform.** Combines linting, type checking, duplication detection, and architectural metrics. Large rule set; GUI-centric reporting.

**Strengths:** One-stop solution; good for organizations wanting single dashboard; handles multiple languages.  
**Weaknesses:** Slower than specialized tools; heavy infrastructure; can feel bloated for teams not needing full platform.

### Comparisons & Trade-offs

| Tool | Approach | Speed | Accuracy | Ease | Use Case |
|------|----------|-------|----------|------|----------|
| **Semgrep** | Pattern rules | Fast | Good | Easy | Custom rules, API security |
| **CodeQL** | Query language | Medium | Excellent | Hard | Complex logic bugs, security |
| **SonarQube** | Multi-faceted | Medium | Good | Medium | Enterprise, multiple languages |
| **Snyk Code** | ML-based | Fast | Good | Easy | Dependency + code security |
| **Checkmarx/Fortify** | Abstract interpretation | Slow | Very good | Hard | Safety-critical |

## Custom Rules & Organization

Most teams write custom linting rules for domain-specific patterns:

- "No direct database access outside DAO layer" (architectural rule)
- "Credentials must be loaded from config/env, never hardcoded"
- "API endpoints must log request/response"

Custom rules live in a shared config (`.eslintrc.js`, `semgrep.yml`, `sonarqube-rules.xml`). CI/CD enforces them; violations block merge. Accumulate rules over time as you discover recurring mistakes.

**Maintenance burden**: Rules expire or become incorrect. Periodically audit false positive rates; disable rules causing more friction than value.

## SAST in CI/CD Pipelines

**Integration pattern:**

```
Developer pushes code
    ↓
CI triggers static analysis (Semgrep, CodeQL, SonarQube)
    ↓
Results: Pass, Warn (advisory), or Fail (blocking)
    ↓
Pass: merge eligible
Warn: merge allowed; developer reviews
Fail: merge blocked; must fix or suppress with justification
```

**Suppression strategy:**
- Explicit suppressions in code (`// semgrep: disable`) with reason
- Tracked centrally; audited periodically
- Suppressions expire (e.g., 1 month); require re-justification

**Performance**: Full scans of monorepos can be slow (5-10 min). Incremental scanning (only changed files) accelerates; full scans run nightly.

## False Positive Management

False positives destroy trust. Teams disable tools generating more noise than signal.

**Mitigation:**
- Tune thresholds (ignore low-confidence detections)
- Prioritize high-severity findings
- Suppress known false positives per rule
- Measure false positive rate; target < 5-10%
- A/B test new rules on pre-commit branch before enforcement

**Empirical validation**: Test tool against historical bugs in your codebase. Does it find real bugs you've had? If tool predicts bugs you never had, likely high false positive rate.

## Code Quality Metrics

Static analyzers output quantity metrics:

- **Cyclomatic complexity**: How many independent paths through a function? > 10 suggests refactoring.
- **Duplication**: % of code that appears elsewhere; indicates copy-paste debt
- **Coupling**: How many dependencies? Lower is better
- **Cohesion**: How related are elements in a module? Higher is better
- **Maintainability index**: Composite score from complexity, lines, duplication

**Caution**: Metrics can be gamed. Focus on trends and priorities (improve high-complexity functions) rather than hitting absolute targets.

## When Static Analysis Fails

**Symbolic limitations**: Symbolic execution can't handle loops of unbounded length or data-dependent behavior.

**Soundness vs. completeness trade-off**: Sound analyses (find all bugs) require severe over-approximation, creating false positives. Unsound tools (fewer false positives) miss real bugs.

**Context loss**: Static analysis sees code in isolation; runtime context (library versions, dependency behavior) invisible.

**Best approach**: Combine static analysis (catch broad classes), dynamic analysis / testing (verify actual behavior), and manual review (nuanced logic, design intent).

## See Also

- [Type Systems](type-systems-theory.md) — Formal properties of type checking
- [Testing Philosophy](testing-philosophy.md) — Complementary testing strategies
- [Testing Mutation](testing-mutation.md) — Mutation testing as complement to static analysis
- [Code Quality Patterns](clean-code.md) — Design principles enabling analysis