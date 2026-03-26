# Incremental Compilation — Language-Specific Strategies, Watch Mode, HMR

## What Incremental Compilation Solves

Incremental compilation reduces the time to respond to source changes by recompiling only what's necessary. Without it, every edit requires a full type-check and code-gen pass. With it, the compiler can reuse results from previous compilation and emit transformed output in milliseconds instead of seconds.

The problem is **state**: compilation is stateful (symbol tables, type information, intermediate representations are built during the pass). Incremental systems either (1) persist state to disk, (2) maintain state in memory during watch mode, or (3) both.

---

## TypeScript Incremental Compilation

### --incremental Flag

```bash
tsc --incremental
```

Activates state persistence. TypeScript writes a `.tsbuildinfo` file containing:
- Symbol table snapshots per source file
- Import/export metadata
- Type information from dependencies

On rebuild with changed files, TypeScript:
1. Compares input hashes to the `.tsbuildinfo` record
2. Skips type-checking unchanged files whose dependencies haven't changed
3. Re-checks only files with new or modified content

### Performance Impact

Benchmarks for a 10,000-file project:
- **Cold build** (no cache): 30–60 seconds
- **Incremental, single-file change**: 100–500ms
- **Incremental, entire node_modules change**: falls back to full rebuild (~30s)

Incremental pays off after the first build, though it adds startup overhead (reading `.tsbuildinfo`).

### Project References

Large monorepos use project references to scale incremental compilation:

```json
{
  "references": [
    { "path": "./packages/core" },
    { "path": "./packages/ui" },
    { "path": "./packages/app", "prepend": true }
  ]
}
```

