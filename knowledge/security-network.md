# Network Security — Firewalls, IDS/IPS, VPNs, and Defense Layers

## Overview

Network security operates across multiple defensive layers: edge filtering (firewalls), behavior detection (IDS/IPS), encrypted channels (VPNs), internal segmentation, and specialized mitigations for targeted attack vectors (DDoS, DNS manipulation, BGP hijacking). No single layer is sufficient; security requires layered controls that assume hostile placement at any network boundary.

---

## Firewalls: Stateless vs. Stateful Inspection

### Packet-Filtering Firewalls (Stateless)

Early firewalls examined each packet in isolation: source/destination IP, port, protocol. No memory of connection state. Decisions are fast but superficial — a firewall rule permitting TCP port 80 cannot distinguish legitimate HTTP responses from unsolicited traffic pretending to be a response.

**Weakness:** Vulnerable to spoofing and protocol-violating attacks. An attacker can craft packets that pass filter rules but violate protocol semantics.

### Stateful Inspection Firewalls

Modern firewalls track **connection state**: TCP handshakes, session lifecycle, and bidirectional traffic flow. Rules specify not just "allow port 80" but "allow established TCP connections from inside to outside on port 80." Inbound responses are implicitly allowed only if they match active outbound sessions.

**Mechanism:** Firewall maintains a state table of active connections. TCP SYN arrived from inside? Record it. SYN-ACK from outside to that same address:port pair? Allow it as a response. Unsolicited inbound SYN-ACK with no matching outbound SYN? Drop it.

**Trade-offs:** Stateful tracking adds CPU and memory overhead but blocks entire classes of spoofing attacks. Most production firewalls are stateful.

### Next-Generation Firewalls (NGFW)

Stateful inspection plus **application-layer awareness**. These firewalls inspect protocol payloads: decode HTTP headers, parse SSL/TLS handshakes, detect encrypted malware signatures. Rules can be written in application terms: "Block all HTTP POST requests containing SQL keywords" (SQL injection detection).

**Complexity:** Application inspection requires CPU-intensive parsing and threat signature databases. Real-time updates are essential as new attack patterns emerge. Encrypted traffic (HTTPS, VPN) is opaque to signature inspection unless the firewall terminates the connection (SSL/TLS decryption), which introduces trust and performance implications.

---

## IDS vs. IPS: Detection vs. Prevention

### Intrusion Detection Systems (IDS)

Monitors network traffic and generates **alerts** when malicious patterns are observed. Placement: either **network IDS** (taps network links, sees all traffic) or **host IDS** (runs on individual servers, sees local activity).

**Operating mode:** IDS is passive — it observes and reports but does not block. Security teams respond to alerts by investigating or manually blocking. Detection uses **signature matching** (known attack patterns), anomaly detection (deviation from baseline behavior), or heuristics (patterns resembling attacks).

**Limitation:** IDS introduces **alert fatigue** — thousands of alerts daily, most false positives. Alert triage is labor-intensive.

### Intrusion Prevention Systems (IPS)

IDS + **active blocking**. When a malicious pattern is detected, the firewall or IPS immediately **drops the connection, resets the session,** or quarantines the host. Placement: must be **in-line** (all traffic flows through it); it cannot be a passive tap.

**Trade-off:** In-line placement creates a single point of failure. If the IPS fails, the network is broken. Signature tuning is critical — aggressive rules cause legitimate traffic to be blocked (false positives are now security incidents, not just alerts).

**Modes:** Passive IDS mode (alert only), active IPS mode (drop), or hybrid (alert + selective drops for high-confidence threats).

---

## VPNs: IPSec, IKE, and WireGuard

### IPSec (IP Security)

Layer 3 encryption: operates on IP packets themselves. Two main modes:

- **Transport mode:** Encrypts IP payload only (the data). IP headers remain visible. Used for host-to-host communication.
- **Tunnel mode:** Encapsulates entire IP packet inside new IP/IPSec headers. Used for VPN gateways. Original source/destination IPs are encrypted and hidden.

**Key establishment:** IKE (Internet Key Exchange) automatically negotiates shared keys using Diffie-Hellman key exchange. Phase 1 (IKE SA) establishes secure channel to negotiate Phase 2 (IPSec SA) parameters.

**Encryption + Authentication:** IPSec uses both AES/ChaCha20 for confidentiality and HMAC/AEAD for integrity. Authenticated Encryption with Associated Data (AEAD) is modern; older HMAC-then-encrypt modes have weakness if not carefully sequenced.

**Complexity:** IPSec is powerful but configuration-heavy. Many deployment options (encryption algorithms, modes, key exchange protocols) lead to misconfiguration. Standardization efforts like Suite B have attempted to reduce choice but adoption lags.

