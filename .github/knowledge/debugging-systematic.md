# Systematic Debugging

## The Debugging Mindset

Debugging is not guessing. It's the scientific method applied to code: observe, hypothesize, test, conclude. The best debuggers are methodical, not lucky.

## The Scientific Method of Debugging

### 1. Reproduce Reliably

Before anything else, get a reliable reproduction. If you can't reproduce it, you can't verify you've fixed it.

```
Steps to reproduce:
1. Start with a clean state (fresh install, empty DB, cleared cache)
2. Document EXACT steps (not "click around until it breaks")
3. Note the environment (OS, browser, versions, config)
4. Identify minimum reproduction case (strip away everything unnecessary)
```

**The #1 debugging mistake: trying to fix something you can't reliably reproduce.**

### 2. Isolate the Problem

Narrow the search space systematically:

**Binary search debugging (wolf fencing):**
- The bug is somewhere in 1000 lines. Add a check at line 500.
- Is the state correct at line 500? If yes, bug is in 500-1000. If no, bug is in 1-500.
- Repeat. 10 iterations to find a bug in 1024 lines.

**Delta debugging:**
- Works on the input, not the code. You have a failing test case.
- Remove half the input. Still fails? Keep the smaller input.
- Doesn't fail? Try the other half. Narrow systematically.

**Git bisect (the ultimate isolator):**
```bash
git bisect start
git bisect bad HEAD          # current version is broken
git bisect good v1.2.0       # this version worked
# Git checks out a middle commit. Test it.
git bisect good              # or git bisect bad
# Repeat until it finds the exact commit that introduced the bug
# Automate: git bisect run ./test.sh
```

### 3. Understand Before Fixing

**Read the error message. Really read it.**
- Stack traces read bottom-to-top. The root cause is often near the bottom.
- The first error matters. Cascading errors are noise.
- "Line 42" means the error was _detected_ at line 42 — the _cause_ may be elsewhere.

**Rubber duck debugging:**
- Explain the code line-by-line to an inanimate object (or a patient colleague).
- The act of articulating your assumptions often reveals the invalid one.
- If you can't explain what a line does, that's your bug.

### 4. Form and Test Hypotheses

Each hypothesis must be testable and falsifiable:

```
❌ "Something is wrong with the database"
✅ "The query returns stale data because the cache TTL is too long"

Test: Disable cache → Does bug disappear?
  Yes → Hypothesis confirmed. Fix the cache.
  No  → Hypothesis falsified. Move to next.
```

**Never change multiple things at once.** One change → one test → one conclusion.

## Common Bug Categories

### Off-by-One Errors
```python
# Bug: skips last element
for i in range(len(items) - 1):  # should be range(len(items))
    process(items[i])

# Bug: fence post error
# Q: How many fence posts for a 100-meter fence with posts every 10m?
# Wrong: 100/10 = 10 posts.  Correct: 11 posts (including both ends)
```

### Null/Nil/None Dereferencing
```
The billion-dollar mistake (Tony Hoare, 2009):
"I call it my billion-dollar mistake. It was the invention of the null reference."

Fix: Use Option/Maybe types. Fail at compile time, not runtime.
```

### Race Conditions
Signs: "works fine in development, fails randomly in production"
- Non-deterministic failures
- Failures under load that disappear when adding logging (Heisenbug — see below)
- Different behavior with different timing

### State Mutation Bugs
```python
# Bug: mutating default argument
def add_item(item, items=[]):  # DEFAULT LIST IS SHARED
    items.append(item)
    return items

add_item("a")  # ["a"]
add_item("b")  # ["a", "b"]  — NOT ["b"]!
```

### Encoding Issues
If you see: `Ã©` instead of `é`, `ðŸ˜€` instead of 😀, or `???` — it's encoding.
- UTF-8 is always the answer. If your system doesn't default to UTF-8, fix that first.
- Double-encoding: data encoded as UTF-8, then encoded again as if it were Latin-1.

