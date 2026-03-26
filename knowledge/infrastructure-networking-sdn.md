# Software-Defined Networking (SDN) — Control Plane, OpenFlow & Network Virtualization

## Overview

Software-Defined Networking separates the **control plane** (decision logic: where packets go, which services to apply) from the **data plane** (packet forwarding: receiving, modifying, transmitting frames). Traditional network equipment embeds both in hardware; SDN centralizes control in software, enabling programmatic network management and rapid policy changes without replacing physical hardware.

## Traditional Network Architecture vs. SDN

### Traditional ("Distributed") Networking

Each network device (router, switch) independently contains:

- **Data Plane**: ASIC (Application-Specific IC) hardware that forwards packets by matching destination MAC/IP and consulting local forwarding tables.
- **Control Plane**: Proprietary firmware running routing protocols (OSPF, BGP) to discover neighbors, calculate optimal paths, and update forwarding tables.
- **Management Plane**: CLI, SNMP, proprietary APIs for operator intervention.

**Consequences**:

- Configuration is device-local; policies are scattered across hundreds of devices.
- Vendor lock-in: each vendor's firmware is proprietary; switching vendors requires relearning CLI and configuration.
- Slow adaptation: changing network policy requires manual CLI replication or waiting for dynamic routing convergence (seconds-to-minutes).
- Difficult troubleshooting: state is distributed; visibility into traffic flows requires packet sniffing on each device.

### SDN Architecture

Control logic centralizes in **SDN Controllers** (software programs running on dedicated or virtualized servers). Controllers interact with **SDN Switches** (hardware forwarding engines) via a standardized southbound protocol (e.g., OpenFlow).

```
┌─────────────────────────────────────┐
│  SDN Controller                     │
│  (Routing logic, policy, analytics) │
└─────────────────────────────────────┘
       ↑ OpenFlow / Protocol
       ↓
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ SDN Switch 1 │  │ SDN Switch 2 │  │ SDN Switch 3 │
│ (Forwarding) │  │ (Forwarding) │  │ (Forwarding) │
└──────────────┘  └──────────────┘  └──────────────┘
```

**Advantages**:

- **Centralized Policy**: All control logic in one place; policies are code (reproducible, version-controlled).
- **Vendor Portability**: If switches support OpenFlow, any SDN controller works with any vendor's switch.
- **Reprogrammability**: Network behavior can change in milliseconds (code update + controller restart) without touching hardware.
- **Visibility**: Controller has global visibility; can log/analyze all traffic patterns.

## OpenFlow Protocol

OpenFlow is the most widely-adopted southbound protocol (switch-to-controller communication). It defines:

### Flow Table and Matching

Each switch maintains **flow tables**. Each flow is a rule:

```
Match: if (destination IP == 10.0.1.5 and protocol == TCP)
  Action: forward to port 2
          apply QoS marker "low-priority"
          count packets
```

Packets are matched against rules in priority order. On match, the associated actions execute (forward, drop, send to controller, modify headers).

### Packet-in and Flow Insertion

When a packet arrives and *doesn't* match any flow:

1. Switch sends **packet-in message** to controller with packet header (first 128 bytes) and arrival port.
2. Controller makes forwarding decision (consult topology, apply policy).
3. Controller sends **flow-mod message** to install matching rule on the switch.
4. Subsequent packets matching this flow forward locally on the switch without controller involvement.

First packet to a flow involves controller latency (~10–100ms typical); subsequent packets forward at line-rate.

### Explicit Congestion Notification and Meter Tables

Beyond routing, OpenFlow supports:

- **Meter Tables**: Rate limiting per flow or priority. Discard or mark packets exceeding rate.
- **Group Tables**: Replication (multicast), fast failover (switch to backup on primary failure).
- **Group Actions**: Load-balanced forwarding across multiple ports or high-availability endpoints.

## Network Overlays and Encapsulation

SDN switches forward based on controller-installed rules, but physical topology might not match logical requirements. **Network overlays** solve this by encapsulating traffic:

### VXLAN (Virtual Extensible LAN)

VXLAN encapsulates Ethernet frames inside UDP packets:

```
Original Frame: [Ethernet: src_MAC, dst_MAC][IP: src_IP, dst_IP][TCP]
    ↓ Encapsulation (inside UDP)
New Packet: [Ethernet: underlay_src, underlay_dst][IP: underlay_src_IP, underlay_dst_IP][UDP 4789][VXLAN Header with VNI][Original Frame]
```

**VNI (VXLAN Network Identifier)**:  24-bit namespace; allows 16 million virtual networks on the same physical infrastructure.

**Use Case**: Multi-tenant cloud. Tenant A's VM using IP 10.0.0.1 and Tenant B's VM also using 10.0.0.1 don't conflict; each tenant's traffic is encapsulated with a different VNI.

### Geneve (Generic Network Virtualization Encapsulation)

Similar to VXLAN but extensible: supports optional TLV (Type-Length-Value) fields for metadata (encryption details, service chain info, source region).

### VLAN Limitations

VLANs use a 12-bit tag (4096 VLANs max) and require spanning-tree algorithms limiting topologies. Overlays are superior for datacenter scale (millions of virtual networks, arbitrary topologies).

