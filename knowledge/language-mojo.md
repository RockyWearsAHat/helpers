# Mojo — Python Superset with Systems Programming

## Overview

Mojo is a programming language that extends Python with systems-level capabilities. The core premise: write Python code that compiles to high-performance machine code via MLIR (Multi-Level Intermediate Representation). Python remains the surface syntax, but Mojo adds explicit memory management, SIMD types, and static compilation.

Mojo targets AI/ML workloads (training, inference) where Python's dynamic nature creates performance bottlenecks. Projects like MAX (Modular's inference platform) use Mojo to optimize tensor operations while maintaining Python expressiveness.

## Python Superset

### Full Python Compatibility

Valid Python is (nearly) valid Mojo:

```python
def fibonacci(n):
    if n <= 1:
        return n
    else:
        return fibonacci(n - 1) + fibonacci(n - 2)

result = fibonacci(10)
print(result)  # works as expected
```

The syntax is identical. Mojo interprets this as Python would, but the compiler can translate it to native code.

### Dynamic vs. Static Execution

By default, Mojo runs functions as Python (dynamic dispatch):

```python
def add(a, b):
    return a + b

add(1, 2)      # i32 + i32
add(1.0, 2.0)  # f64 + f64 (same function, different runtime)
```

But you can declare explicit types to enable static compilation:

```python
def add(a: Int32, b: Int32) -> Int32:
    return a + b

add(1, 2)      # compiles to i32 add instruction
add(1.0, 2.0)  # compile error: f64 not Int32
```

Type annotations **enable** performance but don't require Python changes.

## Ownership and Borrowing Model

Mojo adopts Rust-like ownership to prevent use-after-free and double-free bugs without garbage collection:

### Ownership Rules

1. Each value has one owner
2. Transfer ownership explicitly or default to copy
3. Borrowing allows references without transferring

```python
def take_ownership(x: String) -> None:
    # x is owned by this function
    print(x)
    # x is dropped here

original = String("hello")
take_ownership(original)
# original is no longer accessible (moved)
```

### Move Semantics

Transfer ownership (avoid copying expensive objects):

```python
def make_vector() -> List[Int32]:
    return List[Int32]([1, 2, 3])  # move, not copy

v = make_vector()  # v owns the list
# no reference counted overhead
```

### Borrowing

Allow temporary access without taking ownership:

```python
def borrow_string(ref s: String) -> Int:
    return len(s)  # can read, not consume

text = String("hello")
length = borrow_string(text)
print(text)  # text still valid
```

**Mutable borrows**:

```python
def modify(mut ref v: List[Int32]) -> None:
    v.append(42)

my_list = List[Int32]([1, 2, 3])
modify(my_list)
# my_list is now [1, 2, 3, 42]
```

**Borrow rules**: no mutable and immutable borrow simultaneously (enforced at compile time).

## Parameters vs. Arguments

Mojo distinguishes **parameters** (compile-time) from **arguments** (runtime):

```python
def make_array[T: AnyType](size: Int) -> Array[T]:
    # T is a parameter (compile-time template variable)
    # size is an argument (runtime value)
    return Array[T](size)

arr = make_array[Int32](10)  # [Int32] is a parameter
```

This enables **static specialization**: the compiler generates specialized code for each `T` and `size` combination, like C++ templates.

```python
def process[dtype: AnyType](data: Array[dtype], scale: Float64):
    # generates optimized code for each dtype
```

## SIMD Types and Autotuning

### SIMD (Single Instruction Multiple Data)

Process multiple values in parallel:

```python
from builtin import SIMD

def add_vectors(a: SIMD[DType.float32, 4], 
                b: SIMD[DType.float32, 4]) -> SIMD[DType.float32, 4]:
    return a + b

v1 = SIMD[DType.float32, 4](1.0, 2.0, 3.0, 4.0)
v2 = SIMD[DType.float32, 4](5.0, 6.0, 7.0, 8.0)
result = add_vectors(v1, v2)  # vectorized add
```

`SIMD[dtype, size]` represents a vector of `size` elements of type `dtype`. Operations are vectorized by default.

### Autotuning

