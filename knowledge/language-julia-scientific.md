# Julia for Scientific Computing: Multiple Dispatch, Performance, and Ecosystem

## Multiple Dispatch: The Core Design

Julia's defining feature—function selection based on the types of ALL arguments at call time. This replaces OOP's single-dispatch (method resolution on one receiver object).

```julia
# Single argument (still dispatch)
area(circle::Circle) = π * circle.r^2
area(rect::Rectangle) = rect.w * rect.h

# Multiple dispatch (many-to-many)
collide(a::Circle, b::Circle) = distance(a, b) < a.r + b.r
collide(a::Circle, b::Rectangle) = circle_rect_collision(a, b)
collide(a::Rectangle, b::Circle) = collide(b, a)  # reuse by argument order
collide(a::Rectangle, b::Rectangle) = rect_rect_collision(a, b)

# Generic function resolves at runtime
collide(Circle(0, 0, 1), Rectangle(0, 0, 2, 2))  # dispatch → circle_rect_collision

# Parametric types preserve multiple dispatch
process(v::Vector{T}) where T <: Real = sum(v)
process(v::Vector{T}) where T <: String = join(v)

# Dispatch on abstract types too
operate(x::Number, y::Number) = x + y
operate(2, 3.5)  # Int and Float → abstract Number dispatch
```

**Advantages over single dispatch:**
- No "which class is this method on?" ambiguity
- Symmetric function relationships (e.g., `collide(a, b)` and `collide(b, a)` are equally natural)
- Generic algorithms apply to unrelated types sharing behavior

**Libraries compose strongly** because they all participate in the same dispatch mechanism. Adding a new type automatically gains methods from all libraries.

## Type System and Annotations

Julia has a dynamic type system with optional annotations. Types enable dispatch, optimization, and clarity—but are not required.

```julia
# Untyped (valid but not optimized)
function fibonacci(n)
    n < 2 && return n
    return fibonacci(n-1) + fibonacci(n-2)
end

# Typed (JIT specializes for each type combination)
function fibonacci(n::Int)::Int
    n < 2 && return n
    return fibonacci(n-1) + fibonacci(n-2)
end

# Type annotations in parameters enable dispatch
process(x::Float64) = x * 2.0
process(x::String) = x * x

# Abstract types for flexibility
process(x::Number) = x + zero(typeof(x))

# Union types (multiple possibilities)
function safediv(x::Union{Int, Float64}, y::Union{Int, Float64})
    y == 0 && error("Division by zero")
    return x / y
end

# Type parameters with constraints
struct Matrix{T <: Number}
    data::Vector{T}
    rows::Int
    cols::Int
end

# Composite types (structs)
struct Point{T <: Real}
    x::T
    y::T
end

p1 = Point(1, 2)        # Point{Int64}
p2 = Point(1.5, 2.5)    # Point{Float64}
p3 = Point(1, 2.5)      # Point{Float64} (promoted)
```

**Type inference philosophy:** Let the JIT infer when possible; annotate at public APIs and critical paths for dispatch and performance.

## JIT Compilation and Performance

Julia compiles functions just-in-time using LLVM. Type information enables aggressive optimizations matching compiled C/Fortran performance.

```julia
# Function compiled on first call (for given types)
function mandelbrot(x, y, maxiter)
    z = c = complex(x, y)
    for n in 1:maxiter
        abs(z) > 2 && return n - 1
        z = z^2 + c
    end
    return maxiter
end

# First call: ~0.1 ms (includes compilation)
mandelbrot(0.5, 0.5, 256)

# Subsequent calls: 0.001 ms (precompiled)
[mandelbrot(0.5, 0.5, 256) for _ in 1:1000]

# Specialize = new types = recompile
function operate(x, y)
    x + y
end

operate(1, 2)        # Compiles for Int64
operate(1.5, 2.5)    # Recompiles for Float64
```

**Type stability** is critical: code should return the same type given the same input types. Unstable code defeats JIT.

```julia
# BAD: unstable (sometimes Int, sometimes Float)
function unstable(n::Int)
    if n > 0
        return n
    else
        return 3.14
    end
end

# GOOD: stable (always Float64)
function stable(n::Int)
    if n > 0
        return convert(Float64, n)
    else
        return 3.14
    end
end

@code_warntype unstable(5)  # Shows type instability in output
```

## Array Programming and Broadcasting

Arrays are first-class; operations broadcast across elements implicitly.

