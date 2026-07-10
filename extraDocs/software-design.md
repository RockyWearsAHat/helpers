---
applyTo: "**"
description: "Universal software design principles as taught by University of Utah CS 3500. Language-agnostic. Applies to all source files."
---

# Software Design Principles

These are universal software engineering principles from a university-level software design course (University of Utah CS 3500). They apply to every language and every project — C#, TypeScript, Python, Rust, shell scripts, whatever. The goal is code that is correct, readable, maintainable, and easy for the next developer (or future you) to navigate and extend without breaking things.

**The most penalized mistake across all projects: methods that are hard to trace.** When in doubt, choose the simpler approach.

---

## 1. Clean Build — Treat All Warnings as Errors

Every warning your toolchain emits is a latent bug. Enable "warnings as errors" on every project and resolve them all. This includes:

- Unreachable code — remove it
- Unused variables or parameters — remove or explicitly discard
- Missing documentation on public APIs — write it
- Implicit type conversions that lose information — fix them
- Any suppression must include a comment explaining why it is safe

If you would not accept a warning in production, do not accept it in development.

## 2. Validate at System Boundaries

Public methods are the boundary of your module. Validate all inputs at the entry point — do not scatter null checks and guard clauses through inner logic.

- Reject null at the top of every public method that accepts reference types
- Fail fast with a clear, specific exception and message
- Inner/private methods can trust that callers have validated

```python
# Python example
def add_edge(self, source: str, target: str) -> None:
    if source is None:
        raise ValueError("source must not be None")
    if target is None:
        raise ValueError("target must not be None")
    # ... logic
```

## 3. Single Responsibility

Each function, method, or module does exactly one thing. If you cannot summarize it in one sentence without using "and" or "or", split it.

Signs a function must be decomposed:
- More than ~20 lines of logic (excluding comments/docs)
- Nested conditionals more than two levels deep
- Multiple distinct phases: validate → compute → mutate → notify
- Its name contains "and" or "or"

Extract helpers with precise names. Helpers are not "implementation detail" — they are the design. A well-named private helper makes the calling method self-documenting.

## 4. Comments Explain Why, Not What

Code already says what it does. Comments say why it was done that way — the constraint, the invariant, the non-obvious design decision.

**Bad:**
```javascript
// increment i by 1
i++;
```

**Good:**
```javascript
// Skip the header row which is always at index 0
i++;
```

If a comment just restates the code in English, delete it. Write a better summary for the function instead.

Every public API (class, function, method, property) requires documentation explaining its purpose, parameters, return value, and any exceptions or errors it can produce.

## 5. Return Abstractions, Not Implementations

Return the most abstract type that satisfies the caller's needs. Callers that need a specific concrete type can convert themselves.

- Return `Iterable` / `IEnumerable` / `interface` not `ArrayList` / `List<T>` / concrete class
- Accept the most abstract parameter type that satisfies the contract
- This makes your API stable against internal implementation changes

```typescript
// Bad — leaks implementation
getItems(): Map<string, Item[]> { ... }

// Good — callers only need iteration
getItems(): Iterable<[string, Item[]]> { ... }
```

## 6. Never Swallow Exceptions

Catch exceptions only when you can actually handle them. Catching and ignoring an exception hides bugs.

- Catch the most specific type, not `Exception` / `Exception` / `error`
- If you catch and rethrow, add context: wrap in a domain-specific exception with the original as the cause
- Use `finally` to restore invariants — never rely on the normal path to reset state
- Log + rethrow is acceptable; silent swallow is not

```java
// Bad
try { loadFile(path); } catch (Exception e) { }

// Good
try {
    loadFile(path);
} catch (IOException e) {
    throw new StorageException("Failed to load config from " + path + ": " + e.getMessage(), e);
} finally {
    isLoading = false;
}
```

## 7. Choose Appropriate Data Structures

The choice of data structure determines the algorithmic complexity of every operation that uses it. Wrong choice = hidden performance bugs.

- Need O(1) membership test? Use a set/hash set, not a list
- Need O(1) key lookup? Use a map/dict, not a list with linear search
- Need ordering AND O(log n) insert? Use a sorted structure
- Maintaining two synchronized structures (forward + reverse index) is acceptable when it gives the right complexity for all operations — but enforce the invariant that both are always updated together

Document the performance contract of every non-trivial data structure decision.

## 8. Prefer Deterministic Behavior

