# TCP/IP Protocol Suite — 4-Layer Model, Connection Lifecycle, Congestion Control, Addressing

## Overview

TCP/IP is the foundational protocol suite that powers the modern internet. The TCP/IP model (also called the internet model) is a pragmatic, four-layer abstraction that differs subtly from the seven-layer OSI model. Understanding this model, how TCP manages connections, how it detects and reacts to congestion, and how IP addresses are organized is essential for network engineering.

## The TCP/IP Model: Four Layers

The TCP/IP model collapses the OSI model into four practical layers:

```
  7 Application         ─────→  Application Layer
  6 Presentation        ─────┐  
  5 Session             ─────┴→ (handled by app and transport)
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─   
  4 Transport           ─────→  Transport Layer (TCP, UDP, QUIC)
─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─   
  3 Network             ─────→  Internet Layer (IP, ICMP)
  2 Data Link           ─────┐
  1 Physical            ─────┴→ Link Layer (Ethernet, Wi-Fi, PPP)
```

### Layer 1: Link Layer

Handles physical transmission: Ethernet frames, Wi-Fi, PPP. Provides MAC addresses (48-bit hardware identifiers) for local-network delivery. Frames are addressed to the next *hop*, not the final destination.

**Key protocols**: Ethernet, Wi-Fi (802.11), ARP (Address Resolution Protocol; maps IP to MAC).

### Layer 2: Internet Layer

Routes packets across the globe using IP addresses. Provides best-effort (unreliable) delivery: packets may be lost, duplicated, reordered, or delivered late.

**Key protocols**: 
- **IPv4**: 32-bit addressing, still dominant but address space exhausted
- **IPv6**: 128-bit addressing, designed for the future
- **ICMP**: Internet Control Message Protocol (ping, traceroute, error messages)

### Layer 3: Transport Layer

Provides end-to-end communication semantics *on top* of IP's best-effort delivery.

**TCP (Transmission Control Protocol)**:
- Reliable, ordered delivery
- Connection-oriented (handshake, teardown)
- Flow control and congestion control
- Used by HTTP, HTTPS, SSH, SMTP, Telnet

**UDP (User Datagram Protocol)**:
- Unreliable, unordered delivery
- Connectionless (send and forget)
- Low latency, high throughput
- Used by DNS, NTP, VoIP, video streaming, online games

**QUIC**: UDP-based protocol that adds reliability, multiplexing, and congestion control without the head-of-line blocking of TCP.

### Layer 4: Application Layer

Where user applications live: browsers, mail clients, databases, sensors, APIs.

**Key protocols**: HTTP/HTTPS, SSH, SMTP, DNS, NTP, DHCP, FTP.

## TCP Connection Lifecycle

TCP connections go through distinct states. Understanding these is critical for debugging connection issues, setting socket options, and managing resource limits.

### The Three-Way Handshake (Connection Establishment)

```
Client                                Server
  |                                     |
  |------ SYN, seq=X, window=W -----→   |
  |                                     |
  | ←-- SYN-ACK, seq=Y, ack=X+1 ----   |
  |                                     |
  |------ ACK, seq=X+1, ack=Y+1 ----→   |
  |                                     |
  |<-- Connection established -----→    |
```

1. **SYN**: Client sends initial sequence number X and advertised window W (how many bytes it can receive).
2. **SYN-ACK**: Server responds with its sequence number Y, acknowledges X+1, and sends its window size.
3. **ACK**: Client acknowledges Y+1. Connection is now established.

Each side now knows:
- The other side's initial sequence number
- The other side's window size (flow control limit)
- The path is at least bidirectional (crucial for detecting firewalls)

### Sequence Numbers and Acknowledgments

TCP treats the data stream as a sequence of bytes. Each byte has a sequence number (32-bit counter, wrapping after 2³² bytes). Sequence numbers serve multiple purposes:

- **Ordering**: Out-of-order packets are reordered by the receiver's TCP stack.
- **Duplicate detection**: If two packets have the same sequence number range, the second is a duplicate.
- **Loss detection**: If sequence numbers jump (e.g., seq 100 then 200), the receiver knows bytes 100–199 were lost.

**Acknowledgments**: The receiver sends back the sequence number of the *next* byte it expects. This tells the sender "I've received everything up to byte N; ready for byte N+1 onward."

