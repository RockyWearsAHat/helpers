# Security Fuzzing — Coverage-Guided, Grammar-Based, and Protocol Approaches

## Overview

Fuzzing is automated software testing that injects semi-random, mutated inputs into a program to trigger crashes, hangs, or undefined behavior. Unlike traditional testing (which assumes known inputs), fuzzers generate novel input combinations that developers may never anticipate. Fuzzing bridges the gap between unit tests and real-world attack surface by exploring high-dimensional input spaces programmatically.

## Coverage-Guided Fuzzing

Coverage-guided (or coverage-directed) fuzzing measures how much code is exercised and prioritizes mutations that reach new code paths. This dramatically reduces the search space compared to dumb fuzzing.

### How Coverage Guidance Works

1. **Instrument the binary** — Insert counters at edges or basic blocks
2. **Track coverage** — Record which code paths were hit by each input
3. **Maintain a corpus** — Store "interesting" inputs that trigger unique paths
4. **Mutate corpus** — Generate variants of interesting inputs
5. **Prioritize mutations** — Favor mutants that increase coverage

The key insight: each new code path could introduce a vulnerability, so maximizing coverage increases the likelihood of finding bugs.

### AFL++ (American Fuzzy Lop)

**AFL++** is the most widely deployed open-source coverage-guided fuzzer. It instruments binaries via compile-time or runtime techniques and prioritizes mutations via a deterministic algorithm.

- **Instrumentation**: Compile with `afl-clang-fast` or use LLVM plugin
- **Collects edge coverage**: Tracks 64KB coverage bitmap; hash collisions are acceptable for performance
- **Mutation strategies**: Bit flips, byte flips, arithmetic operations, interesting values, dictionary-based mutations, havoc (random permutations)
- **Deterministic vs. havoc**: Deterministic phase exhaustively tries simple mutations; havoc applies random, aggressive changes
- **Fast feedback loop**: Processes thousands of inputs per second
- **Corpus synchronization**: Multiple instances can share corpus for parallel fuzzing

AFL++ also supports **QEMU mode** (for closed-source binaries), **libAFL** (Rust library for custom fuzzers), and **AFL-utils** for orchestrating multiple fuzzing campaigns.

### libFuzzer

**libFuzzer** is LLVM's coverage-guided fuzzing infrastructure, designed for in-process fuzzing (no fork overhead). Widely adopted in Google's **OSS-Fuzz** project.

- **Stateless harness** — Function signature: `extern "C" int LLVMFuzzerTestOneInput(const uint8_t *Data, size_t Size)`
- **No fork overhead** — Runs inside the same process; fast but must handle crashes/hangs carefully (use timeouts, signal handlers)
- **Coverage** — Built on top of LLVM instrumentation; feedback via `__sanitizer_cov_*` callbacks
- **Corpus management** — Maintains seed corpus + generated corpus; outputs crashes and unique coverage-increasing inputs
- **Artifact size minimization** — Automatically reduces crashing inputs to minimal form
- **Synchronization support** — Works with multiple parallel fuzzers (shared corpus directories)

libFuzzer is the standard for C/C++ projects; tight integration with Address Sanitizer, Undefined Behavior Sanitizer, and Memory Sanitizer.

### Honggfuzz

**Honggfuzz** emphasizes reproducibility and security research applications. Key differentiators:

- **Multi-threaded processing** — Uses threads (not fork) for parallelism
- **Feedback mechanisms**: Edge coverage, value profiling (interesting constants), stack-trace similarity
- **Persistent mode** — Keep process alive across test cases (lower overhead than fork)
- **Linux and macOS support** — Also supports Android (through binary instrumentation for closed-source binaries)
- **Regression detection** — Maintains a log of discovered crashes; useful for continuous integration

Honggfuzz excels in scenarios where binary reproducibility and determinism are critical (e.g., vulnerability research).

## Grammar-Based Fuzzing

Coverage-guided fuzzing works best on binary formats (images, code) where mutations often produce syntactically invalid inputs that fail before interesting logic. Grammar-based fuzzers generate syntactically valid inputs according to a grammar specification.