Do not write code whose output depends on iteration order of hash maps, sets, or other unordered structures. This causes tests to pass inconsistently and produces hard-to-reproduce bugs.

- Never rely on insertion order into a `HashMap` / `Dictionary` / `dict` unless the language guarantees it (Python 3.7+, JS ES2015+)
- Write tests that use set equality, not sequence equality, for unordered collections
- If ordering matters for output, sort explicitly before producing output

## 9. Test Coverage — Every Branch Must Be Reachable

If a line of code cannot be reached by any test, one of two things is true: the test suite is incomplete, or the code is dead and should be removed. Both are problems.

Cover every public method, including:

- **Empty/initial state**: what does the interface do before anything has been added?
- **Normal single-element operation**
- **Multiple elements**
- **Duplicate input**: adding the same thing twice must be idempotent
- **No-op operations**: removing something that does not exist must not corrupt state
- **Boundary conditions**: exactly one element, exactly at a limit
- **Error paths**: every exception can be triggered by a test — no defensive branch that is never reached

For every mutating operation, assert both what changed AND what did not change.

## 10. One Concept Per Test

Each test verifies exactly one behavior. A test that tests two things at once obscures which one failed.

**Naming convention:** `UnitUnderTest_Method_Scenario_ExpectedOutcome`

```
Stack_Push_EmptyStack_SizeIsOne
Graph_Remove_NonExistentEdge_NoChange
Parser_Parse_EmptyInput_ThrowsFormatError
Server_Connect_AlreadyConnected_ThrowsInvalidOperation
```

The four segments force you to specify who, what, under what condition, and what is expected — making every test self-describing.

## 11. Simplicity Over Cleverness

If two approaches solve the problem, prefer the one that is easier to read and trace, even if the clever one saves a few lines. Graders, code reviewers, and your future self will all thank you.

- Avoid unnecessary abstraction layers for one-use problems
- Avoid complex one-liners that require deep knowledge to parse
- Avoid wrapping things that do not need wrapping
- If a grader or reviewer cannot follow the code quickly, it is too complex — regardless of whether it is correct

The right abstraction makes code shorter AND clearer. The wrong abstraction makes code shorter but harder to understand. Only extract when the extraction has a clear, nameable purpose.

## 12. DRY — But Do Not Over-Abstract

Do not repeat the same logic in two places. But do not create an abstraction just to avoid two instances of similar code.

- Duplication that will always change together → extract
- Duplication that may diverge → leave separate and document why
- Abstraction that obscures intent → do not extract

The cost of wrong abstraction is higher than the cost of duplication.

---

## Language-Specific: C# and .NET

The following rules are the C# implementations of the general principles above.

### Nullable reference types

Enable `<Nullable>enable</Nullable>` in every project. Annotate all reference types. Use `ArgumentNullException.ThrowIfNull(param)` at every public boundary. Never use the null-forgiving operator `!` without a comment proving the invariant.

### Documentation

Enable `<GenerateDocumentationFile>true</GenerateDocumentationFile>`. Missing XML docs on public members become warnings, and warnings are errors. Every class, method, property, and private field gets `<summary>`. Classes include domain examples in `<code>` blocks.

### Member structure

```
// ==================== Private Fields ====================
// ==================== Public Properties ====================
// ==================== Constructor ====================
// ==================== Public Methods ====================
// ==================== Private Helpers ====================
```

Private fields: `_camelCase`, all immutable fields `readonly`. Namespace style: file-scoped (`namespace Foo;`).

### Modern idioms

- Collection expressions: `items = []`
- Target-typed new: `TcpListener listener = new(IPAddress.Any, port)`
- Property expression body: `public int Size { get => _size; }`
- `sealed` on classes not designed for inheritance
- `string.IsNullOrWhiteSpace()` over manual null + empty checks

### Thread safety

Use an explicit named lock object: `private static readonly object _lock = new();`. Never `lock(this)` or `lock(typeof(...))`.

### File header

```csharp
// <copyright file="FileName.cs" company="CompanyOrProject">
// Copyright (c) YEAR Author. All rights reserved.
// </copyright>
// Author: Alex Waldmann
// Date: YYYY-MM-DD
```

For course projects: use the course-provided copyright block verbatim and add `// Name:` / `// Date:` directly below it.

### Razor / Blazor

Directive order: `@page`, `@rendermode`, `@using`, `@inject`. `id` attribute on every interactive element. All C# logic in `@code { }` at bottom of file. Conditional CSS computed in `@{ }` block before markup, never inline ternary in `class=`.
