# OS Networking Stack — Kernel Internals, Socket API, Kernel Bypass, and Zero-Copy

## Overview

The Linux networking stack is a layered hierarchy that transforms raw packet arrivals from a network interface card into socket abstractions usable by user-space applications. Understanding the path a packet travels through the kernel, the socket buffer data structures, socket semantics, and modern optimization techniques (kernel bypass, XDP, io_uring zero-copy) is essential for writing efficient networked systems, embedded software, and high-performance servers.

## Packet Flow Through the Kernel

A network packet arrives at the NIC and travels through several layers before reaching an application socket.

### Hardware to Software: Interrupt and IRQ

1. **NIC receives packet**: Frames arrive in the NIC's DMA ring buffer (shared memory between NIC and CPU).

2. **NIC raises interrupt**: Signals the CPU that new data is ready.

3. **Interrupt handler (IRQ context)**: Runs at high priority, cannot sleep. Copies packet to a software queue (typically per-CPU to avoid lock contention) and **schedules a softirq** (software interrupt) for deferred processing.

**Cost**: Interrupt entry/exit is expensive (~1 microsecond per interrupt on modern CPUs). High-speed NICs generate thousands of interrupts per second, each consuming CPU cycles.

### Softirq: NAPI (New API)

The Linux kernel uses **NAPI** (New API) to batch packet processing and reduce interrupt overhead.

```
HIGH-SPEED NIC ARRIVAL:
1. First packet → raise IRQ
2. Handler schedules softirq, disables further IRQs
3. Softirq processes many packets in a **poll loop**
4. When backlog clears → re-enable IRQs and return
```

**Benefit**: 100 Gbps NIC generating 25M packets/sec (if 40-byte minimum) would cause 25M interrupts→collapse. NAPI batches them into thousands of softirqs, each processing thousands of packets.

### Network Driver and sk_buff

The driver receives a packet and wraps it in a **sk_buff** (socket buffer) structure:

```c
struct sk_buff {
    struct net_device *dev;      // Input interface (eth0, etc.)
    unsigned char *data;         // Packet data pointer
    unsigned int len;            // Packet length
    struct timespec ts;          // Receive timestamp
    struct sock *sk;             // Associated socket (if any)
    // ... 100+ more fields
};
```

The sk_buff is the universal packet carrier. It holds metadata (protocol, interface, timestamps) alongside the actual packet bytes.

### Protocol Stack Processing: NIC → IP → Transport → Socket

1. **Device layer** (driver): Allocate sk_buff, DMA data, call `netif_rx()` or similar.

2. **IP layer** (ip_rcv)**: Parse IP header, check dest address. If not for this machine, drop or forward. De-fragment reassembled packets.

3. **Transport layer** (tcp_v4_rcv, udp_rcv)**: Parse TCP/UDP headers, lookup socket matching (daddr, dport, saddr, sport), enqueue to socket's receive queue.

4. **Socket layer** (user space)**: `recvfrom()` or similar system call reads from the socket queue.

**Cost per layer**: Each layer parses headers (cache misses), performs lookups (hash tables, more misses), and memory copies (from stack to user buffer on some code paths).

## Socket API: Abstractions and Semantics

The socket API presents network connections as file descriptors, unifying networking with the Unix I/O philosophy.

### Socket Types

**SOCK_STREAM (TCP)**: Reliable, in-order, connection-oriented. `send()`/`recv()` calls.

**SOCK_DGRAM (UDP)**: Unreliable, connectionless. `sendto()`/`recvfrom()` with addresses.

**SOCK_RAW**: Raw IP packets, for protocol implementations or low-level packet crafting.

### Receive Path

```c
int recvfrom(int sockfd, void *buf, size_t len,
             int flags, struct sockaddr *src_addr, socklen_t *addrlen);
```

System call path:
1. Kernel validates buffer address (user → kernel boundary check)
2. Retrieves socket from file descriptor table
3. Checks socket's receive queue
4. If empty and socket is non-blocking → return EAGAIN
5. If empty and blocking → sleep the process, wait for data
6. Copy data from sk_buff(s) to user buffer
7. Return bytes copied

**Cost**: System call entry/exit (~100-200 cycles), memory copy from kernel to user space (especially for large buffers), context switches on blocking calls.

### Socket Buffers and Backpressure

Each TCP socket has kernel-allocated **receive buffer** (SO_RCVBUF, default ~128 KB) and **send buffer** (SO_SNDBUF). If the receiver cannot keep up, the receive buffer fills and the sender blocks—automatic backpressure.

UDP has no flow control; packets are dropped if the kernel queue overflows.

## Kernel Bypass Techniques

Standard socket I/O involves multiple system calls, context switches, and memory copies. High-frequency trading, live video, and ultra-low-latency systems need faster paths.

### DPDK (Data Plane Development Kit)

**Kernel bypass**: Userspace application directly controls NIC (via UIO driver), bypassing the kernel stack entirely.

