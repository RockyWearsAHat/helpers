# Debugging Techniques — Printf, Breakpoints, Binary Search, Rubber Duck, Post-Mortem, Remote, and Time-Travel

Debugging is a conversation between hypothesis and evidence. Different techniques gather evidence at different speeds and scales. Master multiple strategies; choose based on the problem's size and urgency.

## Printf Debugging (Instrumentation Logging)

The simplest, most portable technique: add output statements to trace execution.

**Strengths:**
- Zero setup; works on every platform and in any environment
- Output is durable; can be stored in logs and analyzed later
- Highly selective; you control exactly what prints
- No debugger overhead; minimal performance impact

**Weaknesses:**
- Reactive; you must predict what to print before running the code
- Requires recompilation or restart to adjust logging
- Produces noise when too verbose; easy to miss the signal
- Doesn't let you inspect state mid-execution without printing

**When to use:** First-pass hypothesis generation, concurrent/timing-sensitive bugs, deployed systems where attaching a debugger is impossible.

**Anti-pattern:** Leaving `console.log()` or `print()` statements in production code without structured logging. Treat instrumentation logging as first-class infrastructure, not debugging debris.

## Debugger Features: Breakpoints, Watchpoints, Conditional Breaks

Interactive debuggers (GDB, LLDB, VS Code Debug) let you pause execution and inspect state directly. Each feature addresses a different question.

### Breakpoints (Pause at a Location)

Execute normally until reaching a line, then pause.

```
break main.py:42           # Line breakpoint
break handle_request      # Function breakpoint
break file.py:100 if x>10 # Conditional breakpoint (only pause if condition is true)
```

**Conditional breakpoints** are powerful for large loops or high-frequency calls. Instead of reaching a breakpoint 1000 times and pressing "continue" 999 times, break only when the condition matters.

**Smart technique:** Use return-value breakpoints to catch error conditions:
```
break process_payment if result != SUCCESS
```

### Watchpoints (Pause When a Variable Changes)

Pause whenever a specific variable or memory location changes value.

```
watch x                    # Pause if x changes
rwatch x                   # Pause if x is read
awatch x                   # Pause if x is read or written
```

Watchpoints are invaluable for unexplained state corruption. Instead of scanning code, let the debugger find where the variable was mutated.

**Caveat:** Watchpoints use CPU breakpoint registers (typically 4 per core). They're expensive to maintain and may not work on all platforms; check your debugger's documentation.

### Stack Inspection

Once paused, examine the call stack to understand how you reached this point.

```
backtrace / bt             # Show all frames in the call stack
frame 3                    # Jump to frame 3
info locals                # Show all local variables in the current frame
info args                  # Show function arguments
print variable_name        # Inspect a variable's value
print *pointer             # Dereference a pointer
```

**Mental model:** Each frame captures a frozen moment in a function call. Walking the stack shows the function's "ancestry."

### Time-Stepping (Single-Stepping)

Execute one line of code, pausing after each line.

```
step                       # Step into function calls (follow into functions)
next                       # Step over function calls (execute entire call)
finish                     # Execute until the current function returns
continue                   # Resume normal execution
```

The distinction matters: `step` into a loop means stepping through every iteration. `next` skips over the entire loop. Use `next` to move faster through code you trust; use `step` when entering suspect code.

## Binary Search Debugging (Divide and Conquer)

When you know the problem started between two commits or the bug is in a function's middle section, use binary search to narrow the scope.

### Technique: Code Bisection

1. **Working state:** You can reproduce the bug. You have a known-good version (e.g., last week's commit).
2. **Search the midpoint:** Test a commit halfway between good and bad.
3. **Eliminate half:** If the midpoint is still good, the bug is in the second half. Mark the first half as good. If the midpoint is bad, the bug is in the first half. Mark the second half as bad.
4. **Repeat:** Binary search the remaining half. O(log N) narrowing instead of O(N) linear scanning.

**Tool support:**
```
git bisect start
git bisect bad HEAD          # Mark current commit as bad
git bisect good v1.0        # Mark v1.0 as good
# Git directs you to midpoints; test each one and report:
git bisect good             # Midpoint is good, search later commits
git bisect bad              # Midpoint is bad, search earlier commits
git bisect reset            # Exit bisect
```

### Within a Function

Use the same principle to narrow down where in a function the state corrupts:
1. Run halfway through the function.
2. Print the value of the suspect variable.
3. If it's already corrupted, the bug is in the first half. If it's clean, the bug is in the second half.
4. Repeat.

**Mental model:** Each test is an experiment. Design tests to eliminate large regions of code, not to inch forward line by line.

## Rubber Duck Debugging

Explain the code line-by-line to a rubber duck (or any inanimate object, or another person). 

**Why this works:** Articulating logic forces you to engage both sides of your brain. Vagueness that sounded reasonable in your head becomes obvious when spoken aloud. The duck doesn't judge; you can try explanations on it without ego.

**The anti-pattern:** Trying to explain code and discovering your explanation doesn't match what the code actually does. This is not failure; it's the moment of insight.

**When to use:** Before opening a debugger. 5 minutes of verbal explanation often catches bugs faster than 30 minutes of stepping through a debugger. Pairs well with the printf technique for generating hypotheses.

## Post-Mortem Debugging: Core Dumps and Crash Analysis

When a production system crashes, you cannot rerun it. Instead, the OS can capture a core dump — a snapshot of memory at crash time. Later, you can analyze the frozen state.

### Enabling Core Dumps

```bash
# macOS: Enable unlimited core dumps
ulimit -c unlimited

# Linux: Enable core dumps
ulimit -c unlimited

# Set a core dump directory (Linux)
echo /var/crash/core.%p > /proc/sys/kernel/core_pattern
```

### Analyzing a Core Dump

```bash
gdb ./my_program /path/to/core.12345

# Inside GDB:
(gdb) bt                   # Show stack trace from the crash
(gdb) frame 0              # Jump to the frame that crashed
(gdb) info locals          # Inspect variables at crash site
(gdb) print errno          # Check error numbers
```

**Constraints:** Core dumps capture memory exactly at crash time. You can examine state but cannot step forward. Time-travel debugging (rr, below) is more powerful but requires setup beforehand.

**Modern practice:** Cloud platforms (AWS, GCP, Azure) offer crash reporting services that do post-mortem analysis automatically. App crash reporting SDKs (e.g., Sentry, Rollbar) capture stack traces and context. Use these for production; they're designed for scale.

## Remote Debugging

Attach a debugger to a program running on a different machine (embedded device, server, container).

### Setup: Connect a GDB Client to a Remote Target

**On the target machine (server/device):**
```bash
# Start a GDB server listening on port 9001
gdbserver localhost:9001 ./my_program arg1 arg2
```

**On the development machine:**
```bash
# Connect the GDB client to the server
gdb ./my_program
(gdb) target remote server-ip:9001
(gdb) break main
(gdb) continue
```

Now you step through the program on the target machine using your local debugger.

### Common Scenarios

**Docker container:** The host machine's debugger cannot introspect container memory directly. Start the container with GDB server, or use container-native tools (e.g., Docker's debug command).

