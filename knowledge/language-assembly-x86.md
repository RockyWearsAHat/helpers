# x86 and x86-64 Assembly Language

## Historical Context

The x86 instruction set traces back to Intel's 8086 processor (1978), a 16-bit design that maintained backward compatibility with the 8080. Each subsequent generation—80286, 80386 (first 32-bit), Pentium, and AMD64 (first 64-bit, designed by AMD in 2003)—layered new capabilities atop the existing encoding. This accretive evolution explains many quirks: variable-length instructions, implicit register usage, and a rich but irregular encoding scheme. The architecture dominates desktops, laptops, and servers, though ARM is increasingly competitive in all three segments.

## Registers

### General-Purpose Registers (64-bit mode)

| 64-bit | 32-bit   | 16-bit   | 8-bit High | 8-bit Low | Conventional Use             |
| ------ | -------- | -------- | ---------- | --------- | ---------------------------- |
| RAX    | EAX      | AX       | AH         | AL        | Accumulator, return value    |
| RBX    | EBX      | BX       | BH         | BL        | Callee-saved base            |
| RCX    | ECX      | CX       | CH         | CL        | Counter, 4th arg (Win)       |
| RDX    | EDX      | DX       | DH         | DL        | Data, 3rd arg (Win)          |
| RSI    | ESI      | SI       | —          | SIL       | Source index, 2nd arg (SysV) |
| RDI    | EDI      | DI       | —          | DIL       | Dest index, 1st arg (SysV)   |
| RSP    | ESP      | SP       | —          | SPL       | Stack pointer                |
| RBP    | EBP      | BP       | —          | BPL       | Frame pointer (optional)     |
| R8–R15 | R8D–R15D | R8W–R15W | —          | R8B–R15B  | Additional GP (x86-64 only)  |

Writing to a 32-bit sub-register in 64-bit mode zero-extends to 64 bits. Writing to 8-bit or 16-bit sub-registers does not—a source of subtle bugs when mixing operand sizes.

### Special Registers

- **RIP**: Instruction pointer (directly addressable in x86-64 for RIP-relative addressing)
- **RFLAGS**: Status flags (CF, ZF, SF, OF, PF, AF) plus control bits
- **Segment registers** (CS, DS, SS, ES, FS, GS): mostly vestigial in 64-bit mode, though FS/GS are used for thread-local storage
- **CR0–CR4**: Control registers (paging, protection mode)
- **XMM0–XMM15** (128-bit), **YMM0–YMM15** (256-bit), **ZMM0–ZMM31** (512-bit): SIMD registers

## Addressing Modes

x86 supports complex addressing within a single instruction, a hallmark of CISC design:

```
[base + index * scale + displacement]
```

- **base**: any GP register
- **index**: any GP register except RSP
- **scale**: 1, 2, 4, or 8
- **displacement**: 8-bit or 32-bit signed immediate

Examples:

```nasm
mov  rax, [rbx]                ; register indirect
mov  rax, [rbx + 8]            ; base + displacement
mov  rax, [rbx + rcx*4]        ; base + scaled index
mov  rax, [rbx + rcx*8 + 16]   ; full SIB form
mov  rax, [rip + label]        ; RIP-relative (position-independent)
lea  rax, [rbx + rcx*2]        ; address computation without memory access
```

RIP-relative addressing is the default in 64-bit code, enabling position-independent executables without performance penalty.

## Instruction Categories

### Data Movement

`MOV`, `MOVZX`, `MOVSX`, `LEA`, `XCHG`, `PUSH`, `POP`, `CMOV` (conditional move). Conditional moves avoid branch misprediction penalties for simple selections.

### Arithmetic and Logic

`ADD`, `SUB`, `IMUL`, `IDIV`, `AND`, `OR`, `XOR`, `NOT`, `NEG`, `SHL`, `SHR`, `SAR`, `ROL`, `ROR`. Division implicitly uses RDX:RAX as the dividend—one of many implicit register conventions.

### Control Flow

`JMP`, `Jcc` (conditional jumps based on flags), `CALL`, `RET`, `LOOP`. Modern branch predictors handle most conditional jumps well, but indirect jumps (`jmp rax`) are harder to predict.

