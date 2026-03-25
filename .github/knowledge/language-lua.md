# Lua Best Practices

## Lua Philosophy

Lua is a lightweight, embeddable scripting language. It's the world's most popular embedded language — found in game engines (Roblox, WoW, Love2D), editors (Neovim), networking (OpenResty/Nginx), and IoT.

- **Minimal core**: The entire language specification fits in 30 pages.
- **Tables are everything**: The only data structure — used for arrays, objects, classes, modules, namespaces.
- **Embeddable**: Designed to be embedded in C/C++ host applications.

## Tables (The Universal Data Structure)

```lua
-- Array-like (1-indexed!)
local fruits = {"apple", "banana", "cherry"}
print(fruits[1])  -- "apple" (NOT fruits[0])
print(#fruits)    -- 3 (length)

-- Dictionary-like
local user = {
    name = "Alice",
    age = 30,
    email = "alice@test.com",
}
print(user.name)       -- "Alice"
print(user["name"])    -- "Alice" (same thing)

-- Mixed
local config = {
    "positional_1",         -- config[1]
    "positional_2",         -- config[2]
    host = "localhost",     -- config.host
    port = 8080,            -- config.port
}

-- Nested tables
local matrix = {
    {1, 2, 3},
    {4, 5, 6},
    {7, 8, 9},
}
print(matrix[2][3])  -- 6
```

## Functions

```lua
-- Functions are first-class values
local function add(a, b)
    return a + b
end

-- Anonymous functions / closures
local double = function(n) return n * 2 end

-- Multiple return values
local function divmod(a, b)
    return math.floor(a / b), a % b
end
local quotient, remainder = divmod(17, 5)

-- Variadic arguments
local function sum(...)
    local total = 0
    for _, v in ipairs({...}) do
        total = total + v
    end
    return total
end

-- Closures (capture upvalues)
local function counter(start)
    local count = start or 0
    return function()
        count = count + 1
        return count
    end
end

local next = counter(0)
print(next())  -- 1
print(next())  -- 2
```

## OOP via Metatables

```lua
-- "Class" pattern using metatables
local Animal = {}
Animal.__index = Animal

function Animal.new(name, sound)
    local self = setmetatable({}, Animal)
    self.name = name
    self.sound = sound
    return self
end

function Animal:speak()
    print(self.name .. " says " .. self.sound)
end

-- "Inheritance"
local Dog = setmetatable({}, {__index = Animal})
Dog.__index = Dog

function Dog.new(name)
    local self = Animal.new(name, "Woof")
    return setmetatable(self, Dog)
end

function Dog:fetch(item)
    print(self.name .. " fetches the " .. item)
end

local rex = Dog.new("Rex")
rex:speak()          -- "Rex says Woof"
rex:fetch("ball")    -- "Rex fetches the ball"
```

## Iterators

```lua
-- ipairs for array traversal (sequential, 1 to n)
for i, v in ipairs(fruits) do
    print(i, v)
end

-- pairs for all keys (unordered)
for k, v in pairs(user) do
    print(k, v)
end

-- Custom iterators
local function range(start, stop, step)
    step = step or 1
    local i = start - step
    return function()
        i = i + step
        if i <= stop then return i end
    end
end

for n in range(1, 10, 2) do
    print(n)  -- 1, 3, 5, 7, 9
end
```

## Error Handling

```lua
-- pcall (protected call) — Lua's try/catch
local ok, result = pcall(function()
    return risky_operation()
end)

if not ok then
    print("Error: " .. tostring(result))
end

-- xpcall with error handler (gets stack trace)
local ok, result = xpcall(
    risky_operation,
    function(err)
        return debug.traceback(err, 2)
    end
)

-- error() to throw
local function validate(port)
    if type(port) ~= "number" or port < 1 or port > 65535 then
        error("invalid port: " .. tostring(port), 2)  -- 2 = blame caller
    end
end
```

## Module Pattern

```lua
-- mymodule.lua
local M = {}

local function private_helper()
    return 42
end

function M.public_function()
    return private_helper()
end

return M

-- Usage:
local mymodule = require("mymodule")
mymodule.public_function()
```

## Key Rules

1. **Local everything.** Always use `local`. Global variables are a debugging nightmare.
2. **1-indexed arrays.** Lua arrays start at 1, not 0. All standard library functions expect this.
3. **`#` operator is unreliable for sparse tables.** Only use `#` on sequence tables with no gaps.
4. **`nil` removes table entries.** `t[key] = nil` deletes the entry.
5. **String concatenation with `..`**, not `+`. Use `table.concat` for many strings (more efficient).
6. **Use LuaJIT** for performance-critical code — 10-50x faster than standard Lua 5.1.

---

_Sources: Programming in Lua (Roberto Ierusalimschy), Lua 5.4 Reference Manual, LuaJIT documentation, Neovim Lua guide_
