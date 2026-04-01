# Container Networking — Bridge, Overlay Networks, CNI Plugins & L7 Observability

## Overview

Container networking bridges the gap between isolated containers and a distributed system where services communicate across hosts. A container is a namespace-isolated process; by default, it has its own network namespace (IP stack, routing table, firewall rules). The networking layer connects containers on the same host via virtual bridges, connects containers across hosts via overlay networks, and enforces network policies. This note covers the mechanics of container networking, the Container Networking Interface (CNI) standard, and implementations like Calico, Cilium, and Flannel.

## Container Network Namespaces & veth Pairs

### Network Namespaces

Each container has its own Linux network namespace, providing an isolated IP stack: IP addresses, routing table, firewall (iptables), and sockets. Two containers with the same namespace are isolated from the host and each other's network traffic.

**Mechanics:** When Docker or containerd creates a container, it calls `unshare(CLONE_NEWNET)` to allocate a new network namespace. The container process (PID 1) runs inside this namespace. The kernel maintains namespace metadata; when the last process in a namespace exits, the namespace is destroyed.

### veth Pairs (Virtual Ethernet Devices)

To connect a container to the host network, the container runtime creates a **virtual ethernet pair (veth)**:
- One end (inside the namespace) appears as `eth0` inside the container
- Other end (on the host) is attached to a bridge and has a name like `veth1234abcd@if2`

When the container sends a packet through `eth0`, the kernel routes it to the other end of the veth pair on the host. Platform-specific tools (Docker daemon, containerd, CRI implementation) manage veth creation and attachment.

## Bridge Networking

### Host Bridge Mode

**Mechanics:** Container shares the host's network namespace entirely. All containers and the host share the same IP stack, routing table, and firewall rules.

**Use:**
- Simple inter-container communication (no extra hops)
- Debugging (direct host tooling available)
- Single-host deployments

**Trade-Offs:**
- No isolation; compromised container can sniff all network traffic on the host
- Port conflicts; two containers can't bind the same port
- Not portable across hosts; requires manual routing configuration

### Container Bridge Mode (Docker Bridge)

**Mechanics:** The host creates a virtual bridge (e.g., `docker0`). Each container's veth pair connects to this bridge. Containers on the same bridge communicate via L2 switching (bridge-based). Communication between containers on different hosts requires port mapping or external routing.

**Example:** Container A (`172.17.0.2:8080`) and Container B (`172.17.0.3:3306`) both connected to the `docker0` bridge (172.17.0.0/16). Container A sends a packet to B; the bridge forwards it directly (L2 switching). No routing table involved; the kernel's bridge module handles it.

**DNS:** Docker embeds a DNS resolver in the `docker-proxy` that translates container names to IPs. Containers can resolve each other by name within the same bridge.

**Single-Host Limitation:** Containers on different hosts have separate bridges, isolated from each other. To connect containers across hosts, you must use port mapping on the host IP and have clients route to the host.

**Trade-Offs:**
- Simple, no external infrastructure
- Container-to-container communication is efficient (bridge is in-kernel)
- Limited to single host; no native multi-host communication
- Not suitable for large scale deployments

## Overlay Networks

### VXLAN (Virtual Extensible LAN)

**Concept:** Tunnel packets between hosts using VXLAN encapsulation. Containers in different data centers or availability zones can appear to be on the same network even though they're on different physical subnets.

**Mechanics:**
1. Container A (host 1, 10.0.0.2) sends packet to Container B (host 2, 10.0.0.3)
2. Both are on VXLAN network 10.0.0.0/16
3. Host 1's VXLAN driver intercepts the packet and encapsulates it:
   - Inner: Original packet (src: 10.0.0.2, dst: 10.0.0.3)
   - Outer (VXLAN header): Identifies the VXLAN ID (24-bit VNID)
   - Outer (UDP): Outer packet (src: host1-IP:4789, dst: host2-IP:4789)
4. Packet travels across the data center network as a regular UDP packet
5. Host 2 receives the UDP packet, extracts the inner packet, and delivers to Container B

**Advantages:**
- True multi-host networking; containers are genuinely on the same overlay subnet
- Scales to many hosts; VXLAN supports up to 16 million virtual networks (24-bit VNID)
- Works across data centers if UDP 4789 is allowed between hosts
- Preserves container-to-container communication semantics (same as bridge mode)

