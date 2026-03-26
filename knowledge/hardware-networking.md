# Network Hardware

Network hardware encompasses the physical components moving data across systems, from packet processing in line-rate switches to protocol offloading on Smart NICs.

## Network Interface Cards (NICs)

**Basic Architecture** — controller + DRAM buffer + TX/RX engines + physical layer transceiver. Connected to host CPU via PCIe (Gen3: 16 GB/s; Gen4: 32 GB/s; Gen5: 64 GB/s).

**TCP/IP Offloading** — modern NICs offload expensive operations to hardware:
- **TSO (TCP Segmentation Offload)**: CPU writes large packet to NIC; NIC segments into MSS-sized packets and adds TCP/IP headers
- **RSC (Receive Side Coalescing)**: NIC reassembles fragmented packets before delivering to OS
- **Checksum offload**: hardware computes IP/TCP/UDP checksums
- **VLAN tagging**

**Receive Side Scaling (RSS)** — distributes incoming packets across multiple CPU cores using hash (src IP, dst IP, src port, dst port) to select queue. Improves cache locality and parallelism; requires careful tuning to avoid reordering (application sees packets OUT OF ORDER if using RSS unwisely).

**eXpress Data Path (XDP)** — eBPF bytecode allows in-kernel/in-NIC packet processing, bypassing full kernel stack. Enables high-performance packet filtering, load balancing, or DOS mitigation at line rate (~100 Gbps with modern NICs).

## Switches and Routers

**L2 Switching (Ethernet)** — forwarding based on MAC destination. Internal fabric typically crossbar or buffered mesh. Line-rate forwarding on fixed-size switching fabric; buffer (DRAM or SRAM) queues packets during contention. Queue length and buffer management (tail-drop, RED) control latency/loss trade-off.

**L3 Routing (IP)** — forwarding based on destination IP, longest-prefix match lookup in routing table. Traditionally table in DRAM (high latency); modern designs use SRAM-based Ternary CAM (TCAM, ~100 ns lookup) at cost of capacity (~1M routes).

**ASIC vs Software** — line-rate switching (100+ Gbps) requires ASICs; software routers (Linux networking stack) typically ~1-10 Gbps. Performance gap driven by parallelism and pipeline depth in hardware.

**Switch Architecture** — input buffering (head-of-line blocking), output buffering (internal contention serializes at output), or virtual output queuing (per-(input,output) queue pair, no HOL blocking but more complex). Deployment varies; internal fabric often oversubscribed (120% traffic rated capacity) to amortize cost.

## Load Balancers

**Hardware LB** — dedicated ASIC forwarding at line rate. Typically stateful: track TCP flows, rewrite packet headers (SNAT/DNAT), balance across multiple backend servers. Modern deployments: 100 Gbps-400 Gbps line rate, sub-microsecond latency.

**Software LB** — kernel-space (Linux iptables/netfilter) or userspace (QUICHE, DPDK). Lower throughput (~10-50 Gbps) but more flexible (custom protocols, complex policies). Deploy in scale-out fashion (N load balancers, consistent hashing distributes flows).

**Algorithms**: round-robin, least-connections, weighted, source-IP hash, or layer-7 (inspect HTTP Host header or request URL for routing).

## SmartNICs and Data Processing Units (DPUs)

**SmartNIC** — NIC with embedded CPU (ARM or custom), enabling complex offloads beyond basic TCP/IP. Examples: Mellanox (NVIDIA) BlueField, Xilinx Alveo.

**Use Cases** — encryption/TLS offload (move crypto operations off host CPU), firewall (stateful packet inspection), virtualization (vSwitch acceleration, nested virtualization support).

**DPU** — generalization of SmartNIC with more compute, memory, and storage. Positioned as infrastructure processor in data centers: handles security, virtualization, networking, and storage operations, freeing the main CPU for application workloads.

## Physical Layer and Interfaces

**Speeds** — evolution: 1 Gbps (2000s) → 10 GbE (2006) → 25 GbE (2014) → 40 GbE / 50 GbE (2016) → 100 GbE (2018) → 200/400 GbE (2020s). Driven by Moore's law and datacenter economics (cost/performance per Gbps); generational jump every 3-4 years.

**RDMA (Remote Direct Memory Access)** — allows NIC to read/write remote memory directly, bypassing remote CPU. Latencies ~1-5 µs (vs TCP/IP ~100 µs). Complex protocol; adoption limited to HPC and some datacenter interconnects (not mainstream). InfiniBand or RoCE (RDMA over Converged Ethernet) encapsulations.

## Network Topologies

**Spine-Leaf** — recommended for datacenters. Leaf switches at ToR (Top of Rack), each connected to all spines. Provides 1:1 oversubscription and enables efficient load-balancing across spines. Replaces hierarchical tree topology (core-aggregation-access) which had bandwidth bottlenecks at aggregation layer.

**Fat-Tree** — oversubscribed tree with multiple uplinks from lower levels to spread traffic. Enables greater throughput than single uplink; tradeoff: higher latency for non-local traffic as packets may take longer paths through tree.

**Packet Scheduling** — output port contention resolved by queuing algorithms: FIFO (simple, unfair), Priority Queuing (high-priority traffic first, starves low-priority), Weighted Fair Queuing (WFQ, per-flow fairness), or Deficit Round Robin (simpler to implement than WFQ).

**Congestion Control** — detect overloads (queue depth, ECN bits in IP header) and throttle sources. TCP congestion control (AIMD: additive increase, multiplicative decrease) common in endpoints. Active Queue Management (random early detection, drop/mark packets early to signal congestion before buffer fills).

**Buffer Requirements** — switch fabric throughput N × line rate; total buffer capacity (DRAM, SRAM) determines how long packets can queue during contention. Undersized buffers lose packets; oversized buffers add latency. Buffering equations (e.g., Gupta et al.) estimate required capacity given traffic patterns.

**Software-Defined Networking (SDN)** — decouples control plane (routing decisions via central controller) from data plane (switch forwarding). OpenFlow protocol (1.0 through 1.6) defines communication. Enables dynamic routing, traffic engineering, and policy enforcement; introduces controller latency and availability concerns.

