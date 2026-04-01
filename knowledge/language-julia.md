# Julia Conventions and Idioms

## Julia Philosophy

Julia delivers the productivity of Python with the speed of C. It achieves this through multiple dispatch, JIT compilation via LLVM, and a sophisticated type system — all without requiring manual type annotations.

- **Multiple dispatch**: The core paradigm. Functions are defined by method specialization on argument types.
- **Write generic code**: Let the compiler specialize. Don't over-annotate types.
- **Composability**: Packages compose because they share the same dispatch mechanism.

## Multiple Dispatch

```julia
# Define methods for different type combinations
area(shape::Circle) = π * shape.radius^2
area(shape::Rectangle) = shape.width * shape.height

# Multi-argument dispatch
collide(a::Circle, b::Circle) = distance(a, b) < a.radius + b.radius
collide(a::Circle, b::Rectangle) = circle_rect_collision(a, b)
collide(a::Rectangle, b::Circle) = collide(b, a)  # reuse

# The compiler picks the most specific method at call time
```

## Type System

```julia
# Abstract types define behavior hierarchies
abstract type Shape end

struct Circle <: Shape
    center::Tuple{Float64, Float64}
    radius::Float64
end

struct Rectangle <: Shape
    origin::Tuple{Float64, Float64}
    width::Float64
    height::Float64
end

# Mutable structs (use sparingly — hurts performance)
mutable struct Particle
    position::Vector{Float64}
    velocity::Vector{Float64}
    mass::Float64
end

# Parametric types
struct Point{T<:Real}
    x::T
    y::T
end

Point(1.0, 2.0)  # Point{Float64}
Point(1, 2)      # Point{Int64}
```

## Performance Patterns

```julia
# Type stability: function should always return the same type
# Type-unstable
function bad_sqrt(x)
    if x < 0
        return "error"  # String return breaks type stability
    end
    return sqrt(x)
end

# Type-stable
function good_sqrt(x)
    x < 0 && throw(DomainError(x, "negative input"))
    return sqrt(x)
end

# Avoid global variables in hot paths
# Allocating
x = 10
f() = x + 1  # type of x is unknown at compile time

# In-place
const X = 10
f() = X + 1

# Pre-allocate outputs
function my_computation!(result, data)
    for i in eachindex(data)
        result[i] = data[i]^2 + 1
    end
    return result
end

# Use @views to avoid array copies
function process(A)
    @views col_sum = sum(A[:, 1])  # no copy
end

# Profile with @time, @btime (BenchmarkTools), @code_warntype
using BenchmarkTools
@btime my_function($args)
@code_warntype my_function(args)
```

## Broadcasting

```julia
# Dot syntax broadcasts any function over arrays
x = [1, 2, 3, 4, 5]
y = sin.(x) .+ cos.(x)          # element-wise
z = x .^ 2 .+ 2 .* x .+ 1      # vectorized polynomial

# @. macro: dot everything automatically
z = @. x^2 + 2x + 1

# Broadcasting custom functions
f(a, b) = a^2 + b
f.(x, 3)  # applies f element-wise with scalar b=3

# In-place broadcasting
y = similar(x)
y .= sin.(x) .+ cos.(x)  # no temporary allocations
```

## Error Handling

```julia
# Exceptions
try
    result = parse(Int, "abc")
catch e
    if e isa ArgumentError
        println("Parse failed: ", e.msg)
    else
        rethrow(e)
    end
finally
    cleanup()
end

# Return Nothing for "no result" (like Option/Maybe)
function find_item(collection, predicate)
    for item in collection
        predicate(item) && return item
    end
    return nothing
end

# Check with isnothing()
result = find_item(data, x -> x > 100)
if !isnothing(result)
    process(result)
end
```

## Packages and Modules

```julia
# Module definition
module MyPackage

export my_function, MyType

struct MyType
    data::Vector{Float64}
end

function my_function(x::MyType)
    return sum(x.data)
end

# Internal (not exported)
function _helper(x)
    return x^2
end

end  # module

# Using packages
using LinearAlgebra    # imports all exports
import Statistics: mean, std  # selective import
```

## Metaprogramming

```julia
# Macros operate on expressions (ASTs), not values
macro assert_positive(expr)
    return quote
        val = $(esc(expr))
        val > 0 || throw(AssertionError(
            string($(string(expr)), " = ", val, " is not positive")
        ))
        val
    end
end

@assert_positive sqrt(4) - 1

# Generated functions (compile-time specialization)
@generated function unroll_sum(x::NTuple{N, T}) where {N, T}
    expr = :(x[1])
    for i in 2:N
        expr = :($expr + x[$i])
    end
    return expr
end
```

## Conventions

1. **Write type-stable functions.** Use `@code_warntype` to check. Type instability is the #1 performance killer.
2. **Don't over-annotate types.** Use type annotations for dispatch and documentation, not for performance (the compiler infers types).
3. **Mutating functions end with `!`** by convention: `sort!`, `push!`, `normalize!`.
4. **Pre-allocate and mutate in-place** for hot loops. Avoid allocations in inner loops.
5. **Use broadcasting (dot syntax)** instead of explicit loops for element-wise operations.
6. **Avoid global variables in performance-critical code.** Use `const` or pass as function arguments.

---

_Sources: Julia Documentation (julialang.org), Julia Performance Tips, Thinking Julia (Tom Kwong), JuliaAcademy_
