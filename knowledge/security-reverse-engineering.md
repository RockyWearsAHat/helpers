# Security Reverse Engineering — Disassembly, Decompilation, Dynamic Analysis & Binary Patching

## Overview

Reverse engineering (RE) is the process of analyzing compiled or obfuscated code to understand its functionality, find vulnerabilities, or circumvent protections. RE spans static analysis (reading binaries without execution), dynamic analysis (running and observing behavior), and intermediate techniques. The field has both defensive applications (threat analysis, vulnerability research) and offensive ones (cracking, cheating). Understanding RE is essential for binary security, malware analysis, and hardening code against tampering.

## Static Binary Analysis

Static analysis examines binaries at rest, without execution, extracting control flow, data structures, and logic.

### Disassembly vs. Decompilation

**Disassembly** converts machine instructions back to assembly language (mnemonic form). It is lossless (every instruction maps to assembly) but human-unfriendly: thousands of lines of assembly are tedious to analyze. Disassembled code loses higher-level constructs (variables, types, loops) that were compiled away.

**Decompilation** reconstructs higher-level languages (C, C++, Java) from binaries. It is lossy (information is lost in compilation) but more readable. Decompilers infer types, structures, variable names, and control flow. Quality varies: some outputs are near-source-like; others are harder to read than assembly.

**Trade-off**: Disassembly is accurate but verbose. Decompilation is readable but may contain errors or unrecognizable idioms (compiler-generated code patterns).

### Disassemblers and Decompilers

**IDA Pro** — Industry-standard interactive disassembler. Strengths: vast plugin ecosystem, scripting (IDAPython), debugger integration, cross-platform binary support. Weakness: expensive, steep learning curve, occasionally incorrect analysis (especially for x86). Widely used in malware analysis and vulnerability research.

**Ghidra** — NSA open-source reverse engineering suite (released 2019). Comparable to IDA Pro in many tasks; weaker plugin ecosystem but growing rapidly. Strengths: free, built-in decompiler (Ghidra Decompiler, based on academic research), collaborative reverse engineering (multi-user analysis), scriptable. Weakness: slower than IDA on large binaries, smaller community.

**Binary Ninja** — Modern alternative emphasizing automation and IL (Intermediate Language). Strengths: intuitive UI, fast SSA-based analysis, scriptable (Python), growing security community. Weakness: smaller user base than IDA, fewer Linux/embedded tool chains.

**Radare2** — Open-source, Unix-philosophy RE framework. Strengths: command-line driven, scriptable, portable. Weakness: steeper learning curve, smaller GUI ecosystem.

