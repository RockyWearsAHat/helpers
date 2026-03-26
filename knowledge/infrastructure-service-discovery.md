# Service Discovery — Registry Patterns, Health Checks & Client vs. Server Models

## Overview

Service discovery solves the fundamental problem of locating services in a distributed system: how does a client find the IP address and port of a service when services are ephemeral, dynamically scaled, and deployed across many machines? The answer separates into two broad categories: **client-side discovery** (clients maintain a registry and make routing decisions) and **server-side discovery** (a load balancer or router absorbs that complexity). Service registries track the availability and location of services through active registration (services announce themselves) or passive observation (discovery system monitors the infrastructure).

## Client-Side vs. Server-Side Discovery

### Client-Side Discovery
The client directly queries a registry to find available service instances, then chooses one and connects. This pattern shifts discovery logic and decision-making into the client.

**Mechanics:**
1. Service starts, registers itself with the registry (often with health metadata)
2. Client queries the registry to get a list of instances
3. Client is responsible for choosing an instance (round-robin, least-loaded, etc.)
4. Client connects directly to the chosen instance

**Advantages:**
- Centralized registry, distributed routing decisions
- Clients can implement sophisticated routing logic (circuit breakers, load balancing algorithms)
- No intermediary adds latency or becomes a bottleneck
- Good visibility into routing decisions within the client

**Disadvantages:**
- Each client language/framework must implement discovery logic
- Clients must handle stale registry data (cache invalidation)
- Load balancing logic duplicated across many client implementations
- Harder to change routing policy globally

### Server-Side Discovery
A load balancer, API gateway, or service mesh acts as the single entry point. Services register with the discovery system, and clients connect to the external load balancer; the load balancer forwards to available instances.

**Mechanics:**
1. Service registers with the registry
2. Client connects to the load balancer (fixed address)
3. Load balancer queries the registry to find available instances
4. Load balancer forwards the request to a chosen instance

**Advantages:**
- Clients don't need discovery logic; they know a single endpoint
- Routing policy can be changed globally without redeploying clients
- The load balancer is the single point of control
- Works well with existing client libraries that don't support discovery

**Disadvantages:**
- Load balancer becomes a bottleneck and single point of failure (though typically deployed for HA)
- Extra hop adds latency
- Load balancer must be available for any communication to happen
- More complex operational footprint

## Registry Patterns

### DNS-Based Discovery

**Mechanics:** Services update DNS records directly; clients resolve names to IP addresses.

**DNS SRV Records:**
The Service Record (RFC 2782) provides service discovery through DNS. A client queries `_service._proto.name` to get `target:port` pairs with priorities and weights.

_Example:_ `_http._tcp.production.internal` returns all HTTP services in production, sorted by priority and weighted by load. Clients can use DNS TTL to cache results and fallback mechanics to handle service failures.

**Strengths:**
- Completely decentralized; no centralized registry needed
- Every service already has DNS; works with standard tools
- Caching via TTL balances freshness and load on DNS servers
- Works across multiple data centers; integrates with external DNS

**Weaknesses:**
- DNS caching is coarse-grained; stale records can persist for TTL duration
- TTL mismatch: clients may cache longer than intended, or DNS resolver caches compound staleness
- Round-robin is the common load balancing strategy; sophisticated algorithms harder to implement
- Changes take time to propagate (TTL + cache delay)
- Not ideal for rapid churn (containers scaling up/down)

**Usage:** Internal Kubernetes DNS services, traditional microservices with SRV records, mDNS for local service discovery.

### Consul (HashiCorp)

A full-featured distributed service mesh and configuration management platform.

**Core Features:**
- **Service Registration:** Services register directly via HTTP API or through a local agent
- **Health Checks:** Periodic HTTP/TCP checks, script-based checks, TTL-based checks (services ping Consul to stay healthy)
- **Multi-Datacenter:** Built-in replication and forwarding across data centers
- **KV Store:** Distributed configuration storage alongside service registry
- **Service Mesh:** Sidecar proxies (Envoy) for transparent L7 routing and observability
- **DNS Interface:** Services accessible via `<service>.service.consul` DNS names

**Health Check Modes:**
- **Periodic:** Consul actively probes the service
- **TTL:** Service must heartbeat to Consul; failure to update within TTL marks instance unhealthy
- **Script:** Custom script exit code determines health
- **Deregister Critical:** Instances can be auto-deregistered after failing health checks N consecutive times

