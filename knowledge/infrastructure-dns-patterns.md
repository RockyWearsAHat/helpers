# DNS Patterns — Routing, Service Discovery, Failover, and Record Types

## Overview

DNS is the distributed hierarchical database that translates names to IP addresses. Beyond basic resolution, modern DNS enables sophisticated patterns: **GeoDNS** (route users to nearby servers), **DNS failover** (switch on endpoint failure), **service discovery** (dynamic service-to-IP mapping), and **encrypted DNS** (privacy and security). Understanding patterns and failure modes is essential for reliability.

## DNS Resolution Path (Review)

**Stub resolver** (client)
→ **Recursive resolver** (ISP or 8.8.8.8)
→ **Root nameserver** (points to TLD)
→ **TLD nameserver** (points to Authoritative)
→ **Authoritative nameserver** (returns answer)

Each step involves caching; TTL (Time To Live) controls cache lifetime. Lower TTL = faster failover (stale answers shorter-lived), but higher load on authoritative server.

## DNS Routing Patterns

### GeoDNS: Location-Based Routing

GeoDNS returns **different IP addresses based on client location**. Client location inferred from source IP address or EDNS Client Subnet (ECS).

**How it works:**
1. Client queries authoritative nameserver
2. Nameserver looks up client IP geolocation (via GeoIP database)
3. Returns IP address closest to client's location

**Example:**
```
Query: api.example.com from user in Tokyo
Response: 203.0.113.50  (Japan data center)

Query: api.example.com from user in London
Response: 198.51.100.42 (Europe data center)
```

**Implementation:**
- Major cloud providers (Route53, Cloudflare, Akamai) support GeoDNS via proprietary interfaces
- Open-source: PowerDNS, Knot DNS support GeoDNS plugins
- Can chain with weighted routing: 80% to preferred DC, 20% to secondary

**Latency optimization**: GeoIP databases have ~100-200ms accuracy (city-level). Not precise; provides "nearby" server but not optimal. Combined with anycast (network-layer optimization) for best results.

**Failure mode**: If nearby DC is overloaded/down, customer still routed there; fails instead of failover. Needs explicit health checks + failover logic.

### Weighted DNS Routing

Return same name with multiple IPs, each weighted:

```
api.example.com A 10.0.0.1    (weight 70%)
api.example.com A 10.0.0.2    (weight 30%)
```

Resolver returns both IPs; client picks one. By returning IPs with different frequencies, can approximate load balancing (70% of clients use IP1, 30% use IP2).

**Use case:** Canary deployments. Route small percentage to new version via weighted DNS.