Mojo's **autotuning** framework searches for optimal parameters:

```python
@autotune
def kernel[dtype: AnyType, tile_size: Int, num_tiles: Int](
    A: Array[dtype], 
    B: Array[dtype], 
    C: Array[dtype]):
    # kernel implementation
    pass

# Mojo tests different tile_size/num_tiles values to find best performance
```

This addresses a key AI problem: optimal kernel parameters vary per hardware. Autotuning generates variants and benchmarks them.

## GPU Programming

Mojo can target GPUs via lower-level compilation:

```mojo
fn gpu_kernel[T: AnyType](data: Array[T]):
    # compiles to GPU kernels (CUDA, Metal, HIP)
    pass
```

The MAX framework (Modular's inference platform) uses Mojo for GPU-accelerated kernels on NVIDIA, AMD, and Apple silicon.

## Python Interop

Call Python libraries from Mojo:

```python
from PythonInterface import Python

let np = Python.import_module("numpy")
let arr = np.array([1, 2, 3])

# NumPy operations work seamlessly
let result = arr * 2
```

**Reverse interop**: Python can call Mojo functions (marshalled as Python callables).

This enables **gradual migration**: write performance-critical sections in Mojo, keep rest in Python.

## Struct vs. Class

### Struct (Value Type)

Lightweight, stack-allocated:

```mojo
struct Point:
    var x: Float32
    var y: Float32
    
    fn distance(self) -> Float32:
        return (x * x + y * y) ** 0.5
```

**Stack semantics**: `Point` is copied when assigned, no indirection.

```mojo
p1 = Point(3.0, 4.0)
p2 = p1  # copy, not reference
p1.x = 0  # p2.x unchanged
```

### Class (Reference Type)

Heap-allocated, garbage-collected (or owned):

```mojo
class User:
    var name: String
    var age: Int32
    
    fn __init__(inout self, name: String, age: Int32):
        self.name = name
        self.age = age
```

**Reference semantics**: assignment shares the same object.

```mojo
u1 = User("Alice", 30)
u2 = u1  # reference, not copy
u1.age = 31  # u2.age also 31
```

**When to use**:
- **Struct**: small data, frequent copies, stack efficiency (Points, SIMD vectors)
- **Class**: large objects, shared state, mutable references

## Type System

Mojo combines Python's duck typing with optional static typing:

```mojo
def dynamic(x):
    return x + x  # works for any x with __add__

def static(x: Int32) -> Int32:
    return x + x  # only Int32

def generic[T: AnyType](x: T) -> T:
    return x + x  # generic, T must support __add__
```

**Traits** define type constraints (inspired by Rust):

```mojo
trait Addable:
    fn __add__(self, other: Self) -> Self: ...

fn sum[T: Addable](items: List[T]) -> T:
    # T must implement __add__
    result = items[0]
    for item in items[1:]:
        result = result + item
    return result
```

## Compilation and Execution

Mojo compiles to MLIR → lower-level IR → LLVM → machine code:

```bash
mojo build myprogram.mojo        # compile to executable
mojo myprogram.mojo              # run directly (JIT)
```

**Optimization levels**:
- `-O0`: no optimization (debugging)
- `-O2`: typical optimization
- `-O3`: aggressive optimization

## Current Limitations

- **Immature ecosystem**: fewer libraries than Python
- **Incomplete standard library**: many Python modules unavailable
- **Interop overhead**: calling Python functions has marshalling cost
- **Type inference**: manual annotations often required
- **Path to 1.0**: Mojo is pre-release (as of 2026); APIs can change
- **GC**: optional but not fully integrated with ownership model

## Common Use Cases

- **High-performance ML**: training kernels, inference optimization
- **Numerical computing**: tensor operations, simulations
- **Systems programming**: where Python is too slow, Mojo is as expressive
- **Gradual optimization**: start in Python, optimize hot paths in Mojo

## see also

- [language-rust.md](language-rust.md) — ownership model comparison
- [language-python.md](language-python.md) — Python conventions
- [compiler-design-backend.md](compiler-design-backend.md) — MLIR compilation
- [genai-training-infrastructure.md](genai-training-infrastructure.md) — ML workload context