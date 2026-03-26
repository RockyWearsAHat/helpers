# Build Systems — Concepts, Dependency Graphs & Incremental Compilation

## What Build Systems Do

A build system transforms source artifacts into output artifacts through a series of dependency-ordered steps. At the most fundamental level, every build system models the same core abstraction: inputs, transformations, and outputs connected by dependency relationships.

The simplest mental model is a function: `build(sources, config) → artifacts`. The complexity arises because real projects have thousands of sources, dozens of transformation types, and intricate dependency relationships between them. Build systems exist to make this process correct (right order, no missing deps), efficient (minimal rework), and reproducible (same inputs → same outputs).

| Concern         | What It Means                                 | Why It's Hard                                              |
| --------------- | --------------------------------------------- | ---------------------------------------------------------- |
| Correctness     | Every target built after its dependencies     | Circular deps, implicit deps, generated sources            |
| Efficiency      | Only rebuild what changed                     | Detecting what "changed" means varies by context           |
| Reproducibility | Same source → same artifact, anywhere         | Host environment leaks, timestamp embedding, floating deps |
| Scalability     | Build time stays manageable as codebase grows | Dependency graph analysis, caching, parallelism            |

## Dependency Graphs and Topological Ordering

Build systems internally represent work as a directed acyclic graph (DAG). Nodes represent build targets or actions; edges represent "depends on" relationships.

```
         app.exe
        /       \
   main.o      lib.o
      |           |
   main.c      lib.c
      \         /
       config.h
```

Topological sorting of this graph yields a valid build order — any ordering where every node appears after all its dependencies. Multiple valid orderings typically exist, which creates opportunities for parallelism.

**Key properties of build DAGs:**

- **Acyclicity is mandatory.** A cycle means A depends on B depends on A — no valid build order exists. Build systems detect and reject cycles, though some handle specific cyclic patterns (like mutually recursive modules) through special mechanisms.
- **Transitive dependencies propagate.** If A depends on B and B depends on C, changes to C can invalidate A even though A doesn't directly reference C.
- **Diamond dependencies** occur when two paths lead to the same node. The build system must ensure the shared dependency is built exactly once.
- **Dynamic dependencies** complicate the graph — some edges aren't known until earlier build steps execute (e.g., a C compiler discovering `#include` directives). Build systems handle this through dependency scanning, depfiles, or conservative over-approximation.

## Incremental Builds

The central optimization problem: given that some inputs changed, determine the minimum set of build steps to re-execute. Two fundamental approaches exist for detecting changes.

### Timestamp-Based Invalidation

Compare modification times of inputs and outputs. If any input is newer than the output, rebuild.

**Advantages:** Fast to check (single stat call per file), universally supported by filesystems, simple mental model.

**Limitations:**

- Clock skew between machines breaks distributed builds
- Touching a file without changing content triggers unnecessary rebuilds
- Generated files with embedded timestamps create non-determinism
- Copying files may or may not preserve timestamps depending on the tool
- Sub-second granularity varies across filesystems (FAT32: 2-second resolution)

### Content-Based Invalidation

Hash file contents (typically SHA-256 or similar) and compare hashes. Rebuild only when content actually changes.

**Advantages:** Immune to clock skew, avoids rebuilds when content unchanged (e.g., reformatting that produces the same output), works correctly across machines.

**Limitations:**

- Hashing large files takes measurable time
- Requires storing hash databases
- Initial cold build must hash everything
- Some outputs are non-deterministic even with identical inputs (timestamps in PE headers, random UUIDs in metadata)

| Factor           | Timestamps                                                 | Content Hashing                      |
| ---------------- | ---------------------------------------------------------- | ------------------------------------ |
| Speed of check   | O(1) stat call                                             | O(n) where n = file size             |
| Correctness      | Can false-positive (touch) and false-negative (clock skew) | Precise if outputs are deterministic |
| Cross-machine    | Fragile                                                    | Robust                               |
| Storage overhead | None                                                       | Hash database                        |
| Best suited for  | Local development iteration                                | CI, distributed builds, caching      |

Many build systems use a hybrid: timestamps for fast initial checks, falling back to content hashing when timestamps suggest a change.

## Build Reproducibility

A build is reproducible when identical source inputs always produce bit-for-bit identical outputs, regardless of when, where, or by whom the build runs.

**Sources of non-determinism:**

- **Timestamps embedded in outputs** — many compilers, archivers, and packaging tools embed build timestamps. PE/COFF headers, ZIP entry timestamps, `__DATE__`/`__TIME__` macros.
- **File ordering** — iterating directories or hash maps yields different orders on different systems or runs. Archive tools that walk directories produce non-deterministic output.
- **Parallelism-induced ordering** — linking objects in completion order rather than a fixed order.
- **Floating dependencies** — `apt install python3` resolves to different versions over time. Unpinned package versions make builds time-dependent.
- **Absolute paths** — embedding `/home/user/project/src/main.c` in debug info makes the build machine-dependent.
- **Random seeds** — some code generation tools use random values without fixed seeds.

