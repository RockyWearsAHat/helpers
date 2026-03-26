# LLVM Ecosystem — Architecture, IR, Passes, and Toolchain

## Overview

LLVM (Low-Level Virtual Machine) is a modular compiler infrastructure designed for retargetability. A program is compiled once to LLVM IR, which is then optimized uniformly, then lowered to any supported target. This design enables rapid target support (30+ architectures) and cross-cutting optimizations.

Architecture: **Frontend (Clang) → LLVM IR → Optimization Passes → Backend → Object Code → Linker (LLD)**

## LLVM IR (Intermediate Representation)

### Design Principles

- **Language-independent**: IR is agnostic to source language (C, Rust, Swift, etc.)
- **Target-independent**: IR works on all targets before lowering
- **Three-level hierarchy**: Modules > Functions > Basic Blocks > Instructions
- **SSA form**: all values are assigned exactly once

### IR Example

```llvm
; Function definition
define i32 @add(i32 %a, i32 %b) {
entry:
  %sum = add i32 %a, %b      ; %sum is SSA value
  ret i32 %sum
}

; Function call
@main = define i32 @main() {
entry:
  %result = call i32 @add(i32 5, i32 3)
  ret i32 %result
}
```

### Type System

LLVM IR is **strongly typed**. Every value has a type:

```llvm
i1, i8, i16, i32, i64          ; Integer types
float, double                  ; Floating-point
[N x type]                      ; Array
{type1, type2, ...}           ; Struct
type*                          ; Pointer (deprecated → opaque pointers)
```

Modern LLVM (15+) uses **opaque pointers**: `ptr` instead of `i32*`, `i8*`. This simplifies the IR and reduces redundancy.

### Instructions

IR instructions map closely to hardware:

```llvm
add, sub, mul, div, rem        ; Arithmetic
and, or, xor, shl, ashr, lshr  ; Bitwise
load, store                    ; Memory
br, call, ret                  ; Control flow
phi                            ; SSA-form merge
```

## Optimization Passes

LLVM uses a **pass manager** that orchestrates sequential or parallel execution of analysis and transformation passes.

### Pass Organization

- **Analysis passes**: compute properties without modifying IR (e.g., alias analysis, dominance tree)
- **Transformation passes**: modify IR based on analysis results
- **Invalidation**: some passes invalidate analyses; manager rebuilds invalidated ones

### Classic Passes

#### Scalar Optimizations

- **Dead Code Elimination (DCE)**: remove unused assignments
- **Constant Folding / Propagation (SCCP)**: simplify constants
- **Global Value Numbering (GVN)**: eliminate redundant computations
- **Common Subexpression Elimination (CSE)**: recognize `x + y` computed twice
- **Instruction Combining**: fuse adjacent instructions (`(a + b) + c` → `a + (b + c)` for associative ops)
- **Loop Invariant Code Motion (LICM)**: hoist loop-invariant computations

#### Loop Optimizations

- **Loop Unrolling**: replicate loop body to reduce branches, enable ILP
- **Loop Vectorization**: transform scalar loops into SIMD operations
- **Loop Interchange**: reorder nested loops to improve cache locality
- **Loop Fusion**: combine adjacent loops
- **Induction Variable Simplification**: recognize induction variables, remove redundant updates

#### Interprocedural Optimizations

- **Inlining**: inline small functions at call sites
- **Function Specialization**: create copies of functions for specific argument patterns
- **Link-time optimization (LTO)**: inline and DCE across compilation boundaries

#### Target-Specific Passes

- **Machine Code Analysis**: track register allocation, latencies
- **Instruction Selection**: pick target-specific instructions
- **Register Allocation (Linear Scan, Graph Coloring)**: assign variables to registers
- **Machine Instruction Scheduling**: order instructions to hide memory latency

### New Pass Manager (NPM)

LLVM has migrated from legacy PassManager to NPM, which provides:

- **Better dependency tracking**: passes declare what they require/invalidate
- **Lazy analysis recomputation**: no unnecessary rebuilds
- **Parallel pass execution**: independent passes run concurrently within pipeline stages

## Backend: Lowering to Machine Code

### Code Generation Phases

1. **Instruction Selection**: LLVM IR → target-specific machine instructions
2. **Scheduling**: order instructions to minimize latency and stalls
3. **Register Allocation**: assign variables to registers (usually ~16-32 general-purpose)
4. **Spilling**: if > registers available, store values to stack
5. **Prologue/Epilogue**: save/restore callee-saved registers
6. **Assembly Emission**: output assembly code or object code

### Target Definition

Each target (x86-64, ARM, MIPS, etc.) defines:

- **Register file**: how many registers, their constraints
- **Instruction set**: which instructions are available, latencies, throughput
- **Calling convention**: how arguments are passed, where return values go
- **Addressing modes**: memory access patterns

## JIT Compilation: ORC Engine

LLVM ORC (On-Request Compilation) is a modern JIT compiler framework.

### Architecture

