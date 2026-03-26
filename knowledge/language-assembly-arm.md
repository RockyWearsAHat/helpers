# ARM Assembly Language

## Historical Context

ARM originated at Acorn Computers in 1985 as the Acorn RISC Machine, one of the first commercial RISC processors. The design philosophy—simple fixed-width instructions, large register file, low power consumption—proved prescient. ARM Holdings licenses the architecture rather than manufacturing chips, enabling an ecosystem spanning smartphones (virtually 100% market share), embedded systems, and increasingly servers and desktops (AWS Graviton, Apple Silicon, Ampere Altra). The architecture exists in two major forms: AArch32 (32-bit, ARM and Thumb instruction sets) and AArch64 (64-bit, introduced with ARMv8-A in 2011).

## AArch32 (ARM and Thumb)

### Register File

| Register | Alias | Purpose                                       |
| -------- | ----- | --------------------------------------------- |
| R0–R3    | A1–A4 | Arguments/return values, caller-saved         |
| R4–R11   | V1–V8 | General purpose, callee-saved                 |
| R12      | IP    | Intra-procedure scratch                       |
| R13      | SP    | Stack pointer                                 |
| R14      | LR    | Link register (return address)                |
| R15      | PC    | Program counter                               |
| CPSR     | —     | Current program status register (flags, mode) |

The program counter is directly accessible as R15, a distinctive design choice that allows PC-relative tricks but also creates some subtleties (PC reads as current instruction + 8 in ARM mode due to pipeline behavior).

### Instruction Encoding

ARM instructions are fixed at 32 bits, Thumb instructions at 16 bits (Thumb-2 mixes 16-bit and 32-bit). Fixed-width encoding simplifies decode logic—a meaningful hardware advantage. Every ARM instruction includes a 4-bit condition field, enabling conditional execution of most instructions without branches:

```arm
CMP    R0, #0
ADDNE  R1, R1, #1    ; executes only if R0 != 0
MOVEQ  R2, #0        ; executes only if R0 == 0
```

This reduces branch misprediction penalties for short conditional sequences. Thumb mode sacrifices this flexibility for code density—important for memory-constrained embedded systems.

### Conditional Execution

The condition codes (EQ, NE, CS, CC, MI, PL, VS, VC, HI, LS, GE, LT, GT, LE, AL) derive from the CPSR flags (N, Z, C, V). Unlike x86 where only jumps are conditional, ARM can conditionally execute arithmetic, loads, stores, and most other instructions. This is one of ARM's most distinctive features, though AArch64 significantly restricts it.

### Barrel Shifter

ARM's barrel shifter allows one operand to be shifted or rotated as part of another instruction at no extra cost:

```arm
ADD  R0, R1, R2, LSL #3    ; R0 = R1 + (R2 << 3)
MOV  R0, R1, ROR #16       ; R0 = rotate R1 right by 16
LDR  R0, [R1, R2, LSL #2]  ; load from R1 + R2*4
```

This effectively provides multiply-by-small-constant and array indexing in a single cycle, reducing instruction count compared to architectures that require separate shift instructions.

## AArch64 (ARMv8-A and later)

### Register File

| Registers | Width   | Purpose                                      |
| --------- | ------- | -------------------------------------------- |
| X0–X7     | 64-bit  | Arguments/return values                      |
| X8        | 64-bit  | Indirect result location                     |
| X9–X15    | 64-bit  | Caller-saved temporaries                     |
| X16–X17   | 64-bit  | Intra-procedure call scratch (IP0/IP1)       |
| X18       | 64-bit  | Platform register (reserved on some OSes)    |
| X19–X28   | 64-bit  | Callee-saved                                 |
| X29       | 64-bit  | Frame pointer (FP)                           |
| X30       | 64-bit  | Link register (LR)                           |
| SP        | 64-bit  | Stack pointer (not a GP register)            |
| XZR/WZR   | —       | Zero register (reads as 0, writes discarded) |
| PC        | —       | Not directly accessible as a GP register     |
| V0–V31    | 128-bit | SIMD/FP registers                            |

AArch64 is a clean-sheet redesign, not a backward-compatible extension. The 32-bit view of registers uses W0–W30 (writing a W register zero-extends to the full X register). The dedicated zero register eliminates the need for compare-with-zero instructions and simplifies many encodings.

### Key Instruction Differences from AArch32

- Conditional execution is limited to conditional select (`CSEL`, `CSINC`, `CSNEG`) and conditional compare (`CCMP`)—no universal condition field
- PC is not a general-purpose register
- More registers (31 GP vs. 15 usable in AArch32)
- Fixed 32-bit instruction width (no Thumb equivalent)
- Immediate encoding uses a clever bitmask scheme for logical operations and shifted/rotated patterns for moves

## Load-Store Architecture

