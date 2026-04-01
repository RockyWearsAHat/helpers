# QUIC Protocol — Connection Establishment, Multiplexing, Loss Recovery & HTTP/3

## Overview

**QUIC** (Quick UDP Internet Connection, RFC 9000) is a modern transport-layer protocol built on UDP, designed to replace TCP + TLS for real-world performance challenges:

- **Head-of-line blocking**: TCP processes bytes in order; a single lost packet blocks all subsequent data until retransmitted. QUIC multiplexes independent streams, so a lost packet for Stream A doesn't block Stream B.
- **Connection migration**: Move connection between networks (WiFi → cellular) without re-establishing. TCP is bound to a 4-tuple (src IP, src port, dst IP, dst port); changing the client's IP breaks TCP.
- **0-RTT resumption**: Resume encrypted sessions immediately without round-trip delay (via session tokens).
- **Cryptography built-in**: TLS 1.3 integrated into the handshake (not layered on top).

QUIC backs **HTTP/3** (the successor to HTTP/2 over TCP). Standardized by IETF; widely deployed by major CDNs and cloud providers.

## Motivation: Problems with TCP

### Head-of-Line Blocking

TCP buffers bytes in order:
```
TCP sender:   [Packet 1] [Packet 2] [Packet 3]
              (1000 bytes each, sent in order)

Network:      Packet 1 arrives ✓
              Packet 2 LOST ✗
              Packet 3 arrives ✓ (but buffered, not delivered to app yet)

TCP receiver: Delivers [Packet 1] to app
              Waits for [Packet 2]
              App sees stall even though [Packet 3] data is available
```

HTTP/2 multiplexes multiple streams over one TCP connection, but a single TCP packet loss impacts all streams sharing that connection.

### Connection Establishment Latency

Establishing a new TCP connection requires:
1. TCP 3-way handshake: SYN → SYN-ACK → ACK (1-RTT)
2. TLS 1.3 handshake: ClientHello → ServerHello {data}, {Finished} (1-RTT)
3. Total: 2-RTT before application data is sent encrypted.

### Mobility and Network Transitions

Switching networks (WiFi to cellular) changes the client's IP. TCP is identified by a 4-tuple; changing IP appears as a new connection. Must re-establish handshake.

QUIC uses a **connection ID** (not IP + port), enabling seamless migration.

## QUIC Connection Establishment

### Initial Handshake (1-RTT)

```
Client                                        Server
  | Initial packet                            |
  | (ClientHello TLS 1.3 messages)           |
  |------------------------------------------→|
  |                                         Initial packet
  |                                         (ServerHello, 
  |                                         EncryptedExtensions,
  |                                          Certificate verification,
  |                                          Finished)
  |                                        Handshake packet
  |                                         (Certificate, 
  |                                          CertificateVerify,
  |                                          Finished)
  |←------------------------------------------|
  |                                           
  | Handshake packet (Finished)               |
  |------------------------------------------→|
  |                                           
  | 1-RTT packet (application data encrypted)
  |←---------         encrypted        -----→|
```

**Key difference from TCP**: TLS is negotiated *within* QUIC's Initial packets, using QUIC's own encryption (a temporary key derived from ClientHello). The server's Handshake packets are encrypted with a key negotiated during the handshake itself, reducing RTT.

### 0-RTT Connection Resumption

After a full handshake, the server sends a **session token** (opaque encrypted data). Client stores it.

On reconnection:
```
Client                                        Server
  | Initial packet + 0-RTT packet            |
  | (ClientHello with PSK identity,          |
  |  + application data encrypted with PSK)  |
  |------------------------------------------→|
  |                                        Handshake packet
  |                                        (new session negotiation)
  |←------------------------------------------|
  |                                           
  | 1-RTT packet                              |
  |------------------------------------------→|
```

Server processes early data from the 0-RTT packet immediately (if the PSK is valid). If PSK is rejected, early data is discarded. Example: An HTTPS GET request can be sent in 0-RTT; the server responds after negotiating the new session.

**Replay risk**: Early data can be replayed. Mitigations:
- Server checks timestamps/version in PSK; reject if too old.
- Application layer ensures idempotency (GET, not mutating requests).

## Stream Multiplexing

QUIC defines **streams**—independent, bidirectional or unidirectional sequences of bytes:

- **Unidirectional streams**: One endpoint sends data; the other receives. Stream IDs: 0, 4, 8… (client initiates even); 1, 5, 9… (server initiates odd).
- **Bidirectional streams**: Both endpoints send/receive. Stream IDs: 0, 2, 4… (client); 1, 3, 5… (server).

Each stream has:
- Independent flow control (window size)
- Independent error handling (stream can be reset; connection persists)
- Sequential delivery *within* the stream (ordered) but independent ordering *across* streams

### Stream States

```
Unidirectional Send Stream:
  Idle → Open → (Data Recvd) → Reset/Fin

Bidirectional Stream:
  Idle
  → Open
  → (Data sent/recvd)
  → Half-Closed (one side finished sending)
  → Closed
```

### No Head-of-Line Blocking

```
Stream 0:  [Frame 1] [Frame 2] [Frame 3]
Stream 1:  [Frame A] [Frame B]

QUIC packet 1: Stream 0, Frame 1 + Stream 1, Frame A
QUIC packet 2: Stream 0, Frame 2 (LOST)
QUIC packet 3: Stream 0, Frame 3 + Stream 1, Frame B

Stream 0: Frame 1 delivered, Frame 3 buffered (waits for Frame 2)
Stream 1: Frame A, Frame B delivered immediately (no blocking)
```

Stream 1 data is available to the application even though Stream 0's Frame 2 is lost.

## Loss Detection and Congestion Control

### Loss Detection (ACK-based)

Sender maintains a *sent packet number space*. Receiver acknowledges received packets via `ACK` frames:
```
ACK frame:
  Largest Acked: 1005
  ACK Delay: 25 (in microseconds)
  ACK Ranges:
    [1000, 1005]  (packets 1000–1005 received)
    [990, 995]    (packets 990–995 received)
```

Sender infers packet loss if a higher packet number is acked but not a lower one:
- Packet 999 sent, Packet 1005 acked but no ack for 999 → **likely 999 is lost** (after timeout or by threshold).

### Congestion Control

QUIC uses **Reno-like congestion control** (similar to TCP NewReno):

1. **Slow Start**: Increase congestion window (cwnd) by 1 MSS per ACK until loss detected. Exponential growth.
2. **Congestion Avoidance**: After loss, reduce cwnd by 50%; increase by 1 MSS per RTT. Linear growth.
3. **Fast Recovery**: From packet loss, immediately retransmit and enter Congestion Avoidance.

Congestion window is per connection (not per stream); shared across all streams.

### Coalescing Packets

QUIC can send multiple frames in a single UDP packet (coalescing):

```
UDP packet:
  | QUIC Initial packet header + frames |
  | QUIC Handshake packet header + frames |
  | QUIC 1-RTT packet header + frames |
```

Allows efficient use of network MTU; avoids underutilization if individual frames are small.

## Connection Migration

QUIC connections are identified by a **Connection ID** (not 4-tuple). When a client's IP changes (WiFi → cellular), it continues using the same Connection ID:

```
Connection ID: 0x12345678

Time 1:
  Client IP: 192.168.1.100 → Server IP: 93.184.216.34
  Sends packet: Connection ID 0x12345678, data encrypted with connection keys

User switches to cellular

Time 2:
  Client IP: 203.0.113.50 → Server IP: 93.184.216.34
  Sends packet: Connection ID 0x12345678, same connection keys
  Server recognizes Connection ID, maps to same connection state
```

**Probing**: Client can send a probe packet (e.g., PING) and wait for response at the new IP before migrating traffic, ensuring connectivity.

**Path validation**: Server can send a challenge-response to validate the client's new address before accepting migration, preventing reflection attacks (attacker spoofs client IP to DOS the server).

## Version Negotiation

QUIC versions (Version field in packet header) allow protocol evolution:

- **Version 1**: RFC 9000 (current standard)
- **Version 2**: Upcoming (in draft)
- **Draft versions**: 0x6b…, 0xff…

On connection start:
```
Client sends Initial packet: version=1, ClientHello
Server sees version=1, supports it → responds with version=1

Client sends Initial packet: version=2 (not yet defined)
Server doesn't support version=2 → responds with Version Negotiation packet
  "I support version=1, draft-39, etc."
Client retries with version=1
```

Allows upgrades without breaking older clients/servers.

## QUIC Packet Types and Encryption

### Packet Types