**Why reproducibility matters:**

- **Security auditing** — verify that a binary was produced from claimed source code
- **Debugging** — reproduce the exact binary a user is running
- **Caching** — content-addressed caching only works when same inputs → same outputs
- **Regulatory compliance** — some industries require build provenance

Achieving full reproducibility is a spectrum, not a binary state. Many projects pursue "practical reproducibility" — deterministic enough for caching and debugging without eliminating every last byte difference.

## Hermetic Builds

A hermetic build is isolated from the host environment. It does not depend on tools, libraries, or configurations installed on the build machine beyond the build system itself.

**What hermeticity prevents:**

- "Works on my machine" — builds depending on locally installed tool versions
- Implicit host dependencies — builds silently linking against system libraries
- Environment variable leakage — `PATH`, `HOME`, `LANG` affecting output

**Hermeticity mechanisms:**

- **Toolchain fetching** — the build system downloads specific compiler/SDK versions rather than using whatever's installed
- **Sandboxed execution** — build actions run in restricted environments without access to the broader filesystem
- **Explicit dependency declaration** — every input must be declared; undeclared reads fail
- **Containerized builds** — running build steps inside containers with controlled base images

The trade-off: hermeticity adds setup complexity and may slow down developers who want to use their preferred local tools. Fully hermetic builds sometimes conflict with IDE integrations that expect standard tool locations.

## Build Caching

### Local Caching

Store build outputs keyed by a hash of their inputs. When the same input hash appears again, copy the cached output instead of rebuilding.

```
cache_key = hash(compiler_version, flags, source_hash, dep_hashes)
if cache[cache_key] exists:
    copy cache[cache_key] → output    # cache hit
else:
    run compilation → output
    store output → cache[cache_key]   # populate cache
```

Local caches accelerate common workflows: switching branches, reverting changes, rebuilding after `clean`.

### Remote / Distributed Caching

Share a cache across a team or CI infrastructure. When developer A builds a target, the result is uploaded to a shared cache. Developer B, building the same target with the same inputs, downloads the cached result instead of rebuilding.

**Considerations:**

- **Network latency vs build time** — caching only helps when download time < rebuild time. Small, fast compilations may be cheaper to redo locally.
- **Cache eviction** — storage isn't infinite; LRU or TTL policies determine what stays cached.
- **Trust** — consuming cached artifacts from others requires trusting their build environment. Hermetic builds make this safer.
- **Cache key design** — too narrow (including irrelevant inputs) reduces hit rates; too broad (omitting relevant inputs) returns wrong results. Getting the key exactly right is one of the hardest problems in build caching.

### Content-Addressable Storage

Some caching systems use content-addressable storage (CAS) — artifacts are stored by the hash of their content. This naturally deduplicates identical outputs and enables integrity verification.

## Parallel Build Execution

Dependency graphs expose natural parallelism: independent nodes can execute simultaneously.

### Task-Level Parallelism

Execute independent build actions (compilation of different files, independent test suites) in parallel.

```
# These three compilations have no dependency relationship
# and can run simultaneously:
compile(a.c → a.o)  ─┐
compile(b.c → b.o)  ─┼─→  link(a.o, b.o, c.o → app)
compile(c.c → c.o)  ─┘
```

The maximum parallelism is bounded by the critical path — the longest chain of sequential dependencies in the graph. No amount of parallelism makes the build faster than the critical path.

### File-Level vs Action-Level Granularity

- **Coarse granularity** (build a library, run all tests) — less scheduling overhead but fewer parallelism opportunities
- **Fine granularity** (compile one file, run one test) — more parallelism but higher scheduling and I/O overhead

The optimal granularity depends on the project: compute-bound builds (heavy template / macro expansion) benefit from fine parallelism; I/O-bound builds may suffer from too many parallel disk operations.

### Resource Management

Parallel builds compete for CPU, memory, disk I/O, and network. Over-subscribing causes thrashing:

- `-j$(nproc)` saturates CPU cores but may exhaust RAM with heavy compilations
- Linking is often memory-intensive; unlimited parallel linking can OOM
- Build systems sometimes support "resource pools" — limiting concurrency for specific resource-intensive action types

## The Monorepo Build Challenge

In a monorepo, thousands of projects share one repository. Naive "build everything" approaches don't scale.

**Affected target detection:** Given a set of changed files, determine which build targets could be affected. This requires analyzing the build graph to find all transitive dependents of the changed files. Only those targets (and their dependencies) need building and testing.

```
changed: lib/auth/token.go
affected: lib/auth/*, service/api/*, service/gateway/*  (transitive dependents)
not affected: service/billing/*, lib/logging/*           (no dependency path)
```

**Build graph query language:** Large monorepo build systems often provide query mechanisms — "what depends on X?", "what does X depend on?", "what's the dependency path between A and B?" These queries support both build optimization and impact analysis.