```
NIC ring buffer
    ↓ (direct DMA)
Userspace app (packets directly)
    ↑ (direct DMA)
NIC transmit ring
```

**Advantage**: Minimal latency (no syscalls, no context switches, no IP/TCP stack processing).

**Disadvantage**: 
- App must implement TCP/IP stack (or use DPDK's stack)
- Loses all kernel features (firewall rules, routing, multicast, etc.)
- Requires pinning packets to same CPU (NUMA-aware memory allocation)
- High development complexity

**Use case**: Financial markets, telecom packet processing, 5G infrastructure.

### XDP (eXpress Data Path) and eBPF

**Kernel-integrated**: Run eBPF (extended Berkeley Packet Filter) programs at the NIC driver level, before the standard IP/TCP stack.

```
NIC driver → [XDP eBPF program] → decision (DROP, REDIRECT, PASS)
                ↓ (if DROP)
            Discard packet (free DMA buffer)
                ↓ (if PASS)
            Continue normal IP/TCP processing
```

**Advantage**:
- Keeps kernel features (routing, firewall rules still work)
- Drop/redirect decisions made early (before expensive allocations)
- Moderate overhead vs kernel stack, much lower than standard socket I/O
- Deploy new logic without recompiling kernel

**Disadvantage**: eBPF program is restricted (limited verifier ensures safety), requires kernel 4.7+, complex debugging.

**Use case**: DDoS mitigation (drop bad traffic early), load balancing, traffic analytics.

### AF_XDP (Address Family XDP)

Userspace socket interface to XDP. Userspace app gets fast packet delivery without implementing a full network stack.

```c
struct xsk_socket *xsk;
while (keep_running) {
    rx_pkts = xsk_ring_cons__peek(&xsk->rx, 32, &idx);
    // Process packets
    xsk_ring_cons__release(&xsk->rx, rx_pkts);
}
```

**Advantage**: Lower latency than standard sockets, faster than full DPDK (kernel still manages resources, routing).

**Disadvantage**: Still below DPDK in raw throughput (kernel overhead), eBPF program complexity for packet filtering.

## io_uring Zero-Copy Networking

**io_uring** is a modern asynchronous I/O interface. Recent extensions add **zero-copy transmit** and **zero-copy receive**.

### Zero-Copy Transmit (MSG_ZEROCOPY Variant)

```c
struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
io_uring_prep_send_zc(sqe, sockfd, buf, len, MSG_NOSIGNAL);
io_uring_submit(&ring);
// Ring handles copying to NIC via DMA; buf can be reused immediately
```

Kernel does **not** copy data to internal kernel buffers; instead, DMA reads directly from user buffer. Network device reads from same buffer (with pinned pages), then returns completion to userspace.

**Advantage**: Eliminates memcpy overhead for outgoing data.

**Disadvantage**: Pages must be page-pinned (fixed in memory), limiting scalability. Only effective for large transfers (< 100 KB overhead is often less than DMA setup cost).

### Zero-Copy Receive (Proposed)

Similar concept: kernel provides pointers to packet buffers without copying data to user buffers. Still experimental; not yet widely deployed.

## Network Namespaces

Linux namespaces isolate kernel resources. **Network namespaces** partition the network stack: each namespace has its own network interfaces, routing table, firewall rules, and socket namespace.

### Namespace Isolation

```bash
# Create a new network namespace
ip netns add mynamespace

# Run a command in that namespace
ip netns exec mynamespace bash
# Inside: `ip link show` shows only lo (loopback)
```

**Use case**: Containers. Each Docker container gets its own network namespace (isolated from the host and other containers).

### Virtual Ethernet (veth)

Connect namespaces using **veth pairs** (virtual Ethernet cable):

```
Host namespace          Container namespace
    eth0  ←→ veth pair ←→  eth0
```

Container's eth0 is actually half of a veth pair connected to the host. Host sees traffic on the container side; container sees traffic on the host side.

### Network Bridge

Multiple containers connect to a host-side bridge (software switch):

```
        ┌─ Container 1 (veth1) ─→ eth0
        │
Host-side bridge → ┼─ Container 2 (veth2) ─→ eth0
        │
        └─ Container 3 (veth3) ─→ eth0
        ↓
    Host network (eth0)
```

Bridge acts as a switch, forwarding traffic between container veth pairs and host eth0.

## Socket Buffer Management and Backpressure

TCP sockets apply **automatic flow control**: if the receiver's kernel buffer is full, `send()` blocks or returns early, stalling the sender. This backpressure prevents memory exhaustion.

UDP has no flow control; the kernel drops packets when its UDP queue overflows. Applications must handle loss or use TCP.

## See Also

- [TCP/IP Protocol Suite](networking-tcp-ip.md) — protocol details (sequence numbers, congestion control)
- [I/O Models](os-io-models.md) — blocking, non-blocking, select, epoll, io_uring
- [Linux Namespaces](os-containers-internals.md) — container isolation in depth