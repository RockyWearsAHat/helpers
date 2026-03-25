# Language Quick-Reference — What the Model Doesn't Already Know

> **Philosophy:** The model already knows syntax, standard libraries, and basic idioms for every major language. This file covers only the NON-OBVIOUS things — philosophy differences, ecosystem gotchas, common traps, and the "personality" of each language that shapes how good code looks in it. When working in any language, research current docs and ecosystem state rather than relying on static knowledge.

---

## How to Think About Languages

Languages aren't interchangeable. Each has a **personality** — a set of values the community optimizes for. Writing good code means writing code that fits the language's personality, not imposing another language's patterns on it.

| Personality             | Languages                     | What It Means                                                                  |
| ----------------------- | ----------------------------- | ------------------------------------------------------------------------------ |
| **Explicit is better**  | Go, Python, Rust, C           | Spell things out. Avoid magic. Readable > clever.                              |
| **Expressive power**    | Haskell, Scala, Rust, Kotlin  | Rich type systems. Encode constraints. Compiler catches errors.                |
| **Pragmatic evolution** | Java, C#, TypeScript, Swift   | Started simple, added features over time. Use modern features.                 |
| **Minimalism**          | Go, C, Lua, Shell             | Do more with less. Small language, small stdlib, small binaries.               |
| **Metaprogramming**     | Ruby, Lisp, Elixir, Julia     | Code writes code. DSLs. Macros. Powerful but dangerous.                        |
| **Safety obsession**    | Rust, Ada, Haskell            | Compiler won't let you do dangerous things. Fight the compiler, then trust it. |
| **Get things done**     | Python, JavaScript, PHP, Ruby | Ship fast. Huge ecosystem. Optimize for developer productivity.                |

---

## The Big Paradigm Shifts

When switching between languages, the hardest part is switching mental models:

### Error Handling

```
Exceptions (throw/catch):    Python, Java, C#, Ruby, JavaScript, PHP
Result types (return errors): Rust (Result<T,E>), Go (val, err), Haskell (Either)
Option types (nullable):     Rust (Option<T>), Swift (Optional), Kotlin (?), Haskell (Maybe)
Error codes (C-style):       C, Shell ($?)
Let it crash (supervisors):  Erlang/Elixir (OTP supervision trees)
```

**Trap:** Don't use exceptions in Go. Don't use error codes in Python. Don't panic in Rust for recoverable errors. Match the language's model.

### Memory Management

```
Manual:           C (malloc/free), C++ (new/delete, but prefer RAII)
Ownership/Borrow: Rust (compile-time, zero-cost)
Reference Count:  Swift (ARC), Objective-C (ARC), Python (with cycle collector)
Garbage Collected: Java, Go, C#, JavaScript, Python, Ruby, Haskell, Elixir
```

### Concurrency

```
OS threads + locks:        C, C++, Java, C#, Rust
Green threads/goroutines:  Go (goroutines + channels)
Actor model:               Erlang/Elixir (processes), Scala (Akka)
Async/await event loop:    JavaScript, Python, Rust, C#, Swift, Kotlin
CSP (channels):            Go, Clojure (core.async)
STM (software txn memory): Haskell, Clojure
```

### Null/Nil Handling

```
Null exists, causes crashes:   Java, C, C++, JavaScript, PHP (billion-dollar mistake)
Optional types enforce checks: Rust, Swift, Kotlin, Haskell, OCaml
Null exists but type-checked:  TypeScript (strictNullChecks), C# (nullable refs)
Everything truthy/falsy:       Python, JavaScript, Ruby (know the falsy values!)
```

---

## Per-Language Non-Obvious Notes

### Python

- **The GIL** — Threads don't give CPU parallelism. Use `multiprocessing` or `asyncio` for I/O.
- **Virtual environments are mandatory** — Never `pip install` globally. Use `venv`, `uv`, or `poetry`.
- **Type hints don't enforce anything at runtime** — They're for tooling (mypy, Pylance, pyright).
- **f-strings > .format() > %** — Use f-strings. They're faster and more readable.
- **Ruff** — Currently the one linter/formatter to use. Replaces flake8, isort, black, pyupgrade.
- **`uv`** — Fast Python package manager (Rust-based). Rapidly becoming the default.

### JavaScript / TypeScript

- **`===` not `==`** — Always. `==` has insane coercion rules.
- **`strictNullChecks: true`** — Non-negotiable in TypeScript. Without it, types lie.
- **`"strict": true`** in tsconfig — Enables all strict checks. Start every project with this.
- **Closures capture by reference** — Classic loop variable bug. Use `let`, not `var`.
- **Node.js is single-threaded** — One event loop. CPU-bound work blocks everything. Use worker threads.
- **ESM over CJS** — `import/export` over `require/module.exports`. The ecosystem is migrating.

### Rust