### Window Size (Flow Control)

Each side advertises how much unacknowledged data it can buffer. The sender can only send up to the advertised window before it must wait for an ACK.

```
Sender's Perspective:
Last Byte Sent = 500
Last Byte Acked = 200
Advertised Window = 300

Effective Window = 300 - (500 - 200) = 300 - 300 = 0
(Sender cannot send more until it receives an ACK)
```

The window is dynamic: the receiver can shrink it if its buffers fill up, forcing the sender to slow down.

### Connection Termination (Four-Way Handshake)

```
Client                                Server
  |                                     |
  |--------- FIN, seq=X ------→         |
  |                                     |
  | ←-------- ACK, ack=X+1 -----        |
  |                                     |
  | ←--------- FIN, seq=Y ------        |
  |                                     |
  |--------- ACK, ack=Y+1 ------→       |
  |                                     |
  | TIME_WAIT (2 × MSL ≈ 60s)           |
```

1. **FINCient initiates close by sending FIN (sequence number X).
2. **Server ACKs** the FIN (ack=X+1). The server may continue sending data.
3. **Server FINs** when it is ready to close (sequence number Y).
4. **Client ACKs** (ack=Y+1).

**TIME_WAIT**: After the client sends the final ACK, it waits in TIME_WAIT state for approximately 2 × MSL (Maximum Segment Lifetime, ~60 seconds). This allows:
- Late retransmissions of the server's FIN to be acknowledged
- Stale packets from previous connections to be discarded
- Prevents connection confusion if the same [client IP, client port, server IP, server port] tuple is reused

TIME_WAIT can be a bottleneck under high connection churn (e.g., thousands of short-lived HTTP connections). Solutions include SO_REUSEADDR socket option or using ephemeral port ranges.

### States

TCP endpoints cycle through well-defined states:

| State | Meaning |
|---|---|
| **LISTEN** | Server waiting for incoming connections |
| **SYN_SENT** | Client sent SYN, waiting for SYN-ACK |
| **SYN_RECEIVED** | Server received SYN, sent SYN-ACK, waiting for ACK |
| **ESTABLISHED** | Connection open, data can flow |
| **FIN_WAIT_1** | Sent FIN, waiting for ACK |
| **FIN_WAIT_2** | Sent FIN, received ACK, waiting for other side's FIN |
| **CLOSE_WAIT** | Received FIN, but not yet ready to close |
| **LAST_ACK** | Sent FIN in response to FIN, waiting for ACK |
| **TIME_WAIT** | Sent final ACK, waiting before cleanup |
| **CLOSED** | Connection closed |

## TCP Congestion Control

Congestion occurs when routers' queues fill up and packets start being dropped. Without congestion control, the internet would collapse: senders would retransmit dropped packets, adding more congestion.

TCP congestion control uses the arrival and loss of packets to infer network conditions and adjust the sending rate.

### The Congestion Window (CWND)

TCP maintains a *congestion window*, separate from the advertised window. The effective window is the minimum of congestion window and advertised window:

```
Effective Window = MIN(CWND, Advertised Window)
```

The sender can only send as many bytes as the effective window allows.

### Additive Increase, Multiplicative Decrease (AIMD)

**Additive Increase**: When data is successfully delivered (ACKs arrive on time), increase CWND by one MSS (Maximum Segment Size) per RTT. This is slow and deliberate—it seeks available bandwidth gradually.

**Multiplicative Decrease**: When loss is detected (timeout or duplicate ACKs), cut CWND in half (or by some factor). This is aggressive to quickly back off when the network is congested.

```
Time ────────────────────────────────────→
CWND     ╱╲                  ╱╲
         ║ ╲                ╱  ╲
         ║  ╲              ╱    ╲ (loss)
         ║   ╲ (loss)     ╱      ╲
         ║    ╲──────────╱────────╲─ (recovery)
       ──╱─────────────────────────╲──
            Additive Increase    Multiplicative Decrease
```

This asymmetry (slow increase, fast decrease) prevents the Internet from oscillating wildly and protects against congestion collapse.

### Slow Start

When a TCP connection opens or after a timeout, CWND starts very small (typically 1–2 MSS). The algorithm is called "slow start" but is actually exponential:

```
RTT 1: CWND = 1,   send 1 segment,  receive 1 ACK,  CWND → 2
RTT 2: CWND = 2,   send 2 segments, receive 2 ACKs, CWND → 4
RTT 3: CWND = 4,   send 4 segments, receive 4 ACKs, CWND → 8
RTT 4: CWND = 8,   send 8 segments, receive 8 ACKs, CWND → 16
...
```

Each RTT, CWND doubles. This ramps up quickly but conservatively, probing initially available bandwidth without crashing the network.

**Slow Start Threshold (SSThresh)**: When loss occurs, SSThresh is set to CWND/2. When CWND grows back and reaches SSThresh, the algorithm switches from slow start (exponential) to congestion avoidance (additive).

### Congestion Avoidance

Once CWND ≥ SSThresh, growth shifts to additive: increase by one MSS per RTT.

```
if (CWND < SSThresh) {
    CWND += MSS  // exponential slow start
} else {
    CWND += MSS / CWND  // additive increase (grows 1 MSS per RTT on average)
}
```

Additive increase is deliberate and safe—it probes for additional bandwidth steadily without aggressive growth.

### Loss Detection and Recovery

**Timeout**: If no ACK arrives within an estimated round-trip time, assume the segment was lost. Retransmit, set SSThresh = CWND/2, set CWND = 1, re-enter slow start.

**Fast Retransmit**: If the receiver gets out-of-order segments, it sends duplicate ACKs. When the sender receives 3 duplicate ACKs (or more precisely, 4 ACKs with the same sequence number), it assumes loss without waiting for timeout. Retransmit immediately, set SSThresh = CWND/2, set CWND = SSThresh + 3×MSS, and re-enter congestion avoidance.

Fast retransmit is faster than timeout-based recovery and is standard in modern TCP (RFC 5681).

### TCP Variants

Different TCP implementations use different congestion control algorithms:

- **TCP Reno** (most common): Slow start, additive increase, fast retransmit, fast recovery
- **TCP Tahoe**: Older; returns to slow start on all losses
- **TCP CUBIC**: Faster growth for high-bandwidth links (Linux default)
- **BBR (Bottleneck Bandwidth and Round-trip time)**: Google's algorithm; models the bottle neck and targets specific CWND over window-based growth

The choice of congestion control algorithm affects throughput and latency. Different algorithms scale differently for high-bandwidth, high-latency links (e.g., intercontinental).

## IP Addressing and Subnetting

### IPv4 Address Structure

IPv4 addresses are 32-bit integers, typically written in dotted decimal notation:

```
192.168.1.1 = 11000000.10101000.00000001.00000001 (binary)
```

### Classless Inter-Domain Routing (CIDR)

CIDR notation: `192.168.1.0/24` means the first 24 bits are the network identifier; the last 8 bits are the host identifier.

```
192.168.1.0/24:
  Network:  192.168.1.0   (all hosts = 0)
  Usable:   192.168.1.1 - 192.168.1.254
  Broadcast: 192.168.1.255 (all hosts = 1)
```

The subnet mask encodes the split: `/24` → mask = `255.255.255.0`.

**Address Classes** (historical, now overshadowed by CIDR):
- **Class A** (/8): `1.0.0.0 – 126.255.255.255` (large organizations)
- **Class B** (/16): `128.0.0.0 – 191.255.255.255` (medium organizations)
- **Class C** (/24): `192.0.0.0 – 223.255.255.255` (small networks)

### Private IPv4 Address Ranges (RFC 1918)

Three ranges are reserved for private networks (never routed on the public internet):

```
10.0.0.0/8     (10.0.0.0  –  10.255.255.255)     16.7 million addresses
172.16.0.0/12  (172.16.0.0  –  172.31.255.255)   1 million addresses
192.168.0.0/16 (192.168.0.0  –  192.168.255.255) 65k addresses
```

These ranges are used inside companies, homes, and NAT gateways. They cannot be routed on the public internet.

### Subnetting

Subnetting divides a network into smaller subnets. For example, `10.0.0.0/8` can be divided into `10.0.0.0/24`, `10.1.0.0/24`, ..., `10.255.0.0/24` (256 subnets of 254 hosts each).

