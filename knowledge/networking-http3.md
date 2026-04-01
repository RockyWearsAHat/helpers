# HTTP/3 — Application Layer on QUIC, 0-RTT & Deployment

## Overview

**HTTP/3** is the HTTP application protocol built on **QUIC** (RFC 9110 for HTTP/3, RFC 9000 for QUIC). Unlike HTTP/1.1 (TCP) and HTTP/2 (TCP + multiplexing), HTTP/3 uses QUIC's UDP-based transport, gaining connection migration, 0-RTT session resumption, and independent stream multiplexing without head-of-line blocking.

HTTP/3 is **standardized and deployed** in major CDNs (Cloudflare, Akamai, Fastly), cloud providers (AWS, GCP, Azure), and browsers (Chrome, Firefox, Safari). However, deployment is **gradual** — TCP/HTTP/2 remains dominant due to legacy infrastructure and firewall restrictions on UDP.

Key improvements over HTTP/2 over TCP:

1. **0-RTT resumption**: Resume session without round-trip delay
2. **Connection migration**: Survive WiFi↔cellular network switching  
3. **Stream-level HOL blocking**: Packet loss on one stream doesn't stall other streams
4. **Faster handshake**: QUIC's 1-RTT establishment vs. TCP's 3-way + TLS

## Protocol Structure

### QUIC as Transport Layer

QUIC is a transport protocol similar to TCP. HTTP/3 is an application protocol that uses QUIC the way HTTP/2 uses TCP.

```
Application (HTTP requests/responses)
            ↓
HTTP/3 frame layer (HEADERS, DATA frames)
            ↓
QUIC stream abstraction (ordered byte delivery per stream)
            ↓
QUIC packet layer (encryption, retransmission, congestion control)
            ↓
UDP datagram
            ↓
IP layer
```

### QUIC Streams vs HTTP Requests

Each HTTP/3 request uses an independent QUIC stream. Streams are multiplexed in QUIC packets:

```
QUIC Packet 1: [Stream 0 DATA | Stream 4 DATA | Stream 8 DATA]
QUIC Packet 2: [Stream 0 DATA | Stream 4 DATA]

One packet loss affects Packet 1's streams (0, 4, 8)
But Stream 0 can be retransmitted independently
Applications on Stream 4 and 8 are NOT blocked waiting for Stream 0
```

This is the **key advantage over HTTP/2**, which multiplexes over a single TCP connection:

```
HTTP/2: All streams share one TCP connection
        One packet loss → TCP retransmitter blocks all streams
        
HTTP/3: Each stream independent
        One packet loss → only affected stream(s) retry
        Unaffected streams proceed
```

For real-world networks with 0.1-1% loss, HTTP/3 delivers lower latency.

## 0-RTT (Zero Round-Trip Time) Session Resumption

### The Problem: HTTP/2 Full Handshake

Establishing an HTTP/2 connection requires:
1. TCP 3-way handshake: 1 RTT (Initiator → Server → Initiator)
2. TLS 1.3 handshake: 1 RTT (Client → Server → Client)
3. First HTTP request: sent in the third leg of TLS (0-RTT in TLS 1.3 terms, or after full TLS)

Total: 2 RTTs before application data (if early data is available) or 3 RTTs if TLS Finished is required.

### QUIC 0-RTT

QUIC's handshake is **1 RTT** for new connections:

1. Client sends Initial packet with TLS Client Hello: 1 RTT
2. Server responds with Initial + Handshake packets
3. Client sends Handshake packet, then application data

For resumed connections (client has cached session state):

```
Client → Server: Initial + 0-RTT protected application data
Server → Client: Handshake packets
Client: Ready to send more app data (no further wait)
```

**Total: 0 RTTs for resumed sessions** (assuming no packet loss). First packet after resumption can include HTTP request data.

### Session Token/Ticket

QUIC tracks resumption via a **session token** (analogous to TLS session ticket):

```
Initial connection:
  Client → Server: Initial, TLS handshake
  Server generates token (encrypted with server secret)
  Server → Client: Handshake, includes token (in NEW_TOKEN frame)

Resumed connection (client has token):
  Client → Server: Initial + 0-RTT DATA frames
                   (proves it has the token, is resuming)
  Server validates token, fast-tracks session
```

Tokens are **server-generated and server-validated** (not sent to client in plaintext, encrypted). Server can include client address for anti-amplification (reject spoofed IPs before expensive processing).

### Use Case: Mobile Web

