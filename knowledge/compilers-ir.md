# Compilers — Intermediate Representations

## Overview

An **intermediate representation** (IR) is a bridge between source code (high-level, language-specific) and machine code (low-level, hardware-specific). The IR abstracts away source-language details while remaining concrete enough for code generation, analysis, and optimization.

Compilers typically use multiple IRs, each at a different abstraction level:
1. **AST** — Abstract syntax tree (source structure)
2. **Lowered IR** — Three-address code, bytecode (simplified operations)
3. **Optimized IR** — SSA form, control flow graph (suitable for optimization passes)
4. **Target IR** — LLVM IR, machine IR (near hardware level)

## Abstract Syntax Tree (AST)

The AST is the output of parsing. It represents program structure as a tree:

```
Source: x = 2 + 3 * y
AST:
       Assignment
       /         \
      x       BinaryOp(+)
              /        \
             2      BinaryOp(*)
                    /        \
                   3          y
```

**Characteristics**:
- **Direct from grammar**: each rule in the grammar has a corresponding AST node type
- **Preserves structure**: parentheses, precedence are embedded in tree shape
- **Loses concrete syntax**: comments, whitespace, irrelevant parentheses are gone
- **Language-specific**: a Python AST differs from a C AST in node types

**Advantages**: easy to work with, strongly typed (if strongly typed language), preserves source intent

**Limitations**: deeply nested (tall trees for deep expressions), different nodes for semantically similar constructs between languages, large memory footprint

## Three-Address Code (TAC)

TAC is a linearized, simplified IR: each instruction performs one operation on at most three operands.

```
Source: x = 2 + 3 * y
TAC:
  t1 = 3 * y
  t2 = 2 + t1
  x = t2
```

Or in prefix notation:
```
  t1 = mul 3 y
  t2 = add 2 t1
  x = mov t2
```

**Characteristics**:
- Linear (sequential instructions)
- Simple operations (binary arithmetic, unary, assignment, jumps)
- Temporary variables for intermediate results
- Explicit control flow (labels, jumps)

**Advantages**: easy to optimize, simple to translate to machine code, suitable for interpretation

**Limitations**: verbose (many temporaries), less suitable for some analyses (e.g., data flow analysis across tree structure)

## Static Single Assignment (SSA) Form

SSA is a refinement where **each variable is assigned exactly once**. To achieve this, variables are renamed at merge points using **phi functions**:

```
Source:
  if (x > 0) {
    y = x + 1
  } else {
    y = x - 1
  }
  z = y + 2

SSA:
  if (x > 0) goto L1 else goto L2
L1:
  y_1 = x + 1
  goto L3
L2:
  y_2 = x - 1
  goto L3
L3:
  y_3 = phi(y_1, y_2)
  z_1 = y_3 + 2
```

The phi function at L3 means: "y_3 is y_1 if we came from L1, else y_2 from L2."

**Advantages**:
- **Use-def chains are trivial**: each use has exactly one defining instruction (the SSA assignment)
- **Data flow analysis is simplified**: no aliasing ambiguity to worry about
- **Aggressive optimizations**: many passes are designed for SSA (e.g., GVN, LICM)
- **Def-use chains**: walking from a def to its uses (or reverse) is O(1) per edge

**Building SSA** (Cytron et al., 1989):
1. For each basic block, compute dominance frontier
2. For each assignment, insert phi functions at dominance frontier
3. Rename variables to ensure single assignment property (top-down DFS through dominator tree)

**Limitations**: phi functions have no direct machine code equivalent (they're conceptual), SSA can have quadratic size blow-up in the worst case (though rare), SSA destruction (converting back) must be careful

## Control Flow Graph (CFG)

A CFG makes control flow explicit: nodes are basic blocks (sequences of instructions with no branches inside), edges are jumps/branches:

```
    Entry
      |
    +---+
    |   |
   B1   |   B1: x = 0; y = 10
    |   |       if x < y goto B2 else goto B3
    |   |
   / \ / 
  B2   B3    B2: x = 1
   \ / \      goto B4
    |   |
    B4  |     B3: y = 2
     |  |      [fallthrough to B4 (if no explicit branch)]
     +--+      
      |        B4: z = x + y
     Exit      return z
```

**Basic block**: maximal sequence of instructions with:
- Single entry (first instruction)
- Single exit (last instruction)
- No jumps in the middle

**Edges**: represent control flow:
- **Fall-through**: next block executed
- **Conditional branch**: true/false targets
- **Unconditional jump**: single target

**Use cases**: optimization (loop detection, dominance), code analysis (reachability), register allocation

## Dominance & Dominance Frontier

**Dominator**: node X dominates Y if every path from entry to Y must pass through X.

**Immediate dominator**: the unique node that strictly dominates Y and is dominated by all other strict dominators.

**Dominator tree**: tree where children of X are the nodes X immediately dominates. Useful for:
- Structured code analysis
- SSA construction (phi placement)
- Loop nesting levels

**Dominance frontier** DF(X): set of nodes Y such that X dominates a predecessor of Y but does not strictly dominate Y. Used to place phi functions: if a variable is assigned in block X, phi functions are needed at all blocks in DF(X).

## LLVM Intermediate Representation

LLVM IR is a concrete, widely-used IR used in LLVM, a modular compiler infrastructure.

```
define i32 @add(i32 %a, i32 %b) {
  %result = add i32 %a, %b
  ret i32 %result
}
```

**Properties**:
- **SSA form**: all instructions produce fresh values
- **Strongly typed**: each value has an explicit type (i32, float, pointer, etc.)
- **Explicit memory ops**: load/store for memory access; no implicit register allocation
- **Three-layer abstraction**:
  - **High-level** (structured loops, type info)
  - **Mid-level** (unstructured jumps, explicit pointers)
  - **Low-level** (machine IR, target-specific)
- **Language-neutral frontend**: many languages compile to LLVM IR

**Advantages**: modular (easy to add passes), well-documented (academic papers), production-grade (used in Clang, Swift, Julia, etc.)

## Sea of Nodes

**Sea of nodes** is an alternative graph-based IR (used in GraalVM, hot spot JIT compilers):

Instead of a CFG, compute a DAG of data dependencies, where:
- Nodes represent operations (add, load, etc.)
- Edges represent data flow
- Control flow is implicit (derived from data dependencies)

```
Example output of sea-of-nodes representation for `z = (a + b) * c:`

    Load a    Load b    Load c
       \        /          |
        Add                 |
         |                  |
         Mul ----------------
         |
        Store z
```

**Advantages**:
- Exposes scheduling freedom: any order respecting data dependencies is valid
- Simplifies some optimizations: CSE is trivial (DAG structure makes common subexpressions obvious)
- Superscalar-aware: instruction-level parallelism is explicit

**Disadvantages**:
- Debugging is harder (no linear sequence of instructions)
- Memory operations require explicit dependence tracking
- Not as widely used as CFG (steeper learning curve)

## Phi Functions Deep Dive

Phi is not a real instruction. At runtime, it's "transparent": execution follows the appropriate predecessor:

```
L3: y = phi(y_1, y_2)    // No machine code for this line
    z = y + 2            // Just use y as if it were assigned once
```

The phi "selects" the correct value based on which predecessor block was executed. Code generation must handle phi destruction: either
- Parallel assignment at each predecessor
- Move the phi logic into each predecessor with an explicit copy

## See Also

- `compiler-design-backend.md` — optimization passes on IR
- `compiler-llvm.md` — LLVM IR specifics
- `compiler-internals.md` — full pipeline overview