# Zig Deep Dive: Comptime, Allocators, and Systems Programming

## Comptime: Compile-Time Code Execution

Zig's `comptime` keyword enables code to run at compile time, eliminating the need for macros or traditional generics. Any computation whose result is known at compile time can execute in `comptime` context.

```zig
// Compile-time constants
const MAX_USERS = 1000;
const PI: f32 = 3.14159;

// Compile-time function evaluation
fn fibonacci(n: comptime_int) comptime_int {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
const FIB_10 = fibonacci(10);  // 55 — computed at compile time

// Compile-time reflection and generation
fn createVector(comptime T: type, comptime size: usize) type {
    return struct {
        items: [size]T,
        len: usize = 0,
        
        fn append(self: *@This(), item: T) void {
            if (self.len < size) {
                self.items[self.len] = item;
                self.len += 1;
            }
        }
    };
}

const IntVec10 = createVector(i32, 10);
var vec: IntVec10 = undefined;
vec.append(42);

// Compile-time @import and conditional compilation
const is_debug = @import("builtin").mode == .Debug;
if (is_debug) {
    std.debug.print("Debug mode active\n", .{});
}
```

**Key insight:** Unlike C macros, `comptime` is type-safe and behaves like normal Zig code. No textual substitution, no hygiene issues. The compiler verifies that the compile-time code is valid before executing it.

## Allocators: Explicit Memory Management

Every dynamic allocation in Zig requires an explicit allocator parameter. This design makes memory ownership and lifetime crystal clear — no hidden allocations.

```zig
// General-purpose allocator (good for development, not production)
var gpa = std.heap.GeneralPurposeAllocator(.{}){};
defer gpa.deinit();
const allocator = gpa.allocator();

// Arena allocator — allocate many small objects, free all at once
var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
defer arena.deinit();
const arena_alloc = arena.allocator();

// Page allocator — raw OS pages for large allocations
const large_buf = try std.heap.page_allocator.alloc(u8, 1_000_000);
defer std.heap.page_allocator.free(large_buf);

// FixedBufferAllocator — allocate from a pre-allocated buffer
var buffer: [1024]u8 = undefined;
var fba = std.heap.FixedBufferAllocator.init(&buffer);
const fba_alloc = fba.allocator();

// Using allocators with data structures
fn createList(allocator: std.mem.Allocator) !std.ArrayList(u32) {
    var list = std.ArrayList(u32).init(allocator);
    try list.appendSlice(&.{ 1, 2, 3, 4, 5 });
    return list;
}
```

**Allocator strategy:** Pass the allocator as a function parameter. This enables dependency injection — callers choose which allocator to use (arena for batch operations, GPA for variable-sized, testing allocator for leak detection).

## Error Unions and Error Handling

Zig treats errors as a language feature, not an afterthought. Error unions (`T!` means "value of type T or an error") are explicit in the type system.

```zig
// Error set — a type representing possible errors
const FileError = error{
    FileNotFound,
    PermissionDenied,
    OutOfMemory,
};

// Error union return type
fn readFile(path: []const u8, allocator: std.mem.Allocator) ![]u8 {
    const file = std.fs.cwd().openFile(path, .{}) catch |err| {
        return error.FileNotFound;
    };
    defer file.close();
    
    const stat = try file.stat();
    const buffer = try allocator.alloc(u8, stat.size);
    const bytes_read = try file.readAll(buffer);
    
    return buffer[0..bytes_read];
}

// try — propagate errors up the call stack
fn processFile(path: []const u8, allocator: std.mem.Allocator) !void {
    const data = try readFile(path, allocator);
    defer allocator.free(data);
    try validateData(data);
}

// catch — handle specific errors
const data = readFile(path, allocator) catch |err| {
    if (err == error.FileNotFound) {
        std.debug.print("File not found, using default\n", .{});
        return default_data;
    }
    return err;  // Propagate other errors
};

// switch on error
const result = readFile(path, allocator) catch |err| switch (err) {
    error.FileNotFound => "using default",
    error.PermissionDenied => "access denied",
    error.OutOfMemory => "out of memory",
};
```

## defer and errdefer: Guaranteed Cleanup

