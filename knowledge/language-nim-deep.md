# Nim Deep Dive: Metaprogramming, Multi-Backend Compilation, and Memory Management

## Metaprogramming: Templates and Macros

Nim's metaprogramming system is a core strength, enabling zero-cost abstractions and compile-time code specialization. **Templates** are syntactic, **macros** operate on ASTs.

### Templates — Syntactic Substitution

Templates perform textual substitution before compilation, respecting Nim's scope rules.

```nim
template `+*`(a, b: int): int = a + (a * b)  # Inline operator

let result = 3 +* 5  # Expands to: 3 + (3 * 5) = 18

# Template with control flow
template safeDiv(a, b: int): int =
  if b != 0:
    a div b
  else:
    0

echo safeDiv(10, 2)   # 5
echo safeDiv(10, 0)   # 0

# Generic templates
template times(n: int, body: untyped): untyped =
  for _ in 0 ..< n:
    body

times(3):
  echo "Hello"  # Prints "Hello" three times
```

### Macros — AST Manipulation

Macros receive and return AST nodes, enabling sophisticated compile-time transformations.

```nim
import std/macros

# Simple macro: generate code at compile time
macro genCode(n: static[int]): untyped =
  result = newStmtList()
  for i in 0 ..< n:
    result.add quote do:
      echo `i`

genCode(3)  # Generates 3 echo statements

# Introspection macro
macro typeInfo(x: untyped): untyped =
  let t = getTypeInst(x)
  result = newLit($t)

echo typeInfo(42)      # "Literal(IntLit(42))"
echo typeInfo((1, 2))  # "Tuple"

# Quasiquoting with backticks
macro debug(x: untyped): untyped =
  result = quote do:
    echo `x` & " = " & $`x`

let foo = 42
debug(foo)  # Outputs: foo = 42

# Advanced: custom DSL
macro dsl(body: untyped): untyped =
  # Walk the AST, transform nodes
  var stmts = newStmtList()
  for node in body:
    if node.kind == nnkCall and node[0].kind == nnkIdent:
      let cmd = node[0].strVal
      if cmd == "say":
        stmts.add quote do:
          echo `node[1]`
  result = stmts

dsl:
  say "Hello, DSL!"
```

## Multiple Backends: C, C++, JavaScript, LLVM

Nim's architecture separates front-end (semantics) from back-end (codegen). This enables targeting multiple platforms from a single codebase.

```nim
# Detect compilation target
when defined(js):
  # JavaScript backend
  proc alert(msg: string) {.importjs: "alert(#)".}
  proc setTimeout(fn: proc(), ms: int) {.importjs: "setTimeout(()=>@[#], #)".}
elif defined(cpp):
  # C++ backend
  {.emit: """
    #include <iostream>
  """.}
else:
  # Default: C backend
  import std/posix

# Conditional imports and pragmas
when defined(release):
  const DEBUG = false
else:
  const DEBUG = true

# Multi-backend function selection
proc fib(n: int): int =
  when defined(js):
    # JavaScript: use iterative approach for speed
    var a, b = 0, 1
    for _ in 0 ..< n:
      (a, b) = (b, a + b)
    return b
  else:
    # C/C++: can afford recursion
    if n <= 1:
      return n
    return fib(n - 1) + fib(n - 2)

# FFI for C backend
when not defined(js):
  {.emit: """
    #include <math.h>
  """.}
  proc cSqrt(x: cdouble): cdouble {.importc: "sqrt".}
```

## Memory Management: ARC vs ORC

Nim 1.6+ introduced **Automatic Reference Counting (ARC)** and **Orc (Cycle-Collecting Reference Counting)**. The runtime no longer has a stop-the-world GC.

### ARC (Automatic Reference Counting)

Increments reference counts on assignment, decrements on scope exit. Fast predictable cleanup.

