# gRPC Deep Dive: Protocol Buffers, Service Model, Streaming, Interceptors & Load Balancing

## Overview

**gRPC** is a high-performance RPC framework layered on HTTP/2, with Protocol Buffers as the default serialization format. The framework provides strongly-typed service definitions, multiple streaming patterns, built-in metadata propagation, cancellation semantics, and production-grade load balancing (xDS, client-side load balancing). Unlike REST (loosely-typed, HTTP semantics), gRPC emphasizes efficiency and operational transparency for microservice communication.

## Protocol Buffers (proto3) Semantics

### Message Definition & Code Generation

`.proto` files define messages, enums, services in a language-neutral syntax:

```proto
syntax = "proto3";
package myapp.v1;

message User {
  int64 id = 1;
  string name = 2;
  string email = 3;
  repeated string tags = 4;
  google.protobuf.Timestamp created_at = 5;
}

service UserService {
  rpc GetUser (GetUserRequest) returns (User);
  rpc ListUsers (ListUsersRequest) returns (stream User);
}
```

Code generator (`protoc` with language plugins: python, go, java, etc.) produces:
- **Message classes**: Python `User()` object with `.id`, `.name`, `.email` properties
- **Service interface**: Abstract base class defining handler methods
- **Serialization stubs**: Encoder/decoder for binary wire format
- **GRPC stubs**: Client and server code

### Field Numbers & Backward Compatibility

Fields identified by number, not name. Changing a field name is backward-compatible:

```proto
// Version 1:
message User {
  int64 id = 1; // field number 1
  string name = 2;
}

// Version 2 (backward-compatible):
message User {
  int64 user_id = 1; // renamed, same field number
  string full_name = 2;
  string nickname = 3; // new field
}

// Version 1 client sends field numbers 1, 2
// Version 2 server receives field numbers 1, 2, ignores unknown 3
// → No breaking change
```

Integers are variable-length (varint): 0 bytes 1 byte (efficient for small values), large numbers use multiple bytes. Optional/repeated modify field encoding (packed arrays compress better).

### proto3 Differences from proto2

- **No explicit optional:** All fields implicitly optional (default value 0, empty string, false)
- **No presence tracking:** Can't distinguish "field not set" from "field set to default value" (proto2 could)
- **Repeated without [packed]:** Encoded as individual tags + values; slower for large repeated fields; newer protos use `packed=true` implicitly for numeric types
- **Map type:** `map<string, User>` generates special wire format

**Presence in proto3:** Recent addition (proto3 with `optional` keyword restores proto2 semantics):

```proto
syntax = "proto3";
message User {
  optional int64 id = 1; // distinct: "not set" vs "set to 0"
  string name = 2; // no presence tracking
}
```

## gRPC Service Model

### Unary RPC (Request-Response)

```proto
rpc CreateUser (CreateUserRequest) returns (User);
```

Wire flow (HTTP/2 + gRPC framing):
```
Client:           Server:
HEADERS frame     (service, method)
  ↓                 ↓
DATA frame        (request body, length-prefixed protobuf)
  ↓                 ↓
          [server processes]
                  ←─ HEADERS frame (status 200)
                  ←─ DATA frame (response body)
                  ← TRAILERS frame (status, message)
```

**gRPC status codes** (not HTTP status):
- 0 = OK
- 1 = CANCELLED (client cancelled)
- 2 = UNKNOWN (general error)
- 3 = INVALID_ARGUMENT (request malformed)
- 4 = DEADLINE_EXCEEDED
- 5 = NOT_FOUND
- 14 = UNAVAILABLE (server temporarily down)

Server handler returns `(response_message, grpc.Status)`. For unary, exactly one response.

### Server Streaming RPC

```proto
rpc ListUsers (ListUsersRequest) returns (stream User);
```

```
Client:           Server:
HEADERS frame     (service, method)
DATA frame        (request)
                  ←─ HEADERS (200 OK)
                  ←─ DATA frame (User 1)
                  ←─ DATA frame (User 2)
                  ←─ DATA frame (User 3)
                  ← TRAILERS (status)
```

Server sends multiple DATA frames (same stream ID), each containing one message. Client buffers or processes as received.

### Client Streaming RPC

```proto
rpc UploadMetrics (stream Metric) returns (UploadResult);
```

```
Client:           Server:
HEADERS frame     (service, method)
DATA frame        (Metric 1)
DATA frame        (Metric 2)
DATA frame        (Metric 3)
  (END_STREAM)
                  ←─ HEADERS (200 OK)
                  ←─ DATA frame (result)
                  ← TRAILERS (status)
```

Client sends multiple messages, closes with END_STREAM. Server waits for all data, then responds once.

### Bidirectional Streaming RPC

```proto
rpc Chat (stream Message) returns (stream Message);
```

```
Client:           Server:
HEADERS frame     
DATA frame        (Message 1)
                  ←─ HEADERS (200 OK)
                  ←─ DATA (Message A from server)
DATA frame        (Message 2)
                  ←─ DATA (Message B from server)
DATA frame        (Message 3)
                  ←─ DATA (Message C from server)
  (END_STREAM)
                  ←─ TRAILERS (status)
```

Both sides send/receive concurrently. Each endpoint can send/receive in any order (independent streams).

## Metadata & Context Propagation

**Metadata:** Key-value pairs sent with request (HEADERS frames) and response (TRAILERS frames).

```go
// Client sending metadata:
md := metadata.Pairs("authorization", "Bearer token123", "x-custom", "value")
ctx := metadata.NewOutgoingContext(context.Background(), md)
user, err := client.GetUser(ctx, &GetUserRequest{Id: 1})

// Server receiving metadata:
md, _ := metadata.FromIncomingContext(ctx)
auth := md.Get("authorization") // ["Bearer token123"]
```