- **JIT Linking Layer**: relocates compiled code, links object files
- **Compile-On-Demand Layer**: lazy compilation (compile functions when first called)
- **Optimization Layer**: apply optimizations before lowering
- **Module Partitioning**: split code into compilation units for parallelism

### Workflow

1. User creates LLVM Module with IR
2. Submit module to JIT
3. JIT compiles to machine code, links into executable memory
4. User gets function pointer, calls it

```cpp
// Pseudocode
LLVMContext ctx;
Module mod("example", ctx);
Function* f = createFunc(mod, ...);

ThreadSafeModule tsm(move(mod), ctx);
JIT jit;
auto sym = jit.addModule(tsm);
auto fn_ptr = cast<int32_t(*)()>(sym.getAddress());
int result = fn_ptr();  // Run JIT-compiled code
```

## MLIR: Multi-Level IR

MLIR (Multi-Level Intermediate Representation) extends LLVM for domain-specific optimization.

### Multi-Level Abstraction

Traditional IR: source → single IR level → machine code

MLIR: source → domain IR (high-level ops) → progressively lower IR (more detailed ops) → LLVM IR → machine code

Example:
```
TensorFlow Graph (HLO)
    ↓
Affine IR (simple nested loops)
    ↓
Linalg IR (linear algebra ops)
    ↓
LLVM IR
```

### Benefits

- **Domain-specific optimization**: apply tensor or GPU-specific transforms at high level
- **Decoupling**: TensorFlow doesn't depend on LLVM IR details
- **Composability**: multiple front-ends share infrastructure

### Dialects

MLIR uses **dialects** to define IR variants:

- `llvm`: LLVM IR operations
- `std`: standard arithmetic, control flow
- `affine`: polyhedral loop nests
- `linalg`: linear algebra operations
- `gpu`: GPU-specific ops (launches kernels)

## Clang: C/C++/Objective-C Frontend

Clang translates source code to LLVM IR. Key features:

- **Fast parsing**: hand-written recursive descent parser
- **Detailed diagnostics**: precise error messages with source highlights
- **AST-based**: semantically rich abstract syntax tree
- **Modular**: used by IDEs (VS Code, Xcode) for code completion, refactoring

## LLD: Linker

LLD is LLVM's linker, replacing GNU ld. Benefits:

- **Architecture portability**: one codebase supports ELF, Mach-O, COFF
- **Speed**: parallel linking via thread pool
- **Correctness**: stronger symbol resolution rules (catches more errors)

## Sanitizers: Runtime Error Detection

LLVM sanitizers instrumentcompiled code to catch bugs at runtime:

### AddressSanitizer (ASan)

Detects memory errors: out-of-bounds access, use-after-free, double-free.

Mechanism: shadow memory (every 8 bytes of user memory maps to 1 byte of shadow). Before memory access, check shadow byte. If illegal, halt.

Trade-off: ~2x slowdown, ~3x memory overhead.

### ThreadSanitizer (TSan)

Detects data races in multithreaded code. Mechanism: track all memory access with thread IDs, detect unsynchronized concurrent access.

Trade-off: ~5-15x slowdown (expensive instrumentation).

### UndefinedBehaviorSanitizer (UBSan)

Detects undefined behavior: signed integer overflow, out-of-range enum values, null pointer dereference (on some targets).

Trade-off: ~1-2x slowdown (less instrumentation than ASan).

## Polly: Polyhedral Optimization

Polly is LLVM's polyhedral optimization framework. It models loop nests as polyhedra (geometric objects defined by linear constraints) to enable advanced optimizations.

### What Polly Does

- **Loop tiling**: divide nested loops into cache-friendly blocks
- **Loop transformation**: reorder loops to improve parallelism
- **Automatic parallelization**: generate OpenMP code for loops
- **GPU offloading**: generate CUDA/OpenCL from loop nests

### Limitations

Only handles **static control flow** (affine loop bounds, affine array indices):

```c
// Polly can optimize
for (int i = 0; i < n; i++)
  for (int j = i; j < n; j++)
    a[i][j] = ...;

// Polly cannot optimize (non-affine condition)
for (int i = 0; i < n; i++)
  if (i * i < threshold)
    a[i] = ...;
```

## Compilation Phases in Practice

```
1. Clang Frontend: C/C++ → LLVM IR
2. Optimization Passes: IR → optimized IR (multiple passes)
3. Target Lowering:
   - Instruction Selection: IR → Machine IR
   - Register Allocation: Virtual Registers → Physical Registers
   - Assembly Emission: Machine IR → Assembly
4. Assembly: .s file
5. LLD Linker: Link object files, resolver symbols, produce executable
```

## See Also

- **Compiler Backend**: compiler-design-backend.md (general IR design)
- **Compiler Optimization**: compiler-optimization.md (SSA, DCE, constant propagation, loop opts)
- **JIT & Dynamic Compilation**: web-javascript-engine.md, runtime-v8.md
- **Polyhedral Compilation**: algorithms-graph.md (constraints, geometric algorithms)