```nim
{.experimental: "strictFuncs".}

# ARC semantics
type Node = ref object
  value: int
  next: Node

proc createList(values: seq[int]): Node =
  var head: Node = nil
  var tail = head
  
  for v in values:
    var node = Node(value: v, next: nil)
    if isNil(head):
      head = node  # Reference count: 1
    else:
      tail.next = node  # Reference count incremented
      tail = node       # Old tail still referenced, count decrements
    # When node goes out of scope, its refcount decrements
  
  return head

# Sink parameters — transfer ownership
proc process(s: sink seq[int]) =
  # s's reference count is not incremented
  echo s[0]

var mySeq = @[1, 2, 3]
process(mySeq)  # mySeq ownership transferred, reference consumed
```

### ORC (Orc)

Extends ARC with cycle detection for data structures with circular references.

```nim
# Enable ORC
{.experimental: "destructors", gc: "orc".}

type Graph = ref object
  value: int
  edges: seq[Graph]

proc createCycle() =
  var a = Graph(value: 1)
  var b = Graph(value: 2)
  
  a.edges.add(b)
  b.edges.add(a)  # Creates cycle
  # ORC detects and collects this cycle automatically

# Destructors provide custom cleanup
type Resource = object
  handle: int

proc `=destroy`(x: var Resource) =
  # Custom cleanup called automatically
  if x.handle >= 0:
    closeHandle(x.handle)

proc `=dup`(x: Resource): Resource =
  # Called on copy
  result.handle = duplicate(x.handle)
```

## Concepts: Structural Typing Constraints

Concepts provide structured, compile-time polymorphism similar to Rust traits or TypeScript structural typing.

```nim
# Define what types must support
type Addable = concept x
  x + x is int

# Generic function constrained by concept
proc sum(items: seq[Addable]): int =
  result = 0
  for item in items:
    result += item + item

echo sum(@[1, 2, 3])           # Works: int is Addable
echo sum(@[1.5, 2.5, 3.5])     # Works: float is Addable

# Concept with multiple operations
type Orderable = concept x
  x < x
  x == x

proc sortedUnique(items: seq[Orderable]): seq[Orderable] =
  var sorted = sorted(items)
  var result = @[sorted[0]]
  
  for i in 1 ..< sorted.len:
    if sorted[i] != result[^1]:
      result.add(sorted[i])
  
  return result
```

## Effect System: Tracking Side Effects

Nim's effect system tracks side effects (IO, exceptions, etc.) at compile time.

```nim
# Tag functions with effects
proc pure(): int {.noSideEffect.} =
  42

proc withIO(msg: string) {.tags: [RootEffect].} =
  echo msg

# Functions without tags don't propagate effects
proc compute(): int =
  pure() + 10  # OK: calling noSideEffect function

# Mutable state also tracked
var global = 0

proc mutates() {.sideEffects.} =
  global += 1

# Effect pragma enforces no side effects
proc safeFn(x: int) {.noSideEffect.}: int =
  x * 2
```

## Async and Await

Nim integrates async/await for non-blocking concurrency, compiling to state machines.

```nim
import std/asynctools

proc fetchData(url: string): Future[string] {.async.} =
  # Simulate async I/O
  await sleepAsync(1000)
  return "data from " & url

proc main() {.async.} =
  let result1 = await fetchData("api1.com")
  let result2 = await fetchData("api2.com")
  
  echo result1, " and ", result2

waitFor main()

# Parallel async operations
proc concurrent() {.async.} =
  let f1 = fetchData("api1.com")
  let f2 = fetchData("api2.com")
  
  let r1 = await f1
  let r2 = await f2
  
  echo r1, " ", r2
```

## Nimble Package Manager

Nimble manages Nim packages, dependencies, and project configuration via `package.nimble`.

```nim
# Package configuration
packageName   = "mylib"
version       = "0.1.0"
author        = "Alice"
description   = "My cool library"
license       = "MIT"

# Dependencies
requires "nim >= 1.6.0"
requires "asynctools >= 1.8.0"

# Tasks
task test, "Run tests":
  exec "nim test tests/test_suite.nim"

task docs, "Generate docs":
  exec "nim doc --project src/mylib.nim"

# Custom build
before build:
  echo "Building..."
```

---

## See Also

- [Paradigm: Metaprogramming](paradigm-metaprogramming.md)
- [Language: Nim Conventions](language-nim.md)
- [Compiler Design — Frontend](compiler-design-frontend.md)
- [Memory Management](memory-management.md)