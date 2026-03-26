# Compiler Optimizations — Techniques, Trade-offs & Implementation

## Overview

Compiler optimizations improve programs by reducing execution time, memory use, or code size without changing observable behavior. Optimizations operate at multiple levels: IR-level (architecture-independent), target-specific (exploiting hardware features), and profile-guided (using runtime data).

The key insight: not all optimizations are profitable for all code. An optimization that saves 100 cycles on a hot loop matters; the same optimization on cold code wastes compilation time. Profitability depends on code properties, target architecture, and optimization budget.

## SSA Form (Static Single Assignment)

### Why SSA Matters

SSA is the canonical IR form for optimization. Each variable is assigned exactly once, making data-flow analysis trivial: the assignment site is literally the only source of a value.

```
# Non-SSA (ambiguous where x comes from)
x = 1
if (cond):
  x = 2
print(x)

# SSA (explicit join point)
x1 = 1
if (cond):
  x2 = 2
else:
  x2 = x1
x3 = φ(x2, x1)    # φ-function: x3 takes x1 or x2 depending on path
print(x3)
```

### Trade-offs

- **Pros**: Easy data-flow analysis, enables many optimizations (dead code elimination, constant propagation, value numbering)
- **Cons**: Renaming explosion (large code can have 10x variables), φ-function overhead in reconstruction

### Building SSA

Standard algorithm: iterate over CFG (control-flow graph) nodes in dominance order, rename variables, insert φ-functions at join points (nodes with multiple predecessors). Cost: O(n) in practice due to careful bookkeeping.

## Dead Code Elimination (DCE)

Code that doesn't affect program behavior is useless. DCE finds and removes it.

### Liveness Analysis

A value is **live** at a point if it can be used in some execution path afterward. A variable is **live-out** at a block if it flows to a successor.

Algorithm: backward pass over CFG:
```
varlive_in = gen ∪ (live_out - kill)
gen = variables used before reassignment
kill = variables reassigned
```

### Why SSA Helps

In SSA, a variable is dead if it has no **uses** (every assignment has a use list). Iteratively remove dead assignments and orphaned instructions.

### DCE vs. Live Dead Code

- **Dead code**: unreachable (orphaned blocks after optimizations)
- **Trivially dead code**: assignments to unused variables

Global DCE requires reachability analysis (fixed-point iteration over CFG).

## Constant Folding & Propagation

### Constant Folding

Evaluate expressions with known constant values at compile time:
```
x = 5 + 3        →  x = 8
y = x * 2        →  y = 16
```

In SSA: track each value as a constant lattice (⊥ = unknown, a value = constant, ⊤ = multiple values). Iterate until fixed point.

### Constant Propagation

Replace variable uses with constants if the only incoming value is constant:
```
x = 5
y = x + 2        →  y = 5 + 2   →  y = 7
```

Sparse conditional constant propagation (SCCP) respects control flow: a variable is only constant on paths where all dominating definitions are constant.

## Loop Optimizations

Loops are where most programs spend time. The performance gains justify aggressive optimization, despite complexity.

### Loop Unrolling

Replicate the loop body to reduce iterations and control-flow overhead:
```
# Original
for (int i = 0; i < 1000; i++) {
  sum += a[i];
}

# Unrolled 4x
for (int i = 0; i < 1000; i += 4) {
  sum += a[i];
  sum += a[i+1];
  sum += a[i+2];
  sum += a[i+3];
}
```

Benefits: fewer branch mispredictions, more instruction-level parallelism (ILP), better register allocation across unrolled iterations.

Cost: code bloat, larger instruction cache pressure on small kernels.

### Loop Fusion

Combine adjacent loops over the same range to improve cache locality:
```
for (i = 0; i < n; i++)
  a[i] = b[i] + c[i];
for (i = 0; i < n; i++)
  d[i] = a[i] * 2;

# Fused
for (i = 0; i < n; i++) {
  a[i] = b[i] + c[i];
  d[i] = a[i] * 2;
}  # a[i] stays in cache
```

Requires alias analysis to prove no data dependence between loops.

### Loop-Invariant Code Motion (LICM)

Move computations that don't depend on loop induction variables outside the loop:
```
for (i = 0; i < n; i++) {
  x = y + z;        # y, z don't change
  a[i] = x;
}

# Hoisted
x = y + z;
for (i = 0; i < n; i++) {
  a[i] = x;
}
```

