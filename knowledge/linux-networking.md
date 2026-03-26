# Linux Networking — netfilter, iptables, nftables, Namespaces, Traffic Control & eBPF

## Overview

Linux networking provides multiple layers of control over packet flow, from low-level kernel packet filtering (netfilter/iptables/nftables) to virtual networking abstractions (network namespaces, veth, bridges, VLANs). Modern Linux systems combine kernel-space policy (firewall rules, routing) with user-space tools for traffic shaping (tc), performance observation, and emerging eBPF-based packet processing (XDP). Understanding these layers is essential for container networking, service mesh implementation, traffic engineering, and kernel-level optimization.

## Netfilter and Packet Filtering

### Netfilter Architecture

The Linux **netfilter** subsystem is a framework for packet processing inside the kernel. It defines **hooks** at key points in the network stack where code can intercept and process packets:

- **NF_IP_PRE_ROUTING**: Before routing decisions
- **NF_IP_LOCAL_IN**: For packets destined to local processes
- **NF_IP_FORWARD**: For packets to be forwarded
- **NF_IP_LOCAL_OUT**: For packets originating locally
- **NF_IP_POST_ROUTING**: After routing decisions

Modules can register handlers at these hooks, executing code to accept, drop, or modify packets. This design allows multiple subsystems (firewall, NAT, connection tracking, etc.) to coexist without reimplementing packet processing.

### iptables: Tables, Chains, and Rules

**iptables** is a userland tool that loads rules into netfilter tables residing in kernel memory. It defines several logical tables:

- **filter**: Default table for deciding accept/drop/forward.
- **nat**: Network address translation (source/destination IP/port rewriting).
- **mangle**: Modify packet headers (TTL, DSCP, mark).
- **raw**: Pre-connection tracking rules (bypass conntrack).
- **security**: SELinux context rules.

Each table contains **chains** (sequences of rules). Built-in chains correspond to netfilter hooks. Traversal is depth-first; a packet matches the first rule and stops (unless the rule action is performed and traversal continues—depends on rule policy: ACCEPT, DROP, REJECT, or user-defined chain).

**Example rule:**
```
iptables -t filter -A INPUT -p tcp --dport 80 -j ACCEPT
```
Appends a rule to the filter table's INPUT chain: if protocol is TCP and destination port is 80, jump (accept) the packet.

**Performance cost**: Each rule is evaluated sequentially. A chain with 1000 rules evaluates all rules in the worst case. Kernel uses linear search; for high-volume filtering, nftables (below) is preferred due to set-based operations.

### nftables: Modern Replacement

**nftables** (available in Linux 3.13+, replaces iptables) uses a different architecture:

- **Single unified table model**: No separate filter/nat/mangle; all rules exist in one namespace.
- **Hierarchical structure**: Rules are organized in **tables** → **chains** → **rules**.
- **Set and map operations**: Can check IP membership in a set in O(log n), instead of sequential rule matching.
- **User-friendly syntax**: More expressive language with variables, functions, and flow control.
- **Dynamic reload**: `nft` command reloads rules atomically without flushing existing connections.

**Example:**
```
nft add table inet filter
nft add chain inet filter input { type filter hook input priority 0 ; }
nft add rule inet filter input tcp dport 80 accept
```

**Advantages over iptables:**
- Set membership checks are much faster (e.g., "is this IP in a deny list?")
- Single atomic transaction: load multiple rules at once (no brief window of inconsistency)
- More readable syntax

**Disadvantage:**
- Smaller tool ecosystem; many existing scripts use iptables

nftables is the recommended path forward for new systems; iptables remains widely used in existing infrastructure and containers.

## Connection Tracking (conntrack)

Both iptables and nftables rely on **connection tracking (conntrack)** to recognize stateful connections.

**Mechanics:** The kernel maintains a hash table of active connections, indexed by (src_ip, src_port, dst_ip, dst_port, protocol). Each entry records the connection state (NEW, ESTABLISHED, RELATED) and associated data (TCP flags, timeout).

When a packet arrives:
1. If it matches an existing connection → state is ESTABLISHED
2. If it's the first packet of a connection → state is NEW
3. Related packets (e.g., ICMP errors) → state is RELATED