**Challenges specific to scale:**

- Parsing the build graph itself takes time at very large scale
- Fine-grained dependency tracking is essential — package-level granularity causes over-building
- Shared libraries changing frequently can create "rebuild the world" cascading invalidations
- Build configuration changes (compiler flags, toolchain versions) can invalidate everything

## Declarative Rules vs Programmatic Build Scripts

### Declarative / Rule-Based Model

Define targets, dependencies, and transformation rules. The build system determines execution order and incrementality.

```makefile
# Conceptual declarative rule
%.o: %.c
    $(CC) $(CFLAGS) -c $< -o $@

app: main.o lib.o
    $(CC) $^ -o $@
```

**Character:** Easy to analyze statically. The build system can reason about the whole graph before executing anything. Parallelism, caching, and remote execution become possible because the system understands the structure.

**Trade-off:** Complex build logic (conditional compilation, generated sources, platform-specific behavior) strains declarative models. Workarounds can become more complex than the equivalent procedural code.

### Programmatic / Imperative Model

Build scripts are programs that call build APIs. Execution order is explicitly coded.

```python
# Conceptual imperative build script
sources = glob("src/*.c")
for src in sources:
    obj = compile(src, flags=CFLAGS)
    objects.append(obj)
app = link(objects, output="app")
```

**Character:** Full programming language expressiveness. Complex logic is natural. Easy for developers familiar with the language.

**Trade-off:** Harder for the build system to analyze statically. Incremental builds require explicit invalidation logic or framework support. Parallelism must be explicitly structured or inferred through analysis of the script's execution.

### Hybrid Approaches

Many modern build systems blend both: declarative target definitions with escape hatches for programmatic logic (custom rules, macros, generators). The tension between "analyzable" and "expressive" is fundamental.

## Build Performance Optimization

### Cold Build

No cache, building everything from scratch. Optimization approaches:

- **Precompiled headers** — parse shared headers once, reuse the parse result across compilation units
- **Unity/jumbo builds** — concatenate multiple source files into fewer compilation units to reduce per-file overhead (includes, template instantiation)
- **Distributed compilation** — spread compilation across multiple machines
- **Compiler selection** — different compilers have different speed/optimization trade-offs at various optimization levels

### Incremental Build

Most developer iterations. Optimization focuses on:

- **Minimal invalidation** — only rebuild what actually depends on the change
- **Header dependency tracking** — distinguish between interface changes (public headers) and implementation changes (source files)
- **Module-level boundaries** — changes to a module's internals shouldn't rebuild dependents if the interface didn't change
- **Persistent worker processes** — keep compilers/tools running between builds to avoid startup costs

### Build Profiling

Understanding where build time goes:

- **Critical path analysis** — which chain of sequential dependencies determines total build time?
- **Action-level timing** — which individual build steps are slowest?
- **Bottleneck identification** — is the build CPU-bound, I/O-bound, or memory-bound?
- **Dependency fan-out** — which targets have the most dependents (and thus cause the most rebuilding when they change)?

## Artifact Management

Build systems produce artifacts: binaries, libraries, packages, container images, documentation. Artifact management concerns:

- **Versioning** — associating artifacts with the source revision and build configuration that produced them
- **Storage** — where artifacts live (local filesystem, artifact repositories, object storage)
- **Retention** — how long to keep old artifacts; balancing disk cost against the ability to roll back
- **Provenance** — recording what inputs, tools, and environment produced an artifact (supply chain security)
- **Distribution** — getting artifacts to where they're consumed (deployment targets, package registries, CDNs)

## Cross-Compilation

Building for a different target platform than the host machine. Adds complexity layers:

- **Host vs target distinction** — tools run on the host; outputs run on the target. Some build steps may produce tools used by later build steps (bootstrapping), complicating the graph.
- **Sysroots** — a directory containing headers and libraries for the target platform, used by the cross-compiler
- **Architecture-conditional logic** — source code, compiler flags, and linked libraries may differ per target
- **Testing** — native tests run directly; cross-compiled tests need emulation or target hardware

Cross-compilation is especially relevant for embedded systems, mobile development, and producing platform-specific binaries from CI.

## Build System Evolution

Build systems tend to evolve along a trajectory as projects grow:

| Stage               | Typical Approach                              | Pain Point That Drives Change                  |
| ------------------- | --------------------------------------------- | ---------------------------------------------- |
| Small project       | Shell scripts, manual commands                | Forgetting steps, wrong order                  |
| Medium project      | Make-like declarative rules                   | Incremental correctness, platform portability  |
| Large project       | Build system with content hashing, sandboxing | Build time, caching efficiency                 |
| Very large monorepo | Custom/specialized build systems              | Graph analysis at scale, distributed execution |

No single point on this spectrum is "correct" — the appropriate build system depends on project size, team size, language ecosystem, and deployment requirements. Over-engineering build infrastructure for a small project wastes effort; under-investing for a large one wastes developer time on every build.
