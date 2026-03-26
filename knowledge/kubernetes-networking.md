# Kubernetes Networking — Pod Connectivity, Services, Ingress, and Gateway API

## Overview

Kubernetes networking is designed around three assumptions: pods are ephemeral, nodes may be transient, and the cluster is a single logical network where any pod can reach any other pod without Network Address Translation (NAT). This model differs fundamentally from traditional VM networking, where hosts are stable and network topology matters.

The Kubernetes network model decouples two orthogonal concerns: **how traffic flows between pods** (Pod Networking via CNI plugins) and **how traffic reaches services** (Service abstraction and Ingress controllers).

## Pod Networking and CNI Plugins

Pod-to-pod communication is responsibility of the **Container Network Interface (CNI)** plugin. The Kubernetes control plane does not implement networking itself; it delegates to plugins.

### CNI Fundamentals

A CNI plugin receives IPAM (IP Address Management), creates network interfaces, and routes packets between pods. It must ensure:

- **Pod-to-pod connectivity across nodes** without NAT
- **IP preservation** — packets carry the pod's actual IP, not the node's
- **Low latency and throughput** comparable to VM networking

### Common Plugin Categories

**Overlay Networks** encapsulate packets (e.g., VXLAN, Geneve) to decouple pod IP ranges from physical network topology. Flannel and Weave are examples. Trade-off: simpler deployment (no physical network changes) vs. CPU overhead for encapsulation.

**Direct Routing** requires the underlying network to understand pod CIDR ranges. Calico and Cilium's policy engine support this. Trade-off: no encapsulation overhead, but requires BGP routing or similar infrastructure integration.

**eBPF-based plugins** (Cilium, Hubble) use kernel-level packet filtering and can enforce security policies and load balancing at the kernel layer, reducing userspace overhead.

### Plugin Selection Context

- **Public cloud (AWS/GCP/Azure):** Use managed CNI plugins; cloud networking handles routing.
- **On-prem with BGP infrastructure:** Direct routing plugins like Calico reduce overhead.
- **Overlay-only deployments:** Flexibility for networking design; more CPU consumed.
- **Security as primary requirement:** Cilium's eBPF policies offer fine-grained control unavailable in traditional plugins.

Changing CNI plugins is disruptive (requires cluster drain); choose early or use Network Policies to avoid tight coupling to a specific plugin.

## Services and Load Balancing

A **Service** is an abstraction that provides stable IP and DNS names for a group of pods selected by labels. Pods are ephemeral; Services provide persistent network identity.

### Service Types

**ClusterIP** (default) creates an internal VIP routable only within the cluster. Used for internal service-to-service communication. The kube-proxy component (or eBPF if using Cilium) performs load balancing by selecting a backend pod and rewriting packets.

**NodePort** exposes the Service on a high-numbered port (30000–32767) on every node. Traffic to any node:NodePort reaches the Service. Use cases: dev/test environments, services needing external access without a LoadBalancer. Trade-off: less elegant than LoadBalancer; port numbers are opaque to users.

**LoadBalancer** provisions an external load balancer (cloud-native or bare-metal controller). Traffic arrives at the LoadBalancer's external IP and is routed into the cluster. Most user-friendly for production external access.

**ExternalName** routes to an external DNS name by returning a CNAME. Used when integrating Kubernetes services with external systems.

### Load Balancing Mechanics

kube-proxy maintains iptables (or IPVS, or eBPF) rules that rewrite outbound traffic destined for Service ClusterIP to the pod's actual IP. Three modes exist:

- **iptables mode:** Chains of kernel-level rules; default in many clusters. Can be slow with thousands of rules.
- **IPVS mode:** Kernel in-memory hash table; scales better.
- **eBPF mode (Cilium):** Kernel-level maps; lowest overhead, finest control.

**Session affinity** (ClientIP) can be enabled to send repeated traffic from a client to the same backend, but is rarely necessary; stateless designs scale better.

## Ingress Controllers

An **Ingress** is a Kubernetes resource describing HTTP(S) routing rules: which hostnames and URI paths reach which backend Services. An **Ingress controller** watches Ingress resources and creates the actual load balancer or reverse proxy configuration.

### Ingress Controller Architecture

The controller is not built into Kubernetes; it's a separate deployment (typically an nginx deployment, HAProxy pod, AWS ALB controller, etc.). It watches the API server for Ingress changes and updates its configuration.

Trade-off: flexibility vs. operational complexity. Multiple controllers can coexist in a cluster using the `ingressClassName` field.

### Common Controllers

**nginx Ingress Controller** is the most widely deployed. It runs nginx as a DaemonSet or Deployment and generates nginx.conf from Ingress resources. Well-documented; heavy adoption means mature patterns but also tight coupling to nginx features.

**NGINX plus (commercial)** and **HAProxy Ingress** offer similar functionality with different performance and feature trade-offs.

**Cloud provider controllers** (AWS ALB Controller, GCP Cloud Load Balancer Integration) abstract the cloud's native load balancers. Lower operational overhead; tighter coupling to cloud infrastructure.

### Ingress Limitations

Ingress is HTTP(S) oriented. TCP/UDP services (e.g., databases, game servers) need Service type LoadBalancer or NodePort instead. TLS termination is supported but SNI configuration can be complex. For advanced routing (weighted traffic, canary deployments), a service mesh (Istio, Linkerd) is often needed.

## Gateway API

The **Gateway API** is a newer Kubernetes standard (beta, not yet stable) intended as the successor to Ingress. It separates concerns: a Gateway represents the load balancer infrastructure; Routes (HTTPRoute, TCPRoute, etc.) define how traffic is routed.

