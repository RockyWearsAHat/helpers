# Systems: Serialization Formats — Speed, Size, Schemas, and Trade-offs

**Serialization** converts in-memory data structures into byte sequences for storage, transmission, or inter-process communication. Different formats optimize for different constraints: human readability, parsing speed, binary size, schema evolution, and type safety. No format dominates all dimensions — every choice reflects priorities shaped by the problem context.

## The Design Space

All serialization formats navigate four tensions:

| Axis | Implications |
|------|--------------|
| Human Readability | Debugging, compliance, data portability |
| Parsing Speed | Latency, throughput, CPU cost |
| Binary Size | Network bandwidth, storage, cache efficiency |
| Schema Support | Type safety, backward compatibility, documentation |

## JSON (Text-Based, Schemaless)

```json
{"name": "Alice", "age": 30, "tags": ["engineer", "rust"]}
```

**Encoding:** UTF-8 text. Data types encoded as literals: `null`, `true`/`false`, numbers, strings, arrays, objects.

**Strengths:**
- Human-readable and editable
- No schema required; supports nested structures
- Ubiquitous tooling in all languages
- Streaming-friendly (line-delimited JSON for log files)

**Weaknesses:**
- Verbose (lots of whitespace, `{"key":` overhead)
- Slow to parse (UTF-8 decoding, newline/quote handling)
- No schema; runtime type checking needed
- Ambiguous: `"123"` (string) vs `123` (number)

**Parsing Speed:** ~1-5 CPU cycles per byte (simdjson optimizes to 3-4 via SIMD on modern CPUs).

**Use Cases:** APIs, config files, logs, human inspection required.

## Protocol Buffers (Binary, Schema-Based)

```protobuf
syntax = "proto3";

message Person {
  string name = 1;
  int32 age = 2;
  repeated string tags = 3;
}
```

**Encoding:** Compact binary using field tags (1-byte integers) and packed encoding. Numbers compressed via varint (variable-length integers). Repeated fields use tag, length, value pattern.

```
Person {name: "Alice", age: 30, tags: ["engineer", "rust"]}

Encoded (hex):
0a 05 41 6c 69 63 65  // tag 1 (0x0a = 1<<3 | 2), length 5, "Alice"
10 1e                 // tag 2 (0x10 = 2<<3 | 0), varint 30
1a 08 65 6e 67 69 6e 65 65 72  // tag 3 (1a), length 8, "engineer"
1a 04 72 75 73 74     // tag 3, length 4, "rust"
```

**Strengths:**
- Compact (2-4x smaller than JSON for typical data)
- Fast to parse (minimal interpretation needed)
- Schema-driven: type safe, documented
- Backward/forward compatible: old code reads new data (unknown fields ignored), new code reads old data (missing fields get defaults)
- Strongly typed