**Embedded device:** Upload the binary to the device, start gdbserver, connect from the host.

**Production server:** Production servers typically cannot run gdbserver due to security and performance. Use post-mortem (core dumps) or time-travel (rr) instead.

## Time-Travel Debugging: Record and Replay (rr)

The most powerful debugging technique for Heisenberg bugs (bugs that disappear when you try to debug them, due to timing changes introduced by the debugger's overhead).

**rr (Record and Replay)** captures all system calls and non-deterministic events during an execution, then replays the execution deterministically with full debugging control.

### Basic Usage

```bash
# Record the execution
rr record ./my_program arg1 arg2

# Replay and debug
rr replay -d gdb           # Open GDB on the recording

# Inside GDB during replay:
(gdb) reverse-step         # Step backward one instruction
(gdb) reverse-continue     # Continue backward until a breakpoint
(gdb) reverse-finish       # Reverse execute until the function returns
```

### When to use rr

- **Timing-dependent bugs:** A program works normally but crashes under high load or timing pressure. The debugger's overhead changes timing. rr captures the original timing and lets you replay it deterministically.
- **Race conditions:** Two threads interact unpredictably. rr locks in the exact scheduling; replaying always produces the same interleaving.
- **Intermittent bugs:** A bug happens randomly. Record many executions until you capture the failure, then replay it as many times as needed to understand.

### Constraints

- **Linux and macOS only** (no native Windows support; WSL works).
- **Significant memory overhead:** Recordings can be gigabytes for long-running processes.
- **Modern systems with many cores:** rr's record overhead increases with CPU features. Single-threaded programs are fastest to record.

### Alternative: Time-Travel Debugging in Cloud

Some cloud platforms (Azure, AWS) and IDEs (Visual Studio) offer time-travel debugging without rr's overhead, using specialized hardware features. If your platform offers it, use it.

## Common Debugging Workflow

1. **Reproduce the bug consistently.** If it's intermittent, use rr to record multiple attempts.
2. **Use printf logging to narrow the region.** Print at key checkpoints. Which checkpoint's output is missing or wrong?
3. **Once narrowed to a function or loop, attach a debugger.** Set a breakpoint near the narrowed region.
4. **Inspect the stack and variables.** Look for state that contradicts your mental model.
5. **Test a hypothesis with watchpoints or conditional breakpoints.** Verify your theory.
6. **If timing-related, replay with rr.** Intermittent bugs often disappear under a traditional debugger.

## Choosing a Technique: Decision Matrix

| Scenario | Best Technique |
|----------|---|
| First encounter with a bug | Printf logging + rubber duck |
| Bug is reproducible, localized | Breakpoint in a debugger |
| Bug is intermittent/flaky | rr (record-replay) |
| Bug only happens on production | Post-mortem (core dumps) |
| Variable corruption, unknown source | Watchpoints |
| Need to step back in time | Reverse debugging in rr |
| Program hangs/deadlock | Breakpoint + backtrace inspection |
| Bug only happens under load | rr, or analyze logs from load test |
| Embedded/cross-platform | Remote debugging (gdbserver) |

## Anti-Patterns

**Debugger thrashing:** Stepping through thousands of lines without a hypothesis. This is expensive and demoralizing. Use printf logging first to form a hypothesis.

**Binary search fatigue:** Running the entire test suite manually after each bisect. Automate: `git bisect run ./test.sh` runs tests automatically.

**Core dump bloat:** Capturing unused core dumps and filling up disk. Set core dump limits and rotate old dumps.

**Ignoring instrumentation:** Production systems should emit logs and metrics. Post-mortem debugging is hard; knowing what happened before the crash is easier with observability.

---

**See also:** debugging-systematic.md (scientific method for reasoning about bugs), systems-debugging-tools.md (GDB, LLDB, strace, perf in detail), error-handling-patterns.md (preventing bugs with error types), observability-logging.md (structured logging for visibility)