On cellular networks where RTT is 50-100ms and TCP handshake + TLS adds 100-200ms latency, 0-RTT provides **noticeable speedup**: page load time improvement of 100-200ms for repeat visits.

## Connection Migration

### The Problem: Network Switching

User on WiFi receives phone call, switches to cellular. IP address changes.

**TCP:**
```
Client: WiFi IP=10.0.0.50
TCP connection identified by (src_ip, src_port, dst_ip, dst_port)
Connection: (10.0.0.50:12345, server:443)

Cellular: Client IP=150.1.2.3
TCP sees (150.1.2.3:12345, server:443) — different 4-tuple
TCP resets; new connection needed
Browser must rebuild TCP + TLS + HTTP state
```

**QUIC:**
```
Client: WiFi, 4-tuple = (10.0.0.50:12345, server:443)
        Connection ID = 0x deadbeef (chosen by client)

Cellular: Client IP=150.1.2.3
Client sends packet with same Connection ID = 0xdeadbeef
Server looks up connection by ID (not 4-tuple)
"It's still the same connection, just new source address"
Connection persists; rekey DCID (destination connection ID) to learn new address
```

**Result:** Seamless migration, no TLS renegotiation, no HTTP stream resets.

## Performance Characteristics

### Handshake Latency

- **HTTP/2 (TCP + TLS 1.3)**: 2-3 RTTs (new connection), ~1 RTT (resumption, TLS Early Data)
- **HTTP/3 (QUIC)**: 1 RTT (new), ~0 RTTs (resumed)

On 50ms RTT networks (LTE, long-haul WiFi):
- HTTP/2: 100-150ms handshake
- HTTP/3: 50ms (new), ~0ms (resumed)

### Stream Multiplexing & HOL Blocking

**Scenario:** Download 10 small files over HTTP/2 vs HTTP/3, one packet lost.

```
HTTP/2:
  Request 1, 2, 3, ..., 10 sent
  Response for Req 8 (10KB) lost
  TCP retransmits at TCP layer (doesn't know about HTTP)
  While waiting for Req 8, Req 9 and 10 can't be delivered to app
  (Even if their packets arrived, TCP buffers deliver in order)
  Waterfall: 1, 2, ..., 7 delivered; pause; 8 arrives; 9, 10 delivered

HTTP/3:
  Request 1-10 in separate streams
  Response for Req 8 lost
  QUIC retransmits Stream 8 specifically
  Streams 9, 10 deliver immediately (no TCP-layer blocking)
  Waterfall: 1-7 delivered, 9-10 delivered, 8 delivered
```

Measured gain: ~10-20% latency reduction for request parallelism on lossy networks.

### Congestion Control

QUIC implements algorithms like BBR (Bottleneck Bandwidth and RTT estimation) or CUBIC (like TCP Reno-based). Same algorithms as TCP but with per-stream visibility:

- TCP congestion window applies to all multiplexed streams
- QUIC can inform streams individually of congestion (backpressure per stream)

Practical difference: marginal (algorithms are similar), but QUIC design is more modular.

## Connection Addressing

### Source Connection ID (SCID) vs. Destination Connection ID (DCID)

Each endpoint generates its own connection ID to identify the connection (not the 4-tuple):

```
Client chooses SCID = 0x11223344
Server chooses SCID = 0xaabbccdd

Client packets have:
  DCID = 0xaabbccdd (so server knows which connection)
  SCID = 0x11223344 (so client can parse responses)

Server packets have:
  DCID = 0x11223344 (so client knows which connection)
  SCID = 0xaabbccdd

Connection ID is opaque to network (not used for routing by load balancers)
```

**Implication:** Connection ID enables:
- Connection migration (change 4-tuple, same Connection ID)
- Load balancer flexibility (can use stateless ID format: encode backend in Connection ID, load balancer decodes without state table)

## Deployment Status & Adoption

### Browser Support

- **Chrome/Chromium**: 51+ (shipped 2016, widely available)
- **Firefox**: 25+, but disabled by default in some builds, enabled in others
- **Safari**: 16+ (iOS 16, macOS 13)
- **Edge**: 79+

### Server & CDN Support

- **Cloudflare**: Enabled for all customers (2020)
- **AWS**: CloudFront supports HTTP/3 (opt-in)
- **Akamai, Fastly**: HTTP/3 available
- **Google**: YouTube, search, Gmail support HTTP/3
- **NGINX**: HTTP/3 support (1.25+)
- **OpenSSH**: HTTP/3 support in recent TLS libraries (BoringSSL, LibreSSL)