**Disadvantages:**
- Encapsulation overhead; every packet is wrapped in outer VXLAN headers, increasing MTU and reducing effective bandwidth
- Broadcast/ARP traffic is also tunneled; inefficient for protocols relying on broadcast
- Requires multicast-capable network (some cloud providers disable multicast by default)
- Debugging is complex; tcpdump on the host shows encapsulated packets, not the original traffic

**Implementations:** Flannel (with VXLAN backend), Docker Swarm, Weave.

### Macvlan & Ipvlan

**Macvlan:** Allocates a unique MAC address to each container and attaches it to the host's physical interface. From the container's perspective, it's on the same L2 network as the host. From external hosts' perspective, the container is a first-class network peer (same as the host).

**Mechanics:**
- Container gets a MAC address and IP address from the same subnet as the host (e.g., 192.168.1.0/24)
- External DHCP server (router) sees the container as a peer and assigns IP
- Container does not go through the bridge; it directly uses the host's NIC in promiscuous mode

**When to Use:**
- Need container to be on the same L2 network as non-containerized hosts (hybrid infrastructure)
- Existing network has flat subnet model; DHCP server is external
- Container needs direct hardware access

**When NOT to Use:**
- Microservices running entirely in containers (typical Kubernetes case)
- NICs don't support promiscuous mode or macvlan isn't available
- Complex switching logic required (use CNI plugins instead)

**Ipvlan:** Similar to macvlan but shares a single MAC address across multiple container IPs. Used for IP-centric environments where MAC address sharing is acceptable.

## Container Networking Interface (CNI) Standard

CNI is a specification for container runtime-agnostic networking plugins. Instead of hardcoding networking logic into runtimes, CNI allows pluggable implementations.

### Mechanics

1. **Runtime invocation:** When container is created, the container runtime calls a CNI plugin binary with arguments:
   - Container network namespace ID
   - Interface name
   - Network configuration (IPAM settings, gateway, DNS)
   - Command: "ADD" (connect container to network) or "DEL" (disconnect)

2. **Plugin responsibility:**
   - Create veth pairs or other interfaces
   - Assign IP addresses (from IPAM)
   - Configure routes and iptables rules
   - Return IP address and DNS settings to the runtime

3. **Runtime applies the response:** Runtime places the returned IP/DNS into the container's namespace.

### Benefits

- **Standardization:** Multiple CNI plugins can work interchangeably
- **Composability:** Chaining plugins (meta-plugins); primary plugin + bandwidth QoS plugin on top
- **Portability:** Same container image runs on Docker (with CNI bridge plugin), Kubernetes (with any CNI plugin), containerd, etc.

### Popular CNI Plugins

**bridge (reference implementation):** Simple Linux bridge; single-host networking. Used in Docker, basic containerd setups.

**ptp (point-to-point):** Direct veth connection between container and host; no bridge. Low latency, single connection.

**ipvlan:** Allocates IP addresses in the same subnet as the host; no bridge. Used for flat-network topologies.

**host-local:** Simple IPAM (assigns IPs from a supplied range) without external DHCP. Used for lab/testing.

## Calico (Tigera)

Calico is a production-grade networking and network policy engine designed for large-scale Kubernetes clusters and on-premises environments.

### Calico's Core Design

**BGP-Based Routing:** Instead of overlay networking (VXLAN/tunnels), Calico uses Border Gateway Protocol (BGP) to advertise container subnets from each host to the rest of the network. The underlying network fabric becomes the overlay; no encapsulation overhead.

**Mechanics:**
1. Each host runs BIRD (BGP daemon) which announces the container subnet hosted on that node (e.g., node1 announces 10.0.1.0/24, node2 announces 10.0.2.0/24)
2. Network switches/routers learn these routes via BGP
3. When a packet destined for 10.0.2.5 arrives at node1, the kernel routing table (populated by BGP) forwards it directly to node2
4. Node2 forwards the packet to the container via a bridge or veth

**When This Works:**
- Network switches support BGP (enterprise data center, cloud providers with BGP support)
- Cluster is within a single AS (autonomous system) or small number of ASes
- Operator is comfortable with BGP operations

**When This Doesn't Work:**
- Network doesn't support BGP (small offices, restricted cloud environments)
- Many small subnets + complex AS topology becomes operational burden
- Cloud provider (AWS, GCP) doesn't expose BGP to users

### Network Policies