Requires data-flow analysis to prove the computation is loop-invariant.

### Vectorization (SIMD)

Transform scalar loops into vector operations when possible:
```
for (i = 0; i < n; i++)
  c[i] = a[i] + b[i];

# Vectorized (SSE/AVX)
vaddps(c[i:i+4], a[i:i+4], b[i:i+4]);  # 4 floats in one instruction
```

Conditions: predictable memory access patterns, no data dependencies across iterations, aligned data.

## Inlining Heuristics

Replacing a function call with its body (inlining) eliminates call overhead but increases code size.

### Profitability Model

Inline if: `call_cost < body_cost * frequency`

- **Call cost**: registers saved/restored, branch misprediction (~50-200 cycles)
- **Body cost**: instruction count, including memory latencies
- **Frequency**: how often the call executes

### Preventing Runaway Growth

- **Inline budget**: stop inlining if cumulative code growth exceeds threshold (e.g., 10% above original)
- **Depth limit**: don't inline deep call chains (can cause stack explosion)
- **Call-site heuristics**: inline small functions unconditionally, medium functions based on frequency, large functions never

### Cold-code variants

Use profile feedback to cold-inline rarely-called functions (they're less expensive to keep uninlined).

## Escape Analysis

Determines if an object allocated in a function escapes (is reachable from outside):
```
Object obj = new Object();
return obj;           // Escapes
```

Non-escaping objects can be:
- **Stack-allocated** instead of heap-allocated (faster, cheaper GC)
- **Scalar-replaced** (promoted to individual variables)

Example (Java/JIT):
```
Object obj = new Object();
obj.x = 5;
int y = obj.x;

# Escape analysis proves obj doesn't escape
# Transforms to:
obj_x = 5;
int y = obj_x;
```

Reduces GC pressure and memory latency.

## Alias Analysis

Determines which pointers can refer to the same memory location. Critical for proving safety of reordering and DCE.

### Levels of Precision

1. **No-alias**: pointers definitely don't overlap (can always reorder)
2. **May-alias**: pointers might overlap (conservative, can't reorder)
3. **Must-alias**: pointers definitely refer to same location

### Classic Algorithms

- **Points-to analysis**: for each pointer, compute set of objects it may point to
- **Flow-sensitive**: value of pointer varies along control flow
- **Flow-insensitive**: treat all definitions as one abstract location (cheaper, less precise)

```
int *p, *q;
p = &x;
q = &y;      // p and q don't alias (must-not-alias is rare)
p = q;       // Now p may alias q
```

## Profile-Guided Optimization (PGO)

Use runtime profiles to make better optimization decisions.

### Workflow

1. Compile program with instrumentation (counters at basic blocks)
2. Run with representative input, collect profiles
3. Recompile, using profiles to inform:
   - Basic-block ordering (hot blocks laid out sequentially for cache)
   - Function inlining decisions (hot callsites inlined aggressively)
   - Code layout (cold functions separated)

### Benefits

- Inlining: avoid inlining rarely-called error paths
- Loop unrolling: unroll hot loops, skip cold ones
- Branch prediction: lay out branch targets correctly

### Trade-off: Representativeness

Profiles from workload A might not apply to workload B. Mismatch can cause regressions.

## Global Optimizations: Fixed-Point Iteration

Many optimizations require reaching a fixed point (applying repeatedly until no more changes):

```
repeat
  DCE (remove dead code)
  Constant propagation (simplify constants)
  LICM (hoist invariants)
until no changes
```

Cost: O(n² · passes) in worst case, but typically n · log n in practice with worklist algorithms.

## Trade-offs: When to Optimize

- **Startup**: minimize compile time → disable expensive opts
- **Steady-state**: maximize throughput → enable aggressive opts
- **Embedded**: code size matters → selective unrolling, no loop fusion
- **Debug**: preserve semantics for stepping and debugging → disable aggressive opts that break debugging

## See Also

- **SSA-based dataflow**: algorithms-concurrency, database-internals-query (similar fixed-point engines)
- **Hardware interaction**: hardware-cpu-architecture (memory hierarchy, branch prediction)
- **JIT compilation**: web-javascript-engine, runtime-v8 (dynamic PGO)