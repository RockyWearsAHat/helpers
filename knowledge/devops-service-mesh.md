# Service Mesh — Network Abstraction and Observability Layer

A service mesh is infrastructure software that handles service-to-service communication in a microservices architecture. Instead of each application handling network concerns (retries, timeouts, encryption, routing), the mesh extracts those concerns into a separate layer managed operationally.

The trade-off: operational simplicity (shared resilience patterns) vs. operational complexity (new layer to debug, monitor, understand).

## Architecture: Sidecar Proxy Model

Most service meshes use the **sidecar proxy** pattern:

- Each service instance is paired with a proxy container running in the same Pod (Kubernetes) or process boundary
- All inbound and outbound traffic flows through the proxy
- Proxy intercepts and transforms traffic (retries, circuit breaking, mTLS, routing rules)
- Proxies are managed by a control plane that configures them

### Envoy Proxy (Used by Istio)

Envoy is a high-performance L3/L4 and L7 proxy written in C++. It's the dominant data plane choice.

**Key capabilities:**
- Dynamic service discovery (knows about new/removed instances)
- Advanced load balancing (weighted round-robin, consistent hashing, maglev)
- Circuit breaking and outlier detection
- Distributed tracing integration
- TLS termination
- L7 (HTTP/gRPC) routing and matching

Envoy is fast — benchmarks show single-digit millisecond overhead per hop. But every service pays that cost.

### Linkerd Proxy

Linkerd is a lightweight service mesh written in Rust with a simpler proxy designed for Kubernetes:

- Smaller resource footprint than Istio
- Auto-mTLS between proxies (no config needed)
- Automatically collects top-line metrics (latency, success rate)
- Less flexible than Envoy (fewer routing options)
- Designed for simplicity over feature completeness

**Philosophy:** Linkerd's tradeoff is fewer knobs, lower complexity.

### eBPF-Based Approaches (Cilium)

Cilium is a container networking solution that also offers service mesh capabilities via eBPF (extended Berkeley Packet Filter). eBPF is kernel-level bytecode that can intercept and modify packets:

**Advantages:**
- No sidecar containers needed (kernel-level enforcement)
- Lower latency and resource overhead than sidecar proxies
- Superior network visibility (Cilium's Hubble observability)

**Disadvantages:**
- eBPF is newer, less mature than sidecar proxies
- Kernel version requirements (eBPF features vary by Linux kernel version)
- Less portable to non-Linux environments

**Status:** Cilium is gaining adoption for greenfield environments. Existing Istio/Linkerd deployments are not easily replaced by eBPF.

## Control Plane Patterns

The control plane configures proxies and maintains service discovery:

### Istio: Centralized Control Plane (istiod)

- Single `istiod` component maintains all proxy configurations
- Listens to Kubernetes API for service/pod changes
- Generates Envoy configurations (EnvoyFilter, VirtualService, DestinationRule)
- Provides mTLS certificate management
- Scaling challenges: one istiod controlling thousands of proxies creates a bottleneck

### Linkerd: Distributed Control Plane

- Policy controller manages traffic policies
- Proxy injector handles mTLS setup
- Lighter weight than istiod
- Less centralization, better fault isolation

### Cilium: Kernel-Embedded

- No separate sidecar proxy, so no traditional control plane
- Cilium agent on each node programs kernel with eBPF rules
- Agent watches Kubernetes API
- Updates are push-based to kernel

## Traffic Management Features

### Canary Deployments

Route percentage of traffic to new version while monitoring metrics. If errors/latency spike, auto-rollback. Requires:
- Service discovery (know which pods are "canary")
- Load balancing (weighted routing)
- Metrics integration (success rate, latency)

**Tools:** Flagger orchestrates canary with Istio/Linkerd; Argo Rollouts integrates with service mesh.

### Circuit Breaking

Prevent cascading failures by stopping calls to unhealthy services:

- After N failures within a window, circuit opens
- Open circuit rejects requests immediately without calling service
- After timeout, circuit attempts recovery (half-open)
- Success = circuit closes; failure = back to open

Istio's OutlierDetection configures circuit breaking per endpoint pool.

### Retries and Timeouts

Mesh can automatically retry failed requests (idempotent operations only). Retries configured in VirtualService:

```yaml
Retries:
  attempts: 3
  perTryTimeout: 10s
```

Over-aggressive retries amplify load during outages.

### Request Routing and Matching

#### L7 (Application-Layer) Routing

Route based on HTTP headers, path, methods, query parameters:

```yaml
- match:
    - headers:
        user-agent:
          regex: ".*Chrome.*"
  route:
    - destination:
        host: service
        subset: canary
```

Enables A/B testing, header-based routing, path-based services.

#### Load Balancing Algorithms

- Round-robin (default)
- Least request
- Ring hash (consistent hashing for stateful services)
- Random
- Maglev (Google's hash algorithm)

### Rate Limiting and Traffic Shaping

Mesh can enforce rate limits per client, service, or global. Also supports traffic mirroring (shadow traffic to new version without affecting primary).

## Security: Mutual TLS (mTLS)

Service mesh can enforce encrypted, authenticated communication between services:

- Each service gets an identity certificate
- Mesh automatically authenticates both sides (mutual TLS)
- Communication is encrypted end-to-end
- Replaces service-level SSL/TLS code

**Activation:** 
- Istio: PeerAuthentication policy sets STRICT (enforce mTLS) vs PERMISSIVE (allow both encrypted and unencrypted)
- Linkerd: Automatic for all services (simpler)

**Limitation:** mTLS doesn't protect service-to-external-service communication. Also doesn't secure data at rest or application-layer authorization.

## Observability Integration

Service mesh inherently observes all traffic passing through proxies:

### Metrics Collection

Proxies emit:
- Request rate (requests per second)
- Success rate (percentage of non-5xx responses)
- Latency (p50, p95, p99)
- Traffic volume by destination, method, path

No application instrumentation needed. Works for all protocols (gRPC, HTTP, TCP).

### Distributed Tracing

Each proxy can inject trace context (trace ID, span ID) into requests. Traces follow requests across service boundaries. Integrates with Jaeger, Zipkin, or other tracing backends.

### Logging and Visualization

Mesh can log request/response metadata. Cilium's Hubble provides network-level observability: which services talk to which, traffic volume, latency.

## Operational Complexity: When NOT to Use a Mesh

Service mesh introduces significant operational burden:

### Resource Overhead

Each sidecar proxy consumes memory (~50-100MB per Envoy instance) and CPU. For applications with hundreds of instances, the per-sidecar cost multiplies. A mesh-wide outage (e.g., misconfigured istiod) brings down all services.

### Learning Curve

Operators must understand:
- Service mesh CRDs (VirtualService, DestinationRule, Gateway, PeerAuthentication)
- Proxy configuration model
- mTLS certificate lifecycle
- Traffic policies and routing rules

Debugging involves understanding proxy behavior, not just application logs.

### When to Skip a Mesh

- **Small deployments:** < 10-20 microservices. Overhead exceeds benefit
- **Simple topologies:** Few services, light traffic between them
- **Edge/resource-constrained:** IoT, embedded systems where per-service costs matter
- **Teams without SRE capacity:** Mesh requires dedicated operational expertise
- **Non-HTTP protocols:** Mesh is primarily HTTP/gRPC; TCP/binary protocols less well-supported
- **Latency-sensitive:** Real-time systems may not tolerate proxy overhead

**Common pattern:** Adopt service mesh once you have high service interdependency and complex traffic patterns (canary, circuit breaking, encryption) that become tedious to implement at application layer.

### Ambient Mesh (Istio's Evolution)

Istio Ambient sideless mesh aims to reduce overhead by moving proxy logic from sidecar to node-level or using eBPF. Status: experimental/early adoption. Reduces per-pod resource cost, improves adoption for resource-constrained environments.

## Multi-Cluster Service Mesh

Extending mesh across clusters:

- **Shared control plane:** One istiod manages proxies across multiple clusters (requires high-bandwidth, low-latency network)
- **Federated control planes:** Each cluster has istiod; they federate service discovery and trust (more complex)
- **External service discovery:** Mesh queries external source (Consul, etcd) for service locations across clusters

**Challenges:** Cross-cluster mTLS (certificate trust), network latency, split-brain scenarios.

## Service Mesh vs. CNI/Network Plugin

**Service mesh** operates at L7 (application layer). Example: "Route this request to service version 2 if header=value."

**Container Networking Interface (CNI) / Network plugins** (Calico, Cilium) operate at L3/L4. Example: "Allow packets from namespace A to namespace B."

**Do you need both?**

- CNI handles pod-to-pod connectivity, network policies
- Service mesh handles application-layer resilience, traffic management, observability

They complement each other. CNI as the network foundation, mesh for service-level concerns.

Cilium blurs this boundary by offering both CNI and service mesh capabilities in one eBPF-based system.

## Ecosystem Positioning

| Tool | Proxy | Philosophy | Complexity | Maturity |
|------|-------|-----------|-----------|----------|
| **Istio** | Envoy | Feature-rich, powerful | High | Production-ready |
| **Linkerd** | Linkerd | Simple, Rust, lightweight | Low | Production-ready |
| **Cilium** | eBPF (no sidecar) | Kernel-native, performance | Medium | Maturing |
| **Consul Connect** | Envoy | HashiCorp ecosystem | Medium | Production-ready |

**Emerging:** Service meshes for non-Kubernetes (VMs, heterogeneous infrastructure) remain early-stage.

## Debugging and Troubleshooting

When things go wrong:
- **Traffic not reaching destination:** Inspect VirtualService, DestinationRule, subset definitions
- **High latency:** Check Envoy proxy metrics, circuit breaker state
- **mTLS failures:** Verify certificates, PeerAuthentication policy, trust domain
- **Configuration not applied:** Inspect proxy configuration (envoy config dump) vs. CRD definitions

Mesh adds a layer between applications and network. Debugging requires understanding proxy state, not just application logs.

## See Also

- Microservices architecture patterns: [architecture-microservices.md](architecture-microservices.md)
- Distributed systems resilience: [architecture-resilience.md](architecture-resilience.md)
- Observability and tracing: [logging-observability.md](logging-observability.md), [devops-opentelemetry.md](devops-opentelemetry.md)
- Container networking: [containers-orchestration.md](containers-orchestration.md)
- Kubernetes concepts: [devops-kubernetes.md](devops-kubernetes.md)