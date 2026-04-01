# DNS Security — Threats, DNSSEC, Privacy, Filtering & Monitoring

## Overview

DNS operates in a fundamentally adversarial environment: the protocol was designed for trust within networks, not across the hostile Internet. Contemporary DNS security spans three orthogonal concerns: **authentication** (verifying responses originate from authoritative sources), **privacy** (preventing observation of queries), and **threat detection** (identifying malicious domains and blocking access). Each layer uses different mechanisms; no single layer suffices.

---

## Attack Surface: Poisoning, Rebinding, Tunneling

### Cache Poisoning (DNS Spoofing)

Cache poisoning exploits the unsigned nature of traditional DNS responses. An attacker on-path or upstream in the network sends forged DNS responses before the legitimate authority responds. If a resolver accepts the forgery, it caches the false mapping, serving poisoned data to all clients.

**Defenses:** Modern resolvers implement **transaction ID randomization** and **source port randomization** (RFC 5452). Together, these require an attacker to guess a 32-bit transaction ID plus a 16-bit port—dramatically raising the threshold. On-path attackers (or highly networked attackers) can still succeed, motivating the adoption of DNSSEC and encrypted transports.

### DNS Rebinding

DNS rebinding exploits browser same-origin policy by serving different IP addresses depending on the resolver's state. An attacker controls a domain and answers queries from the victim's application with one IP initially (the attacker's web server), then changes the response to point at an internal IP (the victim's router, CMS, etc.). The application, having cached the domain → IP mapping, believes the second address is same-origin and makes requests.

**Example:** Application learns attacker.com → attacker's-ip, makes connection. Then, DNS query for attacker.com returns → 192.168.1.1 (victim's router). Browser may allow the connection if it reuses the cached mapping.

**Mitigations:** Pinning IP-to-hostname mappings across TTL boundaries. Some applications DNS-rebind-resistant by validating that responses fall within expected CIDR blocks. Browsers increasingly validate Host headers against DNS results.

### DNS Tunneling

DNS queries and responses are sometimes used as a covert channel to exfiltrate data or tunnel traffic out of restrictive networks, because DNS is rarely blocked. A malware process encodes data in DNS query names and receives instructions in response. Large-scale detection requires behavioral analysis (unusually high volume, unusual domains, unusual query patterns for a host).

---

## DNSSEC: Chain of Trust & Validation

DNSSEC adds cryptographic signatures to DNS records, constructing a chain of trust from the root zone downward.

### The Signing Model

Each zone operator generates a **Zone Signing Key (ZSK)**, signs all records in the zone's apex with this key, and publishes the public ZSK in a **DNSKEY** record. Resolvers retrieve the DNSKEY and verify **RRSIG** (Resource Record Signature) records, confirming that only the zone operator could have signed the data.

DNSSEC uses ECDSA, RSA, or EdDSA. Signing is typically offline or HSM-backed to protect the private key. Smaller zones often sign at query time (on-the-fly signing) but larger zones pre-sign records and rotate ZSK periodically (every 6-12 months for stability).

### Chain of Trust Bootstrap

The root zone publishes DNSKEY records signed by the root. The root's public key is embedded in a **Key Signing Key (KSK)** and published separately. This creates a bootstrap problem: how does a resolver trust the root's public key?

**Solution:** The root KSK is operationally maintained by IANA and pre-loaded in resolver software (Bind, Unbound, systemd-resolved, etc.). The root also publishes a **Delegation Signer (DS)** record at the parent zone for each child. When resolving, a resolver checks the child zone's DNSKEY against the parent's DS record, verifying the chain link.

```
Root KSK (pre-loaded) → Root DS record (for .com) → .com DNSKEY 
    → .com DS record (for example.com) → example.com DNSKEY → example.com records
```

### Negative Assertions (NSEC)

DNSSEC must answer "does this record exist?" securely. Without negative assertions, an attacker could claim a record was removed when it wasn't. DNSSEC uses **NSEC** or **NSEC3** records, which cryptographically prove that a query returned a non-existent name or type. NSEC3 hashes names to obscure zone enumeration (useful for sensitive zones).

### Validation & Deployment Status

DNSSEC validation requires resolvers to verify signatures at each delegation level. **Validate at the resolver** (not at the stub client) to protect all downstream queries. DNSSEC adoption is roughly 30-50% of zones worldwide as of 2026; small zones lag, large enterprises often skip it due to key rotation complexity.

**Common issues:** Missed KSK rotation deadlines cause sudden validation failures. Misconfigured DS records lead to resolution failure for the entire zone.

---

## Privacy: DNS over HTTPS (DoH) & DNS over TLS (DoT)

Traditional DNS queries traverse the Internet in plaintext, exposing a user's browsing history to ISPs, networks, and eavesdroppers. DoH and DoT encrypt queries at the transport layer.

### DNS over HTTPS (RFC 8484)

DoH encapsulates DNS queries in HTTPS (over TLS 1.2+) to a resolver. Queries are formatted as HTTP GET or POST requests. Replies are DNS responses returned in the HTTP response body. To an observer, DoH traffic appears as generic HTTPS traffic, not revealing queried domains.