**Subnet calculation**:
1. Determine the network and host bits: `10.0.0.0/24` = 24 network bits, 8 host bits.
2. The network address is the IP with host bits set to 0: `10.0.0.0`.
3. The broadcast address is the IP with host bits set to 1: `10.0.0.255`.
4. Usable host IPs are `.1` through `.254`.

Subnetting supports hierarchical address allocation and containment of broadcast domains.

## Network Address Translation (NAT)

NAT maps private IP addresses to public IP addresses, allowing multiple internal hosts to share a single public IP.

### How NAT Works

```
Internal Network (private)          NAT Gateway              External Network (public)
10.0.0.5:12345                    203.0.113.1:54321         203.0.113.1:54321
    ↓ (outgoing packet)                 ↓
    └──→ [TCP SYN to 8.8.8.8:80] → Replace source IP/port
         [src: 10.0.0.5:12345] ════ [src: 203.0.113.1:54321]
                                       │
                                    8.8.8.8:80 (external target)
                                       │
                                   (response arrives)
                                       │
    ← [TCP SYN-ACK: dst 203.0.113.1:54321]
         Replace destination IP/port back
    [dst: 10.0.0.5:12345] ← ────── [dst: 203.0.113.1:54321]
```

The NAT gateway maintains a table of active mappings: (internal IP, internal port) → (public IP, public port). Return traffic is rewritten and forwarded back to the internal host.

**Types**:
- **SNAT (Source NAT)**: Rewrites the source address (most common)
- **DNAT (Destination NAT)**: Rewrites the destination address (e.g., port forwarding)
- **Bidirectional NAT**: Both source and destination

### NAT Challenges

- **Port exhaustion**: A single public IP has only 65k ports; under high concurrency, port collisions occur.
- **Connection state**: NAT requires tracking every active connection; state table can fill up.
- **Inbound connections**: Hard to establish inbound connections to internal hosts without port forwarding.
- **Protocol-specific issues**: VoIP, P2P rely on predictable addresses; NAT breaks them.
- **Application visibility**: Applications behind NAT see their private IP, not public IP (confusing for debugging).

## IPv6: The Next Generation

IPv6 uses 128-bit addresses, providing effectively unlimited address space (2^128 ≈ 3.4 × 10³⁸ addresses).

```
IPv6: 2001:0db8:0000:0000:0000:ff00:0042:8329
Shorthand: 2001:db8::ff00:42:8329
Loopback: ::1
```

### IPv6 Advantages

- **No NAT needed**: Every device gets a globally unique address.
- **Simpler header**: Fixed 40-byte header (vs. variable IPv4).
- **Built-in security**: IPsec is mandatory (in practice, often optional).
- **Stateless autoconfiguration**: Devices configure their own addresses from router advertisements.

### IPv6 Adoption Challenges

- **Backward compatibility**: IPv4 and IPv6 don't interoperate; dual-stack (running both) is necessary.
- **Transition mechanisms**: 6to4 tunneling, NAT64, DS-Lite allow coexistence but add complexity.
- **Application readiness**: Many apps don't handle IPv6 well; many APIs still default to IPv4.
- **Deployment**: As of 2026, IPv6 is ~30–40% deployed globally; much of the internet still runs IPv4.

## Practical Networking Patterns

### Calculating Usable Hosts in a Subnet

For a /24 subnet:
- Total IPs: 2^8 = 256
- Network address: .0 (not usable)
- Broadcast address: .255 (not usable)
- Usable hosts: 256 - 2 = 254

### Supernetting (CIDR Aggregation)

Combining multiple subnets into one. For example, `10.0.0.0/24` and `10.1.0.0/24` can be aggregated into `10.0.0.0/23` (covers both).

Aggregation reduces the size of routing tables and improves forwarding efficiency.

### Multicast Addressing

IPv4 multicast addresses: `224.0.0.0/4` (224.0.0.0 – 239.255.255.255). Used for one-to-many group communication (video streaming, IoT broadcasts).

## Related Concepts

- **networking-dns**: DNS queries, DNSSEC use IP for transport
- **networking-protocols**: TCP, UDP deep dive at application/transport boundary
- **security-encryption**: IPv6 mandatory IPsec, TLS/HTTPS over TCP