ARM is a pure load-store architecture: only `LDR`/`STR` family instructions access memory. All computation happens between registers. This contrasts with x86, where many ALU instructions can take a memory operand directly.

```arm
; AArch64: load, compute, store
LDR    X0, [X1]          ; load from address in X1
ADD    X0, X0, X2        ; register-to-register add
STR    X0, [X1]          ; store result

; Addressing modes (AArch64)
LDR    X0, [X1, #16]     ; base + immediate offset
LDR    X0, [X1, X2]      ; base + register offset
LDR    X0, [X1, X2, LSL #3]  ; base + scaled register
LDR    X0, [X1, #8]!     ; pre-index (X1 += 8, then load)
LDR    X0, [X1], #8      ; post-index (load, then X1 += 8)
LDP    X0, X1, [SP], #16 ; load pair with post-increment (common in epilogues)
```

Load/store pair instructions (`LDP`/`STP`) are heavily used for saving and restoring registers, as ARM lacks x86-style `PUSH`/`POP` that implicitly modify SP.

## SIMD: NEON and SVE

### NEON (Advanced SIMD)

Available in both AArch32 (with D0–D31 doubleword and Q0–Q15 quadword views) and AArch64 (V0–V31 as 128-bit registers). NEON provides fixed-width 128-bit SIMD:

```arm
; AArch64 NEON: add 4 packed 32-bit integers
LD1    {V0.4S}, [X0]
LD1    {V1.4S}, [X1]
ADD    V2.4S, V0.4S, V1.4S
ST1    {V2.4S}, [X2]
```

NEON covers integer and floating-point SIMD, with lane sizes from 8-bit to 64-bit. Unlike early SSE, NEON was designed from the start with integer and float parity.

### SVE and SVE2

Scalable Vector Extension (SVE) introduced vector-length-agnostic programming. Instead of fixing vector width, SVE allows hardware implementations to choose widths from 128 to 2048 bits in 128-bit increments. Code written for SVE runs on any implementation without recompilation:

```arm
; SVE: vector add with predication
WHILELT  P0.S, X0, X1      ; create predicate: which lanes are active?
LD1W     Z0.S, P0/Z, [X2]  ; predicated load
LD1W     Z1.S, P0/Z, [X3]
ADD      Z2.S, Z0.S, Z1.S
ST1W     Z2.S, P0, [X4]    ; predicated store
```

SVE's predicate registers (P0–P15) enable per-lane masking without separate blend instructions. SVE2 extends this to integer-heavy workloads (cryptography, codecs, signal processing) that NEON handles less elegantly.

The scalable approach avoids the x86 problem of needing separate SSE/AVX/AVX-512 code paths—a significant software engineering advantage.

## Memory Ordering

ARM has a weakly-ordered memory model, substantially different from x86's Total Store Order. On ARM:

- Loads and stores can be reordered with respect to each other (with some constraints)
- Explicit barriers are required for synchronization between cores

### Barrier Instructions

| Instruction                               | Purpose                                                             |
| ----------------------------------------- | ------------------------------------------------------------------- |
| DMB (Data Memory Barrier)                 | Ensures ordering of memory accesses before/after the barrier        |
| DSB (Data Synchronization Barrier)        | Ensures completion (not just ordering) of memory accesses           |
| ISB (Instruction Synchronization Barrier) | Flushes pipeline, ensures subsequent instructions see prior changes |

### Acquire/Release

ARMv8 introduced load-acquire (`LDAR`) and store-release (`STLR`) instructions that provide one-way barriers matching C11/C++11 memory model semantics directly. These are more efficient than full DMB barriers for typical synchronization patterns:

```arm
LDAR   X0, [X1]      ; load-acquire: all subsequent reads/writes see this load
STLR   X0, [X1]      ; store-release: all prior reads/writes complete before this store
LDAXR  X0, [X1]      ; load-acquire exclusive (for CAS-like patterns)
STLXR  W2, X0, [X1]  ; store-release exclusive (W2 = 0 on success)
```

Code ported from x86 to ARM frequently has latent memory ordering bugs that were masked by x86's strong ordering. Tools like ThreadSanitizer help catch these.

## Exception Levels

AArch64 defines four exception levels (ELs), a cleaner privilege model than x86's rings:

| Level | Typical Use                | x86 Equivalent       |
| ----- | -------------------------- | -------------------- |
| EL0   | User applications          | Ring 3               |
| EL1   | OS kernel                  | Ring 0               |
| EL2   | Hypervisor                 | VMX root mode        |
| EL3   | Secure monitor (TrustZone) | No direct equivalent |

Transitions between ELs are controlled by exception entry (interrupts, system calls via `SVC`, `HVC`, `SMC`) and exception return (`ERET`). TrustZone at EL3 provides hardware-enforced isolation between "secure world" and "normal world"—used for key storage, DRM, and trusted boot.

