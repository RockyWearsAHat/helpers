# gRPC Protocol — HTTP/2 Framing, Protobuf, Streaming, Interceptors & Load Balancing

## Overview

**gRPC** (gRPC Remote Procedure Call) is a high-performance RPC framework built on HTTP/2 and Protocol Buffers. It enables efficient, strongly-typed communication between microservices with support for multiple streaming models, built-in authentication (TLS), metadata, and cancellation propagation.

Unlike REST (request-response over HTTP/1.1), gRPC leverages HTTP/2's binary framing, stream multiplexing, and server push mechanics to achieve:
- **Binary serialization** (no JSON parsing overhead)
- **Multiple concurrent streams** without connection overhead
- **Server push** capabilities
- **Built-in flow control** and cancellation
- **Language-neutral** RPC definitions via protobuf

## Protocol Buffers (Protobuf) Encoding

**Protocol Buffers** are Google's language-neutral, platform-neutral serialization format. Services and messages are defined in `.proto` files:

```proto
service Greeter {
  rpc SayHello (HelloRequest) returns (HelloReply) {}
}

message HelloRequest {
  string name = 1;
}

message HelloReply {
  string message = 1;
}
```

### Binary Encoding

Protobuf encodes messages into a compact binary format (not human-readable like JSON):

- **Field tags** (1–536,870,911) map message fields to their serialized positions. Tag 1 might encode `name`, tag 2 encodes another field.
- **Wire types** (0–5): Varint, 64-bit, Length-delimited (strings, bytes, nested messages), Start group, 32-bit, End group.
- **Varint encoding**: Variable-length integers. Small numbers (< 128) fit in 1 byte; larger numbers span multiple bytes. Makes 0 efficient (1 byte) but 268,435,455 costs 5 bytes.
- **Nested messages**: Encoded as length-delimited byte streams. A `HelloRequest` with a nested `metadata` message is serialized recursively.

