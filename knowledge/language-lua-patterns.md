# Lua Patterns — Metatables, OOP, Coroutines & Embedding

## Overview

Lua achieves sophisticated programming patterns despite a minimal core. The language intentionally omits classes, inheritance, and closures as built-ins; instead, metatables, tables, and functions compose to enable prototype-based OOP, functional closures, and deep runtime introspection. This design philosophy—**mechanisms, not policies**—makes Lua extraordinarily flexible and embeddable.

## Metatables: The Meta-Object Protocol

A **metatable** is a table that defines behavior for another table. Lua consults metatables to handle operations not directly defined on the table itself. Each table has at most one metatable (`getmetatable`, `setmetatable`); the metatable itself is a normal table.

### Core Metamethods

#### `__index`: Default Value Access

`__index` defines what happens when accessing a key not in the table.

```lua
local Prototype = { default_value = 42 }

local mt = {
  __index = Prototype  -- If key missing, look in Prototype
}

local obj = setmetatable({}, mt)
print(obj.default_value)  -- prints 42 (looked in Prototype)
print(obj.missing_key)    -- prints nil (Prototype.missing_key is nil)
```

Or use a function for computed defaults:

```lua
local mt = {
  __index = function(self, key)
    return "computed_" .. key
  end
}

local obj = setmetatable({}, mt)
print(obj.foo)  -- prints "computed_foo"
```

#### `__newindex`: Custom Assignment Validation

`__newindex` intercepts assignment to keys not in the table.

```lua
local mt = {
  __newindex = function(self, key, value)
    print("Setting " .. key .. " = " .. tostring(value))
    rawset(self, key, value)  -- Store the actual value
  end
}

local obj = setmetatable({}, mt)
obj.x = 10            -- prints "Setting x = 10"
```

**Constraint enforcement:**

```lua
local mt = {
  __newindex = function(self, key, value)
    if type(value) ~= "number" then
      error("Expected number")
    end
    rawset(self, key, value)
  end
}

local point = setmetatable({}, mt)
point.x = 5           -- OK
point.y = "invalid"   -- Error: Expected number
```

#### `__call`: Making Tables Callable

Make a table behave like a function.

```lua
local mt = {
  __call = function(self, x, y)
    return x + y + self.offset
  end
}

local adder = setmetatable({ offset = 10 }, mt)
print(adder(5, 3))  -- prints 18 (5 + 3 + 10)
```

#### `__tostring` & `__len`

```lua
local mt = {
  __tostring = function(self)
    return "Object{x=" .. self.x .. ", y=" .. self.y .. "}"
  end,
  __len = function(self)
    return self.x + self.y
  end
}

local obj = setmetatable({ x = 3, y = 4 }, mt)
print(obj)      -- prints "Object{x=3, y=4}"
print(#obj)     -- prints 7 (length operator)
```

#### Arithmetic Operators: `__add`, `__sub`, `__mul`, `__eq`, `__lt`

```lua
local mt = {
  __add = function(a, b)
    local sum = setmetatable({}, mt)
    sum.value = a.value + b.value
    return sum
  end,
  __eq = function(a, b)
    return a.value == b.value
  end,
  __lt = function(a, b)
    return a.value < b.value
  end,
  __tostring = function(self)
    return "Num(" .. self.value .. ")"
  end
}

local a = setmetatable({ value = 5 }, mt)
local b = setmetatable({ value = 3 }, mt)
print(a + b)    -- prints "Num(8)"
print(a > b)    -- true (uses __lt)
```

### Metatable Introspection

```lua
local mt = setmetatable({}, { __index = print })
getmetatable(mt).__index("Called from metatable!")
```

## Prototype-Based OOP: Classes Without Classes

Lua implements OOP via **delegation pattern**: each "instance" has a metatable pointing to a "class" table with shared methods. There are no built-in class keywords; everything is table composition.

### Simple Class Pattern

```lua
local Animal = {}

function Animal:new(name)
  local self = setmetatable({}, { __index = self })
  self.name = name
  return self
end

function Animal:speak()
  print(self.name .. " makes a sound")
end

local dog = Animal:new("Fido")
dog:speak()  -- prints "Fido makes a sound"
```

**Key insight**: `Animal:new()` is syntactic sugar for `Animal.new(Animal, ...)`. The `self` in `:new()` refers to `Animal`. The returned object's metatable's `__index` points back to `Animal`, so method lookup delegates to `Animal`.

### Inheritance via Metatable Chains

```lua
local Animal = {}
function Animal:speak() print("sound") end

local Dog = setmetatable({}, { __index = Animal })
function Dog:speak() print("woof") end

local dog = setmetatable({}, { __index = Dog })
dog:speak()  -- prints "woof" (found in Dog)
```

The chain: `dog` → `Dog.__index` (`Dog` table) → `Dog`'s metatable's `__index` (`Animal` table).

### Multiple Inheritance (Mixins)

