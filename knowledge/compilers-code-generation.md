# Compilers — Code Generation & Lowering

## Overview

**Code generation** is the process of translating an intermediate representation (typically SSA or three-address code) into machine code or bytecode. It bridges the gap between machine-independent IR and hardware-specific instructions.

The phases are:

1. **Instruction selection**: map IR operations to target ISA instructions
2. **Register allocation**: assign variables to hardware registers or memory
3. **Instruction scheduling**: reorder instructions to maximize parallelism
4. **Calling convention & stack frame setup**: organize function prologue/epilogue
5. **Relocation & linking**: resolve addresses, produce executable

## Instruction Selection

Instruction selection maps each IR operation to one or more target machine instructions.

### Tree Matching (Maximal Munch)

Many compilers use **tree matching**: view the IR as expression trees, then match subtrees to instruction patterns:

```
IR tree:
        Add
       /   \
    Mul     Load y
    / \
  Load x  Const 2

Target ISA may have:
  - mov reg, mem         ; load from memory into register
  - imul reg, reg, imm   ; multiply register by immediate
  - add reg, reg         ; add registers
  - mov mem, reg         ; store register to memory

Greedy matching (maximal munch from root):
  1. Const 2 → imm 2
  2. Load x → mov reg_a, [x_addr]
  3. Mul(reg_a, imm 2) → imul reg_a, reg_a, 2
  4. Load y → mov reg_b, [y_addr]
  5. Add(reg_a, reg_b) → add reg_a, reg_b
```

### Burg & Tile Generation

**Burg** (Bottom-Up Rewrite Generate) is a compiler generator that derives optimal instruction selection from a formal specification:

- **Input**: production rules (patterns), costs
- **Output**: pattern matcher code that selects lowest-cost cover of the IR tree

```
// Burg specification excerpt
%term Const Add Mul Load

reg: Const           cost 1   { emit("movl $%a, %c"); }
reg: Load            cost 2   { emit("movl (%a), %c"); }
stmt: Assign(reg, reg)  cost 0   { emit("movl %b, %a"); }
reg: Add(reg, reg)   cost 1   { emit("addl %b, %a"); }
reg: Mul(reg, Const) cost 1   { emit("imull %b, %a"); }
```

Burg tries all possible covers, picks lowest-cost, emits code.

**Advantages**: optimal (or near-optimal) instruction selection, declarative (easy to retarget)