**Advantage:** Piggybacks on existing HTTPS infrastructure; often bypasses network censorship and pervasive monitoring.

**Disadvantage:** Centralizes DNS queries to fewer resolvers (Cloudflare, Google, etc.). Users lose the ability to use organizational/ISP caches. Creates a dependency on an external DNS provider's infrastructure and privacy policy.

### DNS over TLS (RFC 7858)

DoT uses TLS directly on port 853 to connect to a DNS resolver. Queries are standard DNS messages sent over an encrypted channel.

**Advantage:** Uses dedicated port and protocol; cleaner separation from HTTP. Better for infrastructure orchestration (direct TLS cert validation).

**Disadvantage:** Slightly higher latency than DoH due to separate connection. Easier to identify as DNS traffic (port 853 is distinctive).

### Trade-offs & Adoption

DoH and DoT prevent observation of queries by network-level eavesdroppers but do NOT prevent the resolver itself from logging queries. Large public resolvers (Cloudflare 1.1.1.1, Google 8.8.8.8) maintain privacy policies and log retention limits, but are not verifiable without auditing. Corporate and ISP resolvers often inspect encrypted traffic indirectly (via SNI extraction or subsequent HTTPS flow inspection).

DoH/DoT adoption is growing; browsers now default to DoH resolvers, and mobile platforms support DoT natively.

---

## RPZ: Response Policy Zones & Sinkholing

Response Policy Zones (RPZ, RFC 8646) allow DNS operators to intercept queries and return custom responses (block, sinkhole, redirect) based on rules.

### Use Cases

- **Malware sinkholing:** Intercept queries for known command-and-control domains, return the sinkhole IP (e.g., 127.0.0.1 or a monitoring server).
- **Abuse prevention:** Block queries for phishing domains, ransomware payloads, botnets.
- **Compliance:** Enforce corporate DNS filtering policies (block adult sites, prohibit DNS rebinding).
- **DDoS mitigation:** Redirect queries for victim zones to allow legitimate DNS resolution while preventing application-level attacks.

### Mechanism

An RPZ is a zone file containing DNS records representing policy rules. A resolver loads the RPZ and checks incoming queries against its rules. If a match occurs, the resolver returns the specified response instead of following normal resolution.

```
; RPZ example (sinkhole malware C&C)
command-and-control.example.com  A 127.0.0.1
*.malware-domain.net              A 127.0.0.1
```

### Limitations

RPZ operates at the DNS level only—it redirects DNS resolution but cannot prevent application-level exploitation. If an attacker hardcodes an IP address or uses alternate resolvers, RPZ is bypassed. Enterprise deployments often couple RPZ with internal DNS enforcement (DNS server as the only allowed resolver) and client-side DNS hijacking prevention.

---

## Monitoring & Threat Detection

### Query Pattern Analysis

Large-scale DNS monitoring detects anomalies by analyzing query patterns: volume, distribution, unusual domains, suspicious timing. Indicators include:

- **DGA (Domain Generation Algorithm) detection:** Queries for hundreds of pseudo-random domains (e.g., xkjdflk.com, ylskdhf.com). Botnets often generate lists of domains and try querying them; legitimate applications almost never do.
- **DNS tunneling:** Encoded data in DNS query names (very high entropy names or unusual lengths).
- **Exfiltration detection:** Bulk queries for sensitive internal domains or rapid resolution attempts for non-existent hosts (reconnaissance).
- **Botnet beaconing:** Regular, periodic queries for the same hostname from many hosts.

### Passive DNS Feeds

Passive DNS stores historical records of DNS queries and responses. Security teams query passive DNS to understand who queried for a domain, how long it resolved, and whether resolution failed. Used for incident response and threat attribution.

### DNS Sinkhole Monitoring

Sinkhole servers log connections to blocked domains. A spike in connections to sinkhole IPs indicates active malware, phishing, or DNS-based data exfiltration. Some enterprises monitor sinkhole logs in real time to detect mass infection events.

---

## Operational Concerns

### DDoS Amplification

DNS queries can be amplified by non-validating DNS forwarders. An attacker sends a spoofed DNS query (source IP spoofed to the victim's IP) to a public DNS forwarder requesting a large response (e.g., AXFR, ANY). The forwarder responds to the victim with 10–100× the request size, overwhelming the victim. Mitigation: operators should run closed resolvers (not forwarding for arbitrary clients), rate-limit responses, and disable ANY queries or restrict them to authorized clients.

### TTL Confusion

DNS TTL (Time To Live) specifies how long a resolver may cache a record. An attacker with control of a domain can set a very low TTL then, on the next renewal, change the domain's IP suddenly. However, if downstream resolvers or caches honor a higher TTL, the stale IP persists. No perfect solution—TTLs are hints, not guarantees. Defensive applications use multiple DNS resolvers or validate cached results.

---

## Related Topics

See also: [infrastructure-dns-security](infrastructure-dns-security.md), [networking-dns](networking-dns.md), [infrastructure-dns-patterns](infrastructure-dns-patterns.md), [security-network](security-network.md), [cryptography-key-management](cryptography-key-management.md).