# Domain Name System (DNS) — Resolution Mechanics, Security & Failure Modes

## Overview

DNS is the distributed hierarchical system that translates human-readable domain names into IP addresses. It is a critical infrastructure component: its failure disrupts the internet; its misuse enables cache poisoning, DNS hijacking, and information disclosure. Understanding DNS means understanding both its elegant delegation architecture and its failure modes.

## The DNS Hierarchy: Root, TLD, and Authoritative Servers

DNS uses a tree-structured hierarchy to distribute the authority for name resolution. At the top sits 13 root servers (operated globally), which know where the TLD (Top-Level Domain) servers live. TLD servers know where authoritative servers for specific domains are located.

```
User's Computer (stub resolver)
    ↓ (recursive query)
ISP Resolver (recursive resolver)
    ↓ (iterative query)
Root Nameserver ("I don't know, but TLD servers do")
    ↓ (iterative query)
TLD Nameserver (.com, .org, .uk, etc.)
    ↓ (iterative query)
Authoritative Nameserver (example.com's DNS server)
    ↓ (returns IP address)
ISP Resolver (caches result)
    ↓ (returns IP to client)
User's Computer
```

## Recursive vs. Iterative Resolution

These terms describe query behavior, not server types:

**Recursive Resolution**: The resolver says "I'll find the answer for you." The client's resolver sends a query to its ISP resolver, which then queries root, TLD, and authoritative servers on behalf of the client. The client waits for a complete answer. This is what happens in practice for end-user queries.

**Iterative Resolution**: The server says "Here's a referral; go ask them." The root server refers you to the TLD server; the TLD server refers you to the authoritative server. Each referrer provides a pointer but not the final answer. Iterative queries happen internally, between resolvers and authoritative servers.

## DNS Record Types

DNS stores different types of records, each serving a specific function:

| Record Type | Purpose | Example |
|---|---|---|
| **A** | Maps domain name to IPv4 address | `example.com → 93.184.216.34` |
| **AAAA** | Maps domain name to IPv6 address | `example.com → 2606:2800:220:1:248:1893:25c8:1946` |
| **CNAME** | Canonical name (alias) | `www.example.com → example.com` |
| **MX** | Mail exchange server | `example.com → mail.example.com` (priority 10) |
| **NS** | Nameserver delegation | `example.com → ns1.example-dns.com` |
| **SOA** | Start of Authority (zone metadata) | Serial, refresh, retry, expire, TTL |
| **TXT** | Arbitrary text (used for SPF, DKIM) | `v=spf1 include:example.com ~all` |
| **SRV** | Service record | `_service._proto.name → host:port` |
| **PTR** | Pointer (reverse DNS) | `34.216.184.93.in-addr.arpa → example.com` |

The SOA record contains zone metadata: serial number (for zone transfers), refresh interval (when secondaries query), retry interval (if refresh fails), expire threshold (when zone is stale), and default TTL for the zone.

## TTL and Caching

**Time-to-Live (TTL)**: A value (in seconds) attached to DNS records that tells resolvers how long to cache the record before re-querying the authoritative server.

- **Low TTL** (300 seconds): Allows fast updates but increases query load on authoritative servers.
- **High TTL** (86400 seconds, 24 hours): Reduces query load but slows propagation of DNS changes.

Caching happens at multiple levels:

1. **Authoritative server cache** (answers queries)
2. **Recursive resolver cache** (ISP DNS, 8.8.8.8, Cloudflare)
3. **Stub resolver cache** (client OS, browser)

When a record's TTL expires, the resolver forgets it and must query again.

## DNSSEC — Cryptographic Integrity

DNS is vulnerable to cache poisoning and man-in-the-middle attacks because traditional DNS queries are unsigned. DNSSEC (DNS Security Extensions) adds cryptographic signatures to DNS records, creating a chain of trust from the root down to the authoritative server.

**How DNSSEC Works**:

1. **DNSKEY Records**: The zone publishes public keys used to verify records
2. **RRSIG Records**: Resource Record SIGnatures cryptographically sign record sets
3. **DS Records**: Delegation Signer records in the parent zone verify the child's DNSKEY
4. **Chain of Trust**: Root signs TLD's DS record; TLD signs domain's DS record; domain signs its own records

A DNSSEC-validating resolver performs signature verification at each level. If any signature fails, the resolver rejects the entire response (SERVFAIL). This prevents cache poisoning but adds latency and complexity.

**Trade-offs**:
- Blocks certain legitimate DNS tricks (e.g., DNS-based load balancing via random AAAA responses)
- Increases packet size (signatures are large; requires DNS over TCP or EDNS0)
- Zone transfers must be signed; operational complexity increases
- Not universally deployed (adoption is ~30% of TLDs as of 2026)

## DNS over HTTPS (DoH) and DNS over TLS (DoT)

Traditional DNS queries are sent in plaintext over UDP port 53. This allows passive observation: your ISP, network operator, or MitM attacker can see which domains you visit.

**DNS over HTTPS (DoH, RFC 8484)**: DNS queries are wrapped in HTTPS requests and sent over port 443 (TLS 1.3).

```
Traditional DNS:
Client → [nameserver.com] UDP:53 → Resolver → Root → TLD → Auth
(visible to network)

DoH:
Client → [nameserver.com/dns-query] HTTPS POST → Resolver
(encrypted; looks like regular HTTPS traffic)
```