Zig's `defer` and `errdefer` ensure resources are cleaned up, even when errors occur.

```zig
fn acquireResource() !*Resource {
    const resource = try allocator.create(Resource);
    errdefer allocator.destroy(resource);  // Only cleanup if function errors
    
    try resource.initialize();  // If this fails, destroy is called
    
    return resource;
}

fn processMultiple(paths: []const []const u8, allocator: std.mem.Allocator) !void {
    var files = std.ArrayList(*std.fs.File).init(allocator);
    defer {
        // Clean up at scope exit
        for (files.items) |f| {
            f.close();
        }
        files.deinit();
    }
    
    for (paths) |path| {
        const file = try std.fs.cwd().openFile(path, .{});
        try files.append(file);
    }
    
    // Process files...
    // Cleanup happens automatically via defer
}
```

**Stack semantics:** Unlike Go's deferred functions, Zig's `defer` applies to the current block scope. Multiple defers in the same scope execute in reverse order (LIFO).

## Optional Types and null

Optionals (`?T`) are a type-safe way to represent "value or nothing." They're more explicit than nullable pointers and integrate cleanly with the type system.

```zig
const maybe_name: ?[]const u8 = null;
const maybe_age: ?u32 = 42;

// Unwrap with orelse
const name = maybe_name orelse "Unknown";
const age = maybe_age orelse 0;

// Safe navigation with try
if (maybe_name) |name| {
    std.debug.print("Name: {s}\n", .{name});
}

// Force unwrap (throws assertion error if null)
const definite_name: []const u8 = maybe_name.?;  // Avoid unless certain

// Chaining optionals
const user: ?User = getUser(id);
const email = user.?.email;  // Unwrap step-by-step
```

## Packed Structs and Bit-Level Control

`packed struct` gives precise control over memory layout, essential for hardware interaction, serialization, and performance-critical code.

```zig
// Bit fields
const Status = packed struct {
    online: bool,
    admin: bool,
    verified: bool,
    unused: u5 = 0,  // 8 bits total
};

var status: Status = .{
    .online = true,
    .admin = false,
    .verified = true,
};

std.debug.print("Status byte: {}\n", .{@as(u8, @bitCast(status))});

// Network packet parsing
const Ipv4Header = packed struct {
    version: u4,
    header_length: u4,
    dscp: u6,
    ecn: u2,
    total_length: u16,
    identification: u16,
    flags: u3,
    fragment_offset: u13,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src_ip: u32,
    dst_ip: u32,
};

// Exactly 20 bytes, no padding
comptime {
    std.debug.assert(@sizeOf(Ipv4Header) == 20);
}
```

## C Interoperability

Zig's C interop is first-class: calling C libraries is trivial, and Zig code can be called from C.

```zig
// Import C declarations
const c = @cImport(@cInclude("stdlib.h"));

pub fn main() void {
    const ptr = c.malloc(1024);
    defer c.free(ptr);
    
    const random_val = c.rand();
}

// Expose Zig functions to C
export fn zigAdd(a: i32, b: i32) i32 {
    return a + b;
}

// Use translate-c to convert C headers automatically
// zig translate-c -o generated.zig /path/to/header.h
```

## Build System

Zig's build system (`build.zig`) is Zig code itself, offering full programmatic control.

```zig
// build.zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    
    const exe = b.addExecutable(.{
        .name = "myapp",
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    
    b.installArtifact(exe);
    
    // Tests
    const test_exe = b.addTest(.{
        .root_source_file = b.path("src/main.zig"),
        .target = target,
        .optimize = optimize,
    });
    const run_test = b.addRunArtifact(test_exe);
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_test.step);
}
```

## Zig in Bun

Zig is used in Bun, the JavaScript runtime, for performance-critical components. Bun leverages Zig's C interop to bind to native APIs and systems libraries efficiently. Bun's bundler and package manager architecture benefits from Zig's explicit memory management and compile-time computation.

---

## See Also

- [Language Quick-Reference](languages-quick-reference.md)
- [Systems: Memory Allocators](systems-memory-allocators.md)
- [Memory Management](memory-management.md)
- [C Language Modern Practices](language-c-modern.md)