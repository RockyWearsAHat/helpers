# Build Systems — Deep Dive: Dependency Graphs, Caching, Reproducibility

## What Build Systems Solve

A build system is fundamentally a dependency-driven task scheduler. It transforms source artifacts into output artifacts through a directed acyclic graph (DAG) of build steps. The core problem it solves: **determining which tasks to execute, in what order, and when to skip unnecessary work**.

Every build system optimizes for a triangle of sometimes-conflicting goals:

| Goal | Tension | Trade-off |
|------|---------|-----------|
| **Correctness** | Complete after deps, miss nothing | Over-conservative invalidation wastes time |
| **Efficiency** | Skip unnecessary work (incremental) | Detecting "necessary" requires overhead |
| **Reproducibility** | Same inputs → same outputs everywhere | Environment isolation adds complexity |
| **Scalability** | Handle thousands of targets/sources | Dependency analysis parallelism creates bottlenecks |

---

## Dependency Graphs and Topological Ordering

Build systems represent work as a DAG where nodes are build targets (files, tests, packages) and edges represent "depends-on" relationships.

```
      application.exe
       /            \
   main.obj       engine.lib
     /   \          /  \
 main.c  utils.c  engine.c  asset.data
                     |
                  config.h
```

**Three fundamental properties:**

1. **Acyclicity is mandatory.** A cycle (A → B → A) has no valid topological ordering. Build systems detect cycles and fail fast. Some systems handle specific patterns through workarounds (e.g., mutually recursive modules in languages with forward declarations).

2. **Diamond dependencies** occur when multiple paths converge on a shared target. Topological sorting ensures the diamond's base executes exactly once, not redundantly per path.

3. **Transitive dependencies.**  If A depends on B and B depends on C, then changes to C invalidate A even without direct reference. Accurate propagation requires full DAG traversal; missing transitive edges causes incorrect incremental builds.

**Dynamic dependencies** complicate the DAG: edges discovered during build execution (e.g., C preprocessor discovering `#include` directives). Build systems handle this through:
- **Depfiles** (Make's `.d` files, Gradle's input/output declarations): scanner emits edge information during task execution
- **Conservative over-approximation**: treat all `#include` search paths as potential dependencies, even if not found
- **Re-scanning**: detect when dependency structure changed, invalidate dependent tasks

---

## Content-Based vs. Timestamp-Based Invalidation

Determining whether to rebuild hinges on detecting input changes. Two strategies exist.

### Timestamp-Based (Mtime)

Compare file modification times. If input mtime > output mtime, rebuild.

**Advantages:**
- O(1) check per file (single stat call)
- Filesystem-native, universally supported
- Intuitive mental model for developers

**Failure modes:**
- **Clock skew** between machines breaks distributed builds (a remote build on a machine with slow clock can produce outputs with older mtime than local inputs)
- **Implicit dependencies** on timestamps (e.g., generated code with embedded build time appears to change)
- **Filesystem granularity** varies (FAT32: 2-second resolution, modern filesystems: nanoseconds)
- **Copies and symlinks** may or may not preserve timestamps depending on tool flags
- **Build tools cannot distinguish** content change from touching a file

### Content-Based (Hashing)

Hash file contents (SHA-256, Blake3, or similar). Rebuild only if hash changes.

**Advantages:**
- Immune to clock skew across machines
- Distinguishes true changes from metadata updates
- Enables content-addressable caching (key = hash of inputs)
- Non-determinism visible (identical inputs produce different outputs)

**Disadvantages:**
- **Hash computation cost** on large assets (can dominate initial cache population)
- **Requires persistent hash database** (stored locally or in remote cache)
- **Cold build must hash everything** before any caching can occur
- **Non-deterministic outputs** remain a problem (timestamps in binaries, random UUIDs in metadata)

---

## Deterministic and Reproducible Builds

**Reproducibility**: same source → same artifact, bit-for-bit identical, regardless of build machine or timestamp.

Barriers to reproducibility:

1. **Embedded metadata**: timestamps in PE headers, linker output, archive headers
2. **Nondeterministic iteration order**: hash maps in build tools producing different orderings per run
3. **Random data**: version identifiers, UUIDs, seeding without fixed randomness
4. **Environment variables**: compiler flags influenced by developer environment
5. **Floating dependencies**: "latest" versions changing between builds
6. **Filesystem caching**: builds on different machines see cached metadata differently

Build systems address reproducibility through:

- **Hermetic builds** (Bazel): isolate build processes in sandboxes; inputs are explicit; environment is controlled
- **Fixed dependency versions** (lock files, pinned commits)
- **Content-based inputs** ensuring environment differences don't affect outputs  
- **Stripping metadata** (objcopy, newer language toolchains with reproducible-by-default flags)
- **Deterministic code generation** (disabling optimization levels that depend on run order, fixing seed for randomness)

---

## Incremental Build Caching

Two layers of caching:

### Local Task Caching

Check if this task executed before with these exact inputs. Reuse output.

Requires:
1. **Input tracking**: precise declaration of what affects task outcome
2. **Output capture**: which files/artifacts are produced
3. **Hash equality check**: when inputs match a previous run, outputs are guaranteed identical

Challenges:
- **Implicit dependencies** (e.g., compiler version, CPU feature flags, locale settings)
- **Distributed systems** where "same inputs" is ambiguous
- **Garbage collection** of cache entries when disk fills

### Remote Build Caches

Share build artifacts across machines and CI environments. Output of task on machine A can be reused on machine B if inputs match.

**Caching semantics:**

- **Content-addressable caching**: key = hash(inputs). Enables sharing and deduplication.
- **Validation**: remote cache must enforce that identical inputs produce identical outputs (either through hermetic builds or explicit verification)
- **Push vs. read-through**: CI jobs push successful builds; local development reads

**Implementations:**

- **Bazel Remote Execution (RBE)**: execute tasks on remote workers; caching is automatic
- **Nx Cloud**: centralized artifact store for monorepos, shares across CI and local dev
- **Turborepo Remote Cache**: similar pattern for Node.js monorepos, integrates with Vercel
- **GitHub Actions caching**: simpler `key=input-hash` mechanism, less sophisticated isolation

**Trade-offs:**
- Remote cache reduces per-machine build time but adds network latency and infrastructure cost
- False cache hits (corrupted or incorrect artifacts remotely) are catastrophic; validation overhead is essential
- Privacy concerns: build artifacts may contain logs, intermediate code, or sensitive data

---

## Build Graph Parallelism

Topological ordering permits parallelism: tasks with no dependencies can execute simultaneously. Build systems parallelize via:

1. **Task-level parallelism**: Gradle's `--parallel`, Bazel's worker processes
2. **Machine-level parallelism**: Bazel RBE distributes tasks to remote workers
3. **Compilation-unit granularity**: languages with smaller compilation units (Go: packages, TypeScript: files with --incremental) expose more parallelism

Limits to parallelism:
- **Critical path**: longest dependency chain. If one path is much longer, other workers sit idle.
- **Lock contention**: shared resources (linker, single-threaded dependency resolution)
- **Overhead vs. speedup**: spawning workers has cost; worth it only for tasks >100ms

---

## Build System Persistence: Why Make Survives

**Unix Make** (1976) persists despite its flaws (non-standard syntax, implicit rules, fragile inference). Reasons:

1. **Universality**: available on every Unix variant; no dependencies
2. **Enough for small projects**: dependency tracking works for thousands of targets
3. **Integration**: part of language build conventions (C, C++, Go)
4. **Resistance to change**: build systems are "good enough," replacement cost is high

Modern alternatives (Bazel, Gradle, Nix, Buck) solve Make's problems but add complexity: requirement to declare all inputs/outputs, sandbox overhead, longer feedback loops. These are worthwhile for monorepos and enterprise scale; for single packages, Make is still rational.

---

## See Also

- **build-compilation-incremental.md** — Language-specific incremental strategies beyond DAG invalidation
- **build-bazel-gradle.md** — When reproducibility and remote execution matter: Bazel vs Gradle deep dive
- **architecture-monorepo.md** — How build systems enable monorepo scaling
- **process-dependency-management.md** — Dependency versions and lock files interact with build reproducibility