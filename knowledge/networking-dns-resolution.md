# DNS Resolution Deep Dive — Recursive, Iterative, Caching, EDNS, DoH/DoT, Anycast & Attack Surface

## Overview

DNS resolution is the algorithmic process that translates domain names to IP addresses. Beyond the hierarchical architecture (see `networking-dns.md`), this note covers the mechanics of resolution queries, caching strategies, encrypted transports, public resolver trade-offs, and amplification attack vectors.

## Recursive vs. Iterative Resolution Mechanics

**Recursive Resolution** is the dominant client-facing pattern. The stub resolver (on your computer) sends a single query to a recursive resolver, which then coordinates with root, TLD, and authoritative servers on behalf of the client. The recursive resolver must eventually return an answer or a *negative* answer.

```
Stub Resolver
  │ Query: "What is example.com?" (recursive)
  ▼
Recursive Resolver (ISP or public: 8.8.8.8, 1.1.1.1)
  │ Query: "Where is example.com?" (iterative)
  ▼ Referral
Root Nameserver
  │ Query: "Where is example.com?" (iterative)
  ▼ Referral
TLD Nameserver (.com)
  │ Query: "Where is example.com?" (iterative)
  ▼ Answer
Authoritative Nameserver (example.com's server)
  │ Returns: IP address
  ▼
Recursive Resolver (caches result)
  │ Returns: IP address (to stub)
  ▼
Stub Resolver
```

**Iterative Resolution** happens server-to-server. Each queried server either provides the answer or a referral. This is more efficient for resolvers: they maintain connection pools to root/TLD servers and parallelize queries. Root servers, for example, don't cache user data; they only refer queries.

Key difference: A recursive resolver *must answer* (or error); an iterative server may say "go ask them instead." This shapes deployment: recursive resolvers operate for specific clients (ISPs, enterprises); authoritative servers operate globally.

## Resolver Architecture: Stub, Recursive, Authoritative

A **stub resolver** is the minimal client library on your OS (libc's `getaddrinfo()`, Windows' `GetAddrInfoW()`). It knows one upstream recursive resolver and delegates all queries to it. No caching (usually; some stubs implement small caches).

A **recursive resolver** is the workhorse. It:
- Receives queries from millions of stub resolvers (ISP resolver, public resolver)
- Maintains caches to avoid repeated root/TLD queries
- Handles DNSSEC validation (if enabled)
- Implements timeouts, retries, and failure fallback
- Uses query optimization (parallel waterfall: query root + TLD + authoritative simultaneously or sequentially depending on availability)

An **authoritative nameserver** answers queries only for zones it owns. It does not resolve; it does not cache user data. It tells the truth about its zone and refuses to answer outside its zone (via NXDOMAIN).

## DNS Caching: TTL, Negative Caching, Cache Hierarchy

**TTL (Time To Live)** is a record metadata field specifying how long a result remains valid. When a resolver caches an answer, it stores the TTL and counts down. Upon expiry, the resolver must re-query the authoritative server.

```
ttl=300 → 5 minutes cached
ttl=3600 → 1 hour cached
ttl=86400 → 1 day cached
```

TTL trade-offs:
- **Low TTL (1-300s):** Rapid propagation of zone changes, high authoritative server load
- **High TTL (86400+):** Reduced server load, slow propagation, higher failure impact (if the authoritative server goes down, cached entries become stale)

**Negative Caching** caches the *absence* of a record (NXDOMAIN: name does not exist; NODATA: name exists but no A record). SOA records include a field (minimum TTL) specifying how long to cache negative answers, often shorter (hours) than positive caches (days).

Negative caching prevents resolver thrashing: without it, each query for a nonexistent domain would recurse all the way to the authoritative server, which would waste bandwidth and CPU.

**Cache Hierarchy**:
```
Stub Resolver (local cache, small)
  │
ISP Recursive Resolver (large cache, millions of users)
  │
Root server cache (minimal; mainly refers)
```

The ISP recursive resolver is the critical cache layer. Its hit rate directly determines authoritative server load. Queries that miss force the resolver to traverse the full hierarchy, incurring RTT delays (typically 100-500ms).

## EDNS: Extended DNS

EDNS (RFC 6891) adds metadata to DNS queries and responses without changing the basic protocol.

**EDNS Client Subnet (ECS)** attaches the client's subnet (not full IP; usually /24 or /56) to the query. Authoritative servers use this to serve location-aware answers:

```
Query: "What is example.com?" + ECS: "192.0.2.0/24"
Response: IP address geolocation-optimized for 192.0.2.0/24
```

Why: CDNs use ECS to route clients to nearby edge servers. Without ECS, the resolver's IP is used (ISP location is wrong). ECS enables fine-grained geo-routing.

Trade-off: ECS reduces cache hit rates (same domain, different subnets = different answers). Public resolvers typically disable ECS for privacy; authoritative servers lose location context.

**Large UDP Payloads** via EDNS allow responses > 512 bytes. Modern DNS uses UDP 4096 bytes, reducing reliance on TCP fallback. TCP DNS is slower (TCP handshake overhead).