Calico's killer feature is **NetworkPolicy**: fine-grained, Kubernetes-native network access control.

**Example:** Deny all ingress except from namespaces labeled `tier=frontend`:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-frontend
spec:
  podSelector:
    matchLabels:
      tier: backend
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          tier: frontend
```

Calico's **policy engine** (via Felix, the host agent) translates this into iptables/IPVS rules on each host. Flow denied if no matching ingress rule.

### Strengths

- **No Tunnel Overhead:** BGP routing means native network performance; no VXLAN encapsulation
- **Network Policies:** Industry-leading network access control; integrates with Kubernetes RBAC
- **Multi-Cloud:** Works on-premises, AWS (with restricted BGP), Kubernetes clusters
- **Scaling:** Proven at massive scale (100k+ nodes)
- **Active Development:** Feature-rich, good support, open-source

### Weaknesses

- **BGP Operational Requirement:** Team must understand BGP; not suitable for networks without BGP expertise
- **No Cross-Subnet Overlay:** If you need to span multiple network subnets without routing infrastructure, VXLAN is better
- **Overhead on VMs:** When hosts are VMs (not bare metal), BGP daemon adds overhead; overlay networks simpler for VMs

### Use Cases

Large Kubernetes clusters in enterprise data centers, on-premises deployments, organizations prioritizing performance, situations where network fabric supports BGP.

## Cilium (Isovalent)

Cilium is a next-generation networking and security platform powered by eBPF (extended Berkeley Packet Filter), a Linux kernel technology enabling user-space programs to hook into kernel packet processing.

### eBPF Innovation

eBPF allows Cilium to run custom packet processing logic **inside the kernel** without modifying kernel source or adding latency from userspace transitions.

**Mechanics:**
1. Cilium compiles network policies and routing rules into eBPF bytecode
2. eBPF code is loaded into the kernel's `tc` (traffic control) and `XDP` (eXpress Data Path) hooks
3. When a packet arrives, the kernel executes the eBPF program to decide: forward, drop, or transform
4. All processing happens in-kernel; no userspace context switching overhead

### Key Capabilities

**L7 Observability & Policy:** Cilium understands HTTP, gRPC, DNS, Kafka, and other L7 protocols. Policies can be defined at L7 (allow this pod to call `GET /users/*` on that pod). Automatic metrics on call counts, latencies, failures per microservice.

**Identity-Based Security:** Instead of IP-based rules, Cilium tags pods with identities (based on labels) and defines policy between identities, not IP addresses. Automatic as pods rescale; IPs change but identities remain.

**Bandwidth Management:** eBPF-based QoS; shape traffic per-pod or per-connection.

**Encryption:** Optional automatic mTLS between pods (via Cilium); no sidecar proxy required.

**Multi-Cluster Networking:** Connect Kubernetes clusters; cross-cluster pod-to-pod communication with automatic encryption.

**Ebpf-Powered Host Routing:** No overlay encapsulation; packets go directly between hosts via native routing (similar to Calico). eBPF adds L7 policies on top.

### Strengths

- **L7 Observability & Policy:** Unique ability to define policies at application layer, not just IP layer
- **Zero Trust Security:** Identity-based, automatic encryption, no sidecar proxies required
- **Performance:** eBPF in-kernel processing means minimal overhead
- **Advanced Debugging:** Built-in visibility into every packet and flow; excellent for troubleshooting
- **Modern Design:** Built from scratch for cloud-native, Kubernetes-first environments

### Weaknesses

- **eBPF Requires Modern Kernel:** eBPF features used by Cilium (BPF_PROG_TYPE_TC, BPF_PROG_TYPE_SCHED_CLS) require kernel 5.8+. Older on-premises systems may not qualify.
- **Operational Complexity:** eBPF debugging is non-trivial; kernel module issues can be hard to diagnose
- **Less Adoption:** Newer than Calico; smaller community, fewer third-party integrations
- **VM Support:** eBPF in VMs may be restricted or disabled by hypervisor; better suited for bare metal or cloud VMs

### Use Cases

Organizations requiring L7 security policies (microservices, APIs), DevSecOps-focused teams, modern Kubernetes on recent kernels, multi-cluster Kubernetes deployments.

## Flannel

Flannel is a lightweight, overlay-focused CNI plugin maintained by CoreOS/Red Hat.

### Design

Flannel abstracts each host as having a subnet (e.g., node1: 10.0.1.0/24, node2: 10.0.2.0/24). Flannel backend handles cross-host traffic: by default, VXLAN encapsulation.

**How It Works:**
1. Flannel daemon on each node allocates a subnet from a central etcd-stored pool
2. Flannel creates a bridge/routes inside each node
3. Container traffic to other nodes is VXLAN-encapsulated and sent to the destination host
4. Destination host decapsulates and delivers locally

**Backends:** VXLAN (default, most compatible), host-gw (direct host routes, requires host connectivity), UDP (older, less efficient), IPIP (IP-in-IP tunneling).

### Strengths

- **Simplicity:** Easy to install and understand; minimal configuration
- **Lightweight:** Low resource overhead
- **Widely Compatible:** Works on any infrastructure (VMs, bare metal, cloud)
- **Suitable for Learning:** Good for understanding overlay networks before moving to Calico/Cilium

### Weaknesses

- **No Network Policies:** Flannel is routing-only; no access control. Pair with Calico/project-calico for policies.
- **Active Development:** Mature but less feature development than Calico/Cilium
- **VXLAN Overhead:** Default backend adds encapsulation overhead
- **Single Master etcd:** Relies on external etcd for subnet allocation; adds dependency

### Use Cases

Small to medium Kubernetes clusters, learning environments, organizations preferring simplicity over feature richness, scenarios where lightweight is critical.

## Service Mesh Data Plane Integration

Modern service meshes (Istio, Linkerd) use container networking to inject sidecar proxies.

**Mechanics:**
- Service mesh control plane (e.g., Istiod) detects pod creation
- Control plane admits the pod, injects a sidecar proxy spec (e.g., Envoy)
- Container runtime creates two containers: application + sidecar
- Pod's network namespace is shared; application and sidecar are on the same network
- Iptables rules redirect traffic from the application container through the sidecar proxy
- Sidecar enforces policies, retries, circuit breaking, observability

Container networking (veth, bridge) provides connectivity; service mesh (control plane + sidecar) adds L7 semantics.

## Kubernetes NetworkPolicy

NetworkPolicy is a Kubernetes API for defining ingress/egress rules at the pod level.

**Concepts:**
- **Pod Selector:** Which pods the rule applies to
- **Ingress Rules:** Allowed sources (by pod label, namespace, IP block including external)
- **Egress Rules:** Allowed destinations
- **Protocol/Port:** Specific TCP/UDP ports and protocols (SCTP, TCP, UDP)

**Example:** Deny all egress except DNS and specific service traffic:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-all-egress
  namespace: production
spec:
  podSelector: {}
  policyTypes:
  - Egress
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: UDP
      port: 53
  - to:
    - podSelector:
        matchLabels:
          tier: database
    ports:
    - protocol: TCP
      port: 5432
```

**Implementation:** CNI plugin (Calico, Cilium) translates NetworkPolicy into iptables/eBPF rules on each node.

## Multi-Host Networking Trade-Offs

| **Technology** | **Approach** | **Overhead** | **Scaling** | **Setup** | **Use Case** |
|---|---|---|---|---|---|
| **Bridge** | Single-host bridge | Minimal | 1 host | None | Development, single-host |
| **VXLAN** | UDP-encapsulated tunnels | Moderate (encapsulation) | Many hosts | Straightforward | Cloud, restricted networks |
| **Calico BGP** | Native routing via BGP | Minimal | Many hosts | Moderate (BGP knowledge) | Enterprise data centers, bare metal |
| **Cilium eBPF** | Kernel eBPF, identity-based | Minimal | Many hosts | Moderate (eBPF, kernel ≥5.8) | Modern K8s, L7 security, performance |
| **Flannel** | Simple overlay (VXLAN default) | Moderate | Many hosts | Minimal | Learning, small clusters |

## See Also

- [Container Internals — Namespaces, cgroups, Overlay Filesystems](os-containers-internals.md) — Container namespace primitives
- [Software-Defined Networking (SDN)](infrastructure-networking-sdn.md) — Historical and conceptual context for programmable networking
- [Kubernetes Networking](infrastructure-kubernetes-workloads.md) — Kubernetes-specific service networking
- [Service Discovery](infrastructure-service-discovery.md) — Service discovery built on networking layer
- [DevOps: Service Mesh](devops-service-mesh.md) — Service mesh data planes and Envoy integration