# HTTP/3 & QUIC: UDP-Based Transport, Multiplexing & 0-RTT

## Overview

**QUIC** (RFC 9000) is a modern transport-layer protocol that replaces TCP for latency-sensitive applications. **HTTP/3** (RFC 9114) is the HTTP application protocol built on QUIC. Unlike HTTP/2 (TCP + multiplexing), HTTP/3 eliminates head-of-line blocking at the transport layer, enables connection migration across network changes, and supports 0-RTT session resumption without round-trip delay.

QUIC runs over UDP, making it deployable without kernel changes. Major adoption: Cloudflare, Akamai, Fastly, AWS, GCP, Azure, all major browsers. However, gradual rollout persists due to middlebox filtering (some networks drop UDP on port 443) and enterprise firewall policies.

## QUIC Transport Layer Fundamentals

### Connection Establishment (1-RTT vs TCP's 2-RTT)

QUIC combines cryptographic handshake with transport handshake in a single round-trip:

```
TCP + TLS 1.3 (2 RTTs):
  RTT 1: TCP SYN → SYN-ACK → ACK
  RTT 2: TLS ClientHello → ServerHello + certificates → ClientFinished

QUIC (1 RTT):
  RTT 1: Initial packet (ClientHello + crypto handshake)
         → Handshake packet (ServerHello + certificates)
         → Client now has keys, sends 1-RTT protected data
```

**Initial packet structure:**
```
Unencrypted Initial packet:
  - Long header (identifies this as an Initial)
  - Version number (allows future QUIC versions)
  - Destination connection ID (chosen by server)
  - Source connection ID (chosen by client)
  - Token (for address verification / anti-amplification)
  - Packet number (for loss recovery)
  - Payload: TLS ClientHello + other crypto frames
```

Server validates client's IP before processing expensive crypto operations using a token sent in earlier response. Prevents amplification attacks (attacker spoofing victim's IP to flood victim with server responses).

### Packet Numbers & Loss Recovery

Unlike TCP's byte-based sequence numbers, QUIC uses **packet numbers** (per encryption level, per packet space).

```
Packet 1: number=0, encrypted=Initial
Packet 2: number=1, encrypted=Initial
Packet 3: number=0, encrypted=Handshake (different space)
Packet 4: number=1, encrypted=Handshake
Packet 5: number=0, encrypted=1-RTT (application data)
```

**Advantage:** Unambiguous ACK semantics. In TCP, if packet is retransmitted, ACK is ambiguous (did it ack the original or the retransmission?). QUIC packet numbers are unique; ACL of packet number N definitively confirms delivery of that packet.

**Loss Detection:**
- ACK frame lists received packet numbers: `ACK { largest_acked: 42, ack_ranges: [(40-42), (35-38)] }`
- Sender detects loss when: packet N's ACK is missing after several later packets are ACKed
- Timeout-based loss detection: if packet not ACKed within 3× smoothed round-trip time, mark lost
- Explicitly lost packets are retransmitted with new packet number (not resent with same number)

### Stream Abstraction

QUIC provides **ordered, reliable byte streams** to the application, similar to TCP sockets. Each stream is independent:

```
Stream 0: "GET /api/users HTTP/1.1\r\n..."
Stream 4: "GET /api/posts HTTP/1.1\r\n..."
Stream 8: "POST /upload HTTP/1.1\r\n..."

All multiplexed in QUIC packets (may be in same or different UDP datagrams)
```

**Stream IDs:** 0, 1, 2, 3, ... (62-bit integers)
- Even = client-initiated, odd = server-initiated
- Allows either endpoint to open streams concurrently without coordination

**Bidirectional vs. unidirectional:**
- Bidirectional streams: 0, 1, 2, 3, ... (both endpoints send and receive)
- Unidirectional streams: 2, 3, 6, 7, ... (sender designated, other direction unused)

### Independent Stream Multiplexing (No Head-of-Line Blocking)

**The HTTP/2 problem:**
```
HTTP/2 over TCP:
  Client sends requests on streams 0, 4, 8 (multiplexed on same TCP connection)
  Network packet containing stream 0 data is lost
  TCP retransmitter must wait for retransmission before presenting any stream 0 bytes to HTTP/2
  HTTP/2 frames on streams 4 and 8 in same TCP packet are blocked by this retransmission
  → Application processing stalls for all streams until stream 0 recovers
```

**QUIC's solution:**
```
Each QUIC packet contains frames from multiple streams
Packet 1 (100 bytes): Stream 0 frames (50 bytes) + Stream 4 frames (50 bytes)
Packet 2 (100 bytes): Stream 8 frames

Packet 1 is lost. QUIC retransmits stream 0 frames in new packet.
Streams 4 and 8 data is not blocked — application processes their frames immediately.
Only stream 0 is delayed.
```

**Real-world impact:** Over networks with 0.1-1% loss, HTTP/3 achieves 5-10% throughput improvement due to this independence.

## QUIC Congestion Control & Flow Control

### Congestion Control

