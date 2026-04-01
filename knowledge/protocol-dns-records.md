# DNS Record Types — A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, CAA, and More

## Overview

DNS record types encode different kinds of information in the Domain Name System. Each record type (also called an RR type, or Resource Record type) answers a specific query:
- **A**: "What's the IPv4 address?"
- **MX**: "Where do I deliver mail?"
- **CNAME**: "What's the canonical name?"
- **TXT**: "What's the arbitrary text data?"
- And dozens more for specialized functions (security, service discovery, etc.)

Understanding record types is essential for DNS administration, troubleshooting, security configuration, and integrating DNS into systems and services. This note covers common and specialized record types, their use cases, interactions, and TTL strategies.

## Foundational Record Types

### A (Address) — IPv4 Hostname Resolution

Maps a domain name to an IPv4 address.

```
example.com.  3600  A  192.0.2.1
www.example.com.  3600  A  203.0.113.10
```

Queries for `example.com`, `@` or bare name resolve via A records. Multiple A records on the same name enable round-robin load balancing; resolver returns all IPs; client picks one. Simple, no intelligence—does not consider endpoint health.

### AAAA (IPv6 Address)

Maps a domain name to an IPv6 address (RFC 3596). Same role as A records, but for IPv6.

```
example.com.  3600  AAAA  2001:db8::1
```

Dual-stack: a domain can have both A and AAAA records. Resolvers return both (or separately, depending on client query type). Modern clients prefer IPv6 via Happy Eyeballs (RFC 8305): try IPv6, fall back to IPv4 after ~250ms.

**TTL strategy**: Typically same as A records (300–3600s). Long TTL reduces query load but increases failover time for IPv6-only services.

### CNAME (Canonical Name)

Alias one domain name to another (RFC 1035). CNAME is a *redirect at query time*: querying `alias.example.com → CNAME web.example.com → A 192.0.2.1`. Resolvers follow the CNAME and return the final A record.

```
www.example.com.  3600  CNAME  example.com.
api.example.com.  3600  CNAME  api.example.net.
```

**Constraints**:
- CNAME cannot coexist with other record types on the same name (except DNSSEC records). If `www.example.com` is a CNAME, you cannot have an A or MX record there.
- CNAME at the zone apex (`example.com.`) violates the CNAME+SOA coexistence rule; use ANAME or NS/glue if needed to alias a domain.
- CNAME indirection adds a query hop; long CNAME chains degrade performance.