### Structure and Validity

Instead of mutating raw bytes, grammar fuzzers:
- Define a grammar (context-free or BNF-like specification)
- Generate inputs that parse according to that grammar
- Apply mutations that preserve grammar structure

**Example**: For JSON, the grammar ensures outputs are valid JSON objects, not random bytes.

### Tools and Approaches

- **libFuzzer with custom mutators** — Define a `LLVMFuzzerCustomMutator` that applies grammar-aware changes
- **Grammar-based generators** — AFL++ includes a `grammar_fuzzer` option (experimental); other tools include grammarinator, lang-fuzz
- **Fuzzing harnesses** — Parser generators (e.g., ANTLR) can seed grammars

Grammar-based fuzzing is effective for file formats (XML, PDF, protobuf), protocol implementations, and compilers.

## Protocol and API Fuzzing

Protocol fuzzing targets network protocols (HTTP, DNS, TLS, gRPC) and APIs. The challenge: protocols have state machines; pure random inputs rarely reach interesting states.

### Stateful Fuzzing

- **State tracking** — Fuzzer models protocol handshakes and maintains context
- **Valid message sequences** — Generates sequences of valid messages in correct order
- **Constraint satisfaction** — Some fields depend on prior messages (e.g., TLS sequence numbers)
- **Tools**: Peach Fuzzer, Boofuzz, APIFuzzer

### API Fuzzing

For REST/GraphQL APIs:
- **Schema-aware** — Use OpenAPI/GraphQL schema to generate syntactically valid requests
- **Endpoint discovery** — Enumerate or extract endpoint definitions from RAML/Swagger
- **Parameter mutation** — Vary required fields, optional parameters, authentication headers
- **Tools**: REST-assured, Dredd, OWASP ZAP's fuzzing mode, custom harnesses with schema validators

## Corpus Management

The corpus is the set of seed inputs used to kick off fuzzing. A good corpus accelerates coverage discovery; a bad corpus can starve the fuzzer.

### Seed Selection

- **Minimize initial corpus** — Smaller, diverse seeds are better than large redundant ones (faster exploration)
- **Edge case seeds** — Include boundary values, empty inputs, maximum-length inputs
- **Real-world examples** — Actual valid files/messages (for formats, protocols)
- **Real bugs** — Inputs that triggered past crashes (regression prevention)

### Corpus Reduction and Maintenance

- **Delta minimization** — Remove corpus entries that don't increase coverage
- **Seeding new campaigns** — Use outputs from previous fuzzing runs as seeds (progressive deepening)
- **Crash deduplication** — Group crashes by stack trace to identify unique bugs

### Distributed Fuzzing