Each project maintains its own `.tsbuildinfo`. The compiler builds projects in dependency order:
1. Build core (outputs .d.ts + .tsbuildinfo)
2. Build ui (reads core's .d.ts, outputs its own)
3. Build app (reads ui's .d.ts)

Only projects with changed dependencies rebuild. For a monorepo with 50 packages, changing one deep package still recompiles dependents, but skips unaffected packages.

### Limitations

- `.tsbuildinfo` tracks file content hashes; compiler version changes invalidate the cache
- Import graph must be correct; if the compiler misses an edge, stale type info propagates
- **Composite projects** (using project references) are more complex to configure
- Not all configurations support incremental mode (declaration emit, emit for side effects)

---

## Rust Incremental Compilation

### Architecture

Rust's `rustc` uses a **query-based incremental system**. Rust represents compilation as a dependency graph of queries:

```
parse_input_file
  ↓
check_item_types
  ├→ type inference
  ├→ borrow checking
  ↓
codegen_unit
  ↓
link
```

On rebuild, Rust re-executes only queries whose inputs changed.

### Query Fingerprinting

Each query has a **fingerprint** (hash of inputs). Between builds, Rust compares fingerprints:

```
Previous build: parse(main.rs) → fingerprint_A
Current build:  parse(main.rs) → fingerprint_A (unchanged)
→ borrow-checking result from previous build can be reused
```

If a dependency changed (e.g., `lib.rs` was edited), its query fingerprint changes, invalidating all dependent queries.

### Performance

Single-file changes in large Rust projects:
- **Cold compile**: 60–300 seconds (Rust is slow)
- **Incremental with local change**: 1–5 seconds
- **Incremental with dep change**: 10–60 seconds (must re-check borrow rules in dependents)

### Challenges

- **Unstable incremental state**: compiler bugs in fingerprinting have caused incorrect caching. Full rebuilds required (`cargo clean`).
- **Platform bugs**: clock skew on CI destroys fingerprint validity
- **Query dependencies underspecified**: subtle code changes invalidating more than necessary

---

## Go Build Cache

### Per-Package Compilation

Go compiles packages, not files. Each package is its own compilation unit:

```
import "myapp/server"    # Trigger compile of server package
import "myapp/database"  # Separate compilation
```

This package granularity enables:

1. **Selective recompilation**: change one file in package A → recompile A, rebuild binaries using A, skip unaffected packages
2. **Distributed builds**: each package can compile on a different machine

### Build Cache

Located at `$GOCACHE` (default: `~/.cache/go-build/`). Go caches the output of every compilation action:

```
Key:   sha256(source code + deps + compiler version + build flags)
Value: object file + export data
```

On rebuild:
```bash
go build ./...
```

Go recomputes the key for each package. If it's in the cache, reuse the object file. Otherwise, compile from source.

### Semantics

Go's cache is **conservative**: includes all inputs that could affect output:
- Source file content (hash)
- Compiler version
- All transitive dependencies (to detect API breaks)
- Compiler flags (optimization level, target platform)
- Build tags

Changing any one invalidates the cache.

### Performance

Typical incremental build in a 20-package project:
- **Full (`go clean; go build`)**: 5–10 seconds
- **Incremental (one file change)**: 100–500ms (recompile that package + re-link)
- **No code change**: <10ms (cache hits for all packages)

---

## Java Incremental Compilation (Gradle)

### Gradle's Incremental Compilation API

Tasks declare inputs and outputs:

```groovy
task compileJava(type: JavaCompile) {
  source = fileTree('src/java')
  outputs.dir 'build/classes'
}
```

Gradle tracks:
- Input files (source code)
- Output files (class files)
- Intermediate state (annotation processors, generated sources)

On rebuild, Gradle:
1. Checks which input files changed
2. Recompiles only those files + affected dependents (files that reference changed classes)
3. Preserves unchanged class files

### Annotation Processing

Java's annotation processors can generate code and influence compilation. Changes to annotations invalidate the entire compile step unless:

- Processor is deterministic (same inputs → same outputs)
- Processor declares its input/output dependencies via `@*Processed` APIs

### Performance Trade-offs

True incremental Java compilation requires:
- **Tracking inter-class dependencies** (complex: A depends on B.methodSignature; if B's signature changes, recompile A)
- **Isolating annotation processors** whose generated code might differ

Gradle enables incremental compilation but it can have bugs (false positives: missed invalidations). Conservative fallback: full recompile detected via `--rebuild` flag.

---

## Watch Mode Implementations

All modern build tools support file-watching for incremental recompilation on save.

### File System Watcher Layers

**Layer 1: OS-level file monitoring**

- **Linux**: inotify (file system events)
- **macOS**: FSEvents (directory-level events; less granular than inotify)
- **Windows**: ReadDirectoryChangesW

**Layer 2: Watcher libraries**

- [chokidar](https://github.com/paulmillr/chokidar): Node.js library, abstracts OS differences, adds debouncing/filtering
- [watchexec](https://watchexec.github.io/): Rust tool, watches any project
- Built-in: VS Code's watchers use OS APIs directly for low latency

### Debouncing and Filtering

Rapid file changes (e.g., editor's autosave + Prettier format) trigger multiple edit events. Watchers debounce (wait 100ms for activity to settle, then trigger one rebuild) to avoid redundant compilations.

### Scalability

Watch mode struggles on large codebases:
- **Deep nesting** (many directories) → many inotify watches
- **node_modules** (100k+ files) → potential watch limit explosion on Linux
- **Rapid changes** (IDE's auto-import or LSP formatting) → watch event floods

Solutions:
- Ignore patterns (`.gitignore` style)
- Watch only source directories, not build output
- Increase system watch limits (`fs.inotify.max_user_watches` on Linux)

---

## Hot Module Replacement (HMR) Internals

HMR enables replacing code in a running application without full page reload. Vite, Webpack, and others implement it.

### Mechanism

```
1. Dev server running (Vite, Webpack dev server)
2. Browser loads app
3. Dev server establishes WebSocket connection to browser
4. Developer edits file
5. Dev server re-transforms file
6. Dev server sends delta over WebSocket
7. Browser-side HMR runtime intercepts and re-executes module
8. Dependent modules re-run if exports changed
9. App state optionally preserved via HMR hooks
```

### Module Acceptance

```javascript
// src/counter.js
export let count = 0
export const increment = () => { count++ }

// src/main.js
import { count, increment } from './counter.js'

if (import.meta.hot) {
  import.meta.hot.accept('./counter.js', newModule => {
    // counter.js changed; replacement logic here
    increment = newModule.increment
  })
}
```

**Accept patterns:**
- `accept()`: this module accepts its own updates
- `accept(dep)`: this module accepts updates to a dependency
- `dispose()`: cleanup before update
- `decline()`: reject updates, force full reload

### HMR Propagation

If Module C imports B imports A, and A changes:

1. Dev server detects A change
2. Dev server invalidates A's cache, re-transforms it
3. Dev server notifies browser "A updated"
4. Browser-side runtime checks if B has accepted A's updates
5. If yes, B re-imports A; propagates upward
6. If no, full reload required

### Limitations

- **Framework-specific**: React, Vue, Svelte each implement their own HMR adapters
- **State loss**: local variables reset unless explicitly preserved
- **Not universal**: some changes (class redefinition, top-level statements) can't be HMR'd safely
- **Non-determinism**: HMR branches might behave differently than full app restart

---

## Incremental Challenges and Pitfalls

### 1. File System Ordering

On incremental rebuild, file systems don't guarantee ordering of writes. If compiler A emits header file H and binary B, and both are read by a dependent compiler C:

- **Race condition**: C might read H before it's written, or B before it's finalized
- **Timestamp mismatch**: C might have old H cached, new B on disk

Solution: explicit synchronization (wait for H to be written before starting C) or content-based invalidation.

### 2. Compiler Bugs

Incremental state can become corrupted if:
- Compiler incorrectly fingerprints inputs (Rust history)
- Incremental cache outlives code refactors that invalidate assumptions
- Network clock skew in distributed builds

Result: stale cache propagates incorrect behavior. Mitigation: `cargo clean`, `rm .tsbuildinfo`, `gradle clean`.

### 3. Non-Deterministic Generators

If a code generator produces different output for the same input (timestamps, random IDs, environment-dependent paths), incremental systems fail. Caches assume idempotency.

---

## See Also

- **build-systems-deep.md** — Broader incremental build architecture and caching strategies
- **build-bundlers-modern.md** — HMR in Vite and esbuild's watch mode
- **compiler-internals.md** — General compilation pipeline; applies across languages