# Systems: Low-Level Debugging Tools — GDB, LLDB, Tracing, and Profiling

Low-level debugging tools allow inspection and manipulation of running programs at the machine code level. They are indispensable for diagnosing crashes, performance bottlenecks, memory corruption, and race conditions.

## Debuggers: GDB and LLDB

**GDB** (GNU Debugger) and **LLDB** (LLVM Debugger) are the primary interactive debuggers on Unix-like systems. Both support:

- **Breakpoints**: pause execution when reaching a line or address, with optional conditions (e.g., break if variable == 5).
- **Stepping**: execute one line (step) or skip into function calls (next).
- **Stack inspection**: view local variables, parameters, and function call frames.
- **Expression evaluation**: compute expressions in the current scope, invoking functions if needed.
- **Watchpoints**: break when a memory location is read or written.
- **Hardware breakpoints**: break at CPU-level (limited count, e.g., 4 on x86-64), no instruction rewriting needed.

GDB is older and more mature; LLDB is tighter with LLVM/Clang and generally faster. Both support remote debugging (e.g., debugging a program on an embedded device via a stub).

### Debuginfo and Symbols

Debuggers rely on **debug information** (usually DWARF on Unix, embedded in a separate .debug section) to map machine code back to source lines, associate registers with variables, and describe type layout. The `-g` compiler flag enables debug generation; `-O0` disables optimization to keep the code readable. Higher optimization levels inline functions and reorder code, making debugging confusing even with debuginfo present.

## System Call Tracing: strace and ltrace

**strace** intercepts and logs all system calls (syscalls) made by a process: `open()`, `read()`, `write()`, `mmap()`, `fork()`, etc. It shows arguments, return values, and errno codes. Useful for understanding I/O patterns, file access, process spawning, and permission errors.

**ltrace** is similar but traces library calls (function calls within libc, glibc, etc.) instead of syscalls. Combined, they reveal the full call flow from application code through libraries to the kernel.

Both tools use `ptrace()`, a system call that allows one process to inspect and control another. They rewrite instructions (on some architectures) to insert breakpoints, or use the kernel's single-step feature.

## Tracing: dtrace, BPF, and perf

**dtrace** (originally Solaris, ported to macOS and some Linux distributions) allows dynamic instrumentation of the kernel and userspace. A dtrace script can tap into any function entry/exit, system call, or other event, and execute a handler (aggregating statistics, filtering, printing). Unlike strace, dtrace does not pause the process for each event, making it suitable for high-throughput systems.

**eBPF** (extended Berkeley Packet Filter) is a modern alternative, originally for network packet filtering but evolved into a general kernel tracing mechanism. Programs written in a restricted C-like language are JIT-compiled into bytecode and run in the kernel with minimal overhead. Tools like **BCC** and **bpftrace** provide higher-level interfaces. BPF can attach to kernel functions, tracepoints, and user probes, capturing stack traces, aggregating statistics, and feeding data to userspace for display.

**perf** (Linux performance events) combines CPU performance counters (hardware events: cache misses, branch mispredictions, etc.) with software tracing. `perf record` samples the process at intervals (e.g., every 1000 CPU cycles), recording the call stack. `perf report` produces a flame graph or text report showing where time is spent.

The trade-off between tracing and sampling: tracing sees every event (complete but overhead), sampling approximates high-frequency events (fast but probabilistic).

## Memory Debugging: Sanitizers

Compiler-integrated **sanitizers** instrument code to catch memory errors at runtime:

- **AddressSanitizer (ASan)** detects use-after-free, buffer overflow, and double-free by maintaining a shadow memory region (1 byte tracks state of 8 app bytes). Overhead is ~2x slowdown.
- **MemorySanitizer (MSan)** detects use of uninitialized memory by tracking which bytes are initialized. Slower than ASan; used less frequently.
- **ThreadSanitizer (TSan)** detects data races by instrumenting all memory accesses and synchronization operations. Very slow (10-100x), limited to testing.
- **UndefinedBehaviorSanitizer (UBSan)** catches undefined behavior: signed overflow, misaligned pointers, invalid enum values, etc.

