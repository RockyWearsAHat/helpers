# Protocol Buffers — Messages, Encoding, Versioning & gRPC Integration

**Protocol Buffers** (protobuf) is a language-agnostic, binary serialization format developed by Google. It prioritizes performance, type safety, and backward/forward compatibility. Unlike JSON or YAML, protobuf messages are defined in `.proto` files, compiled to language-specific code, and used as input/output for RPC services like gRPC. The wire format is compact and fast; the trade-off is less human-readability than text-based formats.

## Core Concepts: Messages, Fields, Field Numbers

A protobuf message is a collection of typed fields, each identified by a **field number** (not name):

```protobuf
syntax = "proto3";

message Person {
  string name = 1;
  int32 id = 2;
  string email = 3;
}
```

Field numbers (1, 2, 3) encode into the wire format. The name (`name`, `id`, `email`) is metadata; only the number is serialized. **Field numbers cannot be reused** once assigned, even after a field is deleted. Reusing a field number causes decoding ambiguity: older software sees the wrong field. Mitigation: use `reserved` statements:

```protobuf
message Person {
  reserved 3;  // Prevent accidental reuse of field 3
  string name = 1;
  int32 id = 2;
  // email = 3;  (deleted)
}
```

## Scalar Types

Proto3 provides scalar types with default values (0 for numbers, empty string, false for bool):
- Numeric: `int32`, `int64`, `uint32`, `uint64`, `sint32`, `sint64`, `float`, `double`
- Boolean: `bool`
- String: `string` (UTF-8), `bytes` (raw binary)

**Varint encoding:** numbers use variable-length encoding. Small values (0-127) take 1 byte; larger values take more. Proto3 fields have implicit defaults; unset fields are omitted from the wire format to save space.

## Collections and Composite Types

**Repeated fields** (lists):

```protobuf
message SearchResponse {
  repeated string results = 1;
}
```

**Enumerations:**

```protobuf
enum Color {
  COLOR_UNSPECIFIED = 0;  // Proto3 requires a 0 value
  RED = 1;
  GREEN = 2;
  BLUE = 3;
}
```

Enum values start at 0 by convention. Old code interpreting the wire format sees unrecognized enum values as the default (0).

**Maps:**

```protobuf
message Config {
  map<string, string> settings = 1;
}
```

Maps are syntactic sugar for repeated key-value message pairs, optimized for lookup.

**Nested messages:**

```protobuf
message Outer {
  message Inner {
    string value = 1;
  }
  Inner inner = 1;
}
```

## Oneof: Mutually Exclusive Fields

Use `oneof` when only one field can be set at a time (union-like):

```protobuf
message Response {
  oneof result {
    string success = 1;
    string error = 2;
  }
}
```

Deserialization sets exactly one field; setting a new field unsets others. Useful for error handling or alternative data representations.

## Packages and Imports

Organize protos in packages to avoid naming collisions:

```protobuf
package mycompany.v1;

message User {
  string name = 1;
}
```

Import types from other `.proto` files:

```protobuf
import "user.proto";

message UserResponse {
  mycompany.v1.User user = 1;
}
```

## Well-Known Types

Google provides standard message definitions for common scenarios:
- `google.protobuf.Timestamp`: UTC timestamp
- `google.protobuf.Duration`: time interval
- `google.protobuf.Any`: generic container for any message type (with full type name)
- `google.protobuf.Struct`: dynamic key-value map (JSON-like)
- `google.protobuf.Empty`: no fields (for void-like operations)

Example:

```protobuf
import "google/protobuf/timestamp.proto";

message Event {
  string id = 1;
  google.protobuf.Timestamp timestamp = 2;
}
```

## Proto2 vs Proto3

**Proto2:** Complex, labeled fields (`optional`, `required`, `repeated`). Deprecated.

**Proto3:** Simplified, all fields optional by default. Required fields removed. This relaxation aids forward/backward compatibility: old code ignores unknown fields; new code treats missing fields as defaults.

Modern development uses **proto3**.

