# Compiler Backend — IR, Optimization & Code Generation

## Why Intermediate Representations Exist

Compilers do not translate ASTs directly to machine code. The gap between source-level semantics and hardware-level operations is too wide to bridge in a single step. Intermediate representations (IRs) sit between the frontend and backend, providing a uniform substrate for analysis and transformation.

IRs serve multiple purposes:

- **Abstraction decoupling**: Language-specific constructs (classes, closures, pattern matching) are lowered into simpler operations that optimization passes can reason about uniformly.
- **Target independence**: The same IR is optimized before target-specific code generation, avoiding redundant optimization work per target architecture.
- **Analysis enablement**: Certain properties (data flow, aliasing, reachability) are easier to compute on IR than on source ASTs or raw machine code.

### IR Design Trade-offs

| IR Style       | Abstraction Level | Example             | Strengths                          |
| -------------- | ----------------- | ------------------- | ---------------------------------- |
| Tree-based     | High              | AST, BURS trees     | Natural for source-level semantics |
| Stack-based    | Medium            | JVM bytecode, WASM  | Compact, easy to generate          |
| Register-based | Medium-Low        | LLVM IR, Dalvik     | Closer to hardware, efficient exec |
| Graph-based    | Low-Medium        | Sea of Nodes, Graal | Explicit data/control flow         |

Many compilers use **multiple IR levels** — a high-level IR close to the source, lowered progressively to a low-level IR close to machine instructions. Each level strips away abstractions, making different optimizations natural at different stages.

### Three-Address Code

A common IR style where each instruction has at most three operands — one destination and two sources:

```
t1 = a + b
t2 = t1 * c
t3 = load [arr + t2]
store [result], t3
```

This form makes data flow explicit — each value is defined once and used one or more times. The number of temporaries is unlimited (virtual registers), deferring register allocation to a later phase.

## Static Single Assignment (SSA) Form

SSA is a property of an IR where every variable is assigned exactly once. When a variable would be assigned multiple times (e.g., in different branches of an `if`), a **phi function** (φ) merges the values at the join point.

```
Before SSA:                    After SSA:
x = 1                          x1 = 1
if (cond)                      if (cond)
    x = x + 1                      x2 = x1 + 1
else                           else
    x = x + 2                      x3 = x1 + 2
y = x * 3                     x4 = φ(x2, x3)
                               y1 = x4 * 3
```

### Why SSA Simplifies Optimization

With single assignment, each use of a variable has exactly one reaching definition. This makes def-use chains trivial — no need for expensive data flow analysis to determine which definition reaches which use. Optimizations that reason about values (constant propagation, dead code elimination, value numbering) become significantly simpler.

The trade-off: phi functions are artificial constructs that must eventually be lowered. During SSA destruction (typically during register allocation), phi functions become copies, which a subsequent copy coalescing pass tries to eliminate.

### Variants and Extensions

- **Pruned SSA**: Only inserts phi functions where actually needed, reducing IR size.
- **Gated SSA**: Replaces phi functions with gating functions that carry the branch condition, enabling certain optimizations that need to reason about control flow.
- **SSI (Static Single Information)**: Extends SSA to also handle information from branch conditions — after `if (x > 0)`, the true branch gets a renamed `x` known to be positive.

## The Optimization Pass Concept

Optimizations are structured as **passes** — small, focused transformations applied to the IR in sequence. Each pass reads the IR, performs analysis, transforms where legal and profitable, and produces updated IR for the next pass.

### The Pass Pipeline

```
IR → Simplify CFG → Constant Prop → Dead Code Elim → Inline → Loop Opts → Vectorize → ... → Optimized IR
```

Passes compose: inlining may expose new constants, constant propagation may reveal dead code, dead code elimination may simplify the control flow graph. This creates a feedback loop — running the same passes multiple times can produce further improvements.

### Profitability vs Legality

Every transformation must satisfy two criteria:

- **Legality**: The transformation preserves program semantics. Dead code elimination is legal because unreachable code, by definition, cannot affect the result. Reordering memory operations is only legal if no observable behavior changes.
- **Profitability**: The transformation improves some metric (speed, size, energy). Inlining a large function at every call site is legal but may degrade performance through instruction cache pressure.

Legality is typically analyzed through alias analysis (do these pointers overlap?), escape analysis (does this value outlive its scope?), and dependence analysis (do these operations have data or control dependencies?).

## Common Optimizations

### Constant Folding and Propagation

