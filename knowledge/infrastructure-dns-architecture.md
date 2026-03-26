# DNS Infrastructure at Scale — Anycast, GeoDNS, Failover, and Service Discovery

## Scaling DNS Beyond the Hierarchy

The basic DNS hierarchy (stub resolver → recursive resolver → root/TLD/authoritative) works for small deployments. At internet scale, infrastructure providers must solve:

1. **Geographic distribution**: Users worldwide need low-latency DNS
2. **Failover and redundancy**: DNS server failures must not block name resolution
3. **DDoS resilience**: DNS is a critical attack vector; must scale horizontally
4. **Internal service discovery**: Microservices need dynamic DNS without public registration

All solutions layer on top of DNS standards; no protocol changes needed.

## Authoritative Server Replication

Public DNS zones (e.g., example.com) are replicated across multiple authoritative nameservers, typically in different geographic regions:

```
authoritative nameservers for example.com:
  ns1.example-dns.com (US East)
  ns2.example-dns.com (EU)
  ns3.example-dns.com (Asia)
  ns4.example-dns.com (failover)
```

When a recursive resolver queries for a record, it may hit any of these servers. **Zone transfers** (AXFR/IXFR) replicate changes from primary to secondaries. Replication introduces **eventual consistency**: newly created records may not resolve immediately on all servers.

## Anycast: One IP, Many Servers

Anycast is a **routing technique**, not a DNS feature. Multiple servers in different locations are assigned the same IP address (e.g., 8.8.8.8 for Google Public DNS). BGP (Border Gateway Protocol) routes each request to the **closest server** based on network topology.

**How it works**:
1. Each data center announces the same IP prefix via BGP with local AS path
2. Routers prefer shorter AS paths, typically routing to nearest neighbor
3. User queries resolve to geographically closest server
4. Each server independently runs DNS resolver or authoritative service

**Advantages**:
- Low latency (request routes to nearest server)
- Automatic failover (if one server fails, BGP reroutes to next)
- Single IP address (no need for clients to know multiple endpoints)
- Works at network layer (transparent to applications)

**Disadvantages**:
- Requires coordination (BGP announcement, consistent zone data)
- AS path may not reflect actual latency ("closest" by AS hops, not geography)
- Asymmetry: Request routes to one server, response might route differently

**Common deployments**: Root nameservers (13 addresses, ~100 server locations), public DNS (Google 8.8.8.8, Cloudflare 1.1.1.1, Quad9).

## GeoDNS: Geographic Response Steering

GeoDNS modifies **DNS responses based on the client's location**. Unlike anycast, which routes at the network layer, GeoDNS makes **DNS-layer decisions**.

**Implementation approach**:
1. Recursive resolver queries authoritative server
2. Authoritative server determines client location (via source IP or **EDNS Client Subnet**)
3. Returns **different answer** based on geography

Example:
```
Query: What is cdn.example.com?
Resolver IP: 1.2.3.4 (identified as Europe)
Response: 93.184.1.100 (European CDN POP)

Query: Same, but resolver IP: 210.0.0.1 (identified as Asia)
Response: 210.1.1.100 (Asian CDN POP)
```

**Limitations**:
- **Resolver location vs user location**: Public DNS resolvers (8.8.8.8, 1.1.1.1) have global anycast presence. Without EDNS Client Subnet (RFC 7871), the authoritative server sees the resolver's IP, not user's, leading to wrong routing.
- **EDNS Client Subnet** (ECS): Allows recursive resolver to send user's IP to authoritative as a separate field. Improves accuracy but reveals user location (privacy concern).

**Practical use cases**: CDN origin selection, regional failover, compliance (e.g., routing users to geographically appropriate servers).

## DNS Load Balancing Patterns

### Round-Robin DNS

Authoritative server returns multiple A records in random order for the same name:

```
$ dig example.com
example.com 300 IN A 192.0.2.1
example.com 300 IN A 192.0.2.2
example.com 300 IN A 192.0.2.3
```

Client typically uses the first record. Each query gets a different order, distributing load across backends.

**Flaws**:
- **No health checking**: Dead servers still appear in DNS answers
- **Sticky clients**: If client caches result, all traffic goes to same backend
- **Race conditions**: No binding between client and backend; TCP handshake might fail

**Use**: Non-critical applications; better to use L4/L7 load balancing.

### Weighted and Priority DNS

Return records with **SRV** records (RFC 2782) containing weights and priorities:

```
_http._tcp.example.com 300 IN SRV 0 10 80 server1.example.com
_http._tcp.example.com 300 IN SRV 0 5 80 server2.example.com   # Lower weight, less traffic
_http._tcp.example.com 300 IN SRV 100 10 80 backup.example.com  # Higher priority = failover
```

Client should pick by: 1) lowest priority, 2) random selection weighted by weight.

**Advantage over round-robin**: Explicit control of distribution and failover.
**Disadvantage**: Requires clients to understand SRV records (not all HTTP clients do).

## Recursive Resolver Failover

Clients often configure multiple recursive resolvers:

```
nameserver 8.8.8.8           # Google DNS
nameserver 1.1.1.1           # Cloudflare DNS
nameserver 208.67.222.222    # OpenDNS
```

OS resolver tries the first; on timeout/failure, tries the second. Provides basic resilience without DNS infrastructure involvement.