**Context:** gRPC uses Go's `context.Context` (or equivalent in other languages) for:
- **Cancellation:** `ctx.Done()` signals request cancelled; handler should stop processing
- **Deadlines:** `ctx.Deadline()` returns request's expiration time
- **Values:** Arbitrary key-value storage (don't overuse; metadata is for protocol-level info)

### Deadline Propagation

Client sets deadline: "this request must complete within 5 seconds":

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()
result, err := client.SlowOperation(ctx, req)
```

gRPC encodes deadline in `grpc-timeout` header (HTTP/2):
```
grpc-timeout: 5000m (5000 milliseconds)
```

Server receives context with same deadline. If server's operation takes longer → context deadline exceeded → handler exits early ("don't bother finishing, call already expired").

## Interceptors & Middleware

**Unary Interceptor** (called for each unary RPC):

```go
func LoggingInterceptor(ctx context.Context, req interface{}, 
    info *grpc.UnaryServerInfo, handler grpc.UnaryHandler) (interface{}, error) {
  
  start := time.Now()
  fmt.Printf("RPC: %s, Request: %v\n", info.FullMethod, req)
  
  resp, err := handler(ctx, req) // Call actual handler
  
  fmt.Printf("Completed in %v, Error: %v\n", time.Since(start), err)
  return resp, err
}

server := grpc.NewServer(grpc.UnaryInterceptor(LoggingInterceptor))
```

Interceptors chain: request → interceptor 1 → interceptor 2 → ... → handler → interceptor N → ... → response.

Common interceptor patterns:
- **Authentication:** Validate `authorization` metadata; reject if invalid
- **Metrics:** Record latency, error rates, RPC names
- **Tracing:** Add request to distributed trace (OpenTelemetry)
- **Validation:** Check request contents (non-negative IDs, string length limits)

**Streaming Interceptor:** Similar, but handler interface differs:

```go
func LoggingStreamInterceptor(srv interface{}, ss grpc.ServerStream, 
    info *grpc.StreamServerInfo, handler grpc.StreamHandler) error {
  
  fmt.Printf("Stream RPC: %s\n", info.FullMethod)
  err := handler(srv, ss)
  fmt.Printf("Stream completed, Error: %v\n", err)
  return err
}
```

## Error Handling & Cancellation

**gRPC Errors:** Status + Message + Metadata

```go
// Server returning error:
return nil, status.Error(codes.InvalidArgument, "user_id must be positive")

// Client receiving error:
resp, err := client.GetUser(ctx, req)
if err != nil {
  s := status.Convert(err)
  fmt.Printf("Code: %v, Message: %v\n", s.Code(), s.Message())
  // Handle by code
  if s.Code() == codes.NotFound { ... }
}
```

## Client-Side Load Balancing with xDS

**xDS** (Envoy's control plane protocol) allows servers to advertise endpoints and load balancing policies dynamically.

### Service Discovery & Endpoint Resolution

**Round-robin (basic):**
```
Client discovers endpoints: [10.0.0.1:5000, 10.0.0.2:5000, 10.0.0.3:5000]
Request 1: Route to 10.0.0.1
Request 2: Route to 10.0.0.2
Request 3: Route to 10.0.0.3
Request 4: Route to 10.0.0.1 (cycle)
```

**Least-request (adaptive):**
```
Track in-flight requests per endpoint:
  10.0.0.1: 2 requests in-flight
  10.0.0.2: 5 requests in-flight
  10.0.0.3: 1 request in-flight
Route next request to 10.0.0.3
```

**xDS control plane integration:**
```
gRPC client: "What load balancing policy?"
xDS server: "Use least-request among [10.0.0.1, 10.0.0.2, 10.0.0.3]"
(Updates sent periodically as endpoints change)
```

## gRPC-Web

gRPC-Web enables browser (JavaScript) clients to call gRPC services via HTTP/1.1 (browsers don't support HTTP/2 server push from JavaScript for gRPC).

**Differences:**
- Clients send unary requests over HTTP POST
- Responses wrapped in special format (trailers encoded in response body)
- Binary framing still uses gRPC format (efficient)
- CORS-aware (crosses origins)
- Proxied through gRPC-Web gateway (e.g., Envoy, gRPC-Web sidecar)

```javascript
// Browser code
const client = new helloworld.GreeterClient('http://localhost:8080');
const request = new helloworld.HelloRequest();
request.setName('World');
client.sayHello(request, {}, (err, response) => {
  console.log(response.getMessage());
});
```

## Reflection API & Server Reflection

gRPC servers can expose a reflection service, allowing clients to discover services at runtime:

```bash
# Discover available services
grpcurl -plaintext list localhost:5000

# Inspect service definition
grpcurl -plaintext describe localhost:5000.myapp.v1.UserService

# Make RPC call
grpcurl -plaintext -d '{"id": 1}' localhost:5000 myapp.v1.UserService/GetUser
```

Useful for debugging, CLI tools, exploratory testing. Production: often disabled for security (don't advertise internals).

## Compression

gRPC supports message-level compression (deflate, gzip):

```go
// Client
conn, _ := grpc.Dial("localhost:5000", 
  grpc.WithDefaultCallOptions(grpc.UseCompressor(gzip.Name)))

// Server
server := grpc.NewServer(grpc.KeepaliveParams(...))
```

Per-message: compress if message > threshold (e.g., 1KB). Trade-off: CPU for bandwidth. Beneficial for large streaming payloads; less for small unary requests.

See also: networking-grpc.md, networking-http2.md, networking-protocols.md, api-design.md