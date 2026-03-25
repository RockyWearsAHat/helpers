# Nim Best Practices

## Nim Philosophy

Nim is a statically typed compiled language that combines Python-like syntax with C-level performance. It compiles to C, C++, JavaScript, or LLVM, giving it unique deployment flexibility.

- **Efficient and expressive**: Python readability with C performance. Zero-cost abstractions via compile-time evaluation.
- **Metaprogramming**: Templates, macros, and compile-time execution are core to idiomatic Nim.
- **Multi-backend**: Compile to C for systems work, to JavaScript for web, or to C++ for game engines.

## Core Syntax

```nim
# Variables
var mutable = 10      # mutable
let immutable = 20    # bind once
const compiled = 30   # compile-time constant

# Type inference
var name = "Alice"    # inferred as string
var age: int = 30     # explicit

# Strings
let greeting = fmt"Hello, {name}! Age: {age}"

# Sequences (dynamic arrays)
var nums = @[1, 2, 3, 4, 5]
nums.add(6)
echo nums[0]  # 1

# Arrays (fixed-size)
var fixed: array[5, int] = [1, 2, 3, 4, 5]

# Tables (hash maps)
import std/tables
var scores = {"Alice": 95, "Bob": 87}.toTable
scores["Charlie"] = 92
```

## Procedures and Overloading

```nim
# Uniform function call syntax (UFCS)
proc double(x: int): int = x * 2
echo double(5)     # 10
echo 5.double()    # 10  (method syntax)
echo 5.double      # 10  (parens optional with no args)

# Overloading
proc area(r: float): float = PI * r * r           # circle
proc area(w, h: float): float = w * h             # rectangle

# Default and named parameters
proc greet(name: string, greeting = "Hello"): string =
  fmt"{greeting}, {name}!"

echo greet("Alice")                    # "Hello, Alice!"
echo greet("Bob", greeting = "Hi")     # "Hi, Bob!"

# Result variable (implicit return)
proc factorial(n: int): int =
  result = 1
  for i in 2..n:
    result *= i

# Closures
proc makeCounter(): proc(): int =
  var count = 0
  return proc(): int =
    count += 1
    return count
```

## Types and Objects

```nim
# Object types
type
  Shape = ref object of RootObj
    x, y: float

  Circle = ref object of Shape
    radius: float

  Rectangle = ref object of Shape
    width, height: float

# Methods (multi-dispatch)
method area(s: Shape): float {.base.} =
  raise newException(CatchableError, "abstract")

method area(c: Circle): float =
  PI * c.radius * c.radius

method area(r: Rectangle): float =
  r.width * r.height

# Variant types (discriminated unions)
type
  NodeKind = enum nkInt, nkFloat, nkString
  Node = object
    case kind: NodeKind
    of nkInt: intVal: int
    of nkFloat: floatVal: float
    of nkString: strVal: string

# Distinct types (no implicit conversion)
type
  Dollars = distinct float
  Euros = distinct float
# Can't accidentally add Dollars + Euros
```

## Templates and Macros

```nim
# Templates (hygienic, inlined at compile time)
template withFile(f, filename, mode, body: untyped) =
  var f: File
  if open(f, filename, mode):
    try:
      body
    finally:
      close(f)
  else:
    raise newException(IOError, "Cannot open: " & filename)

withFile(file, "data.txt", fmRead):
  echo file.readAll()

# Macros (AST transformation)
import std/macros

macro debug(args: varargs[untyped]): untyped =
  result = newStmtList()
  for arg in args:
    result.add quote do:
      echo astToStr(`arg`), " = ", `arg`

var x = 42
var y = "hello"
debug(x, y)
# Output: x = 42
#         y = hello
```

## Error Handling

```nim
# Exceptions
try:
  let data = readFile("config.json")
  let config = parseJson(data)
except IOError as e:
  echo "File error: ", e.msg
except JsonParsingError:
  echo "Invalid JSON"
finally:
  cleanup()

# Option type
import std/options
proc findUser(id: int): Option[User] =
  if id in db: some(db[id])
  else: none(User)

let user = findUser(42)
if user.isSome:
  echo user.get.name

# Result type (Nim 2.0+)
type Result[T, E] = object
  case ok: bool
  of true: value: T
  of false: error: E
```

## Async/Await

```nim
import std/asyncdispatch

proc fetchUrl(url: string): Future[string] {.async.} =
  let client = newAsyncHttpClient()
  try:
    return await client.getContent(url)
  finally:
    client.close()

proc main() {.async.} =
  let content = await fetchUrl("https://example.com")
  echo content.len, " bytes"

waitFor main()
```

## Key Rules

1. **Use `let` by default.** Only `var` when mutation is needed. `const` for compile-time values.
2. **Nim is case-insensitive and ignores underscores** in identifiers: `fooBar`, `foo_bar`, `foobar` are the same. Pick one style and be consistent.
3. **Use templates for zero-cost abstractions.** They inline at compile time — no runtime overhead.
4. **Prefer value types over ref types** unless you need reference semantics or inheritance.
5. **Use `--gc:arc` or `--gc:orc`** (default in Nim 2.0). ARC is deterministic. ORC handles cycles.
6. **Compile with `-d:release`** for production. Debug builds are intentionally slow (bounds checks, etc.).

---

*Sources: Nim Manual (nim-lang.org), Nim in Action (Picheta), Nim by Example, Nim Standard Library docs*