```lua
local Drawable = { draw = function(self) print("drawing") end }
local Moveable = { move = function(self) print("moving") end }

local GameObject = setmetatable({}, { __index = Drawable })
setmetatable(GameObject, { __index = function(_, key) return Moveable[key] end })
-- Now GameObject delegates to Drawable first, then Moveable
```

## Coroutines: Cooperative Multitasking

**Coroutines** are lightweight, user-managed threads. Unlike OS threads, coroutines suspend/resume explicitly via `yield`/`resume`.

### Basic API

```lua
local co = coroutine.create(function()
  print("A")
  coroutine.yield()
  print("B")
  coroutine.yield()
  print("C")
end)

coroutine.resume(co)    -- prints "A", pauses at yield
coroutine.resume(co)    -- prints "B", pauses at yield
coroutine.resume(co)    -- prints "C", finishes
coroutine.resume(co)    -- does nothing (coroutine exhausted)
```

### Coroutine Status

```lua
print(coroutine.status(co))  -- "running", "suspended", "dead"
```

### Producer-Consumer with Yield

```lua
local function producer()
  for i = 1, 3 do
    print("Producing " .. i)
    coroutine.yield(i)
  end
end

local co = coroutine.create(producer)
local status, val
repeat
  status, val = coroutine.resume(co)
  if val then print("Consumed " .. val) end
until not status or coroutine.status(co) == "dead"
```

### Coroutines in Game Loops

Coroutines enable clean, non-blocking game loops:

```lua
-- Task-based game logic
local tasks = {}

function schedule_task(coro)
  table.insert(tasks, coro)
end

function update()
  local to_remove = {}
  for i, task in ipairs(tasks) do
    if coroutine.status(task) == "suspended" then
      coroutine.resume(task)
    else
      table.insert(to_remove, i)
    end
  end
  for i = #to_remove, 1, -1 do
    table.remove(tasks, to_remove[i])
  end
end

-- NPC AI
schedule_task(coroutine.create(function()
  while true do
    move_to(player.x, player.y)
    coroutine.yield()  -- Pause, resume next frame
    if distance_to_player() < 5 then attack() end
    coroutine.yield()
  end
end))
```

## LuaJIT: JIT Compilation & Performance

**LuaJIT** is a high-performance Lua implementation with JIT (just-in-time) compilation. Vanilla Lua is interpreted; LuaJIT compiles hot code paths to native machine code.

### Trace-Based JIT

LuaJIT uses **trace-based JIT compilation**: it records execution traces (sequences of bytecode instructions encountered during execution) and compiles frequently-executed traces to native code.

```lua
local function sum(n)
  local s = 0
  for i = 1, n do
    s = s + i
  end
  return s
end

sum(1000000)  -- First iterations: interpreted
              -- After ~2000 executions: entire loop trace compiled to native
              -- ~40x faster than vanilla Lua
```

Traces are tier-1; if a trace branches unexpectedly, execution falls back to interpreter.

### FFI: Calling C Functions Directly

**FFI (Foreign Function Interface)** allows Lua code to call C functions and structs without wrapper functions.

```lua
local ffi = require "ffi"

-- Declare C interface
ffi.cdef[[
  int abs(int j);
  void printf(const char *fmt, ...);
]]

-- C library
local C = ffi.C
print(C.abs(-42))        -- prints 42
C.printf("Hello %s\n", "Lua")  -- prints "Hello Lua"
```

**Struct handling:**

```lua
ffi.cdef[[
  struct Point {
    double x, y;
  };
]]

local Point = ffi.typeof("struct Point")
local p = Point(3.0, 4.0)
print(p.x, p.y)  -- 3.0, 4.0

-- Arrays
local points = ffi.new("struct Point[10]")
points[0].x = 1.0
```

FFI is critical for gaming, embedded systems, and systems programming—C interop without overhead.

### Performance Tips

- Hotspots should be free of table lookups or polymorphism (JIT specializes on types).
- Avoid `__index` in tight loops (metatable lookups inhibit JIT).
- Use FFI for performance-critical C code.
- Profile with `jit.dump()` to see compiled traces.

```lua
local jit = require "jit"
jit.dump.on("myfunction")  -- Dump JIT IR for 'myfunction'
```

## Lua in Gaming

### Roblox

**Roblox Studio** runs Lua in a sandboxed environment called **Roblox Lua**, a dialect with:

- **RemoteFunction/RemoteEvent**: Network communication (client ↔ server).
- **Humanoid system**: NPC and player control primitives.
- **Debris service**: Automatic instance cleanup after delays.
- **RunService**: Frame-by-frame game loop integration.

```lua
local Players = game:GetService("Players")
local UserInputService = game:GetService("UserInputService")

UserInputService.InputBegan:Connect(function(input, gameProcessed)
  if gameProcessed then return end
  if input.KeyCode == Enum.KeyCode.E then
    print("E pressed")
    -- Fire RemoteEvent to server
    game.ReplicatedStorage.EventName:FireServer(data)
  end
end)
```

### World of Warcraft

**WoW Lua** (used in addons) accesses the game API for UI automation, combat info, and event hooks.