**Constant folding** evaluates expressions whose operands are all known at compile time: `3 + 4` becomes `7`. **Constant propagation** traces known values through the program: if `x = 5` and there is no redefinition before `y = x + 1`, then `y = 6`.

**Sparse conditional constant propagation (SCCP)** combines constant propagation with unreachable code detection. If a branch condition is a known constant, the false branch is unreachable, which may make further variables constant.

### Dead Code Elimination

Removes instructions whose results are never used. In SSA form, this is straightforward: an instruction with no uses (and no side effects) is dead. Iterative application handles chains of dead code — removing one dead instruction may make its operands' definitions dead.

**Unreachable code elimination** removes basic blocks that cannot be reached from the entry point. Distinct from dead code elimination, which removes reachable but useless instructions.

### Inlining

Replaces a function call with the body of the called function. Benefits include eliminating call overhead and exposing the callee's operations to the caller's optimization context. Costs include code size growth and increased register pressure.

Inlining decisions are among the most impactful in a compiler. Typical heuristics consider:

| Factor           | Favors inlining          | Discourages inlining |
| ---------------- | ------------------------ | -------------------- |
| Callee size      | Small functions          | Large functions      |
| Call frequency   | Hot call sites           | Cold call sites      |
| Caller context   | Known constant arguments | Generic arguments    |
| Recursion        | —                        | Recursive functions  |
| Code size budget | Under budget             | Over budget          |

### Loop Optimizations

Loops dominate execution time in many programs, making them high-value optimization targets.

**Loop-invariant code motion (LICM)** moves computations that produce the same result on every iteration out of the loop.

**Loop unrolling** replicates the loop body multiple times, reducing loop overhead (branch, counter increment) and enabling further optimizations across the unrolled iterations. Full unrolling (when iteration count is known and small) eliminates the loop entirely.

**Loop tiling (blocking)** restructures nested loops to improve cache locality by processing data in cache-sized blocks rather than sweeping through entire arrays.

**Loop fusion** combines adjacent loops over the same range into a single loop, improving data locality. **Loop fission** splits a loop into multiple loops, reducing register pressure.

### Vectorization

Transforms scalar operations into SIMD (single instruction, multiple data) instructions that process multiple data elements in parallel:

```
Scalar:                        Vectorized:
for i in 0..n:                 for i in 0..n step 4:
    c[i] = a[i] + b[i]            c[i:i+4] = a[i:i+4] + b[i:i+4]
```

**Auto-vectorization** analyzes loops for data parallelism. Success depends on: absence of loop-carried dependencies, aligned memory access patterns, and absence of control flow (or its conversion to predicated operations).

**SLP (Superword Level Parallelism)** vectorizes straight-line code by grouping independent isomorphic operations — multiple additions of different operands packed into one vector add.

## The Phase Ordering Problem

Optimization A may enable optimization B, but B's results may also enable A. Running A then B produces different results than B then A. With dozens of passes, the number of possible orderings is factorial.

No known general solution exists. Approaches include:

- **Fixed pipelines**: Carefully tuned orderings based on experience. Most production compilers use this approach.
- **Iterative compilation**: Run multiple orderings and pick the output with the desired properties. Expensive but effective for embedded systems where a single binary ships millions of units.
- **Machine learning**: Train models to predict effective orderings for given code patterns. An active research area with promising but not yet decisive results.

In practice, compilers define a small number of optimization levels (typically -O0 through -O3, -Os for size) with curated pass pipelines for each.

## Register Allocation

Virtual registers (unlimited temporaries in the IR) must be mapped to the finite physical registers of the target machine. This is one of the most performance-critical backend phases.

### Graph Coloring Formulation

Build an **interference graph** where nodes are virtual registers and edges connect registers that are simultaneously live (both hold values needed in the future). Assigning physical registers is equivalent to graph coloring — adjacent nodes get different colors, and the number of colors equals the number of physical registers.

Graph coloring with a fixed number of colors is NP-complete in general, but heuristics work well in practice:

- **Iterated register coalescing**: Alternates between simplification (removing low-degree nodes), coalescing (merging non-interfering copies), and spilling (selecting registers to store in memory when colors are insufficient).
- **Linear scan**: Processes live intervals in order, assigning registers greedily. Faster than graph coloring with somewhat worse results. Commonly used in JIT compilers where compilation speed matters.

### Spilling

When physical registers are insufficient, some values are **spilled** — stored to and reloaded from stack memory. Spilling decisions profoundly affect performance. Heuristics consider:

- **Spill cost**: How frequently is the value used? Spilling a value used in a tight loop is expensive.
- **Rematerialization**: Can the value be recomputed cheaply instead of stored/loaded? Constants and simple expressions are candidates.
- **Live range splitting**: Splitting a long live range into segments, some in registers and some spilled, can reduce overall spill cost.

## Instruction Selection

Instruction selection maps IR operations to target machine instructions. The challenge is that the mapping is many-to-many — a single IR operation may correspond to multiple machine instructions, and a single machine instruction may implement a complex pattern of IR operations.

### Approaches

**Macro expansion** replaces each IR instruction with a fixed sequence of machine instructions. Simple but misses multi-operation patterns. A multiply-and-add IR pattern would generate separate multiply and add instructions, missing a fused multiply-add machine instruction.

**Tree pattern matching** (BURS — Bottom-Up Rewrite System) tiles the IR expression trees with machine instruction patterns, selecting a minimal-cost covering. This captures multi-operation patterns effectively.

**DAG-based selection** extends tree matching to directed acyclic graphs, handling common subexpressions. SelectionDAG in some compilers uses this approach, though the complexity is substantially higher.

### Instruction Scheduling

After selection, instruction scheduling reorders instructions to maximize pipeline utilization and hide latencies. Two main phases:

- **Pre-register-allocation scheduling**: Maximizes parallelism, potentially increasing register pressure.
- **Post-register-allocation scheduling**: Respects physical register assignments, with less freedom to reorder.

The tension between scheduling and register allocation is fundamental — aggressive scheduling increases register pressure, aggressive register conservation constrains scheduling.

## Calling Conventions and ABI Compliance

The **Application Binary Interface (ABI)** defines how functions communicate at the machine level:

| Aspect             | What it defines                                      |
| ------------------ | ---------------------------------------------------- |
| Parameter passing  | Which arguments go in registers vs. stack, and which |
| Return values      | Where results are placed                             |
| Callee-saved regs  | Which registers a function must preserve             |
| Stack frame layout | Alignment, red zones, shadow space                   |
| Name mangling      | How symbols are encoded for the linker               |

ABI compliance is non-negotiable for interoperability — code compiled by different compilers (or different languages) must agree on conventions to call each other. Within a single compilation unit, the compiler may use non-standard conventions for internal calls (e.g., passing extra parameters in registers) when the callee is known.

## Link-Time Optimization (LTO)

Traditional compilation optimizes each translation unit (source file) independently. LTO defers optimization until link time, when the entire program's IR is available.

### Trade-offs

| Aspect               | Benefit                                            | Cost                              |
| -------------------- | -------------------------------------------------- | --------------------------------- |
| Inlining             | Cross-module inlining decisions                    | Full-program IR in memory         |
| Dead code            | Eliminate unreachable cross-module code            | Longer link times                 |
| Devirtualization     | Resolve virtual calls with whole-program knowledge | Breaks separate compilation model |
| Constant propagation | Global constants visible everywhere                | All objects rebuilt on any change |

**Thin LTO** mitigates the memory and time costs by performing cross-module analysis (importing summaries) without loading all IR simultaneously. Each module is optimized independently but with cross-module information available — a middle ground between per-file and whole-program optimization.

## JIT Compilation

Just-in-time compilers operate at runtime, compiling code (typically from bytecode or an IR) as it executes. This enables optimizations impossible for ahead-of-time (AOT) compilers:

- **Speculative optimization**: If profiling shows a virtual call always dispatches to one type, the JIT can inline that target and add a runtime guard. If the assumption breaks, the JIT **deoptimizes** — falls back to unoptimized code.
- **Type specialization**: In dynamically-typed languages, the JIT generates code specialized for observed types (`add_int_int` instead of generic `add`).
- **Adaptive tiering**: Code starts interpreted, then moves through compilation tiers as it gets hotter. Cold code avoids compilation costs entirely.

### The Warm-up Problem

JIT compilation imposes latency: code runs slowly until it is compiled and optimized. Strategies include:

- **Profiling-guided tiering**: Only compile code that executes frequently enough to amortize compilation cost.
- **Ahead-of-time pre-compilation**: Ship pre-compiled code alongside bytecode, falling back to JIT for hot paths.
- **Snapshot/restore**: Serialize the JIT's compilation state for fast startup.

## Profile-Guided Optimization (PGO)

PGO uses runtime profiles from representative workloads to inform compilation decisions:

1. **Instrumented build**: Compile with profiling instrumentation.
2. **Profile collection**: Run the instrumented binary with representative inputs.
3. **Optimized build**: Recompile using the collected profile data.