### WireGuard

Minimalist stateless VPN protocol. Core design: **single crypto primitive** (ChaCha20-Poly1305 for authenticated encryption, Curve25519 for DH key exchange). Configuration is declarative: list of peers, their public keys, allowed IPs. No algorithm negotiation — no missteps.

**Key advantages:** ~600 lines of kernel code vs. IPSec's thousands. Easier to audit, fewer bugs, lower latency. Ships in recent Linux kernels. Configuration is file-based (simple).

**Trade-offs:** WireGuard does not support algorithm agility (cannot swap ChaCha20 if broken). Its opinionated design sacrifices some flexibility for simplicity and security.

**Adoption:** WireGuard is gaining traction for site-to-site and client-VPN use, though IPSec remains entrenched in enterprise and regulatory environments where algorithm flexibility is contractual requirement.

---

## Network Segmentation and DMZ

### Flat Networks vs. Segmented

Early networks were **flat**: all machines could reach all machines. Security relied on perimeter defense (external firewall) + internal trust assumption.

**Flaw:** If attacker breaches perimeter, they have free movement internally. Lateral movement from compromised web server to database is uncontrolled.

### DMZ (Demilitarized Zone)

A **network segment isolated from both the untrusted internet and the internal LAN.** Internet-facing services (web, mail, DNS) run in the DMZ. Internal users and services run behind a second firewall.

**Traffic flow:** Inbound internet → DMZ firewall (allows only web/mail/DNS) → DMZ servers. Internal users cannot directly access DMZ; they request services which then access internal resources via backend firewall, enforcing explicit policy.

### Microsegmentation and Zero Trust

Modern approach: **no implicit trust anywhere.** Every workload is treated as potentially compromised. Network access is fine-grained:

- East-West firewalls (between internal subnets) enforce segmentation at subnet or workload level
- Identity-based access: policy driven by workload identity, not IP address
- Encrypted tunnel between every pair: enforces authentication and provides audit trail

**Risk:** Complexity grows with number of policies. Misconfiguration is common — overly permissive rules defeat segmentation.

---

## DDoS Mitigation Techniques

Distributed Denial of Service attacks flood targets with traffic volume, connection exhaustion, or application-layer requests. Mitigation strategies operate at different layers:

### Volumetric Attacks (Flooding)

**BGP Blackholing (Remote Triggered Black Hole Routing):** Victim announces via BGP that traffic destined for their IP should be dropped at upstream ISP routers. Uses Unicast Reverse Path Forwarding (uRPF) to validate traffic sources. Effect: attack traffic is dropped at ISP routers before reaching victim, but all traffic to victim is lost (legitimate and attack).

**BGP Flowspec (BGP Flow Specification):** More granular. Instead of blackholing all traffic, ISP applies filtering rules ("drop traffic from AS X" or "drop traffic on port Y"). Reduces collateral damage but requires ISP coordination and automation.

**Sinkholing:** Route attack traffic to a honeypot/scrubbing center. Attacker traffic is captured and analyzed. Legitimate traffic is allowed through. Requires operator to distinguish attack signatures in real-time.

### Connection-Exhaustion and Application-Layer Attacks

**Rate Limiting:** Ingress filtering limits connections/requests per source IP or per user. Mitigates application-layer floods (HTTP GET floods).

**Challenge-Response:** Suspected botnet traffic must solve computational puzzle (CAPTCHAs, proof-of-work) before reaching server. Humans pass; bots struggle. Downsides: degrades UX, attackers use residential botnet with legitimate browser clients.

### Detection and Response

**Anycast scrubbing centers:** Normal traffic is anycast-routed to nearest scrubbing center which filters attack traffic and forwards clean traffic to origin. Attacker sees origin server is close (low latency), decides not worth attacking (if attacker lacks sufficient resources).

---

## DNS Security: DNSSEC and DNS Sinkholing

### DNS Vulnerabilities

DNS is unencrypted and unauthenticated. Attacker can:

- **DNS spoofing:** Intercept query, return fake IP for domain. User requests `bank.com`, attacker returns attacker's IP. MITM attack is trivial on open networks.
- **DNS amplification DDoS:** Attacker spoofs source IP (victim's IP), sends recursive DNS query to public resolver. Resolver returns large response sent to victim. Amplification factor: 50-1000x.

### DNSSEC (DNS Security Extensions)

Adds cryptographic authentication: DNS responses are digitally signed. Chain of trust: root zone signs TLDs, TLDs sign domain operators, domain operators sign resource records.

**Mechanism:** Resolver fetches DNSKEY records (public keys) and RRSIG (signatures). Verifies each signature in the chain using keys. If signature fails, response is rejected.

**Limitations:**
- DNSSEC does not encrypt DNS queries — eavesdropper sees all queries (privacy leak)
- Chain of trust requires all layers to properly sign and validate. Misconfiguration breaks validation.
- Performance cost: DNSSEC validation requires additional DNS lookups (DNSKEY, DS records) and signature verification

**Adoption:** DNSSEC is deployed at TLD and root level but sparse at domain level. Many resolvers don't validate (treating DNSSEC failures as normal timeout), reducing incentive to sign.

### DNS Sinkholing

Operator adds fake DNS record: `malware.com A 127.0.0.1` (localhost). When malware queries for C2 server domain, it gets 127.0.0.1 and cannot connect. Sinkholing databases are maintained by threat intelligence teams and pushed to authoritative resolvers.

**Effectiveness:** Requires that malware domain is in blocklist. Rapid domain rotation (malware registers new C2 domain daily) evades sinkholing.

---

## BGP Security and Route Hijacking

Border Gateway Protocol (BGP) is the internet's routing system. Routers announce IP blocks ("I can reach 192.0.2.0/24") and peers propagate announcements. No authentication required for announcements.

### BGP Hijacking

Attacker announces a prefix as their own (spoofs a large provider) and peers accept it. Traffic intended for the legitimate owner is rerouted through attacker. Attacker can eavesdrop, modify, or black-hole traffic.

**Historical example:** Pakistan Telecom accidentally (or deliberately) announced ownership of YouTube's IP range; for periods, YouTube was unreachable globally.

### RPKI (Resource Public Key Infrastructure)

Cryptographic signing of route announcements. IP block owners digitally sign their BGP announcements at the RIR (Regional Internet Registry). Routers configured to validate RPKI signatures reject unsigned or invalid announcements.

**Limitations:** RPKI prevents hijacking but not misconfigurations (legitimate operator announcing wrong prefix). Adoption requires coordination across thousands of ASNs; incentive to deploy is weak until hijacking damage is severe enough.

### BGP Origin Validation (BGP-OV)

Lighter-weight: each AS validates that the announcement source is authorized by the address holder, without full RPKI chain. Reduces computation but is coarser validation.

---

## Intrusion Detection: Signature vs. Anomaly

### Signature-Based Detection

Known attack patterns are encoded: specific packet sequences, payload content, protocol violations. IDS matches traffic against signatures.

**Strength:** Fast, interpretable (security team understands exactly why alert fired), low false positives if signatures are precise.

**Weakness:** Only detects known attacks. Zero-day exploits bypass signature databases.

**Maintenance burden:** Signature vendors publish updates constantly; stale databases miss recent attacks.

### Anomaly-Based Detection

Learns baseline of normal traffic: typical packet sizes, port distributions, protocol mixtures, geographies. Flags deviations as anomalies.

**Strength:** Potential to detect novel attacks; no signature database needed.

**Weakness:** High false positives — legitimate changes (new application, traffic spike) look anomalous. Tuning is manual and fragile. Attackers can train a shadow model and craft attacks that blend into normal behavior.

**State:** Largely research/prototype. Few production deployments use purely anomaly-based model; most use hybrid (signatures + anomaly outlier flagging).

---

## Practical Deployment Tensions

**Performance vs. inspection depth:** Deep packet inspection adds latency. ISP backbone firewalls often do stateless or light-state filtering; edge firewalls can afford heavier inspection.

**Centralization vs. flexibility:** Centralized firewall is easier to audit but single point of failure. Distributed firewalls (e.g., per-subnet) are resilient but harder to manage consistently.

**False positives vs. detection:** Sensitive IPS rules catch attacks but block users. Loose rules miss attacks but business continues. Tuning is ongoing.

**BGP security friction:** RPKI/BGP-OV require coordination across autonomous systems with misaligned incentives. Deployment takes decades even when benefits are clear.

**Encryption opacity:** NGFW's signature detection cannot inspect encrypted TLS payloads without decryption (which breaks end-to-end security, requires storing keys in middleboxes). Tradeoff between transparency and privacy.

---

## See Also

- [networking-websockets.md](networking-websockets.md) — WebSocket protocol security considerations
- [networking-http.md](networking-http.md) — HTTP/HTTPS security and TLS handshake
- [security-best-practices.md](security-best-practices.md) — Defense-in-depth principles
- [security-web-application.md](security-web-application.md) — Application-layer defenses
- [cloud-aws-security.md](cloud-aws-security.md) — Cloud-specific network security (VPC, security groups)