**Capstone** — Disassembly library (not a full tool); used as backend for many analysis frameworks. Supports x86, ARM, ARM64, MIPS, SPARC, etc. Language bindings (Python, C#, Java).

**Frida and Reverie** — Instrumentation frameworks (dynamic analysis, discussed below).

### Intermediate Representations

Advanced disassemblers use Intermediate Representations (IRs) for analysis abstraction:

- **IDA Pro MICROCODE** — Lifts x86 to a simpler IL for optimization and analysis
- **Binary Ninja's BNIL/LLIL/MLIL** — Multilevel IL hierarchy (low/mid/high level)
- **Ghidra's PCode** — Abstract, processor-independent IL

IRs enable complex analyses (data flow, type inference) and are the foundation for decompilation.

### Analysis Challenges

**Obfuscation** — Code intentionally mangled (variable renaming, control flow flattening, string encryption) to hinder RE. Decompilers struggle with heavily obfuscated code.

**Indirect jumps and calls** — Pointers computed at runtime; analysis must conservatively assume all possible targets. This inflates control-flow graphs and hinders function boundary detection.

**Packed and encrypted binaries** — Executable compressed or encrypted; the decompressor is small and trivial. Analyzers must unpack first (require execution or emulation).

**Position-independent code (PIE)** — Addresses are offsets, resolved at load time. Disassemblers must track relocations or emulate load to get accurate addresses.

**Mixed code/data** — Some architectures embed data in code or self-modify; distinguishing code from data is NP-hard without execution.

## Dynamic Analysis

Dynamic analysis observes program behavior at runtime: which functions are called, which memory is accessed, what state machine does it follow?

### Debugger-Based Analysis

**GDB (GNU Debugger)** — POSIX standard debugger. Capabilities: set breakpoints, step through code, read memory/registers, call functions, attach to running processes. Strength: universal, scriptable. Weakness: line-level granularity (requires debug symbols).

**WinDbg** — Windows kernel and user-mode debugger. Essential for Windows kernel development and driver analysis. Strengths: kernel integration, live kernel debugging. Weakness: Windows-only.

**LLDB** — LLVM debugger, primary on macOS. Capabilities similar to GDB; better C++ support. Python scripting via command-line extension API.

**Debugger scripting** — Write scripts to automate breakpoints, memory leaks, fuzzing. E.g., GDB with Python: `python print(gdb.execute('info registers'))`.

### Instrumentation and DBI Frameworks

**DBI (Dynamic Binary Instrumentation)** — Modify code on-the-fly during execution to insert observation points, without recompilation.

**Frida** — Leading open-source DBI framework. Inject JavaScript into running processes; inspect memory, call functions, patch behavior. Works on Linux, macOS, Windows, iOS, Android. Example:
```javascript
Interceptor.attach(Module.findExportByName(null, 'strlen'), {
  onEnter(args) { console.log('strlen:', Memory.readUtf8String(args[0])); }
});
```

**Valgrind** — DBI framework specifically for memory debugging. InstrumentationTools: Memcheck (find leaks/corruption), Helgrind (threading bugs), Massif (heap profiling).

**Intel Pin** — DBI tool for analyzing instruction-level behavior. Lower-level than Frida; used in academic research and performance characterization.

**DynamoRIO** — Similar to Pin; used for malware sandbox analysis, code coverage measurement.

### Tracing and Execution Flow

**Strace** — Trace system calls on Linux. Useful for understanding what a process does (files, network, signals).

**Ltrace** — Trace library calls (libc, pthreads). Similar to strace but at a higher level.

**ETW (Event Tracing for Windows)** — Windows kernel tracing infrastructure. Capture kernel context switches, disk I/O, registry access.

**tcpdump, Wireshark** — Network packet capture and analysis. Essential for protocol reverse engineering.

### Emulation

**QEMU** — Full-system or user-mode emulation. Run binaries for different architectures (ARM, MIPS) on x86 developers' machines. Used in malware analysis sandboxes.

**Unicorn** — Lightweight emulation engine (Python bindings); useful for emulating code snippets without full OS simulation.

## Symbol Resolution and Type Inference

Compiled binaries strip much information. Reverse engineering recovers some:

**Debug symbols** — If present (rarely in production), map addresses to source functions/variables. Tools like `objdump -t` or debuggers' `info symbol` expose symbol tables.

**Type inference** — Decompilers observe how data is used (array indexing, struct member access) and infer types. This is heuristic-based and imperfect.

**API recognition** — Known library signatures (symbol tables for standard C library, Windows API). Decompiler databases (e.g., IDA Pro's FLIRT) match sequences of instructions to library functions, reducing RE burden.

## Binary Patching

Modifying compiled binaries without source code.

### In-Memory Patching

**Debugger breakpoints + calls** — Set breakpoint, inspect/modify memory, resume. Used for one-off testing (`gdb` or `lldb`).

**Ptrace instrumentation** — Write your own debugger; intercept processes and inject behavior.

**Binary patching tools** — Modify executable file on disk:
- **Hex editing** — Manual byte-level changes (error-prone, breaks checksums/signatures)
- **Automated tools** — `patool`, custom scripts using `capstone` + `keystone` (assemble/disassemble)

### Issues with Binary Patching

**Instruction encoding** — x86 instructions have variable length; patching a 5-byte call into a 2-byte instruction requires careful branch redirection.

**Relative vs. absolute addressing** — Relocating code may require updating all address references.

**Digital signatures** — PE/Mach-O/ELF signatures must be regenerated or removed.

**Control-flow integrity (CFI)** — Modern OS kernels verify code hasn't been patched (e.g., macOS System Integrity Protection, Windows Driver Signing); circumventing CFI is often infeasible without root.

## Anti-Reverse-Engineering Techniques

**Obfuscation** — Intentional code complexity: renamed variables, flattened control flow, encrypted strings, bogus branches.

**Tamper detection** — Code detects checksums/signatures being recomputed or memory being modified; crashes or malfunctions if detected.

**Code signing** — OS-level verification on iOS, Android, Windows; unsigned or modified binaries are rejected.

**Packing and encryption** — Compress/encrypt the binary; decrypt at runtime. Unpacking requires either reverse engineering the unpacker or emulating execution up to decryption.

**Hardware-backed security** — TPM (Trusted Platform Module), secure enclaves (Intel SGX, ARM TrustZone) limit what code an attacker can observe or modify, even with physical access. Runtime attestation verifies code hasn't been tampered with.

**Jitter and RNG** — Non-deterministic behavior complicates instrumentation; attacker can't easily replay execution.

## Reverse Engineering in Practice: CTF and Vulnerability Research

**Capture The Flag (CTF)** — Competitions where participants exploit intentionally vulnerable binaries or systems. RE skills are core: find the vulnerability in a stripped binary, find the flag.

**Vulnerability research** — Analyze patched vs. unpatched versions of a program; the differences reveal patches, which inform hypotheses about the vulnerability being fixed.

**Malware analysis** — Understand what a piece of malware does, identify how it communicates with the attacker, discover indicators of compromise (IoCs).

## Defensive Perspectives: Hardening Against RE

**Minimize attack surface** — Keep code simple; easier to reverse but less vulnerable.

**Cryptographic verification** — Self-checking code (checksums, signatures) that crashes if modified. Not foolproof (can be patched out) but raises the bar.

**Compartmentalization** — Sensitive logic in hardware-enforced secure enclaves (SGX, TrustZone) where attacker code can't easily observe.

**Noise** — Redundant code, fake branches, and complexity slow RE without impacting functionality. Cost: code bloat, maintenance burden.

**Supply-chain signing** — Code signing prevents arbitrary binary modifications on OS-enforced platforms (iOS, Windows with UEFI Secure Boot).

## See Also

- security-best-practices (context for defensive strategies)
- security-container (binary integrity in containerized environments)
- antipatterns-hall-of-infamy (reverse-engineered vulnerabilities and lessons learned)