### Deployement Challenges

1. **UDP Blocked**: Many corporate firewalls, cellular carriers block UDP on non-standard ports. Fallback to HTTP/2 is essential.
2. **NAT/Firewall Complexity**: UDP doesn't benefit from stateful firewall connection tracking (TCP-specific). Causes issues in some networks.
3. **Legacy Infrastructure**: Old load balancers, middle-boxes don't understand QUIC packets; require upgrade.
4. **Client Diversity**: Not all clients support HTTP/3; servers must support HTTP/2 fallback.

### Adoption Curve

As of 2024-2025:
- Major CDNs have HTTP/3 enabled by default
- ~20-30% of web traffic is HTTP/3 (from QUIC statistics on CDNs)
- Adoption accelerating as browsers and servers default-enable

## TLS 1.3 Integration

QUIC **requires TLS 1.3** (RFC 8446). Key differences from HTTP/2 over TCP + TLS:

### Session Resumption

TLS 1.3 pre-shared keys (PSK) are used for 0-RTT:

```
Initial connection:
  Client, server agree on PSK and PSK identity
  
Resumed connection:
  Client sends PSK identity in Client Hello
  Server can send app data after 1 RTT
```

QUIC combines TLS PSK with its own session token mechanism.

### Handshake Optimization

QUIC and TLS handshakes are interleaved:

```
QUIC Initial:        [TLS Client Hello]
QUIC Handshake:      [TLS Server Hello, Finished]
QUIC application:    [TLS Finished, app data]
```

TLS messages are fragmented across QUIC packet types for optimal pipelining.

### Certificate and Key Management

Same as HTTP/2 (X.509 certificates, OCSP stapling, etc.). No changes to PKI.

## Performance Trade-offs & Considerations

### Advantages

- **Lower latency** (0-RTT, 1-RTT handshake, stream-level HOL elimination)
- **Connection migration** (transparent network switching)
- **Independent stream head-of-line blocking** (packet loss on one stream doesn't stall others)
- **Modern crypto** (TLS 1.3 integration)
- **Future-proof** (QUIC design allows algorithm evolution)

### Disadvantages & Uncertainties

- **UDP deployment challenges** (firewall blocking, NAT/ALG unfriendliness)
- **CPU overhead** (UDP processing, per-packet encryption can be higher than TCP in some scenarios)
- **Newer technology** (less battle-tested than TCP; edge cases discovered in deployments)
- **Middle-box interference** (NAT boxes, firewalls, proxies may not understand QUIC; require updates)
- **Debugging difficulty** (UDP packets more opaque than TCP; packet sniffing requires QUIC awareness)

### When to Use HTTP/3

- High-latency networks (satellite, mobile, long-haul)
- Mobile apps with frequent network switching
- Repeat visits / resumed connections (0-RTT benefit)
- High packet loss networks (stream-level HOL elimination helps)

### When HTTP/2 Suffices

- Low-latency LANs (handshake overhead doesn't matter)
- Single-request-per-connection (no multiplexing gain)
- Client requires UDP-blocking-proof deployment (fallback to HTTP/2)
- Infrastructure not yet updated (legacy middle-boxes)

## Implementation & Testing

### Server Configuration (NGINX Example)

```
# Modern NGINX with QUIC module:
server {
  listen 443 quic reuseport;
  listen 443 ssl http2;  # fallback
  
  http3_max_idle_timeout 65000;
  http3_max_ack_delay 20;
}
```

### Client Testing

```bash
# curl with HTTP/3 support:
curl --http3 https://example.com

# See negotiated protocol:
curl -I --http3 https://example.com -w "%{protocol}\n"
```

### Observability

QUIC connections are harder to monitor (packet capture requires QUIC aware tools like Wireshark 3.0+). Logging typically relies on application metrics, not network packet inspection.

## See Also

- [networking-quic.md](./networking-quic.md) — QUIC protocol details, handshake, congestion control
- [networking-http.md](./networking-http.md) — HTTP protocol evolution, semantics, caching
- [networking-tls-handshake.md](./networking-tls-handshake.md) — TLS 1.3 handshake details
- [web-http-caching.md](./web-http-caching.md) — HTTP caching in HTTP/3 (same semantics as HTTP/2)
- [infrastructure-cdn.md](./infrastructure-cdn.md) — CDN deployment of cutting-edge protocols