### Gateway vs. Ingress

**Ingress:** Monolithic; a single resource describes path rules and TLS. Assumes one platform-wide policy.

**Gateway:** Layered. Gateway resources define infrastructure capabilities and policies controlled by platform teams; HTTPRoute/TCPRoute resources define application routing controlled by app teams. Enables **multi-tenancy** and **role separation**.

Gateway API supports:

- **TCPRoute, UDPRoute:** Protocols beyond HTTP(S)
- **BackendRefs with filters:** More expressive routing logic than Ingress
- **Weighted traffic:** Native support for canary deployment patterns
- **Policy attachment** (e.g., RateLimitPolicy): Extensible security and traffic control

### Adoption State

Gateway API is production-viable but not yet ubiquitous in all distributions. Most clusters still use Ingress. Gateway API adoption is growing in communities focused on advanced routing and multi-team clusters.

## DNS and Service Discovery

**CoreDNS** is the default cluster DNS resolver. It watches Kubernetes API for Service and Endpoint objects and serves DNS responses.

### DNS Resolution Patterns

**Service name within namespace:** `nginx` resolves to `nginx.default.svc.cluster.local` (FQDN). In-pod DNS `/etc/resolv.conf` has `search` directives enabling short names.

**Service from other namespace:** `nginx.production` resolves to `nginx.production.svc.cluster.local`.

**Pod DNS:** Individual pods receive DNS names like `pod-ip-10-0-0-5.default.pod.cluster.local`, rarely used in practice.

**ExternalName Services:** Resolve to external endpoints, enabling gradual migration of external systems into Kubernetes.

### DNS Troubleshooting Pattern

DNS failures often manifest as connection timeouts. Check:

1. Pod's `/etc/resolv.conf` — should have `nameserver` pointing to kube-dns Service IP
2. CoreDNS pod logs for query errors
3. Endpoint objects for the Service — must exist for DNS to resolve
4. Network policies — may block DNS (port 53 UDP)

## Network Policies

A **NetworkPolicy** is a firewall rule: it selects pods and defines which ingress/egress traffic is allowed.

### NetworkPolicy Semantics

Default: all pods can reach all pods (open). NetworkPolicies are **additive** — if a policy matches, it allows specific traffic. If NO policies select a pod, all traffic is allowed.

**Ingress rules** restrict traffic arriving at the pod (source-based filtering).
**Egress rules** restrict traffic leaving the pod.

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-allow
  namespace: production
spec:
  podSelector:
    matchLabels:
      role: api
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              tier: frontend
      ports:
        - protocol: TCP
          port: 8080
```

This allows traffic to pods labeled `role:api` in the `production` namespace **only from** pods in namespaces labeled `tier:frontend` on port 8080.

### NetworkPolicy Limitations

- **Not enforced by all CNI plugins.** Calico, Cilium, Weave support it; some basic plugins do not. Check before relying on policies.
- **No explicit "deny" semantics.** Policies are allow-lists; default-deny requires a catch-all policy with no ingress rules.
- **Pod-to-pod granularity only.** Cannot reference Services or individual nodes directly.

### Service Mesh Integration

Service meshes (Istio, Linkerd) offer richer traffic policies (rate limiting, retries, weighted routing) and often include mutual TLS (mTLS) enforcement. Trade-off: added operational complexity for powerful control.

## IPv4/IPv6 Dual-Stack

Modern Kubernetes clusters can operate with both IPv4 and IPv6 addresses simultaneously. Each pod gets both an IPv4 and IPv6 address.

**Implementation detail:** Requires dual-stack CNI plugin, dual-stack IPAM, and dual-stack Service support. Most cloud providers and popular CNI plugins support this, but not all older clusters do.

**Common pattern:** IPv4 as primary for backward compatibility; IPv6 for future-proofing. Services can be single-stack (IPv4 or IPv6 only) or dual-stack.

## Mental Model

Kubernetes networking is: **(CNI layer) + (kube-proxy / load balancing) + (DNS) + (optional policies + service mesh)**. Each layer is pluggable and can be understood independently. Pod-to-pod connectivity is the responsibility of the CNI; Services are a Kubernetes abstraction on top. Ingress and Gateway API are convenience layers for HTTP routing, not fundamental to cluster communication.

The model assumes your infrastructure (physical or cloud) is a single flat network; if it's not, some CNI features (direct routing, policy enforcement) may be limited.

## Common Integration Patterns

### Multi-Cluster Service Discovery

In multi-cluster deployments, services in cluster-A need to reach services in cluster-B. Common patterns:

**Flat network with CNI overlap:** If both clusters use the same CNI and non-overlapping pod CIDR ranges, traffic naturally routes across clusters (requires BGP or similar).

**Ingress chaining:** Cluster-A's Ingress controller routes to an external endpoint in cluster-B.

**Service mesh (Istio, Linkerd):** Extends service mesh across cluster boundaries; manages cross-cluster traffic explicitly. More complex but fine-grained control.

### Observability and Monitoring

Network traffic in Kubernetes is often invisible without explicit tooling. Common approaches:

**Cilium Hubble:** Built-in observability for Cilium-based clusters. Shows pod-to-pod communication flows, policy violations, latency.

**Service mesh observability (Kially, Grafana):** Service meshes provide built-in traffic metrics and dashboards.

**Network policies debugging:** Verify policies are correctly allowing/denying traffic. `kubectl describe networkpolicy` shows rules; tcpdump inside pods confirms if traffic actually arrives.