- **Fight the borrow checker, then trust it** — If it compiles, it's memory-safe. Really.
- **`unwrap()` is a code smell in production** — Handle errors with `?`, `match`, or combinators.
- **Lifetimes are about relationships** — `'a` means "this reference lives at least as long as that thing."
- **`clone()` is fine while learning** — Optimize later. Premature optimization is worse in Rust.
- **`cargo clippy`** — The best linter in any ecosystem. Run it, listen to it.

### Go

- **Accept interfaces, return structs** — The Go proverb. Keeps signatures flexible, returns concrete.
- **Errors are values, handle them explicitly** — `if err != nil` is idiomatic, not ugly. Embrace it.
- **`go vet` + `golangci-lint`** — The standard linting setup.
- **Small interfaces** — `io.Reader` is one method. Compose small interfaces, don't build large ones.
- **No generics until 1.18** — Code written before that uses `interface{}` everywhere. Modernize when you can.

### Java

- **Modern Java (17+) is not your father's Java** — Records, sealed classes, pattern matching, virtual threads. Use them.
- **`var` is fine for local variables** — Since Java 10. Use when the type is obvious from the RHS.
- **Virtual threads (Loom)** — Java 21+. Lightweight threads that block without blocking OS threads.
- **Spring Boot is the de facto web framework** — But know what it's doing under the covers (dependency injection, auto-config).

### C / C++

- **C: Every buffer is a potential CVE** — Bounds check everything. Use `snprintf`, not `sprintf`. Use `strncpy`, not `strcpy`.
- **C++: RAII is not optional** — If you're managing resources manually, you're writing C, not C++.
- **Smart pointers: `unique_ptr` by default** — `shared_ptr` only when truly shared. Raw pointers for non-owning references only.
- **`std::string_view` over `const std::string&`** — Since C++17. Avoids unnecessary copies for read-only access.

### Swift

- **Value types by default** — `struct` over `class` unless you need reference semantics.
- **Protocol-oriented design** — Prefer protocols over class hierarchies.
- **`guard let` for early returns** — Not `if let` nested 5 levels deep.
- **Actors for concurrency** — Swift concurrency (async/await + actors) is the modern model. Avoid GCD for new code.

### Shell (Bash/Zsh)

- **`set -euo pipefail`** — Start every script with this. `-e` exits on error, `-u` errors on undefined vars, `-o pipefail` catches pipe failures.
- **Quote everything** — `"$var"` not `$var`. Word splitting is the #1 source of shell bugs.
- **`[[ ]]` not `[ ]`** — Double brackets are safer (no word splitting, supports `&&`/`||`/regex).
- **ShellCheck** — Run it on every shell script. It catches real bugs.
- **Arrays are tricky** — Bash arrays have surprising behavior. Test edge cases.

### Ruby

- **Convention over configuration** — Rails defines where everything goes. Follow the conventions.
- **Blocks are closures** — `do...end` and `{ }` are not just syntax sugar. They capture scope.
- **Duck typing** — "If it quacks like a duck." Don't check types, check capabilities.

### Elixir

- **Pattern matching everywhere** — Function heads, case, with. It's the primary control flow mechanism.
- **Processes are cheap** — Thousands are fine. Millions are possible. Use them for isolation.
- **OTP supervision trees** — "Let it crash" assumes supervised restarts. Without supervisors, it's just crashing.
- **Immutable data** — All data is immutable. "Mutation" creates new copies (efficiently, structurally shared).

### SQL

- **Indexes are not optional for production** — Every `WHERE` column, every `JOIN` column, every `ORDER BY` column. Check `EXPLAIN`.
- **ORMs hide the query** — When performance matters, read the generated SQL. Optimize the query, not the ORM call.
- **N+1 is the default bug** — Eager load (`JOIN`/`INCLUDE`) when you know you need the related data.
- **Transactions have cost** — Long transactions hold locks. Keep them short.

---

## Language Selection Heuristic

When choosing a language for a new project, the strongest signal is usually the ecosystem, not the language:

```
Need: Fast web API → Go, Rust, Java, C#, TypeScript (Node/Deno/Bun)
Need: Data science / ML → Python (not even close)
Need: iOS app → Swift (or React Native / Flutter for cross-platform)
Need: Android app → Kotlin (or React Native / Flutter for cross-platform)
Need: CLI tool → Go, Rust (single binary, fast startup)
Need: System programming → Rust, C, C++
Need: Quick scripting → Python, Shell, Ruby
Need: Frontend → TypeScript (React/Vue/Svelte/etc.)
Need: High-concurrency server → Go, Elixir, Rust
Need: Enterprise backend → Java, C#, Kotlin
Need: What the team already knows → That language (usually the right answer)
```

**The last line is usually the most important.** A team that knows Python will ship faster in Python than in the "theoretically better" language they'd need to learn.