Sanitizers require recompilation with `-fsanitize=address` (or similar) and typically abort on first error, allowing post-mortem analysis. They are sensitive (catch things other tools miss) but noisy (false positives in some cases).

## Bounds Checking: Valgrind

**Valgrind** is a binary instrumentation framework that intercepts and redirects all memory access and system calls through a dynamic binary translator. Its primary tool, **Memcheck**, detects memory leaks (heap objects never freed) and invalid access (use-after-free, overflow) by maintaining metadata for every heap block.

Valgrind trades speed for thoroughness: ~20-50x slowdown. It requires no recompilation but cannot detect uninitialized reads as accurately as MSan. Useful for regression testing and development, not production.

## Core Dumps and Post-Mortem Analysis

When a process crashes (segfault, abort), the OS can write an image of memory and registers to a **core file**. GDB can load the core file (`gdb program corefile`) and inspect state at crash time: registers, memory, variables, stack. A core file is a snapshot; the process cannot be stepped or resumed.

Core generation is controlled by `ulimit -c`. Very large processes produce multi-gigabyte core files; tools like `eu-unstack` extract only the interesting parts (stacks, relevant memory).

## Reverse Debugging and Time Travel

Some debuggers (rr, Pernosco) support **record and replay**: a program is recorded (capturing all non-deterministic events: syscalls, signals, timing) then replayed deterministically. The debugger can then step backward through time, exiting a function to see its caller, etc. Powerful for race conditions and intermittent bugs, but recording overhead is high.

## Breakpoint Mechanisms

Debuggers use different mechanisms to implement breakpoints:

1. **Instruction rewriting**: replace the instruction at the breakpoint with a trap (int3 on x86), reserving the original instruction. Requires writable code (impossible on some systems) and must handle concurrency.
2. **Hardware breakpoints**: configure CPU debug registers (4-8 on many Intel chips) to trap on memory access. No code modification, but limited by CPU support.
3. **Kernel support**: some kernels offer a sys_breakpoint syscall or similar.

The choice depends on OS and architecture. Instruction rewriting is most flexible; hardware breakpoints are used when code is read-only.

## Profiling and Flame Graphs

**Profilers** sample execution periodically (e.g., every 10 ms) and record the call stack. Over time, a profile shows where the program spends most time. **Flame graphs** visualize this: a horizontal bar for each function, width proportional to time spent, stacked to show call chains. Flame graphs guide optimization by highlighting hot paths.

On-CPU profilers (perf, py-spy, pprof) focus on functions holding the CPU. Off-CPU profilers trace what functions are blocked waiting for I/O, locks, etc. Combined, they reveal if slowness is CPU or I/O-bound.

## Static and Dynamic Analysis Integration

Debuggers often integrate with static analysis: breakpoints can be set by source symbol name (gdb: `break foo.c:10` or `break function_name`), and expressions evaluated using type information from debuginfo. Modern debuggers can also evaluate Python / custom scripts at breakpoints (gdb's Python API, lldb's Python scripting), automating complex inspections.

## Trade-offs and Limitations

- **Overhead**: strace, ltrace, and TSan are slow; sampling profilers are fast but may miss short-lived events.
- **Visibility**: tracing sees all events but requires analysis; sampling approximates behavior efficiently. Hardware events (perf) expose CPU internals but miss application-level semantics.
- **Intrusiveness**: inserting instrumentation changes timing and can hide races or Heisenbugs (bugs that disappear under observation).
- **Scope**: user-level debuggers cannot inspect kernel code; kernel debuggers (kdb, gdb over serial) are OS-specific.

For reliable root cause analysis, combine multiple tools: strace to understand syscalls, perf to identify hot paths, ASan to catch memory errors, and interactive debugging to confirm hypotheses.

See also: memory management, concurrency patterns, binary formats, performance optimization.