**Trade-off**: Each failover adds latency (timeout = typically 1-5 seconds per resolver).

## Private DNS Zones

Private DNS zones are **not registered in public DNS hierarchy**; they only resolve within a network (corporate intranet, VPC).

**Architecture**:
- **Internal resolver**: VPC/network-local resolver
- **Private zone**: Zone file stored only on internal resolver
- **Forwarding**: May forward `.example-internal` queries to corporate DNS, others to public resolvers

**Use cases**:
- Internal service names (db.internal, queue.internal)
- Service discovery in Kubernetes (service.namespace.svc.cluster.local)
- Corporate directory (employees.corp.internal)

**Privacy advantage**: Internal names leak no information to public internet. Records never queryable from outside.

## Split-Horizon DNS (Split-View)

Serve **different answers for the same name depending on query source** (internal vs external).

```
Example: api.example.com

From internal network: 10.0.1.5 (private IP)
From internet:         93.184.1.100 (public IP)
```

**Implementation**:
1. Configure two zones with same name; one in private zone, one public
2. Internal resolver answers from private zone
3. Public (authoritative) resolver answers from public zone

**Common pattern**:
- Internal clients talk directly to internal service (lower latency)
- External clients hit public endpoint (API gateway, CDN)

**Benefit**: Reduced public surface; internal traffic never leaves VPC.

## DNS for Service Discovery

Modern infrastructure treats DNS as a **service discovery mechanism**.

**Kubernetes example**:
```
Service: my-app in namespace production
DNS name: my-app.production.svc.cluster.local → cluster IP
Individual pod: my-app-0.my-app-headless.production.svc.cluster.local → pod IP
```

DNS queries trigger API server lookups; no zone file. **Dynamic**: services created/deleted, DNS automatically reflects state.

**Consul pattern** (HashiCorp):
```
- Service registration: On startup, service registers with local Consul agent
- DNS interface: Consul provides DNS resolver on localhost:8600
- Health checking: Consul health-checks services; DNS excludes unhealthy instances
- Query: dig web.service.consul @127.0.0.1 -p 8600 → returns IPs of healthy web services
```

**Advantages over manual DNS**:
- No manual zone file updates
- Automatic failover (health checks remove bad instances)
- Instant propagation (milliseconds, not minutes)

## Cloud DNS Services

### AWS Route 53

**Routing policies**:
- **Simple**: Single resource
- **Weighted**: Distribute traffic by percentage (10% to us-west, 90% to us-east)
- **Latency-based**: Route to region with lowest latency
- **Failover**: Route to primary; on health check failure, route to secondary
- **Geolocation**: Route by country/continent
- **Multivalue**: Return multiple IPs (like round-robin, but with health checks)

**Health checks**: Monitor endpoints (HTTP, TCP, CloudWatch alarms); automatically remove unhealthy targets from DNS responses.

**Integration with AWS services**: Alias records map to ALB, CloudFront, S3 websites (no additional CNAME cost).

### Cloudflare DNS

**Anycast network**: 200+ data centers globally.
**Intelligent routing**: Combines latency-based steering, failover, WeGLB (Weighted GeoDNS Load Balancing).
**DDoS protection**: DNS-layer filtering, DNSSEC enforcement.
**Workers**: Serverless functions can intercept DNS queries for custom logic.

### Google Cloud DNS

**Private zones**: Internal DNS for VPCs.
**Cross-cloud peering**: VPCs in GCP can resolve names in on-premises DNS (via Cloud VPN).
**Cloud Run integration**: Automatically manages DNS for serverless services.

## Global Server Load Balancing (GSLB) Architecture

GSLB combines DNS + health checks + latency measurement for **global failover and optimization**.

**Typical deployment**:
```
1. Multiple data centers (US, EU, Asia) each with load balancer
2. GSLB controller (usually cloud-hosted or vendor-provided)
3. Health checks from GSLB to each data center (HTTP, TCP, ICMP)
4. Latency probing (synthetic queries measure RTT to each DC)
5. DNS via anycast or GeoDNS steers traffic to best DC
6. If DC fails health check, GSLB updates DNS to exclude it
```

**Decision factors**:
- Geographic proximity
- Health status
- Current load (if provided by DC)
- Latency percentile (p95 latency, not average)

**Failure scenario**: Primary US data center becomes unhealthy. GSLB health check fails. GSLB stops returning US IPs in DNS. Clients resolve instead to EU DC. Traffic reroutes within seconds (DNS TTL matters here).

## Key Considerations

- **TTL (Time-to-Live)**: Lower TTL (60s) enables faster failover but increases resolver load. Higher TTL (3600s) reduces load but prolongs outage detection.
- **Caching layers**: ISP and local resolvers cache results; GeoDNS/GSLB changes propagate gradually.
- **DNSSEC**: Adds cryptographic signing; performance cost (signature verification) and complexity (key rotation).
- **Privacy**: DNS queries leak metadata; DoH (DNS over HTTPS) encrypts queries end-to-end (but enables single point of failure at HTTPS resolver).

## See Also

- networking-dns (basic DNS mechanics, hierarchy, record types)
- infrastructure-load-balancing (L4/L7 load balancing, health checks, failover algorithms)
- system-design-distributed (distributed systems principles; failure modes, consensus)