Rules can match on state: `iptables -A INPUT -m state --state ESTABLISHED -j ACCEPT` allows replies to outgoing connections. This creates a "stateful firewall" effect without explicitly listing all return paths.

**Cost**: Conntrack hash table grows with the number of active connections. Under DDoS or high concurrency, conntrack can become a bottleneck. Tuning: increase hash table size via sysctl `net.netfilter.nf_conntrack_max`.

## Network Namespaces and Virtual Networking Primitives

### Network Namespaces

A **network namespace** isolates the network stack: each namespace has its own IP addresses, routing table, firewall rules (separate netfilter hooks), and sockets. A process in one namespace cannot directly communicate with a process in another; they must be connected by virtual interfaces or the root namespace.

**Creation:**
```bash
ip netns add ns1
ip netns exec ns1 ip link set lo up
ip netns exec ns1 bash
```

**Use cases:**
- Container isolation (each container lives in a namespace)
- Virtualization and machine emulation
- Testing and sandboxing

### veth Pairs

A **virtual ethernet pair (veth)** is two-end virtual NIC: packets sent on one end appear on the other. Used to connect namespaces:

```bash
ip link add veth0 type veth peer name veth1
ip link set veth1 netns ns1
# Now veth1 is inside ns1; send packets from host (veth0) → appears in ns1 (veth1)
```

Both ends are independent devices; they can have different IP addresses and participate in different subnets/routing domains. In containers, every container has a veth connected to the host bridge; the other end sits in a bridge on the host namespace.

### Bridge

A **bridge** is a virtual L2 switch. It forwards packets between multiple network interfaces based on MAC addresses. Interfaces attached to a bridge receive and relay traffic:

```bash
ip link add br0 type bridge
ip link set eth0 master br0
ip link set veth0 master br0
```

Now `eth0` and `veth0` are in the same broadcast domain; packets sent on one appear on the other (if src MAC is unknown to the bridge, it floods).

**Use in containers**: Docker creates a default `docker0` bridge and attaches each container's veth to it. Containers connected to the same bridge can reach each other at L2.

### VLAN (Virtual LAN)

A **VLAN** is a logical segmentation of traffic on a physical link using 802.1Q tags. A tagged frame includes a 12-bit VLAN ID; switches route tagged frames only to ports in that VLAN.

**On Linux:**
```bash
ip link add vlan100 link eth0 type vlan id 100
ip addr add 10.0.100.1/24 dev vlan100
ip link set vlan100 up
```

Now `vlan100` is a virtual interface that sends/receives frames with tag 100 on the physical `eth0` link. The kernel automatically tags/untags frames.

**Use**: Segmentation of traffic without physical cables; a single physical NIC can carry multiple logical networks.

## Traffic Control (tc)

**tc (Traffic Control)** is part of iproute2 and allows shaping, rate limiting, and prioritizing traffic on a per-interface or per-flow basis. It operates at the queueing discipline (qdisc) layer.

### Queueing Disciplines (qdisc)

When a network driver sends packets onto the network, packets are queued. A **qdisc** is a packet scheduler that decides which packets to transmit and in what order. Linux provides multiple qdisc types:

- **pfifo_fast**: Default FIFO (first-in, first-out) with three priority bands.
- **htb (Hierarchical Token Bucket)**: Hierarchical rate limiting; useful for per-class shaping.
- **fq_codel (Fair Queue CoDel)**: Per-flow fair queuing with congestion control; modern choice.
- **tbf (Token Bucket Filter)**: Simple rate limiting.

**Example: Rate limit to 100 Mbps:**
```bash
tc qdisc replace dev eth0 root tbf rate 100mbit burst 32kbit latency 400ms
```

**Use cases:**
- Prevent one application from hogging the network
- Prioritize traffic (voice over bulk data)
- Test network conditions (latency, packet loss)

### Connection Marking and Policy Routing

Traffic control often combines with **marking** (iptables mangle table) and **policy routing** (ip rule) to apply different policies based on connection properties:

1. Mark packets in iptables: `iptables -t mangle -A POSTROUTING -p tcp --dport 80 -j MARK --set-mark 1`
2. Create a qdisc tied to mark: `tc filter add dev eth0 protocol ip parent 1:0 prio 1 handle 1 fw classid 1:10`
3. Create a class with specific rate: `tc class add dev eth0 parent 1: classid 1:10 htb rate 10mbit`