### String Operations

`REP MOVSB`, `REP STOSB`, `CMPSB`, `SCASB`. Modern CPUs optimize `REP MOVSB` with fast-string operations that can approach `memcpy` performance for large copies.

### System

`SYSCALL`/`SYSRET` (64-bit), `INT` (legacy), `CPUID`, `RDTSC`, `MFENCE`, `LFENCE`, `SFENCE`.

## Calling Conventions

### System V AMD64 ABI (Linux, macOS, BSD)

| Aspect             | Detail                                                             |
| ------------------ | ------------------------------------------------------------------ |
| Integer args (1–6) | RDI, RSI, RDX, RCX, R8, R9                                         |
| Float args (1–8)   | XMM0–XMM7                                                          |
| Return values      | RAX (integer), XMM0 (float)                                        |
| Callee-saved       | RBX, RBP, R12–R15                                                  |
| Stack alignment    | 16-byte aligned before CALL                                        |
| Red zone           | 128 bytes below RSP (leaf functions can use without adjusting RSP) |

### Windows x64

| Aspect             | Detail                                           |
| ------------------ | ------------------------------------------------ |
| Integer args (1–4) | RCX, RDX, R8, R9                                 |
| Float args (1–4)   | XMM0–XMM3                                        |
| Shadow space       | 32 bytes reserved by caller above return address |
| Callee-saved       | RBX, RBP, RDI, RSI, R12–R15                      |
| Red zone           | None                                             |

The divergence between these two conventions means cross-platform assembly requires care or abstraction layers. Both agree on callee-saved XMM6–XMM15 (Windows) vs. no callee-saved XMM (System V)—a frequent source of register-clobbering bugs in mixed code.

## Stack Frames

A typical function prologue/epilogue with frame pointer:

```nasm
my_function:
    push  rbp
    mov   rbp, rsp
    sub   rsp, 32          ; allocate local space

    ; ... function body ...
    ; locals at [rbp - 8], [rbp - 16], etc.
    ; args (if spilled) at [rbp + 16], [rbp + 24], etc.

    mov   rsp, rbp
    pop   rbp
    ret
```

Frame pointer omission (`-fomit-frame-pointer`) frees RBP for general use, with `.eh_frame` or `.pdata` unwind tables enabling stack unwinding for debugging and exceptions. Most optimized code omits frame pointers today.

## SIMD Extensions

### Evolution

| Extension  | Width   | Year      | Key Capabilities                                |
| ---------- | ------- | --------- | ----------------------------------------------- |
| MMX        | 64-bit  | 1997      | Integer SIMD (shared with x87 FP registers)     |
| SSE        | 128-bit | 1999      | Single-precision float SIMD                     |
| SSE2       | 128-bit | 2001      | Double-precision, integer; baseline for x86-64  |
| SSE3/SSSE3 | 128-bit | 2004/2006 | Horizontal ops, shuffle improvements            |
| SSE4.1/4.2 | 128-bit | 2007/2008 | Blend, round, string processing, CRC32          |
| AVX        | 256-bit | 2011      | 3-operand encoding (VEX), wider float ops       |
| AVX2       | 256-bit | 2013      | 256-bit integer SIMD, gather                    |
| AVX-512    | 512-bit | 2016      | Masking, scatter/gather, diverse sub-extensions |

### Practical Considerations

SSE2 is the baseline assumption for modern x86-64 code. AVX introduces VEX encoding that avoids destructive two-operand patterns. AVX-512 offers powerful masking and wider operations but has been controversial—some CPUs downclock when executing 512-bit instructions, and Intel has removed it from some consumer parts. Runtime feature detection via `CPUID` and multiple code paths are standard practice for performance-critical libraries.

```nasm
; AVX2 example: add 8 packed 32-bit integers
vmovdqu  ymm0, [rsi]
vpaddd   ymm0, ymm0, [rdi]
vmovdqu  [rdx], ymm0
```

## Memory Model

x86 provides a relatively strong memory model (Total Store Order), which simplifies concurrent programming compared to weaker architectures:

- Loads are not reordered with other loads
- Stores are not reordered with other stores
- Stores are not reordered with earlier loads
- **But**: loads can be reordered with earlier stores to different addresses (store-buffer forwarding)

Atomic operations use the `LOCK` prefix (e.g., `LOCK XADD`, `LOCK CMPXCHG`). `MFENCE` provides a full barrier, though it is rarely needed on x86 except for specific patterns like Dekker's algorithm or non-temporal stores.

## Inline Assembly

### GCC/Clang Extended Asm

```c
uint64_t rdtsc_value;
asm volatile ("rdtsc"
    : "=a" (lo), "=d" (hi)   // outputs
    :                          // inputs
    : "memory"                 // clobbers
);
```

Constraint letters (`a` = RAX, `d` = RDX, `r` = any register, `m` = memory) map C variables to operands. The `volatile` qualifier prevents reordering or elimination. Getting constraints wrong causes silent miscompilation—one reason intrinsics (`_mm_add_ps`, etc.) are generally preferred for SIMD.

### MSVC Inline Assembly

MSVC dropped inline assembly in 64-bit mode entirely, pushing users toward intrinsics or separate `.asm` files compiled with MASM.

## Debugging at Assembly Level

- **GDB/LLDB**: `disassemble`, `stepi`, `info registers`, `x/` (examine memory), watchpoints on addresses
- **Single-stepping**: `stepi` (step one instruction) vs. `nexti` (step over calls)
- **Reading compiler output**: `gcc -S -O2 -masm=intel` or Compiler Explorer (godbolt.org) for rapid comparison
- **Common pitfall recognition**: uninitialized flags from `TEST`/`CMP` reuse, off-by-one in loop unrolling, misaligned SIMD accesses (pre-AVX requires 16-byte alignment for `movaps`)
- **Performance counters**: `perf stat`, `perf record` on Linux; VTune for detailed microarchitectural analysis

## When Hand-Written Assembly Still Matters

Compilers produce excellent code for general-purpose computation, and hand-written assembly should be a last resort. Cases where it remains relevant:

- **Cryptographic primitives**: constant-time execution requirements that compilers cannot guarantee, plus use of carry flags and AES-NI instructions
- **Kernel entry/exit paths**: context switching, interrupt handlers, and system call stubs that need precise register control
- **Bootloaders and firmware**: real-mode or early protected-mode code where no compiler runtime exists
- **SIMD inner loops**: when compilers fail to auto-vectorize a critical loop (though intrinsics are usually sufficient)
- **Lock-free data structures**: when precise memory ordering and atomic sequences matter

For most application code, reading and understanding compiler output is more valuable than writing assembly. The performance-critical path typically narrows to a few hot loops that benefit more from algorithmic improvements and cache-friendly data layouts than from hand-tuned instructions.

## Encoding and Instruction Length

x86 instructions range from 1 to 15 bytes. The encoding includes optional prefixes (REX, VEX, EVEX, segment overrides, operand size, address size), an opcode (1–3 bytes), a ModR/M byte, an optional SIB byte, displacement, and immediates. This variable-length encoding complicates instruction fetch and decode but enables compact code for common operations. Decoders in modern CPUs translate x86 instructions into fixed-width micro-ops internally—effectively an x86-to-RISC translation layer in hardware.

## Microarchitectural Awareness

Writing or reading assembly benefits from understanding that modern x86 CPUs are deeply out-of-order, superscalar machines. Key concepts:

- **Micro-op fusion**: adjacent instructions (e.g., CMP + JCC) fused into one micro-op
- **Macro-op fusion**: load + ALU folded into a single dispatched operation
- **Register renaming**: eliminates false dependencies (WAR, WAW)
- **Branch prediction**: modern predictors (TAGE-like) handle most patterns, but indirect branches and cold code remain challenging
- **Store forwarding**: a load can read from a pending store if the address matches, but partial overlaps cause stalls

Performance tuning at the assembly level is architecture-specific and version-specific—optimal instruction sequences for Zen 4 may differ from those for Golden Cove.