**Strengths:**
- Comprehensive; handles discovery, config, secrets, and mesh in one platform
- Strong Byzantine fault tolerance (consensus-based, handles network partitions gracefully)
- Rich metadata per instance (tags, port mappings, etc.)
- Excellent for multi-region/multi-cloud deployments
- Built-in sidecar mesh simplifies canary deployments and traffic shifting

**Weaknesses:**
- Adds operational complexity; requires maintaining a Consul cluster
- Heavier than DNS-only solutions
- Requires agents on every node; more moving parts

**Usage:** Complex microservices environments, multi-region deployments, organizations migrating to service mesh.

### etcd (CoreOS/Cloud Native Computing Foundation)

A distributed, consistent key-value store used by Kubernetes for service discovery.

**Mechanics:**
- **Strongly Consistent:** All reads see the latest write (not eventually consistent)
- **TTL/Leases:** Keys can be set with a TTL or attached to a lease; expiry auto-cleans stale registrations
- **Watches:** Clients subscribe to key/range changes and receive notifications in real-time
- **Transactions:** Multi-key atomic operations ensure consistency

**Service Discovery Pattern:**
Services register at `/services/{service-name}/{instance-id}` with a lease. Clients watch for changes on `/services/{service-name}` and maintain a local cache of available instances. When a service crashes or lease expires, the instance is automatically deregistered.

**Strengths:**
- Minimal, focused tool; does one thing very well (consistent distributed state)
- Excellent real-time notification system via watches
- Zero stale-data windows; watches provide instant updates
- Lightweight and battle-tested
- Tight integration with Kubernetes (all K8s state lives in etcd)

**Weaknesses:**
- Requires external heartbeat/lease renewal logic (more client-side code)
- Lower throughput than eventually-consistent systems
- Partition intolerant during consensus (favors consistency over availability)
- No built-in HTTP health checks; you provide your own

**Usage:** Kubernetes service discovery (native), cloud-native environments with custom clients, systems requiring strong consistency.

### Eureka (Netflix)

The service registry that powered Netflix's migration to microservices.