## Editions: The Future Direction

**Editions** (proto editions e.g., 2023) extend the language without major version jumps. Editions specify features (e.g., field presence) individually rather than bundling them into proto2/3 versions. Rolling out gradually; proto3 remains the mainstream today.

## Tags: Wire Type Metadata

Fields encode a **wire type** (3 bits) indicating whether the field contains:
- `0`: varint (int, bool, enum)
- `1`: 64-bit fixed (double)
- `2`: length-delimited (string, bytes, messages)
- `5`: 32-bit fixed (float)

This enables a decoder to skip unknown fields without parsing them.

## Backward/Forward Compatibility Rules

**Adding fields:** New code adds field N; old code ignores it. OK.

**Deleting fields:** Old code sends field N; new code ignores it. OK if field is not reused.

**Changing field type:** Unsafe. `int32` and `uint32` have different encodings. Causes corruption. **Never do this.**

**Changing field numbers:** Equivalent to deleting and re-adding. Unsafe.

**Adding enum values:** New code generates a value N; old code treats it as default (0) or uses the raw number for unknown values. Safe.

**Renaming fields:** Safe. Field numbers are what matter; names are metadata.

**Making a repeated field optional (and vice versa):** Generally safe but tricky; use reserved numbers to prevent accidents.

The **golden rule**: field numbers and their types are immutable. Everything else is flexible.

## Encoding and Performance

Protobuf wire format is compact and fast:
- Varints save space for small numbers.
- No metadata overhead between fields.
- No type tags (unlike JSON); type is known from schema.
- Binary parsing is straightforward (no state machine like XML/HTML).

Typical size: 30-40% smaller than equivalent JSON. Parsing ~5-10x faster than JSON for typical messages.

Example: `string name = 1; int32 id = 2;` with values `"Alice"` and `42`:
- JSON: `{"name":"Alice","id":42}` ≈ 27 bytes
- Protobuf: ~15 bytes (varint field tag + name length + name + id tag + varint)

## gRPC Integration

gRPC services are defined in `.proto` files:

```protobuf
service UserService {
  rpc GetUser(UserId) returns (User);
  rpc ListUsers(Empty) returns (stream User);
  rpc CreateUser(User) returns (UserId);
  rpc UpdateUser(UpdateRequest) returns (Empty);
}
```

Unary (single request/response), server streaming, client streaming, bidirectional streaming. Code generation creates client and server stubs. gRPC runs over HTTP/2. Protobuf is the default serialization.

## The buf Ecosystem

**buf** (buf.build) is a modern protobuf toolchain:
- `buf lint`: style checks and best practices
- `buf format`: auto-format `.proto` files
- `buf generate`: language-specific code generation
- `buf push`: publish protos to the buf registry
- `buf export`: manage dependencies from the registry

Buf enforces consistency and simplifies dependency management, replacing the need for git submodules or manual proto distribution.

## Connect-RPC (gRPC Alternative)

**Connect** is a newer RPC protocol built on HTTP/2 (or HTTP/1.1) that uses protobuf for serialization. It's simpler than gRPC's bidirectional streaming model and easier to integrate with existing proxy infrastructure. Both gRPC and Connect are protobuf-compatible for message schema.

## When to Use Protobuf

**Good fit:**
- RPC services (gRPC, Connect)
- High-performance messaging (Kafka, etc.)
- Systems requiring strict schema evolution
- Generated client/server code is an asset

**Poor fit:**
- Configuration files (YAML, TOML, JSON are more readable)
- Human-editable data
- Ad-hoc, exploratory APIs
- Environments where code generation is unwelcome

**Adoption:** Highly standard in Google infrastructure, emerging microservices, Kafka ecosystem. JSON is still more common for REST APIs.

## Related

See also: [networking-grpc.md](networking-grpc.md), [networking-grpc-deep.md](networking-grpc-deep.md), [data-serialization-formats.md](data-serialization-formats.md), [api-design.md](api-design.md), [web-api-patterns.md](web-api-patterns.md)