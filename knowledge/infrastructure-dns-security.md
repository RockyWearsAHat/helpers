# DNS Security — DNSSEC, Encryption, Filtering & Attack Mitigation

## Overview

DNS security addresses two distinct concerns: **authentication** (verifying that DNS responses come from the authoritative source and haven't been tampered with) and **privacy** (preventing eavesdroppers from observing which domains users are querying). Traditional DNS is vulnerable to both—responses are unsigned and queries traverse the Internet in plaintext. This layer summarizes DNSSEC (authentication) and transport-layer security (DoH, DoT, DoQ).

## DNSSEC: Signing and Chain of Trust

DNSSEC adds cryptographic signatures to DNS records, enabling resolvers to verify that records were published by the authoritative server and not forged in transit.

### Public Key Cryptography Model

Each zone is signed with a **Zone Signing Key (ZSK)**, typically an ECDSA or RSA key. The public key is embedded in a **DNSKEY** record and published by the zone's authoritative server. Resolvers retrieve this key and use it to verify **RRSIG** (Resource Record Signature) records, which cryptographically sign the zone's data. The zone owner keeps the private key secure; it signs the records offline or via an HSM (Hardware Security Module).

### The Chain of Trust

DNSSEC resolves the bootstrap problem—how does a resolver trust the root zone's public key?—by building trust from the root downward:

1. **Root Trust Anchor**: Operationally maintained by IANA and pre-loaded in resolver software. This is the starting point.
2. **Delegation Signing (DS Records)**: When a parent zone delegates to a child (e.g., the .com TLD delegates to example.com), the parent includes a DS record describing the child's public key. The DS is signed by the parent, establishing the link.
3. **Validation Walk**: A resolver validates a domain by walking the chain: root → TLD → authoritative. At each step, it verifies the RRSIG against the parent's key.

```
Root Zone (pre-loaded trust)
  ↓ DS record for .com TLD
.com TLD Zone (signed)
  ↓ DS record for example.com
Authoritative Zone for example.com (signed)
  ↓ RRSIG records verify A records
A Records (e.g., 192.0.2.1)
```

**Broken Trust**: If any signature in the chain is invalid, or the child's key is not properly described by the parent's DS record, the resolver rejects the entire response. This is **DNSSEC failure** and may result in a SERVFAIL response instead of resolution (depending on resolver policy).

### Key Rollover

Keys expire and must be rotated for operational and security reasons. Key rollover is complex because resolvers store the trust anchor and DNS is eventually consistent:

- **Pre-signing**: Before rolling, the zone publishes both the old and new keys (dual-signing). Resolvers gradually see the new key in the zone.
- **Validator Learning**: Resolvers update their internal cache of the zone's new key through interactions with the authoritative server.
- **Grace Period**: TTLs on DNSKEY records are typically long (1 day–1 week) to prevent rapid key changes causing resolver confusion. During rollover, trust anchors coexist for the TTL duration, then the old key is retired.
- **KSK vs. ZSK**: The Key Signing Key (signs DNSKEY records themselves) is rolled less frequently than the Zone Signing Key (signs data records). Separating these reduces key exposure and simplifies rotation.

## DNS over HTTPS (DoH) and DNS over TLS (DoT)

Traditional DNS queries are sent over UDP/TCP port 53 in plaintext. DoH and DoT encrypt queries and responses using TLS, preventing eavesdroppers (ISPs, network operators, attackers on shared networks) from observing which domains users visit.

### DNS over TLS (DoT)

DoT wraps DNS queries in a TLS 1.2+ connection, typically over TCP port 853. Queries and responses are end-to-end encrypted. The resolver (recursive DNS server) presents a certificate; the client validates it.

- **Server Authentication**: The resolver certificate must be valid for its domain (e.g., `dns.example.com`). Clients perform standard TLS verification.
- **Connection Reuse**: Since TLS setup is expensive (full handshake ~100-300ms), DoT connections are persistent—multiple queries reuse the same TLS session.
- **Negotiation**: DoT is a point-to-point protocol; clients explicitly connect to a known DoT server (hardcoded or configured).
- **Network Observability**: DoT still reveals that DNS queries are happening (via port 853 traffic patterns) but not their content or frequency.

### DNS over HTTPS (DoH)

DoH embeds DNS queries inside HTTPS requests, tunneling them over standard HTTPS (port 443). From a network perspective, DoH traffic is indistinguishable from regular web browsing.

- **URI Integration**: Queries are encoded as HTTP GET or POST requests to a DoH URI (e.g., `https://resolver.example.com/dns-query?dns=...`).
- **Obfuscation**: A network observer sees TLS handshakes and HTTPS traffic but cannot determine if requests are DNS queries or normal web traffic.
- **Connection Pooling**: DoH reuses the HTTP/2 or HTTP/3 connection, reducing setup overhead compared to DoT.
- **Client Implementation**: Browsers, OS resolvers, and applications can directly use DoH without changing port configurations.

### Comparison: DoH vs. DoT

| Property | DoT | DoH |
|----------|-----|-----|
| Port | 853 | 443 |
| Transport | TLS (raw) | HTTPS (HTTP over TLS) |
| Network Visibility | DNS traffic recognizable | Indistinguishable from web traffic |
| Connection Reuse | TLS sessions | HTTP persistent connections |
| Setup Latency | ~200ms first query | ~200ms first query |
| Caching | Limited (per-connection) | HTTP cache headers supported |
| Adoption | Slower (requires resolver update) | Faster (browsers support natively) |

**Privacy Implication**: DoH better resists passive network surveillance (an ISP cannot log queries), but DoT is more defensible against active attacks requiring TLS session reconstruction.

## DNS Filtering and Sinkholing

DNS filtering intercepts queries for known-malicious domains and returns a sinkhole address (often 0.0.0.0 or a honeypot server), rather than the legitimate IP.

### Deployment Models

- **Enterprise Firewalls**: Intercept all outbound DNS (UDP/53 port) and apply custom policy. Users cannot bypass by using external resolvers unless they use DoH.
- **ISP-Level Filtering**: ISPs operate recursive resolvers and filter queries for domains they determine are malicious.
- **Endpoint Software**: Antivirus or DNS client software intercepts queries before they reach the network resolver.

### Attack Detection

Malware often queries known C2 (command-and-control) infrastructure, DGA (Domain Generation Algorithm) seeds, or phishing URLs. Filtering blocks these:

- **Threat Feeds**: Feed of known-bad domains (botnet C2, phishing, malware distribution) supplied by security vendors. Queries for these domains are blocked or rate-limited.
- **Behavioral Analysis**: Unusual query patterns (high volume to non-existent domains, repeated failures, querying private RFC 1918 addresses) trigger blocks or challenges.

**Circumvention**: Attackers evade filtering by:
- Using legitimate domains compromised or rented by the attacker (filter rules move slower than attacker rotation).
- Tunneling queries over HTTPS or other encrypted protocols.
- Querying through decentralized DNS systems like ENS or Unstoppable Domains (outside traditional filtering).

## DNS Rebinding

DNS rebinding exploits the assumption that a domain resolves to the same IP across multiple queries. An attacker controls a domain (example-attacker.com) and varies its DNS response:

1. Attacker's website (example-attacker.com) embeds JavaScript that queries `example-attacker.com` via the browser.
2. First query: the attacker's nameserver returns their server IP (legitimate).
3. Browser loads and executes the JavaScript.
4. JavaScript makes a second request to `example-attacker.com`; attacker's nameserver now returns a private IP (e.g., 192.168.1.1), pointing to the victim's router.
5. The browser, trusting the same origin (same domain), sends credentials and performs actions as if talking to the attacker's website, but actually targets the victim's local network.

### Defenses

- **DNS Pinning**: Browsers cache DNS results briefly; rebinding successful only if TTL expires between requests.
- **DNSSEC Validation**: If enabled, rebinding attempts often fail because the attacker's responses are unsigned or inconsistent with the DNSSEC chain.
- **Response Rate Limiting** (RRLimit): Authoritative servers rate-limit rapid queries to the same domain and different responses, slowing rebinding.
- **Hostname Allowlists**: Applications only trust specific, pre-approved domains, ignoring DNS results entirely.

## Split-Horizon DNS and Security Implications

Split-horizon DNS returns different results to internal vs. external queries. Example: querying `intranet.example.com` from within the corporate network returns a private IP (192.168.1.100); querying from the Internet returns an error or redirects.

**Security Tradeoff**: Attackers can probe split-horizon boundaries to map internal infrastructure:

```
nslookup intranet.example.com 208.67.222.222  # From external resolver → error
nslookup intranet.example.com 192.168.1.1     # From inside → 192.168.1.100
→ Attacker infers: 192.168.1.* is internal
```

Mitigations:
- Return consistent (dummy) IPs to external queries, not errors.
- Disable zone transfers or AXFR (full zone export).
- Monitor DNS logs for repeated failed queries (reconnaissance patterns).

## Response Policy Zones (RPZ)

RPZ allows DNS operators to insert custom policy at query-response time, independent of the authoritative zone. An operator maintains a meta-zone listing:

- Domains to block (sinkhole)
- Domains to redirect (NXDOMAIN, but retry with alternate responder)
- IP addresses to rate-limit

Example: A recursive resolver applies an RPZ from a threat intelligence feed, automatically blocking domains in that feed without modifying the authoritative zones.

## DNS Tunneling Detection

DNS is ubiquitous and often trusted, making it an exfiltration channel for malware. Clients tunnel traffic through DNS queries:

- **TXT Records**: Encode data in TXT record queries (`base64-data.attacker.com` where the attacker's nameserver receives the query and decodes it).
- **Subdomain Encoding**: Each subdomain label encodes bits of data.

**Detection**:

- **Query Anomalies**: Unusually long domain names, rapid-fire queries to random subdomains, non-ASCII characters in queries.
- **Query Volume**: Sustained high query rates to a single attacker-controlled domain.
- **Behavioral Baseline**: Deviations from typical query patterns (most users query few domains; tunneling generates unique queries).

Defenses are imperfect—distinguishing tunneling from legitimate long queries is difficult without false positives.

## Bootstrapping and Operational Complexity

DNSSEC and DoH/DoT add operational burden:

- **Maintenance**: Key expiry monitoring, rotation ceremonies, synchronization across authoritative replicas.
- **Troubleshooting**: Signature validation failures are hard to debug; resolvers may silently fail or return SERVFAIL.
- **Adoption Chicken-and-Egg**: DNSSEC's security gains require validators to actually validate; if most resolvers don't validate (due to legacy support or performance concerns), security is illusory.

Current state (2026): DNSSEC is widely implemented by major authoritative servers and root/TLD zones but validator adoption lags. DoH/DoT adoption is faster due to browser and OS support, though some enterprises block DoH for monitoring reasons.

See also: networking-dns, infrastructure-dns-architecture, security-network, web-browser-security.