**Limitations:**
- Resolver behavior unspecified; not all implement weighted round-robin
- Requires client retry logic if chosen IP fails
- No feedback to DNS (if IP is overloaded, DNS doesn't know)

**Better approach:** Use application-level load balancer instead (more predictable).

### DNS Failover: Active-Passive

Return primary IP; if unreachable, failover to secondary.

**Passive monitoring:**
```
api.example.com A 10.0.0.1    (primary; checked externally every 30s)
api.example.com A 10.0.0.2    (secondary; used if primary unreachable)
```

External health checker queries primary endpoint; if fails, removes from DNS response and returns only secondary. Time to failover = health check interval + client TTL cache expiry. Typical: 30s check + 300s TTL = 5+ minutes to failover.

**Trade-off:** Longer TTL = fewer DNS queries (lower load), but longer failover time. Typical: TTL 300 seconds.

**Example workflow:**
1. Primary DC fails at t=0
2. Health checker detects at t=30 (30s check interval)
3. Removes primary from DNS at t=30
4. Client queries DNS at t=timeNext (depends on old TTL)
5. Receives secondary IP
6. Client switches at t=timeNext

Total failover time: 30s + 300s (worst case) = ~330s, often shorter if client's TTL expires sooner.

### Anycast: Network-Layer Routing

Anycast is **network layer**, not DNS layer. Multiple servers announced with same IP; BGP routes each request to nearest.

**Setup:**
```
3 data centers announce 203.0.113.0/24 via BGP
Data Center NYC: AS12345
Data Center London: AS54321
Data Center Tokyo: AS99999
```

**How it works:**
1. Each DC announces the same prefix to its upstream ISP
2. ISPs propagate announcements; routers see multiple paths to same IP
3. Routers prefer shortest AS path (fewest hops through autonomous systems)
4. Each request routes to nearest DC

**Advantages over GeoDNS:**
- Automatic (no GeoIP database required)
- Works at network layer (transparent to applications)
- Asymmetry: Request routes to one DC, response might route differently (acceptable for most use cases)

**Disadvantages:**
- Requires BGP setup (complex; requires AS number, upstream ISP coordination)
- AS path doesn't always reflect geography (may not be "closest" geographically)
- High operational bar for small teams

**Common deployment:** Root nameservers (13 root IPs, ~100+ server locations globally using anycast).

## DNS Service Discovery (SRV Records)

**SRV record**: Contains service name, protocol, port. Allows clients to discover service endpoints dynamically.

```
_http._tcp.example.com SRV 10 50 80 server1.example.com
_http._tcp.example.com SRV 10 50 80 server2.example.com
_ldap._tcp.dc._msdcs.example.com SRV 0 100 389 dc1.example.com
```

**Format:** `_service._proto.name SRV priority weight port target`

- **Priority**: Lower = higher priority. Client tries priority 0 first, then 1, etc.
- **Weight**: Load balancing within priority. Higher weight = more requests.
- **Port**: Service port (80 for HTTP, 389 for LDAP, etc.)
- **Target**: FQDN of server host

**Kubernetes Service Discovery Example:**
```
_http._tcp.myapp.default.svc.cluster.local SRV 10 100 8080 pod-1.myapp.default.svc.cluster.local
```

Kubernetes DNS automatically populates SRV records for services.

**Client workflow:**
1. Query `_http._tcp.example.com SRV`
2. Receive multiple records with priority/weight
3. Pick highest priority (or by weight within same priority)
4. Query target hostname for A record (server IP)
5. Connect to IP:port

**Use cases:**
- Database failover (priority determines primary vs replica)
- Load balancing across instances
- Protocol negotiation (which backend handles this protocol?)

## DNS Failover with Health Checks

**Health-based failover** = actively monitor endpoints; remove failed ones from DNS.

**Implementation:**
1. External service (Route53 health checks, Cloudflare edge) periodically queries endpoint (HTTP GET, TCP, UDP probe)
2. If endpoint responds, mark as healthy; include in DNS
3. If endpoint fails N times, mark unhealthy; remove from DNS
4. Restore if endpoint recovers

**Configuration:**
```
Endpoint: api1.example.com:443
Protocol: HTTPS
Path: /health
Interval: 10 seconds
Failure threshold: 2 (fail if 2 consecutive checks fail)
Success threshold: 1 (recover if 1 check succeeds after failure)
```

**Failover time:** Health check interval × failure threshold + client TTL

Example: 10s interval × 2 failures + 300s client TTL = ~320s (5 minutes). Faster with lower TTL (but higher load).

**Failure modes:**
- **False positives**: Health check detects failure when actually healthy (e.g., flaky network). Increases failover churn.
- **Health check endpoint different from actual service endpoint**: /health endpoint works, but main service is down (rare but possible).

Mitigation: Test health checks regularly; ensure they accurately reflect service health.

## DNS Over HTTPS (DoH) and DNS Over TLS (DoT)

Standard DNS queries unencrypted; ISP or network observer can see which sites visited.

**DoH**: DNS queries over HTTPS
```
POST https://dns.google/dns-query
Content-Type: application/dns-message

[binary DNS query]
```

Encrypted end-to-end; ISP can't see query content (only sees connection to DNS provider).

**DoT**: DNS queries over TLS
```
TCP port 853
TLS handshake
Send DNS query in TLS tunnel
```

Same privacy; different transport. DoH uses HTTP/2 multiplexing; DoT dedicated TCP connection.

**Client support:**
- Modern browsers: Support DoH (can configure default DNS provider)
- OSes: macOS, Windows, Linux increasingly support DoH/DoT
- Applications: If app uses OS DNS resolver, inherits DoH/DoT

**Operational implication:** Network-based DNS centralization harder (can't intercept DoH/DoT). Must trust ISP/DNS provider for privacy.

**Privacy caveats:** DNS provider still sees all queries; can log them. Choose trusted provider (Cloudflare, Quad9, Mozilla offer public DoH) or run own resolver.

## DNSSEC: Cryptographic Protection

DNSSEC adds cryptographic signatures to DNS responses. Client verifies response authenticity.

**How it works:**
1. Zone owner generates signing key (Zone Signing Key, ZSK)
2. Zone owner signs all records (A, MX, CNAME, etc.) with ZSK
3. Client queries; receives record + signature
4. Client verifies signature with ZSK
5. Client verifies ZSK itself (signed by Key Signing Key, KSK)
6. Client verifies chain up to root

**Record types:**
- **RRSIG**: Signature of record set
- **DNSKEY**: Public signing key
- **DS**: Digest of KSK (used to certify KSK in parent zone)

**Deployment:**
- Zone owner enables DNSSEC in zone (sign all records)
- Registrar installs DS records (KSK digest) in parent zone
- Resolver validates chain

**Advantages:**
- Prevents DNS poisoning (attacker can't forge response; signature won't validate)
- Prevents cache poisoning (malicious records rejected)

**Disadvantages:**
- Operational complexity (key rotation, signing)
- Larger DNS responses (signatures add bytes; 1kb → 3-4kb per query)
- Resolver must support DNSSEC (not all do; ISP resolvers often don't)

**Adoption:** ~5-10% of domains support DNSSEC. Gradual increase but not mainstream.

## Subdomain Delegation

Divide large zones by delegating subdomains to separate authoritative servers.

**Example:**
```
example.com nameservers: ns1.example.com, ns2.example.com
api.example.com nameservers: ns1.api.example.com, ns2.api.example.com
```

When resolver queries `api.example.com`:
1. Query example.com NS (gets ns1.example.com)
2. Query ns1.example.com for api.example.com (gets referral to ns1.api.example.com)
3. Query ns1.api.example.com for api.example.com (gets answer)

**Use case:** Separate teams manage subdomains. Team API owns `api.example.com`, Team CDN owns `cdn.example.com`.

**Automation:** Terraform, CloudFormation can manage delegated zones and nameserver updates.

## DNS Record Types (Deep Dive)

### A (IPv4) and AAAA (IPv6)

**A**: IPv4 address
```
example.com A 93.184.216.34
```

**AAAA**: IPv6 address
```
example.com AAAA 2606:2800:220:1:248:1893:25c8:1946
```

Modern deployments should include both. Dual-stack (both A + AAAA) allows IPv4-only and IPv6-only clients to reach service.

### CNAME (Canonical Name)

Alias; maps one name to another:
```
www.example.com CNAME example.com
```

**Gotcha**: Can't use CNAME with other records at same level. This fails:
```
example.com CNAME other.com
example.com MX 10 mail.example.com  # Invalid: CNAME + MX at same level
```

**Solution**: Use ALIAS (AWS Route53) or root-record handling (Cloudflare).

### MX (Mail Exchange)

Specifies mail server for domain:
```
example.com MX 10 mail1.example.com
example.com MX 20 mail2.example.com
```

Priority: Lower = preferred. If mail1 down, try mail2 (priority 20).

### TXT (Text Records)

Arbitrary text; used for verification and security:
```
example.com TXT "v=spf1 ip4:192.0.2.1 include:_spf.google.com ~all"  (SPF)
example.com TXT "v=DKIM1; p=MIGfMA0BgQ..."  (DKIM public key)
_acme-challenge.example.com TXT "..." (ACME DNS validation)
```

**Common uses:**
- SPF (Sender Policy Framework): Prevent email spoofing
- DKIM (DomainKeys Identified Mail): Digitally sign emails
- ACME challenge: Prove domain control for Let's Encrypt

### SRV (Service)

Service locator (covered in DNS Service Discovery section above).

### CAA (Certification Authority Authorization)

Specifies which CAs can issue certificates for domain:
```
example.com CAA 0 issue "letsencrypt.org"
example.com CAA 0 iodef "mailto:security@example.com"
```

CA must check CAA before issuing; prevents unauthorized issuance (e.g., attacker buys cert from rouge CA).

**Practical impact:** Let's Encrypt, DigiCert, others check CAA. Set CAA to permit only trusted CAs.

### NAPTR (Naming Authority Pointer)

Advanced record type for service registration and protocol selection:
```
example.com NAPTR 100 10 "u" "E2U+sip" "!^.*$!sip:info@example.com!" .
```

Maps domain to service URI (SIP, mailto, etc.). Rarely used; mostly historical.

## TTL (Time To Live) Selection

TTL controls cache lifetime. Recursive resolvers cache responses; short TTL = faster updates, higher load.

$$\text{Query frequency} \propto \frac{1}{\text{TTL}}$$

**Common values:**
- **300 seconds (5 min)**: Standard. Balance between cache benefit and update latency.
- **60 seconds (1 min)**: Fast failover scenario. Higher resolver load.
- **3600 seconds (1 hour)**: Stable endpoints. Lower load; slower to update.
- **86400 seconds (1 day)**: Rarely changes (e.g., company website). Minimal load.

**Trade-off:** Lower TTL = faster failover, higher resolver load. For failover, combine with active health checks; don't rely solely on TTL.

## Common Failure Modes and Debugging

**Propagation delay**: New DNS records take 24-48 hours to propagate globally (some resolvers cache longer than TTL).

**Stale cache**: Resolver caches response beyond TTL (misconfiguration). Old IPs returned until cache invalidated.

**NXDOMAIN loops**: Typo in zone file (e.g., missing record). Resolver returns NXDOMAIN (not exists). Debugging: `dig @ns1.example.com missing-record.example.com`.

**Circular delegation**: Nameserver for delegated zone points back to parent (circular reference). Causes infinite loop. Check: `dig +trace`.

**No authoritative nameserver**: Domain owner misconfigured NS records. No records returned. Check: `dig ns example.com`.

Debug tool: `dig +trace` shows full resolution path; helps identify where things break.

## See Also

[infrastructure-dns-architecture.md](infrastructure-dns-architecture.md), [networking-dns.md](networking-dns.md), [infrastructure-dns-security.md](infrastructure-dns-security.md), [infrastructure-certificate-management.md](infrastructure-certificate-management.md)