Multiple fuzzer instances running in parallel:
- **Central corpus coordinator** — Shared directory or service (e.g., AFL++'s sync_dir)
- **Propagation** — Interesting inputs discovered by one instance shared with others
- **Round-robin corpus** — Each instance maintains unique portions to avoid redundant work

## Crash Triage and Deduplication

Fuzzers generate hundreds or thousands of crashes. Not all are unique bugs; many are duplicates.

### Stack Trace Hashing

- **Normalized stack traces** — Extract function names, strip addresses/line numbers that vary across runs
- **Hash bucketing** — Group crashes by hash (same hash = likely same bug)
- **False negatives** — Different code paths triggering the same underlying bug may hash differently

### Semantic Clustering

- **Root cause analysis** — Trace crash to the instruction (use debugger, sanitizer output)
- **Write-what-where** — Memory corruption bugs; identify the pointer and value written
- **Null dereference** — Identify which pointer was NULL
- **Integer overflow** — Identify which operation overflowed and its impact

### Reproducibility

- **Input minimization** — Use AFL++ or libFuzzer's built-in minimization; sometimes manual (binary search over input)
- **Deterministic reproduction** — Disable ASLR if needed; use fixed RNG seeds
- **Shallow vs. deep bugs** — Heap corruption may manifest far from the bug (allocator-dependent); use MSAN or Valgrind for clarity

## Sanitizer Integration

Sanitizers detect memory and concurrency errors that might not crash the program.

### Widely Used Sanitizers

| Sanitizer | Detects | Overhead |
|-----------|---------|----------|
| Address Sanitizer (ASAN) | Buffer overflows, use-after-free, double-free, memory leaks | ~2x slowdown |
| Memory Sanitizer (MSAN) | Uninitialized memory reads | ~3x slowdown, limited to instrumented code |
| Undefined Behavior Sanitizer (UBSAN) | Signed overflow, division by zero, out-of-bounds shifts, null dereference | ~1x slowdown |
| Thread Sanitizer (TSAN) | Data races, some deadlocks | ~5-15x slowdown |

**Compile flags** — Most fuzzers auto-enable sanitizers via LLVM instrumentation:
```bash
-fsanitize=address,undefined
```

### Crash Detection

- **Sanitizer signal** — Kills process on first detected error (fail-fast)
- **Error reports** — Sanitizers write detailed diagnostics (variable names, allocation history, exact instruction)
- **Suppression files** — Can suppress known benign errors (leaks in third-party libraries)

## Fuzzing CI/CD Integration

### Pipeline Placement

1. **Local fuzzing** — Developer runs fuzzer locally before committing (fast, shallow)
2. **PR fuzzing** — Brief fuzzing run on pull requests (24 hours); flags regressions
3. **Continuous fuzzing** — Always-on cluster running fuzzers against latest code
4. **OSS-Fuzz integration** — Google's managed service for open-source projects

### Metrics and Reporting

- **Coverage trends** — Track coverage growth over time (should asymptote)
- **Crash discovery rate** — How many unique crashes per day (indicates saturation)
- **Time-to-crash** — How long fuzzing ran before finding first bug (lower = earlier discovery)
- **Corpus growth** — Number of interesting inputs (explosion = poor feedback; plateau = good)

### Maintenance Challenges

- **Flaky crashes** — Non-deterministic bugs require multiple runs to reproduce; use libFuzzer's `seed=` flag
- **Timeout tuning** — Too aggressive timeouts mask real hangs; too lenient allows infinite loops
- **Regression prevention** — Preserve crashing inputs in test suite

## Differential Fuzzing

Differential fuzzing runs multiple implementations (or versions) of the same spec and compares outputs. Divergence indicates a bug.

### Applications

- **Compiler testing** — Run multiple optimization levels on same code; compare results (Csmith, Yarpgen)
- **Protocol implementations** — Fuzz client and server separately; compare state
- **Library versions** — Fuzz old and new versions; flag semantic changes
- **Format parsers** — Parse same reference file with multiple decoders; detect inconsistencies

### Challenges

- **Non-determinism** — Real-world programs may behave differently (randomness, threading, system dependencies)
- **Equivalence checking** — Defining "same output" is nontrivial (floating-point rounding, human-readable formats)
- **Performance** — Running multiple implementations multiplies fuzzing cost

## Anti-Patterns and Pitfalls

**Shallow corpus coverage** — Fuzzer spends all time on early code paths; corpus not diverse enough. Often resolved by seeding with real-world examples or using grammar-based generation.

**Slow test harness** — Fuzzer stalls waiting for I/O, complex setup. Optimize hot path; consider persistent mode (libFuzzer, Honggfuzz).

**Unreachable code** — Coverage never increases; fuzzer may be blocked by authentication, checksums, or magic bytes. Weaken validation in test harness or provide helper functions.

**False positives** — ASAN reports leaks in third-party libraries or OS-managed memory. Use suppression files; verify via manual inspection or alternative sanitizers.

**Over-tuning parameters** — Spending time optimizing dictionary, timeouts, or mutation weights yields diminishing returns. Profile with built-in metrics first.

## See Also

- security-best-practices, security-devsecops (integration into development workflows)
- quality-static-analysis (complementary approach)
- antipatterns-hall-of-infamy (common mistakes in security testing)