## DNS over HTTPS (DoH) and DNS over TLS (DoT)

Traditional DNS is plaintext. ISPs, firewalls, and VPNs can eavesdrop on all DNS queries, revealing browsing history.

**DoH (RFC 8484)** tunnels DNS queries over HTTPS:
```
Client HTTPS POST /dns-query?dns=<base64-encoded-query>
  ↓
DoH Server TLS termination
  ↓
DoH Server queries authoritative servers (same as normal DNS)
  ↓
Response over HTTPS (encrypted)
```

Advantages:
- Encrypted queries (privacy from ISP, middleboxes)
- Indistinguishable from other HTTPS traffic (if using a shared DoH endpoint like DNS.Google)
- Easier for browsers to implement (existing HTTPS stack)

Disadvantages:
- Slower: HTTPS overhead (TLS handshake) before each query
- Loss of cache efficiency: If DoH endpoint is shared by millions, each recursive resolver must forward its own queries, bypassing ISP cache
- Breaks local network DNS: queries to internal names must still reach the ISP resolver

**DoT (RFC 7858)** is similar but uses TLS over a dedicated port (853) or opportunistic upgrade. Slightly more efficient than DoH (no HTTPS overhead), but queries are distinguishable from other traffic.

**Practical deployment:**
- Browsers (Chrome, Firefox) support DoH; you configure a server (default: CloudFlare, Google)
- Operating systems increasingly support DoT (Windows 11, Android 9+)
- ISP resolvers and corporate networks may intercept DoH/DoT by blocking port 853 or requiring certificate pinning to ISP servers

## Public Resolvers: 8.8.8.8, 1.1.1.1, Quad9, others

**Google Public DNS (8.8.8.8)**:
- Global anycast network, low latency
- Good cache hit rates, high reliability
- Collects query metadata (privacy concern; read Google's privacy policy)
- DNSSEC validation enabled

**CloudFlare 1.1.1.1**:
- Fast anycast network, similar to Google's
- Privacy-focused (claims no query logging)
- Integrated filtering (malware, adult content options)
- DoH and DoT support

**Quad9 (9.9.9.9)**:
- Nonprofit, focused on blocking malware/phishing domains
- Community-driven, transparent operation
- DNSSEC validation

**Choosing resolvers:**
- **Speed:** All major resolvers are fast (< 10ms in most regions)
- **Privacy:** CloudFlare claims no logging; Google logs with anonymization
- **Reliability:** Google has better global coverage; CloudFlare is catching up
- **Filtering:** Quad9 blocks malware; CloudFlare offers optional filtering

## Anycast DNS: Routing and Scaling

**Anycast** (RFC 1546) is an IP routing technique. The same IP address is announced from multiple geographic locations. The internet's routing protocol (BGP) directs queries to the closest location (by hop count or latency).

```
8.8.8.8 announced from:
  - Mountain View, CA
  - Dublin, Ireland
  - Singapore
  - São Paulo, Brazil
  - ... (50+ locations worldwide)

Query from London: BGP routes to Dublin (closest)
Query from Sydney: BGP routes to Singapore (closest)
```

Advantages:
- **Automatic scaling:** Add resolver instances in new regions; BGP handles routing
- **Discovery:** Clients don't know the instance; BGP does the work
- **Fault tolerance:** If one location fails, BGP re-routes traffic within seconds

Disadvantages:
- **Sticky routing:** A query may go to one instance; a follow-up to another (cache desynchronization)
- **BGP complexity:** Misconfigurations cause global outages (rare but notable incidents)
- **Latency variance:** Same domain may resolve with different latencies depending on routing changes

## DNS Amplification Attacks

A **DNS amplification attack** is a volumetric DDoS exploiting DNS's reflexive property: a small query triggers a large response.

```
Attacker (spoofing victim's IP):
  Query: "ANY example.com?" (small: ~40 bytes)
  Sent to: Open recursive resolver (misconfigured)

Open Resolver:
  Receives query with spoofed source (victim IP)
  Responds with all records (large: ~4KB typical; up to 64KB possible)
  Sends to: Victim (amplification: 100-1000x)

Victim receives:
  Thousands of resolver responses per second
  Network flooded; unavailable
```

Mitigation:
- **Rate-limiting:** Resolvers limit responses to any single source
- **Source validation:** Modern resolvers require DNSSEC or skip open recursion
- **Response size caps:** Limit responses to 512 bytes (pre-EDNS) or 1280 bytes (target)
- **Access control:** Recursive resolution only from known clients (not open internet)

Most modern resolvers block DNS amplification by requiring legitimate recursive resolution (not open to random queries). ISPs and hosting providers filter outgoing DNS responses from spoofed sources.

## See Also

- `networking-dns.md` — DNS hierarchy, record types, failure modes
- `infrastructure-dns-security.md` — DNSSEC, DNS filtering
- `infrastructure-dns-architecture.md` — Large-scale DNS deployment
- `api-authentication.md` — Identity and authentication (zero-trust DNS)