- **Initial**: Unencrypted (with temporary encryption from ClientHello) for handshake.
- **0-RTT**: Encrypted with PSK key for early data.
- **Handshake**: Encrypted with handshake keys during negotiation.
- **Short Header (1-RTT)**: Encrypted with application-layer secret. Short header reduces overhead.

### Encryption Keys

QUIC derives keys at different stages:

1. **Initial Keys**: Derived from a salt + initial_secret (publicly known). Used for Initial packets. Anyone can decrypt Initial packets (not confidential, but authenticated).
2. **Handshake Keys**: Derived after ClientHello/ServerHello exchange. Confidential to participants.
3. **Application Keys**: Derived after handshake completion. Used for all post-handshake traffic.

**Key Scheduling** (HKDF-based):
```
Initial Secret = HKDF-Extract(salt, client_random)
Initial Keys = HKDF-Expand(Initial Secret, ...)

HandshakeSecret = HKDF-Extract(0, PSK)  // PSK from ClientHello
HandshakeKeys = HKDF-Expand(HandshakeSecret, ...)

MasterSecret = HKDF-Extract(0, HandshakeSecret)
ApplicationKeys = HKDF-Expand(MasterSecret, ...)
```

## HTTP/3 Over QUIC

**HTTP/3** replaces HTTP/2's TCP + TLS stack with QUIC. HTTP methods and semantics are identical to HTTP/2, but the transport changes:

- HTTP/2 frame (e.g., HEADERS, DATA, SETTINGS) are reused but sent over QUIC streams.
- Each HTTP request is sent on a single QUIC stream (no stream ID multiplex at HTTP level; QUIC handles that).
- Server push still exists (server initiates stream to push resource).
- Settings negotiated via QUIC SETTINGS frames (not HTTP/2 protocol-specific).

Example:
```
HTTP/3 GET /index.html

→ QUIC unidirectional stream (stream ID: 2, client initiates)
  | HEADERS frame: GET, /index.html, host=example.com
  → QUIC bidirectional stream (stream ID: 4, client initiates)
  | HEADERS frame: 200 OK
  | DATA frame: <html>...</html>
```

## Deployment Challenges

### Network Middlebox Interference

Many enterprise firewalls, proxies, and middleboxes are optimized for TCP and drop unfamiliar UDP traffic (not recognizing QUIC).

**Workarounds**:
- UDP port 443 (same as HTTPS) for QUIC, so firewalls may not block (assume it's DNS-over-QUIC or similar).
- Version negotiation allows fallback to TCP + HTTP/2 if QUIC fails.

### CPU Overhead of Crypto

QUIC integrates TLS 1.3 directly, requiring more per-packet cryptographic operations vs. TCP (where TLS is handled asynchronously). Can be mitigated with hardware offloading (AES-NI, VAES).

### Lack of OS-Level Support

TCP is deeply integrated in OS kernels (optimized, FQ scheduling, etc.). QUIC is typically user-space (libraries), less optimized. Kernel-level QUIC requires OS updates.

### NAT and Stateful Firewall Traversal

QUIC uses Connection ID to survive IP changes, but stateful firewalls often time out UDP flows after 30–60 seconds of inactivity. TCP's keep-alive mechanisms are less prone to timeout. Mitigation: applications send periodic PING frames to keep firewall state alive.

## QUIC Performance in Practice

**Latency gains**:
- **Connection retry**: 0-RTT vs TCP + TLS 1.3 2-RTT = 1-RTT saved (~30–50 ms).
- **Packet loss recovery**: Independent streams not blocked. Streaming video/multiplayer games see less jitter.

**Bandwidth efficiency**:
- **Shorter connection setup**: Fewer round-trips → fewer retransmissions.
- **Congestion window growth**: QUIC has simpler congestion control, responding faster to network changes.

**Real-world adoption**:
- **Google Chrome, Cloudflare CDN, Akamai, AWS CloudFront** deploy QUIC widely.
- **HTTP/3 support**: ~50% of top 10,000 websites (as of 2025).
- **ISP/enterprise**: Limited adoption due to middlebox issues; growing as upgrades happen.

## See Also

- [Networking Protocols — Packets to Applications](networking-protocols.md) — TCP/IP context
- [Networking — HTTP](networking-http.md) — HTTP/2 and HTTP/3 semantics
- [TLS Handshake Deep Dive](networking-tls-handshake.md) — TLS 1.3 fundamentals
- [Congestion Control & Networking](infrastructure-networking.md) — broader congestion control theory