# Zig Best Practices

## Zig Philosophy

Zig is a systems programming language designed to be a better C. It prioritizes simplicity, explicitness, and optimal runtime performance. No hidden control flow, no hidden allocations, no garbage collector.

- **No hidden allocations.** Every allocation is explicit via allocator parameters.
- **No hidden control flow.** No operator overloading, no exceptions, no implicit function calls.
- **Comptime.** Compile-time code execution replaces generics and macros.

## Error Handling

```zig
// Errors are a first-class union type
const FileError = error{
    FileNotFound,
    PermissionDenied,
    DiskFull,
};

// Functions that can fail return error union: !T
fn readFile(path: []const u8) ![]u8 {
    const file = std.fs.cwd().openFile(path, .{}) catch |err| {
        return err;
    };
    defer file.close();  // Always runs on scope exit
    return file.readToEndAlloc(allocator, max_size);
}

// try — propagate errors (like Rust's ?)
fn process() !void {
    const data = try readFile("config.txt");
    defer allocator.free(data);
    try parseConfig(data);
}

// catch — handle errors
const value = getValue() catch |err| switch (err) {
    error.NotFound => default_value,
    else => return err,
};

// errdefer — cleanup only on error
fn createResource() !*Resource {
    const r = try allocator.create(Resource);
    errdefer allocator.destroy(r);  // Only runs if function returns error
    try r.init();
    return r;
}
```

## Allocators

```zig
// Allocators are passed explicitly — no hidden malloc
fn createList(allocator: std.mem.Allocator) !std.ArrayList(u8) {
    var list = std.ArrayList(u8).init(allocator);
    try list.append(42);
    return list;
}

// Common allocators
const gpa = std.heap.GeneralPurposeAllocator(.{}){};
const allocator = gpa.allocator();

const arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
defer arena.deinit();  // Free everything at once

// Testing allocator (detects leaks)
const testing_allocator = std.testing.allocator;
```

## Comptime (Compile-Time Execution)

```zig
// Generic-like behavior via comptime
fn max(comptime T: type, a: T, b: T) T {
    return if (a > b) a else b;
}

// Compile-time string formatting
const msg = std.fmt.comptimePrint("buffer size: {}", .{buffer_size});

// Comptime assertions
comptime {
    if (@sizeOf(Header) != 16) {
        @compileError("Header must be exactly 16 bytes");
    }
}

// Type reflection at comptime
fn isNumeric(comptime T: type) bool {
    return switch (@typeInfo(T)) {
        .int, .float => true,
        else => false,
    };
}
```

## Slices & Memory

```zig
// Slices are a pointer + length (like Rust's &[T])
fn sum(items: []const i32) i32 {
    var total: i32 = 0;
    for (items) |item| {
        total += item;
    }
    return total;
}

// Sentinel-terminated slices (for C interop)
const c_string: [:0]const u8 = "hello";

// Pointer arithmetic is explicit and bounded
const ptr: [*]const u8 = slice.ptr;
```

## Defer & Errdefer

```zig
fn processFile(path: []const u8) !void {
    const file = try std.fs.cwd().openFile(path, .{});
    defer file.close();  // Guaranteed cleanup

    const buffer = try allocator.alloc(u8, 4096);
    defer allocator.free(buffer);  // Guaranteed cleanup

    // Multiple defers execute in reverse order (LIFO)
    try doWork(file, buffer);
}
```

## Testing

```zig
test "addition" {
    const result = add(2, 3);
    try std.testing.expectEqual(@as(i32, 5), result);
}

test "allocations" {
    // Testing allocator detects memory leaks
    var list = std.ArrayList(u8).init(std.testing.allocator);
    defer list.deinit();
    try list.append(42);
    try std.testing.expect(list.items.len == 1);
}

// Run with: zig build test
```

## Interop with C

```zig
// Import C headers directly
const c = @cImport({
    @cInclude("stdio.h");
    @cInclude("stdlib.h");
});

pub fn main() void {
    _ = c.printf("Hello from C\n");
}

// Zig can be used as a C compiler
// zig cc -o output source.c
// zig build-lib (create C-compatible libraries)
```

## Key Patterns

- **No null pointers by default.** Optional types: `?*T` (nullable pointer), `?T` (optional value).
- **No undefined behavior** (in safe builds). Overflow, out-of-bounds, and null access are detectable.
- **Build system is written in Zig** (build.zig) — no Make, CMake, or external tools.
- **Cross-compilation is trivial**: `zig build -Dtarget=x86_64-linux-gnu`.

## Tooling

| Tool | Purpose |
|------|---------|
| **zig build** | Build system (self-hosted) |
| **zig test** | Built-in test runner |
| **zig fmt** | Code formatting |
| **ZLS** | Language server |
| **zig cc** | Drop-in C/C++ compiler |

---

*Sources: Zig documentation, Zig Language Reference, Andrew Kelley (creator) talks, Zig community wiki*