**Advantages over JSON**:
- Smaller size (binary vs. text)
- Faster parsing (no JSON tokenizer overhead)
- Strongly-typed (the sender and receiver agree on field order and types; no type guessing)
- Backward/forward compatibility (field numbers, not field names, determine identity; adding optional fields doesn't break old clients)

### gRPC Message Format

Each gRPC message is **length-prefixed**:
```
[Compression flag (1 byte)] [Message length (4 bytes)] [Protobuf message bytes]
```

Example: A 100-byte message with no compression:
```
0x00 0x00 0x00 0x00 0x64 [100 bytes of protobuf data]
```

This framing allows HTTP/2 to split messages across DATA frames safely; the receiver reconstructs them using the length prefix.

## gRPC over HTTP/2 Framing

HTTP/2 frames the gRPC protocol:

### Connection Establishment
1. Client connects via TLS (typically port 443) or cleartext (port 5000 in dev).
2. TLS handshake negotiates cipher suite + ALPN protocol ID (`h2` for HTTP/2).
3. HTTP/2 connection preface (client sends `PRI * HTTP/2.0\r\n` + magic + SETTINGS frame).
4. Server acks with SETTINGS frame (window size, max concurrent streams, etc.).

### Request-Response Flow (Unary Call)

```
Client                                           Server
  | HEADERS frame (method, path, authority,      |
  |                content-type: application/grpc)
  |-----------> /helloworld.Greeter/SayHello --->|
  |                                              | Processes request
  | DATA frame (length-prefixed protobuf)       |
  |-------------------------------------------->|
  |                                           HEADERS frame
  |                                           (status: 200)
  |<---------------------------------------------|
  |                                           DATA frame
  |                                           (response message)
  |<---------------------------------------------|
  |                                           DATA frame
  |                                           (trailers, status)
  |<---------------------------------------------|
```

### Streaming Calls

gRPC supports four communication patterns:

1. **Unary RPC**: Request → Response (1 server response). HTTP/2: Single DATA frame per direction.
   
2. **Server Streaming**: Request → Stream of Responses. Server sends multiple DATA frames; client receives all with same stream ID.
   ```
   Client: HEADERS + DATA (request)
   Server: HEADERS + DATA (response 1) + DATA (response 2) + ... + TRAILERS
   ```

3. **Client Streaming**: Stream of Requests → Response. Client sends multiple DATA frames; server responds once after all data received.
   ```
   Client: HEADERS + DATA (request 1) + DATA (request 2) + ... + END_STREAM
   Server: HEADERS + DATA (response) + TRAILERS
   ```

4. **Bidirectional Streaming**: Concurrent Request Stream ↔ Response Stream. Both client and server send/receive data concurrently (independent DATA frames).
   ```
   Client: HEADERS + DATA + DATA + ... (requests ongoing)
   Server: HEADERS + DATA + DATA + ... (responses ongoing)
   ```

### HTTP/2-Level Details

- **Stream ID**: Unique per RPC call within a connection. Client initiates odd IDs (1, 3, 5…); server initiates even (2, 4, 6…). gRPC typically has client driving all calls, so stream IDs are odd.
- **Flow Control**: HTTP/2 window-based flow control (separate for connection-level and stream-level). Prevents sender from overwhelming receiver with data. Default window size: 65535 bytes. Frames include `WINDOW_UPDATE` to advertise available buffer space.
- **Priority and Dependencies** (rarely used in gRPC): Frames can specify priority; rarely exercised because gRPC treats all streams equally.
- **Trailers**: Final metadata sent after the response body. Includes gRPC status code (`0` = OK, `3` = invalid_argument, `14` = unavailable etc.) and optional message.

Example trailers:
```
grpc-status: 0
grpc-message: 
```

## RPC Lifecycle and Cancellation

### Deadline (Timeout)

Client includes a **deadline** (timeout) in the request metadata:
```
grpc-timeout: 1000m (1000 milliseconds)
```

Server must:
1. Parse the deadline on request reception.
2. Start a timer; abort if the timer fires before the RPC completes.
3. Return `DEADLINE_EXCEEDED` (status code 4) if the deadline passes.

Clients can set per-RPC deadlines or use a default. **Best practice**: Always set deadlines to prevent indefinite resource holding on the server.

### Cancellation Propagation

If a client cancels a request (closes the stream or sends `RST_STREAM` frame), the server is notified:
- HTTP/2 `RST_STREAM` frame is sent to the server, immediately terminating the stream.
- Server-side RPC context is marked as cancelled; any goroutine (Go) or task (Java) executing the RPC can check the cancellation signal.
- Cancellation propagates down the call chain: if Service A calls Service B with a deadline, and Service A's deadline is cancelled, the cancellation should be forwarded to Service B's RPC.

Example (Go):
```go
ctx, cancel := context.WithCancel(context.Background())
defer cancel()

resp, err := client.SayHello(ctx, &HelloRequest{Name: "World"})
// If context is cancelled, gRPC automatically sends RST_STREAM
```

## Metadata and Custom Headers

**Metadata** is key-value pairs (string or binary) attached to requests and responses:

### Request Metadata
Client sends immediately after HEADERS frame:
```
metadata headers:
  custom-user-id: "42"
  custom-correlationid: "abc-123"
  authorization: "Bearer token123"
```

### Response Metadata
Server sends response metadata in HEADERS frame after request body; trailing metadata in trailers.

**Use cases**:
- **Authentication**: Pass JWT, API key.
- **Request correlation**: Trace ID for logging.
- **Custom business logic**: Feature flags, version negotiation.

## Interceptors

**Interceptors** are middleware that intercept RPC calls:

### Client-Side Interceptors

Intercept before and after a call:
```go
func UnaryClientInterceptor(ctx context.Context, method string, req, reply interface{}, cc *grpc.ClientConn, invoker grpc.UnaryInvoker, opts ...grpc.CallOption) error {
  // Before
  start := time.Now()
  
  // Call
  err := invoker(ctx, method, req, reply, cc, opts...)
  
  // After
  log.Printf("RPC %s took %v", method, time.Since(start))
  return err
}
```

**Common uses**:
- **Logging/tracing**: Log all RPC calls, add trace span.
- **Metrics**: Record latency, success/failure counts.
- **Authentication**: Inject JWT token into metadata.
- **Retry logic**: Automatically retry failed calls.

### Server-Side Interceptors

Intercept incoming requests:
```go
func UnaryServerInterceptor(ctx context.Context, req interface{}, info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (interface{}, error) {
  // Before
  log.Printf("Serving %s", info.FullMethod)
  
  // Handle
  resp, err := handler(ctx, req)
  
  // After
  if err != nil {
    log.Printf("Error: %v", err)
  }
  return resp, err
}
```

## Load Balancing

gRPC clients use **client-side load balancing** by default:

### Load Balancing Policies

1. **pick_first**: Connect to the first address; only fail over to the second if the first is unavailable. Simple but not load-distributed.

2. **round_robin**: Cycle through addresses in order. Each RPC goes to the next address in the list. Fair distribution but doesn't account for server load.

3. **ring_hash**: Hash the RPC's metadata (e.g., user ID) to a position on a ring; route to the same address for that hash. Sticky sessions; useful if the service has per-client state (e.g., memcached-style cache).

4. **xDS** (eXtensible Discovery Service): Delegates load balancing decisions to an xDS server (e.g., Envoy, Consul). Server provides the list of healthy endpoints and policies (weights, priorities). Gears allows dynamic updates; client doesn't hardcode addresses.

```go
conn, _ := grpc.Dial("discovery-service:5000", 
                     grpc.WithDefaultServiceConfig(`{"loadBalancingConfig": [{"round_robin":{}}]}`))
```

## Health Checking

gRPC defines a standard **health check** protocol (GRPC Health Checking Protocol):

```proto
service Health {
  rpc Check(HealthCheckRequest) returns (HealthCheckResponse) {}
  rpc Watch(HealthCheckRequest) returns (stream HealthCheckResponse) {}
}

enum ServingStatus {
  UNKNOWN = 0;
  SERVING = 1;
  NOT_SERVING = 2;
}
```

**Usage**:
- Load balancers periodically call `Check()` to verify server health.
- Load balancers remove servers returning `NOT_SERVING` or encountering errors.
- Servers can dynamically report transitioning status (e.g., graceful shutdown).

Example (Go):
```go
healthServer := health.NewServer()
healthServer.SetServingStatus("helloworld.Greeter", health.ServingStatus_SERVING)
grpc.RegisterHealthCheckServiceServer(s, healthServer)
```

## Common gRPC Patterns and Pitfalls

### Message Size Limits

gRPC enforces message size limits (default: 4 MB). Oversized messages are rejected. Mitigate by:
- Streaming large data instead of single message.
- Splitting into multiple RPC calls.
- Configuring higher max size (risky if untrusted clients).

### Connection Pooling

A single gRPC connection supports up to 2^31 − 1 concurrent streams (per HTTP/2 spec). Clients typically reuse one connection (no per-RPC connection overhead). Avoid opening new connections per call.

### Graceful Shutdown

Server should:
1. Send a goaway frame (HTTP/2 protocol) to new connections.
2. Wait for in-flight RPCs to complete (or timeout).
3. Close existing connections.

```go
s.GracefulStop() // waits for pending RPCs
```

### Semantic Versioning

gRPC services are typically deployed across infrastructure with mixed old/new versions. Protobuf supports backward/forward compatibility:
- Add optional fields (ignore if not present on old clients).
- Never reuse field numbers.
- Use `deprecated = true` on removed fields.

## See Also

- [Networking — HTTP](networking-http.md) — HTTP/2 framing fundamentals
- [Web API Patterns](web-api-patterns.md) — REST vs gRPC design trade-offs
- [Protobuf Encoding](serialization-protobuf.md) — encoding details (if a separate note exists)
- [Service Mesh — Observability & Load Balancing](devops-service-mesh.md) — xDS, health checks in production