**DNS over TLS (DoT, RFC 7858)**: DNS queries are sent over TLS on port 853. Similar encryption but uses a separate protocol, not piggy-backed on HTTP.

**Benefits of DoH/DoT**:
- Encrypts queries from observation
- Authenticates the DNS server (TLS certificate validation)
- Hides DNS traffic from network analysis

**Trade-offs**:
- Centralization risk: users route traffic to a single DoH provider (Cloudflare, Google) rather than their ISP resolver. These providers see *all* your queries.
- DNS server selection bypasses local network policies (enterprises cannot intercept).
- Increased latency (TLS handshake, HTTPS overhead).
- Not a cure for ISP eavesdropping; the ISP still sees that you connected to a DoH provider.

## Common DNS Failure Modes

### 1. Cache Poisoning

An attacker injects a false DNS response into a resolver's cache. Subsequent queries return the attacker's IP (e.g., phishing site instead of the real one).

**Example**: Attacker sends a fake response claiming `example.com → attacker.com`. If the resolver accepts it and caches it, all clients see the attacker's IP.

**Mitigations**:
- DNSSEC validation (rejects unsigned responses)
- Source port randomization (responses must come from the correct port)
- DNS ID randomization (responses must match the query ID)
- Rate limiting on resolvers

### 2. DNS Hijacking

An attacker controls the authoritative nameserver or intercepts zone transfers. All queries for that domain return the attacker's IP.

**Mitigations**:
- DNSSEC
- AXFR (zone transfer) restrictions
- Access control on nameserver updates

### 3. TTL Expiration and Stale Data

If the authoritative server goes down and caches expire, clients cannot reach the domain. If a migration changes IP addresses and the old TTL is high, clients may see stale IPs for hours.

**Mitigation**: Choose appropriate TTL based on change frequency. Low TTL before migrations; high TTL for stable addresses.

### 4. DNS Amplification DDoS

An attacker sends a small query to a public DNS resolver, spoofing the source IP to be the victim's IP. The resolver responds with a large answer (often 20–100× larger than the query). Multiply this by thousands of resolvers, and the victim is flooded.

**Mitigation**: 
- EDNS0 cookie (spoofing prevention)
- Rate limiting
- Firewall filtering of DNS from untrusted sources

### 5. Single Points of Failure

If a zone has only one authoritative nameserver and it fails, no one can resolve the domain. If a recursive resolver is down, all its clients are DNS-blind.

**Mitigation**:
- Deploy multiple authoritative nameservers in different geographic regions
- Use diverse recursive resolvers or operate your own
- Monitor resolver availability

## DNS Protocol Mechanics

### Query Structure

A DNS query consists of:
- **Header**: Query ID (16-bit), flags (recursive bit, etc.), counters
- **Question Section**: Domain name, record type (A, AAAA, MX, etc.), class (IN for internet)
- **Answer Section**: (empty for queries)

### Response Structure

A DNS response contains:
- **Header**: Same query ID, flags set (response, authoritative, truncation, etc.)
- **Question Section**: Echo of the client's question
- **Answer Section**: The requested records
- **Authority Section**: Nameservers for the zone
- **Additional Section**: A records for nameservers (for convenience)

### UDP vs. TCP

DNS typically uses UDP port 53. UDP is fast but has a 512-byte limit (512 bytes for answer size in traditional DNS). If the response is larger (e.g., DNSSEC, many records), the resolver truncates it and the client retries over TCP.

**EDNS0** (Extension Mechanisms for DNS) allows larger UDP payloads (up to 4096 bytes), reducing TCP fallback. However, large responses can still trigger rate limiting or fragmentation issues.

## Caching Layers and the Cascading Query

When a client queries `www.example.com`, here's what typically happens:

1. **Stub resolver (client OS)** checks its cache; miss
2. **Stub resolver** sends recursive query to **ISP resolver** (8.8.8.8, 1.1.1.1, etc.)
3. **ISP resolver** checks *its* cache; miss
4. **ISP resolver** sends iterative query to a **rootserver** (e.g., a.root-servers.net)
5. **Root server** responds: "I don't know, but here are the .com TLD servers"
6. **ISP resolver** queries a **TLD server**
7. **TLD server** responds: "I don't know, but here are the nameservers for example.com"
8. **ISP resolver** queries an **authoritative server** (e.g., ns1.example.com)
9. **Authoritative server** responds with the A record for www.example.com
10. **ISP resolver** caches the response (respecting TTL) and returns it to the stub resolver
11. **Stub resolver** caches and returns to the application
12. **Application** opens a TCP connection to the IP address

If any step is slow or fails, the entire lookup stalls.

## Operational Considerations

- **TTL before changes**: Lower TTL 1–2 hours before IP migrations so old caches expire quickly.
- **Zone transfers**: Restrict AXFR to secondary nameservers; use TSIG (authenticated zone transfers).
- **Resolver monitoring**: Track query latency, SERVFAIL rate, cache hit rate.
- **DNSSEC adoption**: Weighs security vs. complexity; adoption is increasing but not universal.
- **DoH deployment**: Enterprise may block DoH to enforce local DNS filtering; home users benefit from privacy.

## Related Concepts

- **networking-tcp-ip**: The Internet Layer (IP) where DNS responses arrive
- **networking-protocols**: Higher-level perspective on DNS among application protocols
- **security-cryptography**: DNSSEC and DoH use public-key cryptography