```julia
# Vector operations
v = [1, 2, 3, 4, 5]
v .* 2  # [2, 4, 6, 8, 10]  (broadcast: .* vs *)
v .+ 1  # [2, 3, 4, 5, 6]

# Matrix operations
A = [1 2; 3 4; 5 6]
B = [1 2 3; 4 5 6]

A * B    # Matrix multiplication (3×2 * 2×3 = 3×3)
A .* B'  # Element-wise multiplication after transpose

# Broadcasting with scalars
v .- mean(v)  # Subtract mean from each element
M ./ sum(M)   # Normalize matrix

# .= broadcasts and assigns
v .= v .* 2   # In-place element-wise multiply

# Ranges and indexing
1:10          # Range (lazy)
collect(1:10) # Vector

A[1, :]       # Row 1
A[:, 2]       # Column 2
A[1:2, :]     # Rows 1-2
```

**Performance tip:** Broadcasting avoids allocating intermediate arrays. Fusing operations (e.g., `a .+ b .* c` fuses into one kernel) saves memory bandwidth.

## Package Ecosystem for Scientific Computing

**Flux.jl**: Neural networks and deep learning
```julia
using Flux

# Simple neural network
model = Chain(
    Dense(28*28, 128, relu),
    Dense(128, 64, relu),
    Dense(64, 10)
)

loss(x, y) = Flux.crossentropy(model(x), y)
ps = Flux.params(model)
opt = Flux.ADAM()

# Training loop
for epoch in 1:10
    Flux.train!(loss, ps, train_data, opt)
end
```

**DifferentialEquations.jl**: ODE/PDE solvers
```julia
using DifferentialEquations

# Define ODE: dy/dt = -2y
f(y, p, t) = -2*y
y0 = 1.0
tspan = (0.0, 1.0)

prob = ODEProblem(f, y0, tspan)
sol = solve(prob, RK45())  # Runge-Kutta 4th/5th order

# Plot solution
using Plots
plot(sol)
```

**LinearAlgebra.jl, Statistics.jl**: Standard numerical ops

**Plots.jl, Makie.jl**: Visualization

**Interact.jl**: Interactive dashboards

**DataFrames.jl**: Tabular data (like pandas)

## Performance Tuning Tips

1. **Avoid global variables**: JIT can't specialize on them.
```julia
# Bad
const threshold = 0.5  # Global (reduces performance)
function check(x)
    return x > threshold
end

# Good: pass as argument or computed once
function check(x, threshold = 0.5)
    return x > threshold
end
```

2. **Ensure type stability**: Use `@code_warntype` to detect instability.
```julia
@code_warntype function foo(x)
    if x > 0
        return 1
    else
        return 1.0  # Type instability!
    end
end
```

3. **Use in-place operations** (suffixed with `!`):
```julia
# Allocates new vector
u_new = u .+ dt .* f(u)

# Modifies in-place (faster)
u_new .= u .+ dt .* f(u)
```

4. **Pre-allocate large arrays** outside loops:
```julia
# Bad: allocates inside loop
for step in 1:1000
    result = zeros(N)  # Allocate each iteration
    # ...
end

# Good: allocate once
result = zeros(N)
for step in 1:1000
    fill!(result, 0)  # Reset, reuse
    # ...
end
```

5. **Use `@inbounds` / `@simd` judiciously**:
```julia
# Skip bounds checking in tight loop
@inbounds for i in 1:length(A)
    A[i] = i^2
end

# Request SIMD vectorization
@simd for i in 1:length(A)
    A[i] += i
end
```

## Interoperability: Python and C

**Call Python from Julia:**
```julia
using PyCall

# Import Python module
np = pyimport("numpy")

# Use Python objects
data = np.array([1, 2, 3])
result = filter(x -> x > 1, data)
```

**Call C from Julia:**
```julia
# Declare C function
ccall((:sqrt, "libm"), Float64, (Float64,), 9.0)  # Result: 3.0

# C library (custom)
ccall((:my_func, "./mylib.so"), Int32, (Int32,), 42)
```

**Use Julia from Python:**
```python
from juliacall import Main as jl

result = jl.eval("1 + 1")  # 2
fibonacci = jl.seval("n -> n < 2 ? n : fibonacci(n-1) + fibonacci(n-2)")
print(fibonacci(10))  # 55
```

## See Also

- [language-julia.md](language-julia.md) — core conventions
- [algorithms-dynamic-programming.md](algorithms-dynamic-programming.md) — DP algorithms
- [paradigm-functional-programming.md](paradigm-functional-programming.md) — FP concepts
- [cs-type-system-practice.md](cs-type-system-practice.md) — type systems