## Bug Taxonomy by Name

### Heisenbug
Changes behavior when you try to observe it. Adding a print statement "fixes" it. Removing the debugger makes it reappear.
- **Cause**: Usually timing-related. The observation changes timing enough to mask a race condition.
- **Fix**: Use logging that doesn't alter timing. Use thread sanitizers. Review synchronization.

### Bohrbug
Deterministic, reproducible. The well-behaved bug. Follows well-defined conditions.
- **Fix**: Standard debugging. Reliable reproduction → binary search → fix.

### Mandelbug
Appears chaotic with complex, hard-to-trace causality. Depends on many interacting factors.
- **Cause**: Complex state interactions, external dependencies, timing windows.
- **Fix**: Simplify. Reduce state space. Add invariant assertions.

### Schroedinbug
Code that should never have worked but somehow did — until someone read it and noticed the bug, at which point it stopped working.
- **Cause**: Undefined behavior that happened to produce correct results by coincidence.

### Aging Bug (Lapsed Listener / Memory Leak)
System works initially, degrades over time. Restarts "fix" it temporarily.
- **Cause**: Resource leaks — memory, file handles, DB connections, event listeners.
- **Fix**: Profiling over time. Watch for monotonically increasing metrics.

## Debugging Tools by Domain

### General
- **Debugger**: Step through code. Inspect state at breakpoints. Conditional breakpoints for loop bugs.
- **Logging**: Strategic print statements with context (timestamp, request ID, variable state).
- **Assertions**: `assert` statements that document and verify invariants.

### Memory
- **Valgrind** (C/C++): Memory leaks, use-after-free, buffer overflows.
- **AddressSanitizer** (ASAN): Compile-time instrumentation for memory errors.
- **Heap profilers**: Chrome DevTools (JS), `tracemalloc` (Python), `pprof` (Go).

### Concurrency
- **ThreadSanitizer** (TSAN): Data races in C/C++/Go.
- **Helgrind** (Valgrind): Lock ordering violations.
- **Go race detector**: `go test -race`.

### Performance
- **Profilers**: `perf` (Linux), Instruments (macOS), `py-spy` (Python), `pprof` (Go).
- **Flame graphs**: Visualize where CPU time is spent. Look for wide bars (hot paths).
- **APM tools**: Distributed tracing (OpenTelemetry, Jaeger, Datadog).

### Network
- **Wireshark/tcpdump**: Packet-level inspection.
- **curl -v**: HTTP request/response debugging.
- **mitmproxy**: HTTPS interception for API debugging.
- **DNS: `dig`, `nslookup`**: Resolve DNS issues before blaming the server.

## Debugging Heuristics

1. **It's probably your code.** Not the compiler. Not the OS. Not the hardware. Your code.
2. **The most recent change is the most likely suspect.** What changed since it last worked?
3. **Simplify until it works, then add back complexity.** Minimum viable reproduction.
4. **Check the boundaries.** First item, last item, empty collection, single item, maximum size.
5. **Check the types.** String "0" vs integer 0. `null` vs `undefined` vs `""` vs `0` vs `false`.
6. **"Works on my machine"** means your environment differs from the failing one. Find the difference.
7. **If you're stuck for 30 minutes, take a break.** The answer often appears in the shower, on a walk, or after sleep.
8. **Explain it to someone else.** If no one is available, write the question for Stack Overflow. You'll often solve it while writing.

## The Post-Mortem

After fixing a bug that cost significant time:

1. **What was the root cause?** (not the symptom)
2. **How was it introduced?** (what process failed?)
3. **How was it detected?** (monitoring? user report? accident?)
4. **How can we prevent the class of bug?** (type system? test? lint rule? assertion?)
5. **Write the test that would have caught it.**

---

*Sources: "Why Programs Fail" (Andreas Zeller), "Debugging" (David Agans), "The Practice of Programming" (Kernighan & Pike), ACM/IEEE software debugging literature*