```lua
-- Register event listener
frame:RegisterEvent("PLAYER_LOGIN")
frame:SetScript("OnEvent", function(self, event)
  if event == "PLAYER_LOGIN" then
    print("Logged in!")
  end
end)

-- Query game state
local name, realm = UnitName("player")
local level = UnitLevel("player")
```

## Embedding Lua in C/C++

Embedding Lua in a host application—game engine, config system, script mod—requires the Lua C API.

### Basic Stack Manipulation

```c
#include <lua.h>

int main() {
  lua_State *L = luaL_newstate();  // Create Lua state
  luaL_openlibs(L);                // Load Lua libraries

  // Execute Lua code
  luaL_dostring(L, "print('Hello from Lua')");

  // Push values onto stack
  lua_pushnumber(L, 42);
  lua_pushstring(L, "hello");

  // Get values from Lua table
  lua_getglobal(L, "my_var");      // Push my_var onto stack
  int value = lua_tointeger(L, -1);

  lua_close(L);
  return 0;
}
```

### Calling Lua Functions from C

```c
lua_getglobal(L, "my_function");     // Push function
lua_pushnumber(L, 10);               // Push arg
lua_call(L, 1, 1);                   // Call with 1 arg, 1 return value
int result = lua_tointeger(L, -1);   // Get result
lua_pop(L, 1);                       // Clean stack
```

### Registering C Functions for Lua

```c
static int c_add(lua_State *L) {
  int a = luaL_checkinteger(L, 1);
  int b = luaL_checkinteger(L, 2);
  lua_pushinteger(L, a + b);
  return 1;  // Number of return values
}

lua_pushcfunction(L, c_add);
lua_setglobal(L, "add");  // Lua can now call add(5, 3)
```

## Neovim Lua API

**Neovim** (fork of Vim) permits init configuration and plugins in Lua instead of VimScript. The Lua API accesses editor state and events.

### Init Configuration

```lua
-- init.lua (replaces init.vim)
vim.opt.number = true
vim.opt.tabstop = 2
vim.opt.expandtab = true

-- Key mappings
vim.keymap.set('n', '<leader>ff', require('telescope.builtin').find_files)
```

### Neovim Plugin Structure

```lua
-- plugin/my_plugin.lua
local M = {}

function M.setup(config)
  config = config or {}
  -- Plugin initialization
end

function M.hello()
  vim.cmd("echo 'Hello from Lua plugin'")
end

return M

-- In init.lua:
-- require('my_plugin').setup({ option = true })
```

### LSP Configuration (Language Server Protocol)

```lua
local lspconfig = require('lspconfig')

-- Setup TypeScript language server
lspconfig.ts_ls.setup {
  capabilities = require('cmp_nvim_lsp').default_capabilities(),
  on_attach = function(client, bufnr)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, { buffer = bufnr })
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, { buffer = bufnr })
  end
}
```

### Async Patterns

```lua
-- Neovim's async I/O
vim.loop.spawn("git", { args = { "status" } }, function(code, signal)
  vim.cmd("echo 'Git status complete'")
end)
```

## Advanced Patterns

### Class-Based OOP with Private Fields

```lua
local Class = {}

function Class:new()
  local private = {}  -- Closure captures private state
  local self = setmetatable({}, {
    __index = function(_, key)
      return Class[key] or private[key]
    end,
    __newindex = function(_, key, value)
      if Class[key] == nil then
        private[key] = value  -- Store in closure
      end
    end
  })
  return self
end

function Class:set_private(key, value)
  rawset(self, key, value)  -- Only this method can access
end
```

### Decorator Pattern

```lua
local function with_logging(func)
  return function(...)
    print("Calling " .. func)
    local result = func(...)
    print("Returned: " .. tostring(result))
    return result
  end
end

local add = with_logging(function(a, b) return a + b end)
add(5, 3)  -- Prints call/return with logging
```

## Pitfalls & Best Practices

### Avoid Modifying Global Metatable

```lua
-- WRONG: affects all tables
getmetatable({}).__index = my_default

-- RIGHT: use setmetatable per instance
local mt = { __index = my_default }
local obj = setmetatable({}, mt)
```

### Coroutine Cleanup

Coroutines don't automatically clean up; store references carefully:

```lua
local tasks = {}
function schedule(func)
  local co = coroutine.create(func)
  table.insert(tasks, co)
  return co
end

-- Later, GC can collect finished coroutines if removed from tasks
```

### LuaJIT FFI Security

FFI bypasses sandboxing; it should only call trusted C libraries:

```lua
-- RISKY: calling arbitrary C code with FFI
-- Use only for vetted, performance-critical functions
```

## See Also

- **language-lua**: Lua idioms, tables, embeddability
- **language-wasm-deep**: WebAssembly portable execution (alternative to Lua embedding)
- **paradigm-oop-patterns**: Object-oriented design patterns across languages
- **runtime-wasm-runtimes**: Runtime engines comparable to LuaJIT compilation strategies