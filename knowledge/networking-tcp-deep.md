# TCP Deep Dive — Three-Way Handshake, Congestion Control, SACK, Nagle, Keep-Alive & TIME_WAIT

## Overview

TCP (Transmission Control Protocol) provides reliable, ordered, connection-oriented delivery on top of IP's unreliable packet service. This note covers the mechanics of connection setup, congestion control algorithms, loss recovery, and socket tuning knobs that affect performance and latency.

## Three-Way Handshake and Connection State Machine

**Opening a TCP connection** involves three messages: SYN, SYN-ACK, ACK. This establishes shared state (sequence numbers, window sizes) and ensures both endpoints are ready.

```
Client                                    Server
  │                                         │
  ├─ [SYN, seq=100, wnd=65535] ───────────>│ SYN_RCVD
  │                              (1 RTT)   │
  │<────── [SYN-ACK, seq=200, ack=101] ────┤
  │        ESTABLISHED                      │
  ├─ [ACK, seq=101, ack=201] ────────────>│ ESTABLISHED
  │                              (0 RTT)   │
  │ (connection ready; data can flow)      │
```

The sequence number (`seq`) initializes randomly to prevent spoofing. The acknowledgment (`ack`) is the next expected sequence number. The `wnd` is the receive window size.

**TCP State Machine** (simplified):

```
CLOSED
  │ (open)
  ▼
LISTEN (server)
  │ (SYN received)
  ▼
SYN_RCVD
  │ (ACK received)
  ▼
ESTABLISHED ◄─── (client SYN sent, ESTABLISHED immediate on ACK)


ESTABLISHED
  │ (close requested)
  ▼
FIN_WAIT_1 (sent FIN; awaiting ACK)
  │
  ▼
FIN_WAIT_2 (received ACK; awaiting FIN from peer)
  │
  ▼
TIME_WAIT (received FIN; sent ACK)
  │ (2 * MSL timeout, ~60 seconds)
  ▼
CLOSED
```

**Key timing:**
- Handshake: 1 RTT (round-trip time) minimum. With TLS, add 1-2 RTTs.
- Closing: 1 RTT if unidirectional close (FIN-ACK); 2 RTTs if both sides send FIN.

## Congestion Control: Slow Start, AIMD, CUBIC, BBR, BBRv2

TCP adjusts its send rate dynamically based on network congestion, measured by packet loss and RTT. The congestion window (`cwnd`) is the maximum outstanding (in-flight) data.

### Slow Start

Initial phase: rapidly increase `cwnd` until loss.

```
cwnd = 1 MSS (1 packet)
Send 1, receive ACK → cwnd = 2
Send 2, receive ACK → cwnd = 4
Send 4, receive ACK → cwnd = 8
... (exponential growth until loss or ssthresh)
```

RTT time scale: each RTT doubles the sending rate. Advantage: quickly utilize available bandwidth. Disadvantage: overshoots (causes loss).

### AIMD (Additive Increase, Multiplicative Decrease)

After slow start, reduce growth:

```
No loss: cwnd += 1 (per RTT) [additive increase]
Packet loss: cwnd *= 0.5 [multiplicative decrease]
```

AIMD is fair (multiple flows converge to equal bandwidth share) and stable. It is the theoretical basis for congestion control.

### CUBIC

Modern TCP default (Linux 2.6.19+, Windows Server 2008+). CUBIC is less aggressive than AIMD when recovering from loss:

```
After loss at cwnd = W:
  cwnd = W * 0.7 (back off to 70%, not 50%)
  
Recovery phase:
  cwnd increases following cubic function (not linear AIMD)
  Faster recovery than AIMD
  More aggressive probing for additional bandwidth
```

CUBIC is optimized for long-distance, high-bandwidth links (satellite, trans-oceanic). It recovers faster after loss and uses more of the link on high-RTT paths.

### BBR (Bottleneck Bandwidth and RTT)

Google's algorithm (Linux 5.9+, some CDNs). BBR models the network as a bottleneck:

```
Delivery Rate = min(Bandwidth, Sending Rate)
Minimum RTT provides link latency

Model:
  Bandwidth estimate: track recent delivery rate
  RTT estimate: track minimum recent RTT
  Pacing rate: slightly exceed bottleneck (~1.5x) to probe for available capacity
```

Key difference: BBR does not rely on packet loss to detect congestion. It measures delivery *rate* (packets delivered per time). This means:
- **Lower latency:** Never queues aggressively
- **Faster convergence:** Finds optimal rate faster
- **Works with packet loss:** Doesn't interpret loss as congestion signal; instead, measures delivery rate drop

Trade-off: BBR is less fair than CUBIC when flows share a bottleneck (BBR flows may dominate). This is contentious in network research.

### BBRv2

Iteration on BBR addressing fairness and performance on variable-bandwidth links (cellular, Wi-Fi). BBRv2:
- Probes for additional capacity conservatively (less aggressive than BBR v1)
- Better coexistence with CUBIC flows
- Still lower latency than AIMD-based algorithms

## Fast Retransmit and SACK (Selective Acknowledgment)

**Fast Retransmit** avoids waiting for the retransmission timeout. If the sender receives 3 duplicate ACKs, it assumes a packet is lost and retransmits immediately (not waiting for the timeout).

```
Sender      Receiver
  │ Seq=1        │
  ├──────────────>│ ACK=2
  │ Seq=2        │
  ├──────────────>│ (lost)
  │ Seq=3        │
  ├──────────────>│ ACK=2 (duplicate)
  │ Seq=4        │
  ├──────────────>│ ACK=2 (duplicate)
  │ Seq=5        │
  ├──────────────>│ ACK=2 (duplicate, 3rd duplicate)
  │              │
  │ Retransmit 2 │
  ├──────────────>│ ACK=6
  │              │
```