### What Profiles Enable

| Profile data           | Optimization enabled                              |
| ---------------------- | ------------------------------------------------- |
| Branch frequencies     | Layout hot paths as fall-through, cold paths away |
| Call frequencies       | Better inlining decisions                         |
| Loop trip counts       | Unrolling decisions, vectorization hints          |
| Value profiles         | Speculative devirtualization, switch optimization |
| Memory access patterns | Prefetch insertion, data layout optimization      |

The profile is representative only of the training workload. If production behavior diverges significantly from the training run, PGO may degrade performance. Continuous profiling in production (feeding back into the next compilation) addresses this but adds infrastructure complexity.

**Sampling-based PGO** avoids the instrumented build step by using hardware performance counters (e.g., perf, Intel VTune) to sample at low overhead. The profiles are less precise but require no recompilation.

## Compilation Speed vs Code Quality

The tension between time spent compiling and quality of the generated code is a central design axis.

| Level | Typical name | Compilation speed | Code quality | Use case                     |
| ----- | ------------ | ----------------- | ------------ | ---------------------------- |
| 0     | -O0          | Fastest           | Lowest       | Debug builds, fast iteration |
| 1     | -O1          | Fast              | Moderate     | Development with some opt    |
| 2     | -O2          | Moderate          | High         | Release builds               |
| 3     | -O3          | Slow              | Highest      | Performance-critical code    |
| s     | -Os          | Moderate          | Size-focused | Embedded, mobile             |

Debug builds (-O0) disable most optimizations to ensure source-level debugging works: every variable is in memory, every source line maps to instructions, execution order matches source order. This can be 2-10x slower than optimized code.

**Build system design** often splits the difference: optimize library dependencies at -O2 (done once) while compiling project code at -O0 (done frequently during development). LTO and incremental compilation further amortize costs.

## Debug Information

Debug information maps optimized machine code back to source constructs — line numbers, variable locations, type descriptions. Formats include DWARF (Unix/macOS), PDB (Windows), and CodeView.

### The Debugging-Optimization Tension

Optimizations destroy the clean mapping between source and machine code:

- **Inlining** means a machine instruction may correspond to source in a different function.
- **Code motion** moves instructions away from their source location, making single-stepping confusing.
- **Register allocation** means a variable may be in different registers (or memory) at different points, or optimized away entirely.
- **Dead code elimination** removes variables the programmer expects to inspect.

Compilers address this with:

- **Location lists**: A variable's location is described as a list of (address range, location) pairs — "from address 0x100 to 0x120, x is in register R3; from 0x120 to 0x140, it's at [RSP+8]."
- **Inlining records**: Chains of inlining context — "this instruction is from foo(), inlined into bar(), inlined into main()."
- **Salvage operations**: When an optimization would drop a debug value, attempt to express it in terms of surviving values — "x was y+1, and y is still available."

### Variable Availability

In optimized code, inspecting a variable in a debugger may show "optimized out" — the value exists nowhere at that program point. This is a fundamental consequence of optimization, not a compiler bug. The trade-off between debuggability and performance has no universal answer — it depends on whether the code is being actively debugged or deployed in production.

Some compiler designs offer intermediate optimization levels that preserve selected debug information while still enabling impactful optimizations. The exact behavior and guarantees vary across implementations and target platforms.

## Target-Specific Considerations

### Instruction Set Architecture (ISA) Impact

The target ISA shapes many backend decisions:

| ISA characteristic | Backend impact                                    |
| ------------------ | ------------------------------------------------- |
| Register count     | Spill frequency, calling convention complexity    |
| SIMD width         | Vectorization strategy, loop residue handling     |
| Branch prediction  | Code layout, speculative execution considerations |
| Memory model       | Fence insertion for concurrent code               |
| Addressing modes   | Instruction selection patterns                    |

CISC architectures (x86) offer complex instructions that fold multiple operations, making instruction selection more impactful. RISC architectures (ARM, RISC-V) have simpler instructions where scheduling is more critical.

### Cross-Compilation

When the host architecture differs from the target, the compiler cannot execute target code for constant evaluation, cannot use target-specific features of the host, and must carry target-specific knowledge (instruction costs, pipeline characteristics) as explicit data rather than intrinsic behavior.

Cross-compilation is the norm for embedded systems, mobile development, and WebAssembly targets. The abstraction boundaries between target-independent optimization and target-specific code generation become critical in this context — a clean separation means adding a new target requires implementing only the target-specific phases.
