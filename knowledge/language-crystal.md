# Crystal Conventions and Idioms

## Crystal Philosophy

Crystal is a statically typed, compiled language with Ruby-like syntax. It aims to be "fast as C, slick as Ruby" — combining Ruby's expressiveness with compile-time type safety and native performance.

- **Ruby syntax, C speed**: If you know Ruby, you know Crystal. The compiler handles types for you.
- **Null safety**: The compiler tracks nil at compile time. No nil-reference crashes at runtime.
- **Concurrency via fibers**: Lightweight green threads with channels (CSP-inspired).

## Core Syntax

```crystal
# Type inference — rarely need annotations
name = "Alice"          # String
age = 30                # Int32
pi = 3.14               # Float64
active = true           # Bool

# Everything is an object
5.times { |i| puts i }
"hello".upcase          # "HELLO"
[1, 2, 3].map(&.*(2))  # [2, 4, 6]

# String interpolation
puts "#{name} is #{age} years old"

# Symbols (compile-time constants)
status = :active

# Tuples (fixed-size, potentially heterogeneous)
tuple = {1, "hello", true}  # Tuple(Int32, String, Bool)

# Named tuples
config = {host: "localhost", port: 8080}
puts config[:host]
```

## Null Safety

```crystal
# Nil is tracked in the type system
def find_user(id : Int32) : User?  # User | Nil
  users[id]?
end

user = find_user(42)
# user is User?

# Compile error if you call methods without nil check:
# user.name  # Error: undefined method 'name' for Nil

# Safe access
if user
  puts user.name  # compiler knows it's User here (flow typing)
end

# Or use try
puts user.try(&.name) || "Unknown"

# Not-nil assertion (when you're sure)
puts user.not_nil!.name  # raises if nil
```

## Classes and Structs

```crystal
class User
  property name : String
  property age : Int32
  getter email : String       # read-only
  setter score : Float64      # write-only

  def initialize(@name : String, @age : Int32, @email : String)
    @score = 0.0
  end

  def greet : String
    "Hello, I'm #{@name}"
  end
end

# Structs (value types, stack-allocated)
struct Point
  property x : Float64
  property y : Float64

  def initialize(@x, @y)
  end

  def distance(other : Point) : Float64
    Math.sqrt((x - other.x) ** 2 + (y - other.y) ** 2)
  end
end

# Abstract classes
abstract class Shape
  abstract def area : Float64
end

class Circle < Shape
  def initialize(@radius : Float64)
  end

  def area : Float64
    Math::PI * @radius ** 2
  end
end
```

## Modules and Generics

```crystal
# Modules for mixins
module Serializable
  abstract def to_json(builder : JSON::Builder)

  def to_json_string : String
    JSON.build { |json| to_json(json) }
  end
end

# Generics
class Stack(T)
  def initialize
    @data = Array(T).new
  end

  def push(item : T) : self
    @data.push(item)
    self
  end

  def pop : T
    @data.pop
  end

  def empty? : Bool
    @data.empty?
  end
end

stack = Stack(Int32).new
stack.push(1).push(2).push(3)
```

## Concurrency (Fibers + Channels)

```crystal
# Fibers (lightweight green threads)
channel = Channel(String).new

spawn do
  result = fetch_data()
  channel.send(result)
end

# Do other work while waiting...
data = channel.receive

# Multiple producers
results = Channel(Int32).new(10)  # buffered

10.times do |i|
  spawn do
    sleep(rand(0.1..1.0))
    results.send(i * i)
  end
end

10.times do
  puts results.receive
end

# Select (wait on multiple channels)
channel1 = Channel(String).new
channel2 = Channel(Int32).new

select
when msg = channel1.receive
  puts "Got string: #{msg}"
when num = channel2.receive
  puts "Got number: #{num}"
end
```

## Macros

```crystal
# Compile-time metaprogramming
macro define_method(name, content)
  def {{name.id}}
    {{content}}
  end
end

define_method(:greet, "Hello!")

# Macro for generating getters from a map
macro json_mapping(properties)
  {% for key, type in properties %}
    @[JSON::Field(key: {{key.stringify}})]
    property {{key.id}} : {{type}}
  {% end %}
end

class Config
  include JSON::Serializable
  json_mapping({
    host: String,
    port: Int32,
    debug: Bool,
  })
end
```

## Conventions

1. **Trust the type inference.** Only annotate types in method signatures and instance variables. The compiler infers everything else.
2. **Handle nil at compile time.** Use flow typing (`if user`), `try`, or `not_nil!`. The compiler prevents nil crashes.
3. **Use structs for small value types.** Points, colors, coordinates — stack-allocated, no GC pressure.
4. **Use fibers for concurrency.** They're cheap (~8KB each). Channels for communication between fibers.
5. **Use `@[JSON::Serializable]`** (or `include JSON::Serializable`) for JSON handling. It generates efficient serialization at compile time.
6. **Crystal is not Ruby.** Despite syntax similarity, Crystal has no `method_missing`, no `eval`, no runtime metaprogramming. Use macros for compile-time generation.

---

_Sources: Crystal Language Reference (crystal-lang.org), Crystal Programming (George Dietrich & Guilherme Bernal), Crystal Standard Library docs_