## eBPF Networking and XDP

### eBPF (Extended Berkeley Packet Filter)

**eBPF** is a restricted bytecode VM running in the kernel. Originally for packet filtering (Berkeley Packet Filter), it evolved to support general-purpose kernel instrumentation. Programs are written in C (subset of C), compiled to eBPF bytecode, JIT-compiled to native code, and run in the kernel with sandboxing.

**Advantages:**
- Zero-copy: processes packets without copying to userspace
- Low latency: runs at the instant packet arrives
- Dynamic loading: insert/remove programs without recompiling kernel
- Safety: verifier ensures program terminates and doesn't corrupt kernel memory

### XDP (eXpress Data Path)

**XDP** is an eBPF hook triggered immediately upon packet arrival, before any netfilter or qdisc processing. An XDP program can:
- **XDP_DROP**: Discard packet immediately (fast firewall)
- **XDP_PASS**: Forward to normal stack (Linux processing)
- **XDP_REDIRECT**: Send packet to another interface or userspace ring buffer

**Use:**
- DDoS mitigation: drop suspicious packets at line rate before conntrack overhead
- Load balancing: redirect based on flow 5-tuple
- Packet processing: modify headers, decapsulate tunnels

**Example (pseudocode):**
```c
int xdp_drop_port(struct xdp_md *ctx) {
    void *data = (void *)(long)ctx->data;
    struct ethhdr *eth = data;
    if (eth->h_proto != htons(ETH_P_IP)) return XDP_PASS;
    
    struct iphdr *ip = data + sizeof(*eth);
    if (ip->protocol == IPPROTO_TCP) {
        struct tcphdr *tcp = (void *)ip + (ip->ihl << 2);
        if (tcp->dest == htons(666)) return XDP_DROP;  // Drop port 666
    }
    return XDP_PASS;
}
```

Load with: `ip link set dev eth0 xdp obj program.o sec xdp`

## Socket Options and TCP Tuning

### Socket Options (setsockopt)

Applications can tune socket behavior via `setsockopt()`:

- **SO_REUSEADDR**: Allow binding to a port in TIME_WAIT state (avoids "Address already in use" errors on restart)
- **SO_REUSEPORT**: Multiple processes can bind to the same (IP, port) pair; kernel load-balances incoming connections. Used in load balancers and multi-process servers.
- **SO_KEEPALIVE**: Enable TCP keep-alives (periodic probes to detect dead connections)
- **SO_SNDBUF / SO_RCVBUF**: Buffer sizes for send/receive
- **TCP_NODELAY**: Disable Nagle's algorithm (send small packets immediately instead of waiting to coalesce)
- **TCP_WINDOW_CLAMP**: Limit TCP window size (can degrade throughput if misconfigured)

### TCP Tuning via sysctl

Kernel parameters (readable/writable via `/proc/sys/net/ipv4/` or `sysctl` command) control TCP stack behavior:

- **net.ipv4.tcp_max_syn_backlog**: Size of SYN queue (limits SYN flood impact)
- **net.ipv4.tcp_tw_reuse**: Reuse TIME_WAIT connections for outgoing (efficiency, but can cause port conflicts)
- **net.core.somaxconn**: Max listen backlog size
- **net.ipv4.tcp_keepalive_time / interval / probes**: Tune keep-alive timers
- **net.ipv4.ip_local_port_range**: Range of ephemeral ports available for client connections
- **net.ipv4.tcp_timestamps**: Enable TCP timestamps (improves PAWS / Protect Against Wrapped Sequences)
- **net.ipv4.netfilter.nf_conntrack_max**: Max connection tracking entries

Tuning is often workload-specific. A high-concurrency web service may want high SYN backlog and adjusted window scaling. A batch processing system may prioritize throughput over latency.

## See Also

- **infrastructure-container-networking.md** — Bridge networking and overlay networks
- **os-networking-stack.md** — Kernel packet flow and zero-copy optimizations
- **performance-profiling.md** — Measuring network performance
- **security-network.md** — Firewall and network defense strategies