QUIC uses **cubic congestion control** (similar to Linux TCP's cubic):

```
Slow start: cwnd (congestion window) doubles each RTT until loss detected
Linear increase: cwnd += MSS each RTT (steady state)
Congestion avoidance: cwnd /= 2 on loss event (sharp drop, then recover)
```

Sender tracks:
- **cwnd**: congestion window (max bytes in flight)
- **bytes_in_flight**: total unacknowledged bytes
- Before sending packet, confirm: bytes_in_flight + packet_size <= cwnd

Loss detected → cwnd reduced → sender slows transmission → network congestion eases.

**Bottleneck Bandwidth & RTT (BBR):** Alternative algorithm increasingly deployed; BBR prioritizes throughput over latency jitter, suitable for video streaming.

### Flow Control

Separate from congestion control; prevents receiver from being overwhelmed:

- **Stream-level:** Each stream has RX window (max bytes receiver buffers)
  ```
  Receiver: "I can receive 64KB on stream 0"
  Sender: respect this limit
  Once receiver processes data, it sends window update frame
  ```
- **Connection-level:** Total bytes across all streams
  ```
  Receiver: "I can receive 1MB across all streams"
  Sender: respect this aggregate limit
  ```

Flow control prevents a slow consumer from stalling sender, reducing memory pressure on receiver.

## QPACK Header Compression

**Problem:** HTTP/2 uses HPACK (stateful header compression); if header block is lost in a packet, subsequent compressed headers are corrupted because decoder state diverged.

**QPACK solution:** Encoder stream sends instructions out-of-order (via separate QUIC stream); decoder can replay instructions to recover state even if a data packet is lost.

```
Encoder stream (reliable, in order):
  Instruction 1: Add ("content-type", "application/json") to dynamic table @ index 62
  Instruction 2: Add ("authorization", "Bearer ...") @ index 63

Data packet (application/http):
  HEADERS frame: Ref index 62, 63 (references dynamic table entries)

If data packet lost, encoder stream not affected; next data packet's HEADERS frames still valid.
```

**Compression ratio:** Comparable to HPACK (~60-70% reduction for typical headers). Trade-off: additional encoder stream overhead in latency-sensitive scenarios.

## Connection Migration

### Layer 3 (IP) vs. Layer 4 (Transport) Identity

TCP identifies connection by 4-tuple: (source IP, source port, destination IP, destination port). If client's IP changes (WiFi → cellular), 4-tuple changes → new connection required.

QUIC uses **Connection ID** (CID), independent of IP address:

```
Initial connection establishment:
  Client: generates source CID = 0x1a2b3c4d
  Server: generates destination CID = 0x5e6f7a8b
  
Communicated via Initial packet; both endpoints agree on CIDs.

WiFi → Cellular (IP address changes):
  Client: same CIDs, different source IP
  Server: recognizes CID, knows it's from same client
  → Connection continues without reset
```

### Migration Validation

Because CID is in packet header, server can route packet to correct connection handler immediately. However, server should verify client isn't spoofing the address:

**Client Address Validation:**
```
1. Client sends packet from IP 10.0.0.50
2. Server records this association and sends response
3. Later, client sends packet claiming to be from 10.0.0.50 but destination IP is different
   (packet might be from attacker using IP spoofing)
4. Server: "I haven't validated this new address yet"
5. Server sends PATH_CHALLENGE frame to new address
6. Client responds with PATH_RESPONSE frame (proves it can receive at that address)
7. Only after response does server migrate to new address
```

This prevents connection hijacking where attacker spoofs client's address to inject data.

## 0-RTT Early Data

QUIC enables sending application data in Initial packet if client has cached session state (connection ID, ciphers, server certificates).

```
Resumed connection:
  Client: Initial packet (0-RTT protected)
            + CRYPTO frame (session resumption)
            + STREAM frames (HTTP request)
  Server: processes 0-RTT data (speculatively; may replay if handshake fails)
          + Handshake packet (confirms session)
  
Result: HTTP request arrives at server in same RTT as handshake response.
No additional round-trip for application data.
```

**Caveat:** 0-RTT data lacks replay protection. If attacker captures Initial packet and resends it, server may execute request twice. Idempotent requests (GET, HEAD) are safe; state-changing requests (POST, DELETE) need protection:

```
Server-side replay detection:
  - Log 0-RTT request transaction IDs
  - Reject duplicates from same client CID + transaction ID
  - Or use implicit monotonicity: 0-RTT requests must not have side effects
```

Most QUIC deployments forward only safe requests (GET, HEAD) in 0-RTT.

## HTTP/3 Application Layer

HTTP/3 frames (HEADERS, DATA, CANCEL_PUSH, PUSH_PROMISE, MAX_PUSH_ID, GOAWAY, HEADERS, SETTINGS, SETTINGS_ACK, PRIORITY_UPDATE) map directly to QUIC streams:

- Each HTTP request = single QUIC stream (bidirectional)
- HEADERS frame: HTTP method, path, headers (QPACK-compressed)
- DATA frames: request or response body
- Stream closure = request complete

Key difference from HTTP/2: No server push (removed in HTTP/3; simpler, less problematic). Stream dependencies and prioritization now per-request, not tree-based.

## Deployment Challenges & Fallback

**Middlebox filtering:** Many corporate firewalls drop UDP 443 or rate-limit it → clients fall back to HTTP/2.

**Connection migration complexity:** Some WiFi routers drop packets from changing IPs in anti-spoofing measures → migration fails.

**Stateless reset:** Server restarts and forgets connection IDs → must send STATELESS_RESET frame to client, closing connection.

**Paths forward:**
- QUIC protocol extension for firewall traversal / explicit signaling (DPRIVE)
- Happy eyeballs for clients: race QUIC and TCP; use whichever succeeds first
- Incremental adoption by CDNs/hyperscalers first; enterprise adoption lagging

See also: networking-quic.md, networking-http3.md, networking-tls-handshake.md, networking-protocols.md