Fast Retransmit reduces latency of loss recovery from seconds (RTT timeout) to milliseconds (3 RTTs).

**SACK (RFC 2018)** enhances this. Instead of only acknowledging the last in-order byte, SACK lists gaps in received data:

```
Receiver gets: Seq=1-10, Seq=20-30 (Seq=11-19 lost)
Without SACK: ACK=11 (only acknowledges contiguous)
With SACK: ACK=11; SACK=20-30 (lists received gaps)

Sender knows: Seq=11-19 are lost; Seq=20-30 were received
Retransmits only Seq=11-19, not redundantly retransmitting Seq=20-30
```

SACK dramatically improves throughput when multiple packets are lost per RTT.

## Nagle's Algorithm vs TCP_NODELAY

**Nagle's Algorithm** (RFC 896) buffers small sends to reduce tiny packets.

```
Application sends "A", then "B" (two bytes, two syscalls)
Nagle: Buffer "A"; wait for ACK of previous data
  Then coalesce "A" + "B" → send as one packet
Result: Fewer packets, less overhead
```

This is great for bandwidth-constrained links (1980s modem era). But for interactive apps (telnet, gaming, SSH):

```
User presses key, app sends character
Nagle buffers it, waiting for ACK
ACK arrives 50-200ms later
User perceives 50-200ms latency
```

**TCP_NODELAY** disables Nagle:

```c
int flag = 1;
setsockopt(sock, IPPROTO_TCP, TCP_NODELAY, &flag, sizeof(flag));
// Send immediately, no buffering
```

Modern best practice:
- **TCP_NODELAY=1 (disable Nagle):** Default for low-latency apps (gaming, trading, VoIP, SSH). Assume network is fast; don't add RTT latency.
- **TCP_NODELAY=0 (enable Nagle):** Rare; specific cases with many tiny sends and low bandwidth (legacy systems).

## Keep-Alive and Zombie Connections

**TCP Keep-Alive** detects dead connections. If no data flows for a period (default TCP_KEEPIDLE, usually 2 hours on Linux), the sender sends a probe packet (ACK with no data). If no response, the connection is declared dead.

```
No data for 2 hours
  │
  ├─ Send keep-alive probe (ACK)
  │
No response after 9 probes (TCP_KEEPCNT) at 75-second intervals
  │
Connection marked dead; application error
```

Why 2 hours? Legacy default; modern apps use application-layer keep-alive (HTTP keep-alive with heartbeats, application pings).

**Socket tuning** (Linux):

```bash
sysctl net.ipv4.tcp_keepalive_time=300      # 5 minutes instead of 2 hours
sysctl net.ipv4.tcp_keepalive_intvl=60      # interval between probes
sysctl net.ipv4.tcp_keepalive_probes=3      # number of probes
```

## TIME_WAIT State

After **close()**, TCP enters **TIME_WAIT** for 2 * MSL (~60 seconds). Why?

1. **Delayed packet absorption:** Old packets from the connection (with the same source/dest/port) may arrive late. If a new connection reuses the same port immediately, late old packets could be misinterpreted as new data. TIME_WAIT ensures all old packets are discarded.

2. **Remote host cleanup:** If you send FIN but the remote doesn't receive your final ACK (of their FIN), they'll retransmit their FIN. You must stay around to acknowledge it again.

```
You              Remote
 │ FIN
 ├───────> (enters FIN_WAIT_1)
 │
 │<─────  FIN (ACK lost)
 │
 ├───────> ACK (late; remote resends FIN)
 │
 │<─────  FIN (retransmitted)
 │
 ├───────> ACK
 │ (TIME_WAIT for 2*MSL to catch retransmitted FIN)
```

**Cost:** For high-connection-rate servers (API servers, load balancers), TIME_WAIT exhausts available ports (65535 possible; 10k+ connections/sec × 60s = many time_wait connections).

**Mitigation:**
- **SO_REUSEADDR:** Allow binding to a port in TIME_WAIT (not safe for all cases; can cause data misinterpretation)
- **tcp_tw_reuse:** Linux kernel option to reuse TIME_WAIT ports for new outgoing connections (safer than reuse_addr)
- **Increase port range:** Use high-port ranges (64k-65k) for outgoing connections
- **Load balancing:** Distribute connections across multiple address/port pairs

## TCP Backlog and Listen Queue

When a client initiates a connection (SYN), the OS places the connection in a queue before the application **accept()** it.

```
Client sends SYN
  ↓
OS receives SYN; responds with SYN-ACK
Connection placed in listen backlog (SYN_RCVD or ESTABLISHED queue)
  ↓
Application calls accept()
Connection dequeued; handed to application
```

**Backlog size** is set by **listen(sock, backlog)**. If the backlog fills, new SYNs are dropped, causing clients to retry.

```bash
listen(sock, 128)  # OS-dependent; Linux actually uses min(128, /proc/sys/net/core/somaxconn)
```

High-traffic servers tune this:

```bash
sysctl net.core.somaxconn=4096       # increase listen backlog
sysctl net.ipv4.tcp_max_syn_backlog  # SYN backlog (pre-ESTABLISHED)
```

## See Also

- `networking-tcp-ip.md` — TCP/IP model, addressing, general overview
- `networking-quic.md` — QUIC as a TCP alternative with better congestion control
- `networking-tls-handshake.md` — TLS handshake on top of TCP