**Limitations**: assumes cost model (costs aren't always accurate on modern CPUs), can generate large pattern matcher

### Naive vs. Smart Selection

**Naive**: one instruction per IR operation (may be suboptimal):
```
IR:  t = x + 2
Naive:  mov reg, x
        add reg, 2
        mov t, reg
```

**Smart**: combine operations, use addressing modes:
```
x86 addressing modes: [base + scale*index + disp]
IR:  t = a[i] + 2
Smart:  mov reg, [rcx + rsi*4 + 0]  ; load from array with one instruction
        add reg, 2
        mov t, reg
```

## Register Allocation

Given unlimited virtual registers in IR, assign each to a hardware register or memory location.

### Graph Coloring Approach

Build an **interference graph**:
- **Nodes**: variables (or SSA values)
- **Edges**: connect variables that are live simultaneously (interfere; can't use same register)

**Live range**: set of instructions where a variable holds a value. Two variables interfere if their live ranges overlap.

```
Example:
  x = 10
  y = 20
  z = x + y    <- x and y both live here; they interfere
  w = x + 5
  v = z + w

Interference graph edges:
  x --- z
  y --- z
  x --- w
  z --- w
  z --- v
```

**k-coloring problem**: assign each node a "color" (register) such that no adjacent nodes share a color, using at most k colors (k = number of physical registers).

**NP-complete** in general, but effective heuristics exist:

1. **Chaitin's algorithm** (1982):
   - Remove nodes with < k neighbors (they always have a color available)
   - Simplify (remove these nodes) until all removed or only high-degree nodes remain
   - Spill (move to memory) high-degree nodes if needed
   - Restore nodes, assigning colors greedily

2. **Heuristics for tie-breaking**:
   - **Most constrained first** (FCRA): prioritize high-degree nodes (limited color choices)
   - **Least constrained first** (LCRA): easier nodes first (larger pool of available colors)

**Spilling**: if k-coloring is impossible, move some variable to memory (spill code):
```
Original: x = y + z  (all in registers)
Spilled:  mov x, [sp + offset]    ; store x to stack
          mov reg_x, [sp + offset] ; reload when needed later
```

Spilling adds memory ops, so optimal register allocation minimizes spills.

### Linear Scan Allocation

**Linear scan** (Poletto & Sarkar, 1999) offers an alternative to graph coloring:

1. Order variables by live range start point
2. For each variable in order:
   - Allocate the earliest free register (no conflict with current live ranges)
   - If no register free, spill the variable with the farthest next use

**Advantages**: O(n log n) instead of O(n²) for graph coloring, simpler implementation, single pass

**Limitations**: may use more registers than optimal (fewer free registers remain after greedy allocation), less aggressive optimization

Modern compilers often use linear scan in JIT (fast), graph coloring in AOT (offline optimization).

## Calling Conventions

Calling conventions specify how functions are called, parameters passed, return values handled.

Common conventions:

### x86-64 System V (Unix/Linux/macOS)
- **Parameters**: rdi, rsi, rdx, rcx, r8, r9 (first 6 args), rest on stack
- **Return**: rax (int/pointer), rdx:rax (128-bit), xmm0-xmm1 (floats)
- **Caller-saved**: rax, rcx, rdx, rsi, rdi, r8-r11 (caller must save if needed)
- **Callee-saved**: rbx, rbp, r12-r15, rsp (function must restore)

### x86-64 Microsoft x64 (Windows)
- **Parameters**: rcx, rdx, r8, r9 (first 4 args), rest on stack
- **Return**: rax, rdx:rax
- **Caller-saved**: rax, rcx, rdx, rsi, rdi, r8-r11
- **Callee-saved**: rbx, rbp, rsi, rdi, r12-r15, rsp

This affects code generation: when generating a function call, register parameter values must be loaded into specified registers.

## Stack Frame Layout

When entering a function, the CPU uses a **stack frame** to store local variables, spilled registers, saved return address:

```
x86-64 System V frame (simplified):
   [higher addresses]
   parameter 7
   parameter 6
   [return address from call instruction]
   [rbp saved by callee]
   local variable 1
   local variable 2
   [lower address = stack pointer (rsp)]
```

**Frame pointer (rbp)**: points to start of frame (often saved at [rsp]); lets function index locals as `[rbp - offset]`.

**Stack pointer (rsp)**: points to top of stack; must remain 16-byte aligned (on x86-64 ABI) across function calls.

Prologue (entering function):
```
push rbp              ; save old frame pointer
mov rbp, rsp          ; establish new frame
sub rsp, local_size   ; make room for locals
```

Epilogue (exiting function):
```
mov rsp, rbp          ; deallocate locals
pop rbp               ; restore saved rbp
ret                   ; return to caller
```

## Relocation & Linking

After code generation, the compiler produces **object code** (`.o` files) with symbolic references.

### Example

```
Generated x86-64 code (object file):
  0: 48 8d 05 00 00 00 00   lea rax, [rip + 0]           ; address of 'x' (unresolved)
  7: 48 8b 00                mov rax, [rax]
  ...
Relocation entry: offset 0x3, type R_X86_64_PC32, symbol 'x'
```

This relocation entry says: "at offset 0x3, apply a PC-relative 32-bit relocation for symbol 'x'."

### Linking

The **linker** combines multiple object files:

1. **Symbol resolution**: match symbol references across files
2. **Address assignment**: assign each symbol a final address
3. **Relocation**: rewrite instructions with the final addresses

**Relocation types**:
- **Absolute**: address of symbol (need symbol address)
- **PC-relative (RIP-relative)**: offset from instruction pointer (good for position-independent code)
- **GOT**: Global Offset Table (for dynamic linking)

### Static vs. Dynamic Linking

**Static linking**: all code included in executable, relocation happens at link time, binary is self-contained.

**Dynamic linking**: external libraries referenced, relocation happens at load time, linker resolves symbols from `.so` / `.dll` files.

## ELF Format

**ELF** (Executable and Linkable Format) is the standard on Unix/Linux.

Structure:
```
ELF header
  magic number, version, target ISA, entry point, ...
Program header table
  segment info (memory layout at runtime)
Section header table
  sections (.text, .data, .rodata, .bss, .symtab, .strtab, .rel.text, ...)
Sections
  .text: executable machine code
  .data: initialized data
  .rodata: read-only data
  .bss: uninitialized data (zeroed at load time)
  .symtab: symbol table (names, addresses, types)
  .strtab: string table (symbol names)
  .rel.text: relocation entries for .text section
```

Dynamic relinking (at load time) uses `.rel.dyn` and `.rel.plt`.

## Instruction Scheduling

After instruction selection and register allocation, **instruction scheduling** reorders instructions to:
- Minimize pipeline stalls (can't execute next instruction until previous finishes)
- Exploit instruction-level parallelism (especially superscalar CPUs)

Example:
```
Unscheduled:
  mov rax, [x]      ; load x from memory (cache miss, ~200 cycles)
  add rax, 1        ; must wait for rax
  mov [y], rax      ; must wait for rax

Scheduled (interleave unrelated work):
  mov rax, [x]      ; load x (stall starting)
  mov rbx, [z]      ; load z (while x is pending)
  mov rcx, 10       ; independent compute
  add rbx, 1
  add rax, 1        ; rax likely ready by now
  mov [y], rax
  mov [w], rbx
```

Scheduling is NP-hard in general, but heuristics (list scheduling, genetic algorithms) are effective.

## See Also

- `compiler-design-backend.md` — broader backend pipeline
- `compiler-optimization.md` — IR-level optimizations
- `hardware-cpu-architecture.md` — CPU pipeline, cache behavior