**Weaknesses:**
- Binary; not human-readable (requires `protoc` to decode)
- Requires schema; must be versioned and deployed consistently
- No native support for maps (proto3 added maps, but they're syntactic sugar)

**Parsing Speed:** ~1-2 CPU cycles per byte (very fast).

**Use Cases:** gRPC, internal service communication, data warehousing (e.g., Dremel), proven at scale (Google, Uber, Stripe).

## FlatBuffers (Binary, Zero-Copy)

**Key Idea:** Data layout mirrors in-memory representation; no deserialization step. Direct memory access via offsets.

```
Serialized buffer:
[offset to "Alice"] [offset to tags] [30] [offsets to "engineer", "rust"]

Access:
buf[name_offset].read_string()  // No copying; ptr arithmetic only
```

**Strengths:**
- Instant deserialization: no parsing step, direct memory access
- Same size as Protocol Buffers
- Good for memory-constrained systems
- Supports nested structures

**Weaknesses:**
- More complex to implement
- Mutable updates require reserializing entire message
- Less mature tooling than Protocol Buffers
- Schema support, but less battle-tested

**Use Cases:** Game engines (Unity), real-time systems, memory-constrained devices.

## Cap'n Proto (Binary, Zero-Copy, Streaming)

Similar to FlatBuffers but with additional guarantees:
- **Capability-based security:** Field offsets can reference capabilities, not just data
- **Streaming:** Can send messages without knowing full size upfront
- **RPC:** Built-in RPC protocol (not just serialization)

```capnp
struct Person {
  name @0 :Text;
  age @1 :UInt32;
  tags @2 :List(Text);
}
```

**Strengths:**
- Zero-copy on little-endian systems
- Efficient streaming
- Type-safe capability model
- Strong schema evolution guarantees

**Weaknesses:**
- Smaller community than Protocol Buffers
- More complex specification
- Fewer language bindings

**Use Cases:** Sandboxing, capability-based systems, streaming environments.

## MessagePack (Binary, Compact)

Simpler than Protocol Buffers, but not zero-copy. Compact binary format using type markers (1-byte prefixes).

```
MessagePack encoding:
{name: "Alice", age: 30, tags: [...]}

0x82              // fixmap with 2 entries
0xa5 41 6c 69 63 65  // fixstr "name"
0xa5 41 6c 69 63 65  // fixstr "Alice"
0xa3 61 67 65        // fixstr "age"
0x1e                 // uint8 30
...
```

**Strengths:**
- Compact (similar size to protobuf)
- Fast to parse
- No schema required; like JSON but binary
- Simple specification

**Weaknesses:**
- No schema; type checking required at runtime
- Not zero-copy
- Less battle-tested for critical systems than protobuf

**Use Cases:** Cache systems (Redis, memcached protocols), lightweight protocols, config serialization.

## Avro (Binary, Schema-Based, Schema Registry)

Avro decouples schema from data. The schema can be transmitted separately or referenced via schema registry.

```json
{
  "type": "record",
  "name": "Person",
  "fields": [
    {"name": "name", "type": "string"},
    {"name": "age", "type": "int"}
  ]
}

// Payload includes only values, not field names.
// Reader uses schema to interpret bytes.
```

**Strengths:**
- Schema registry enables versioning and governance
- Compact binary (field names not in payload)
- Schema evolution: add/remove fields without breaking old data
- Excellent for data pipelines (Kafka, Spark)

**Weaknesses:**
- Requires schema registry infrastructure
- Less commonly used for service-to-service RPC
- Schema inference on read can be expensive

**Use Cases:** Event streaming (Kafka), data pipelines (Hive, Spark), schema governance.

## CBOR (Concise Binary Object Representation, RFC 7049)

Compact binary format similar to MessagePack but more flexible and standardized (IETF RFC).

```
{name: "Alice", age: 30}

0xa2              // map with 2 pairs
0x64 6e 61 6d 65  // text "name" (0x64 = 1-byte length 4)
0x65 41 6c 69 63 65  // text "Alice"
0x63 61 67 65     // text "age"
0x18 1e           // uint 30
```

**Strengths:**
- IETF standard
- Supports tags for arbitrary type extensions
- Compact and human-decodable (via hex dumps)
- Good for embedded systems, IoT

**Weaknesses:**
- Smaller adoption than JSON or protobuf
- Less mature tooling
- Not zero-copy like FlatBuffers

**Use Cases:** IoT, CBOR Web Tokens (COSE), embedded systems, MQTT.

## Comparison Matrix

| Format | Size | Speed | Schema | Human-Readable | Zero-Copy | Maturity |
|--------|------|-------|--------|-----------------|-----------|----------|
| JSON   | 100% (baseline) | Slow (1-5 cyc/byte) | No | Yes | No | Excellent |
| Protobuf | 30-50% | Fast (1-2 cyc/byte) | Yes | No | No | Excellent |
| FlatBuffers | 35-50% | Instant | Yes | No | Yes | Good |
| Cap'n Proto | 35-50% | Instant | Yes | No | Yes | Good |
| MessagePack | 40-60% | Fast (2-4 cyc/byte) | No | No | No | Good |
| Avro | 40-60% | Fast | Yes (registry) | No | No | Excellent (data pipelines) |
| CBOR | 40-60% | Fast | No | Partial | No | Fair |

## Performance Measurements

Typical latencies on modern hardware (per-message):

| Format | Serialize | Deserialize | Payload Size | Use Case |
|--------|-----------|-------------|-------------|----------|
| JSON | 200-500ns (small msg) | 500-1000ns | 1.5KB (message entity) | Web APIs |
| Protobuf | 50-100ns | 50-100ns | 300-500B | Internal RPC |
| FlatBuffers | 30-80ns (prepare offsets) | 0-5ns (lazy read) | 300-500B | Real-time systems |
| CBOR | 100-300ns | 100-300ns | 400-600B | IoT |

## Backward and Forward Compatibility

**Backward Compatibility** (new code reads old data):
- **JSON**: Breaks if field is required.
- **Protobuf**: Optional fields receive defaults; works by design.
- **FlatBuffers**: Similar to protobuf; unknown fields ignored.
- **Avro**: Schema can add/remove fields; evolution rules prevent breaks.

**Forward Compatibility** (old code reads new data):
- **JSON**: Breaks if field is removed or type changes.
- **Protobuf**: New fields are ignored; old code continues.
- **Avro**: Complex; depends on schema registry version.

## Choosing a Format

1. **JSON**: Already widely adopted, human inspection needed, low volume, simple schema
2. **Protocol Buffers**: Service-to-service RPC, high volume, schema governance, gRPC
3. **FlatBuffers/Cap'n Proto**: Embedded systems, real-time, zero-copy mandatory, memory-constrained
4. **MessagePack**: Lightweight, no schema overhead, cache/session protocols
5. **Avro**: Event streaming, Kafka/Spark pipelines, schema registry infrastructure
6. **CBOR**: Standards compliance, IoT, embedded, tokens (COSE)

## Related Concepts

- See [data-serialization-formats.md](data-serialization-formats.md) for broader data format taxonomy (row/columnar for analytics)
- See [networking-grpc.md](networking-grpc.md) for Protocol Buffers + gRPC integration
- See [api-design.md](api-design.md) for API versioning and schema evolution strategies