## Network Virtualization and Intent-Based Approaches

### Classical Network Virtualization

Create isolated virtual networks on a shared physical substrate. Each virtual network has:

- Virtual routers and switches (implemented as software)
- Isolated routing tables, forwarding state
- Traffic entirely encapsulated/segregated from other tenants

Typically implemented via:

- **Hypervisor Networking**: Each VM connects to virtual switch (vSwitch) in hypervisor; vSwitch carries traffic to physical network via overlay.
- **Network Function Virtualization (NFV)**: Packet processing (firewalls, load balancers, intrusion detection) implemented as software running on general-purpose hardware, not dedicated appliances.

### Intent-Based Networking

Traditional SDN requires operator to code policies ("if destination is 10.0.1.5, drop"); intent-based approaches abstract this:

```
Intent: "Allow developers to reach the database"
  ↓ Intent Engine
Translated to firewall rules, ACLs, routing entries, load balancer config
  ↓ Applied to network devices
```

Operators declare business intent; the system translates to device configurations. Advantages:

- Operator doesn't debug low-level rules.
- Policy is portable across network changes (adding switches doesn't require rewriting rules).
- Compliance auditing is simplified (trace intent to applied rules).

Current state: Emerging; most SDN deployments still require explicit rule coding.

## Container Networking Interface (CNI) Plugins

Kubernetes and container orchestrators need to wire containers into networks. CNI is the standard plugin interface for container networking:

### Plugin Requirements

Each CNI plugin handles:

1. **ADD**: Allocate IP, attach container to network, configure routing.
2. **DEL**: Release IP, disconnect container.
3. **CHECK**: Verify connectivity state.

### Popular Implementations

**Flannel**: Simplest CNI; provides pod-to-pod connectivity via VXLAN or direct routing. Suitable for basic Kubernetes networking; limited policy.

**Calico**: Policy-rich; uses eBPF (extended Berkeley Packet Filter) to enforce microsegmentation. Each pod gets individual IP; policies are Linux firewall rules compiled to eBPF.

**Cilium**: Also eBPF-based but with strong service mesh integration (Envoy proxy aware) and advanced visibility. Fewest latency overhead vs. traditional iptables.

All expose similar APIs (allocate IP, configure routes) but differ in enforcement mechanism (iptables, eBPF, VXLAN routing).

## eBPF for Networking

eBPF (extended Berkeley Packet Filter) is a lightweight VM inside the Linux kernel. Programs written in C (or compiled to eBPF bytecode) attach to kernel hooks and execute on packet arrival, before userspace sees the packet.

### Advantages

- **Kernel-Level Enforcement**: Policies apply in kernel space; no context switch to userspace (fast, low latency).
- **Observability**: Attach probes to track packet flow, function calls, without modifying application code.
- **Programmability**: Not limited to predefined rules; arbitrary logic in eBPF can implement any forwarding or filtering policy.

### Networking Use Cases

- **Packet Filtering**: Match packets, apply actions (drop, forward, modify) without entering userspace networking stack.
- **Load Balancing**: Rewrite packet headers (source IP, port) at kernel level; significantly faster than userspace proxies.
- **Service Mesh Data Plane**: Cilium uses eBPF to enforce service mesh policies (mTLS, rate limits, circuit breakers) without Envoy proxy sidecar overhead.

Trade-off: eBPF programs must be safe (kernel forbids unbounded loops, memory access outside traced regions), limiting complexity vs. userspace.

## Service Mesh Data Plane

Service meshes (Istio, Linkerd, Cilium) provide app-level policies (mTLS, circuit breakers, traffic splitting) in the data plane. They're SDN concepts applied to application networking:

- **Control Plane**: Centralized (Istiod, Linkerd control plane) managing policy distribution.
- **Data Plane**: Envoy proxies (or eBPF in Cilium) sidecar each workload, enforcing policies on outbound traffic.

Service mesh replicates SDN themes: centralized policy distribution, programmatic network management, and decoupling from underlying infrastructure.

## SDN in Production: Trade-offs and Adoption Barriers

### Advantages (Realized)

- **Programmability**: Network behavior changes in minutes, not days.
- **Scalability**: One controller manages thousands of switches; policies scale logically, not per-device.
- **Vendor Openness**: OpenFlow equipment from multiple vendors interoperate.

### Challenges

- **Controller Availability**: Single controller is a single point of failure; clusters of controllers introduce state synchronization complexity.
- **Latency**: First packet to a new flow requires controller round-trip (typically acceptable, but gaming/real-time apps notice).
- **Debugging**: Network failures now include controller bugs; visibility is harder than reading routing tables.
- **Operator Retraining**: Operators trained on CLI must learn programming and data plane mechanics.

### Adoption Status

- **Enterprise**: Gradual adoption for datacenters; brownfield networks mix traditional + SDN.
- **Hyperscalers**: Heavy SDN use. Google's internally-developed SDN controls their backbone; AWS uses SDN-like abstractions beneath VPC.
- **Enterprises / Branches**: Adoption slower; most still traditional.
- **Kubernetes / Container Networking**: SDN (via CNI) is the assumed model; traditional networking is legacy.

See also: containerization-orchestration, cloud-aws-networking, infrastructure-load-balancing, security-network.