## ARM vs. x86 Trade-offs

### Decode Complexity

| Aspect               | ARM                        | x86                                       |
| -------------------- | -------------------------- | ----------------------------------------- |
| Instruction width    | Fixed 32-bit               | Variable 1–15 bytes                       |
| Decode stages        | Simpler, fewer transistors | Complex pre-decode + micro-op translation |
| Code density         | Lower (fixed width)        | Higher (variable width)                   |
| Decode width scaling | Easier to widen            | Harder due to length determination        |

ARM's fixed-width encoding means the decoder knows where each instruction boundary is without scanning—enabling wider decode and lower power consumption. x86 CPUs spend significant silicon on instruction length determination and micro-op cracking.

### Power Efficiency

ARM's architectural simplicity translates to better performance-per-watt in most scenarios. Apple's M-series chips demonstrated that ARM can match or exceed x86 single-thread performance while consuming substantially less power. The advantage compounds at scale—ARM server chips like Graviton and Ampere Altra reduce datacenter power budgets.

Contributing factors beyond decode simplicity: smaller instruction cache footprint relative to work done, fewer implicit dependencies between instructions, and a culture of power-conscious microarchitecture design.

### Performance Characteristics

Raw peak performance comparisons between ARM and x86 depend heavily on specific implementations rather than ISA properties. Both architectures support deep out-of-order execution, wide issue, and sophisticated branch prediction. The ISA differences that historically mattered (complex addressing modes, conditional execution, barrel shifter) are increasingly secondary to microarchitectural factors like cache hierarchy, prefetch behavior, and execution port layout.

### Ecosystem Considerations

x86 benefits from decades of software ecosystem optimization. Profilers, debuggers, JIT compilers, and libraries are deeply tuned for x86 microarchitectures. ARM's ecosystem is rapidly catching up, accelerated by Apple Silicon's mainstream adoption and AWS Graviton's server presence, but gaps remain in some specialized domains.

## The Shift to ARM in Servers and Desktops

Several concurrent trends drive ARM adoption beyond mobile:

- **Apple Silicon** (M1, 2020 onward): proved ARM can lead in desktop single-thread performance while maintaining all-day battery life
- **AWS Graviton** (2018 onward): demonstrated 20–40% better price-performance for many server workloads
- **Ampere Altra**: 128-core ARM server chips targeting cloud-native workloads
- **Windows on ARM**: improving with x86 emulation (though compatibility gaps persist)
- **RISC-V emergence**: validates the general trend toward simpler ISAs, creating competitive pressure on both ARM and x86

The transition is not uniform—workloads with heavy x86-specific optimizations (some databases, financial trading systems, legacy enterprise software) maintain x86 preference. But the direction is clear: ISA choice is becoming a performance-engineering decision rather than an ecosystem lock-in.

## Debugging ARM Assembly

- **GDB with OpenOCD/JTAG**: standard for embedded ARM debugging, providing register inspection, memory examination, and hardware breakpoints
- **LLDB**: primary debugger on Apple platforms; `register read`, `disassemble`, `memory read`
- **`objdump -d`** and **`llvm-objdump`**: disassembly for post-mortem analysis
- **Instruction tracing**: ARM CoreSight provides hardware trace (ETM/ETB) for non-intrusive instruction-level recording—more capable than x86's Intel PT in some respects
- **Simulator/emulator**: QEMU for user-mode or full-system ARM emulation; the Fixed Virtual Platform (FVP) for architecture exploration

## Inline Assembly (AArch64, GCC/Clang)

```c
uint64_t read_cycle_counter(void) {
    uint64_t val;
    asm volatile("mrs %0, cntvct_el0" : "=r" (val));
    return val;
}
```

ARM inline assembly uses `"r"` for general register constraints (simpler than x86's letter soup). System register access (`MRS`/`MSR`) is a common use case. For SIMD, intrinsics via `<arm_neon.h>` (NEON) or `<arm_sve.h>` (SVE) are preferred over inline assembly, as they let the compiler handle register allocation and scheduling.

## Instruction Set Extensions

| Extension | Purpose                                                   |
| --------- | --------------------------------------------------------- |
| NEON      | Fixed-width SIMD (128-bit)                                |
| SVE/SVE2  | Scalable vector processing                                |
| SME       | Scalable matrix extension (matrix tiles for ML workloads) |
| CRC32     | Hardware CRC computation                                  |
| AES/SHA   | Cryptographic acceleration                                |
| BTI       | Branch Target Identification (control-flow integrity)     |
| MTE       | Memory Tagging Extension (use-after-free detection)       |
| PAC       | Pointer Authentication Codes (return address protection)  |

MTE and PAC represent ARM investing in hardware-assisted security features that have no direct x86 equivalent, addressing memory safety and control-flow hijacking at the hardware level—a differentiating strategy as software security requirements tighten.