**Use case**: CDN aliases (serve static content from a CDN's CNAME), shared infrastructure (multiple services aliasing to a load balancer).

### NS (Nameserver)

Delegates a subdomain (zone) to a nameserver. Authoritative for the domain lists NS records for the zone itself.

```
example.com.  3600  NS  ns1.example.com.
example.com.  3600  NS  ns2.example.net.
```

At zone cuts (e.g., `example.com` and `sub.example.com`), NS records tell resolvers "this nameserver is authoritative for the subdomain; query there." Glue records (A records for NS hostnames) may be included in the auth section to avoid circular lookups.

### SOA (Start of Authority)

Metadata for the zone; present on every zone apex:

```
example.com.  3600  SOA  ns1.example.com. admin.example.com. (
  2024032501  ; Serial
  3600        ; Refresh (secondary polls every 6 hours)
  1800        ; Retry (on failure, retry after 30 min)
  604800      ; Expire (secondary forgets zone after 7 days)
  86400       ; Minimum TTL (negative cache TTL)
)
```

Fields:
- **Primary NS**: `ns1.example.com` (the "master" for this zone)
- **Responsible email**: `admin.example.com` (@ → .)
- **Serial**: Incremented on zone changes; secondaries use this to detect updates (zone transfer trigger)
- **Refresh/Retry/Expire**: Zone transfer timing for secondary nameservers
- **Minimum TTL**: Used for negative caching (NXDOMAIN, NODATA) and as a default for all records

Lowering Minimum TTL increases negative cache hits (queries for non-existent subdomains expire faster), but increases query load.

## Mail Routing & Delivery

### MX (Mail Exchange)

Specifies servers that accept mail for a domain. Prioritized by preference number (lower = higher priority).

```
example.com.  3600  MX  10  mail1.example.com.
example.com.  3600  MX  20  mail2.example.com.
example.com.  3600  MX  30  mail.backup-provider.net.
```

SMTP clients query MX records for the recipient domain. If `mail1` is down, clients try `mail2`, then the backup. Weight/preference determines order, not load distribution. Multiple clients independently try MX1 first, then MX2; no intelligent failover without external monitoring.

**TTL strategy**: Typically 3600s or longer (mail routing changes infrequently). Short TTL forces re-evaluation on each mail retry, wasting queries.

### TXT (Arbitrary Text)

Stores arbitrary text, often for mail authentication, domain verification, or HSTS policy:

```
example.com.  3600  TXT  "v=spf1 include:_spf.example.net ~all"
_dmarc.example.com.  3600  TXT  "v=DMARC1; p=reject; rua=mailto:admin@example.com"
selector1._domainkey.example.com.  3600  TXT  "v=DKIM1; k=rsa; p=MIGfMA0BgkqhkiG9w0BAQEFAAOBjQAwgYkCgYEA..."
_acme-challenge.example.com.  60  TXT  "<base64-validation-token>"
```

One name can have multiple TXT records (255 characters each; split long values across multiple strings). Common uses:
- **SPF**: Which IPs are authorized to send mail
- **DMARC**: Mail authentication policy
- **DKIM**: Public key for email signatures
- **ACME challenges**: LetsEncrypt challenge tokens (very short TTL for rapid rotation)

## Service Discovery & Advanced Records

### SRV (Service Record, RFC 2782)

Discovers services dynamically. Format: `_service._protocol.name TTL SRV priority weight port target`

```
_ldap._tcp.example.com.  3600  SRV  10  60  389  ldap1.example.com.
_ldap._tcp.example.com.  3600  SRV  10  40  389  ldap2.example.com.
_ldap._tcp.example.com.  3600  SRV  20  100 389  ldap-backup.example.net.
```

Clients query `_service._protocol.name`, receive all SRV records. **Priority** (lower wins) determines server selection; **weight** (higher wins within a priority group) enables load distribution. Port is **not** 443 or 80 (it's specified in SRV). Widely used in enterprise for LDAP, SIP, Kerberos; declining in cloud-native (prefer application-level service discovery like Consul, etcd).

### CAA (Certification Authority Authorization, RFC 6844)

Authorizes specific CAs to issue certificates for a domain.

```
example.com.  3600  CAA  0  issue  "letsencrypt.org"
example.com.  3600  CAA  0  issuewild  "letsencrypt.org"
example.com.  3600  CAA  0  iodef  "mailto:admin@example.com"
```

Directives:
- **issue**: CA allowed to issue certificates for exact domain
- **issuewild**: CA allowed to issue wildcards
- **iodef**: Email/URL for incident reporting

CAs are **required** by industry standards (CAB Forum Ballot) to check CAA records before issuance. If no CAA records exist, all CAs can issue (permissive). Explicit CAA limits issuance scope, mitigating rogue certificate attacks.

### NAPTR (Naming Authority Pointer, RFC 3401)

Maps names to services using regex pattern matching. Used in ENUM (E.164 Number Mapping) for phone number lookup:

```
4.3.2.1.5.5.5.1234.e164.arpa.  NAPTR  100  10  "U"  "E2U+sip"  "!^.*$!sip:info@example.com!"  .
```

Rarely used in modern systems; complex and specific to telecom. Falling out of favor for cloud-native service discovery.

### PTR (Pointer Record)

Reverse DNS mapping: IP address → hostname. Used for reverse lookups (e.g., spam detection, identity verification).

```
1.2.0.192.in-addr.arpa.  3600  PTR  mail.example.com.
```

IPv4: reverse zone is `1.2.0.192.in-addr.arpa.` (octets reversed, `.in-addr.arpa` suffix).
IPv6: reverse zone is `1.0.0.0...in6.arpa.` (half-octets reversed).

PTR records are typically managed by ISPs or hosting providers (reverse DNS delegation). Configuring PTR is essential for mail server reputation (SPF, DKIM, DMARC check PTR during authentication).

## DNSSEC & Security Records

### DS (Delegation Signer) & DNSKEY

DNSSEC records for zone signing and trust anchors.

**DNSKEY**: Zone signing key. Public key used to verify RRSIGs (DNSSEC signatures).

```
example.com.  3600  DNSKEY  257  3  8  AwEAAa...  (KSK, flags 257)
example.com.  3600  DNSKEY  256  3  8  AwEAAb...  (ZSK, flags 256)
```

**DS**: Digest of the KSK, published in parent zone as trust anchor.

```
example.com.  3600  DS  12345  8  2  A1B2C3D4E5F6...
```

Parent zone lists DS record for child zone. Resolvers trust KSK via DS; child zone's DNSKEY must match DS digest. Enables DNSSEC chain of trust from root to zone.

### TLSA (TLSA, RFC 6698)

Binds TLS certificates to a domain/service. Used by DANE (DNS-based Authentication of Named Entities) for certificate pinning.

```
_443._tcp.example.com.  3600  TLSA  3  1  1  d2abde...
```

Parameters:
- **3**: Full certificate (PKIX-End-Entity)
- **1**: SHA-256 hash
- **1**: Compare full certificate
- **d2abde...**: Hash of server's TLS certificate

TLSA replaces or supplements CA-issued certificates. Clients verify the TLSA record matches the server's certificate, bypassing the CA trust chain. TLSA mitigates CA compromise; requires DNSSEC validation for security.

### SSHFP (SSH Fingerprint, RFC 4255)

Records SSH server public key fingerprint for SSH key verification.

```
ssh.example.com.  3600  SSHFP  1  1  abcdef0123456789...
```

Parameters:
- **1**: RSA algorithm
- **1**: SHA-1 hash (deprecated)
- **abcdef...**: Fingerprint of public key

When connecting via SSH, client can verify the server's key against the SSHFP record (requires DNSSEC to be secure). Reduces MITM risk without manual key acceptance.

## DNS Record Type Variations & Edge Cases

### Wildcard Records

`*` matches any subdomain, enabling blanket rules.

```
*.example.com.  3600  A  192.0.2.1
```

Query for `foo.example.com` (non-existent) returns `192.0.2.1`. Query for `foo.bar.example.com` does NOT match (wildcards match single label only; `*` ≠ `*.`).

Wildcards are lower priority than explicit records. If both `*.example.com` (wildcard) and `api.example.com` (explicit) exist, querying `api.example.com` returns the explicit record.

Use wildcards cautiously: they mask configuration errors (typos in subdomains still resolve) and complicate DNSSEC validation.

### ANAME & ALIAS Records

**ANAME** (RFC 8945): Vendor-specific (not IANA-standardized) "alias at zone apex." Allows `example.com` (zone apex) to alias to another domain (unlike CNAME, which can't exist at apex with other records).

```
example.com.  3600  ANAME  target.example.com.
```

ANAME is automatically flattened by the DNS provider; resolver sees A records, not ANAME. Different providers have different implementations (Cloudflare CNAME, Route53 Alias, Dnsimple ANAME). **Use cautiously**: not standardized, provider-dependent, can break DNS consistency.

### Conditional Record Responses

DNS providers (Route53, Cloudflare) support conditional responses:
- **Latency-based routing**: Return different IP based on resolver location
- **Failover**: Return primary IP; if health check fails, return secondary
- **Geolocation**: Return IP based on client geography

These are NOT DNS record types; they're provider features implemented via modified resolver logic or proprietary extensions (e.g., EDNS Client Subnet). Clients see standard A/AAAA records, not special types.

## TTL Strategies & Caching

### TTL Trade-offs

- **Short TTL (60–300s)**: Fast adaptation to changes, but high query load on authoritative server. Use for volatile services (canaries, blue-green deployments) or during migrations.
- **Long TTL (3600–86400s)**: Reduced query load, but slow failover. Use for stable infrastructure.
- **Negative TTL (via SOA minimum)**: Controls how long NXDOMAIN responses are cached. Short negative TTL = faster detection of new subdomains; long TTL = fewer queries for spam/DoS (attackers probe non-existent subdomains).

### DNS Cache Hierarchy

1. **Stub resolver** (OS, browser) — optional, often OS TTL
2. **Recursive resolver** (ISP, 8.8.8.8) — respects TTL
3. **Authoritative nameserver** — source of truth

**Cache inconsistency**: If TTL expires at different times across caches, clients see different answers briefly. Not a problem for most use cases, but critical for transactions or security policies.

## DNS Provider Comparison

Different providers support different record types and extensions:

| Capability | Route53 | Cloudflare | Dnsimple | Google Cloud DNS |
|-----------|---------|-----------|---------|-----------------|
| DNS64 (IPv6 translation) | No | Yes | No | Yes |
| ANAME/Alias at apex | Alias | CNAME | ANAME | No |
| Direct latency routing | Yes (Route53) | No | No | No |
| CAA enforcement | No | Partial (warnings only) | No | No |
| DNSSEC signing | Yes | Yes | Yes | Yes |
| Wildcard records | Yes | Yes | Yes | Yes |
| TTL control | Per-record | Per-record | Per-record | Per-record |

## Common Record Type Patterns

### Email Setup

```
example.com.  MX  10  mail.example.com.
example.com.  TXT  "v=spf1 include:_spf.google.com ~all"
selector1._domainkey.example.com.  TXT  "v=DKIM1; k=rsa; p=..."
_dmarc.example.com.  TXT  "v=DMARC1; p=reject; rua=..."
mail.example.com.  A  203.0.113.100
```

### Web Service

```
example.com.  A  192.0.2.1
example.com.  AAAA  2001:db8::1
www.example.com.  CNAME  example.com.
api.example.com.  A  192.0.2.2
*.example.com.  CNAME  cdn.example.net.
```

### DNSSEC & Security

```
example.com.  DNSKEY  257  3  8  AwEAAa...
example.com.  DS  12345  8  2  A1B2C3...
_443._tcp.example.com.  TLSA  3  1  1  d2ab...
example.com.  CAA  0  issue  "letsencrypt.org"
```

## Related Notes

See [DNS Infrastructure at Scale](infrastructure-dns-architecture.md) for GeoDNS, failover patterns, and provider architecture. See [DNS Security](security-dns.md) for DNSSEC validation, DNS privacy, and attack mitigation. See [Email Infrastructure](networking-email.md) for SPF/DKIM/DMARC security beyond DNS records.