**Core Features:**
- **Peer-to-Peer:** All Eureka servers in a cluster replicate to each other; no master
- **Self-Preservation:** When many instances fail to heartbeat, Eureka enters "self-preservation mode" to tolerate network partitions (doesn't deregister instances just because heartbeats are late)
- **Client-Side Registry Cache:** Clients cache the entire registry locally; discovery queries go to the local cache, not the server
- **Periodic Heartbeats:** Services must heartbeat every 30 seconds (default TTL: 90 seconds)

**Strengths:**
- Handles network partitions gracefully without hard failures
- Local caching means clients can continue routing even when registry is temporarily unavailable
- Proven at scale (Netflix)
- Simple to understand: heartbeat model is intuitive

**Weaknesses:**
- Self-preservation mode means potentially stale instances stay registered longer
- Eventual consistency; replicas may briefly disagree on state
- Client-side caching combines with periodicity, increasing staleness windows
- Less active development (Netflix shifted focus to Kubernetes)

**Usage:** Spring Boot microservices (has first-class support), on-premises deployments, organizations already using Spring Cloud Eureka.

### Kubernetes Services and Endpoints

Kubernetes abstracts service discovery through the Service API and automatic Endpoint management.

**Mechanics:**
- **Service:** Represents a logical set of pods and a stable name/IP
- **Endpoints:** Automatically populated by the control plane with the list of healthy pod IPs matching the service's selector
- **kube-proxy:** Runs on every node, watches Services/Endpoints and programs iptables/IPVS rules to forward traffic
- **DNS:** Creates DNS records for services (e.g., `my-service.default.svc.cluster.local`), returning the stable Service IP, which kube-proxy intercepts and load-balances

**Service Types:**
- **ClusterIP:** Stable internal-only IP; kube-proxy load-balances across pod IPs
- **NodePort:** Exposes service on a fixed port on every node; inherits ClusterIP behavior
- **LoadBalancer:** Requests cloud provider to provision load balancer (NodePort underneath)
- **ExternalName:** Simple DNS CNAME to external services

**Strengths:**
- Automatic; pods are discovered the moment they're selected by the service selector
- Tight integration with the entire K8s ecosystem
- Declarative; separates logical service identity from physical instances
- Handles pod restarts/replacements transparently
- Built-in DNS makes client code trivial

**Weaknesses:**
- Tightly coupled to Kubernetes; not portable to VMs or external systems
- kube-proxy has limitations (doesn't support service-to-service mesh rules without add-ons)
- Debugging service discovery issues requires understanding kube-proxy and DNS mechanics

**Usage:** All Kubernetes deployments (mandatory); primary discovery method for container workloads.

## mDNS and Local Service Discovery

**Multicast DNS (mDNS)** is a zero-configuration service discovery mechanism (RFC 6762/6763). Services advertise themselves via multicast, and clients query via multicast to discover services on the local network.

**Mechanics:**
- Service registers with mDNS (typically via `avahi` daemon on Linux, `Bonjour` on macOS)
- Service publishes `_service._proto.local` records
- Clients query `_service._proto.local` which triggers multicast on the local subnet
- Responders send back their addresses

**Strengths:**
- Zero central infrastructure; works peer-to-peer
- Good for edge networks, IoT, and single-site deployments
- Fully local; no external dependencies

**Weaknesses:**
- Doesn't scale beyond a local network (no routing)
- Multicast may not work reliably across network boundaries
- Limited to LAN speeds and accessibility
- Less common in modern cloud deployments

**Usage:** IoT networks, containerized development environments (Docker Desktop), home automation, hyperlocal service discovery.

## Health Check Design

Service discovery is only useful if the registry reflects reality. Health checking ensures registered instances are actually healthy.

**Common Patterns:**

**Passive Checks (Client Probes):** A discovery service or load balancer periodically makes requests to instances and marks them unhealthy if responses fail. Simple and language-agnostic, but adds latency; slow to detect failures.

**Active Checks (Service Heartbeat):** Service must periodically report health to the registry (or renew a lease). Faster failure detection, but requires service-side code to heartbeat correctly. TTL-based (service must ping before lease expires) or liveness-based (service reports pass/fail).

**Connection-Level Checks:** Detect TCP connection failures; coarse-grained but catches total outages. Used with other checks.

**Protocol-Specific Checks:** HTTP health endpoints (e.g., `/health` returning 200 on success), gRPC health proto (bidirectional stream), custom TCP probes.

**Composite Health:** Combine multiple checks (one unhealthy = instance marked unhealthy). Example: healthy if HTTP /health passes AND process is not in a "graceful shutdown" state.

**Cascading Failures:** Configure check frequency, retry backoff, and deregister thresholds carefully. Slow checks or aggressive retry can overwhelm the registry; too lenient and the registry doesn't detect failures fast enough.

## Trade-Offs Summary

| **Dimension** | **Client-Side** | **Server-Side** |
|---|---|---|
| **Implementation** | Distributed; logic in many clients | Centralized; logic in router/gateway |
| **Failure Tolerance** | Client goes down → no routing (unless cached) | Router down → all clients fail |
| **Latency** | Direct connection (no hop) | Extra router hop |
| **Operational Control** | Each team owns their client logic | Central control over routing policy |
| **Scalability** | Excellent; registry query load distributed | Load balancer may become bottleneck |

| **Technology** | **Consistency** | **Scale** | **Setup** | **Use Case** |
|---|---|---|---|---|
| **DNS SRV** | Eventual | High | Low | Traditional microservices, multi-region |
| **Consul** | Strong (leader-based) | Medium | High | Complex mesh architectures, multi-cloud |
| **etcd** | Strong (consensus) | Medium-High | High | Kubernetes, custom consistency-critical apps |
| **Eureka** | Eventual | High | Medium | Spring Cloud microservices |
| **Kubernetes Services** | Strong | High | None (built-in) | Container orchestration |
| **mDNS** | Eventual | Low (LAN-local) | None (peer-to-peer) | IoT, local networks, edge |

## See Also

- [Distributed Coordination](distributed-coordination.md) — Service registry backends (Zookeeper, etcd, Consul)
- [Kubernetes Services](infrastructure-kubernetes-workloads.md) — K8s-specific service discovery mechanics
- [DNS Infrastructure at Scale](infrastructure-dns-architecture.md) — DNS as a discovery backbone
- [API Gateway Patterns](infrastructure-api-gateway-patterns.md) — Server-side discovery via gateways
- [Microservices Architecture](architecture-microservices.md) — Service discovery in larger context