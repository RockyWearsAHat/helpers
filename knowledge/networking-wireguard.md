# WireGuard — Noise Protocol, Cryptokey Routing & VPN Implementations

## Overview

**WireGuard** is a modern VPN protocol designed around simplicity, auditability, and performance. It abandons the complexity of IPSec in favor of a minimal cryptographic footprint: one encryption protocol (ChaCha20-Poly1305 for authenticated encryption), one key exchange mechanism (Curve25519), and a straightforward peer-to-peer model where each endpoint declares its allowed IP ranges.

Unlike IPSec's negotiated algorithms and multi-phase handshakes, WireGuard is **opinionated**: you use its chosen crypto primitives or you don't. This trades flexibility for security-through-simplicity. The Linux kernel module is ~600 lines of code (vs. IPSec's thousands), making auditing tractable. WireGuard is also **stateless** — the protocol doesn't require traditional connection establishment; a peer can be brought online or offline without synchronization overhead.

The design philosophy: maximize security per line of code, assume cryptographic primitives are vetted (Noise framework), eliminate configuration mistakes through declarative peer management.

## Noise Protocol Framework

WireGuard's cryptographic construction is built on the **Noise Protocol Framework** (RFC 7539 equivalent, though Noise predates RFC standardization). Noise is a framework for designing authenticated encryption handshakes using Diffie-Hellman variants and symmetrical encryption.

### Handshake Pattern

WireGuard uses the **Noise_IK** pattern (Initiator has a static identity, encrypted with Responder's static public key):

```
Initiator → Responder:  [encrypted payload with Initiator's static key]
            → Initiator: [encrypted response using session keys]

Out-of-band key exchange: peers must know each other's Curve25519 public keys
                           (configured in wg0.conf or coordinated server)
```

**Primitives:**
- **DH (Diffie-Hellman):** Curve25519 for key agreement (elliptic curve, 128-bit symmetric strength)
- **Cipher:** ChaCha20 for confidentiality (12-byte nonce, 256-bit key, stream cipher)
- **Authenticator:** Poly1305 for message authentication (combines with ChaCha20 as ChaCha20-Poly1305 AEAD)
- **Hash:** BLAKE2s for hashing and HKDF-derived key material

This is sufficient for VPN use. Other handshake patterns (Noise has 13+) are not needed; WireGuard chose one pattern and hardcoded it.

### No Renegotiation

TCP connections negotiate algorithms in TLS handshakes (as TLS versions change, servers support multiple cipher suites). WireGuard's hardcoded Noise pattern means **no version negotiation, no algorithm selection by the server**. Both peers run identical code. This removes an entire class of negotiation attacks (downgrade attacks, algorithm confusion).

## Cryptokey Routing

The core WireGuard concept is **static, declarative peer configuration**. Each peer has:

```
[Peer]
PublicKey = <peer's Curve25519 public key>
AllowedIPs = 10.0.0.2/32
Endpoint = peer.example.com:51820
PersistentKeepalive = 25  (optional)
```

**Semantics:**
- If a packet arrives destined for `10.0.0.2/32`, route it to the peer with that `PublicKey`
- Encrypt with that peer's public key using Noise IK
- If the peer's `Endpoint` field is populated (hardcoded IP:port), send to that address
- If `Endpoint` is not set, send to the source address of the most recent successfully decrypted packet from that peer (**endpoint learning via reverse path**)

This is **cryptokey routing**: **encryption keys and routing tables are unified**. Unlike traditional networks where routing and encryption are separate, WireGuard ties them together. You don't have a separate static route table; you declare "this IP range is reachable via this peer's encrypted tunnel."

**Implication for roaming:** A mobile client can leave WiFi, connect to cellular, and send a packet with a new source IP. WireGuard sees it's from a known peer (decrypts successfully), learns the new endpoint, and future packets destined for that peer go to the cellular IP. No re-registration, no session renegotiation. This is **connection migration for free**.

## Kernel vs. Userspace Implementation

### Kernel Implementation (Linux)

WireGuard ships as a Linux kernel module. By default, it's compiled into recent kernel versions (6.1+). Running in kernel space provides:

- **Performance:** Direct access to network stack. Minimal copying. Packet processing happens at kernel priority.
- **Transparency:** All applications (TCP, UDP, any protocol) automatically tunnel through WireGuard with zero per-application overhead.
- **Scheduling:** Uses kernel CPU scheduling; can saturate a full NIC if CPU permits.

**Downsides:**
- Security bugs in WireGuard are now kernel vulnerabilities (though the small code size mitigates this)
- Requires kernel compilation or distribution-provided module
- Device-wide (cannot have per-process or per-user VPN; must tunnel entire device or just specific interfaces)

### Userspace Implementation

Alternatives exist: **boringtun** (Cloudflare's implementation in Rust), **wireguard-go** (Go implementation by Wire developers), **wireguard-rustls** (Rust with rustls for TLS).

**Trade-offs:**
- Userspace implementations can run on unsupported platforms (Windows, macOS, iOS) without kernel changes
- **Performance hit:** additional copying (kernel ↔ userspace), context switching, reduced scheduling flexibility
- **Easier deployment:** no kernel module compilation, no admin privileges except at setup
- **Safer:** userspace process crash doesn't compromise kernel stability
- Can use per-application routing policies (one user's packets via WireGuard, others bypass)

**Deployment patterns:**
- Linux high-performance servers: kernel WireGuard
- Mobile devices (iPhone via WireGuard app): userspace implementation
- Windows/macOS: userspace (wireguard-go or Commercial VPN clients wrapping it)

## Configuration & Peers

### Static Peer Model

Every peer must be pre-configured with public keys. This is manual or semi-automated (configuration management, IaC, or coordination server).

```bash
# On client:
[Interface]
PrivateKey = <client private>
Address = 10.0.1.100/24

[Peer]
PublicKey = <server public>
AllowedIPs = 10.0.1.0/24
Endpoint = vpn-server.example.com:51820

# On server:
[Interface]
PrivateKey = <server private>
Address = 10.0.1.1/24
ListenPort = 51820

[Peer]
PublicKey = <client public>
AllowedIPs = 10.0.1.100/32  (client's subnet)
```

**Per-peer settings:**
- `PersistentKeepalive`: Send keepalive packets every N seconds (forces NAT traversal; needed if peer is behind masquerading router)
- `Endpoint`: server-side only; client learns server address; server learns client address by reverse path
- `AllowedIPs`: both directions — specifies what IP ranges are reachable via this peer AND what is accepted from this peer

### Peer Rotation & Key Management

Static configuration becomes cumbersome at scale. WireGuard's answer: **change the configuration and reload**. Signal SIGHUP or use `wg set` commands at runtime.

```bash
# Add peer at runtime
wg set wg0 peer <public-key> allowed-ips 10.0.1.200/32 endpoint 192.168.1.5:51820

# Remove peer
wg set wg0 peer <public-key> remove

# View current state
wg show wg0
```

This allows coordination servers to manage peer lifecycle without stopping the tunnel.

## Tailscale: WireGuard + Coordination

**Tailscale** is a commercial mesh VPN built on WireGuard. It solves WireGuard's "who coordinates the peer list?" problem via a **coordination server** and **DERP relays**.

### Coordination Server

Tailscale runs a control plane server that:

1. **Peer discovery:** Each device registers its public key + endpoint
2. **Network construction:** Control plane tells each device about all peers in its network (mesh topology)
3. **Configuration push:** dynamically updates WireGuard config as peers join/leave

Result: clients need only know the Tailscale coordination server address; everything else is automated.

### DERP Relays

Direct peer-to-peer WireGuard works only if peers can reach each other (NAT traversal via hole punching). If both peers are behind symmetric NAT or corporate firewalls, direct connectivity fails. 

**DERP** (Designated Encrypted Relay Protocol): Tailscale's proprietary relay protocol. When direct peer-to-peer fails, traffic is relayed through a DERP server (encryption wraps the WireGuard packets, so Tailscale still can't see plaintext, but the relay costs adds latency).

**Protocol stack:**
```
Application traffic (encrypted by app)
↓
WireGuard (encrypted again)
↓
DERP (encrypts the WireGuard packet if relaying)
↓
Network
```

This is **double encryption** in relay scenarios, adding CPU overhead but maintaining end-to-end encryption even when relayed.

### MagicDNS

Tailscale nodes are reachable by hostname (e.g., `laptop.tailnet-name.ts.net`). MagicDNS intercepts DNS queries for Tailscale domains and returns WireGuard interface IPs. Applications don't know they're using a VPN; they just resolve hostnames.

## Headscale & Netbird (Self-Hosted Coordination)

WireGuard + coordination is powerful, but Tailscale is proprietary. Self-hosted alternatives:

### Headscale

Open-source implementation of Tailscale's coordination API. Runs your own control plane on your own server. Clients use Tailscale's client software (or compatible open-source clients) but communicate with your Headscale server instead of Tailscale's SaaS control plane.

**Features:**
- Per-device key generation and renewal
- Policy-based access control (who can reach whom)
- DERP relay support (can use Tailscale's free DERP or run own)
- ACLs, device tagging, user management

**Trade-off:** operationally simpler than raw WireGuard (no manual key management), but still your infrastructure to run and maintain.

### Netbird

Mesh VPN combining WireGuard with a management dashboard. Similar to Headscale but with extra features: encrypted sync of configuration, built-in firewall rules, user provisioning via OIDC. Also self-hosted.

## Comparison to Alternatives

### WireGuard vs. IPSec

| Aspect | WireGuard | IPSec |
|--------|-----------|-------|
| **Code complexity** | ~600 lines | Thousands |
| **Cryptography agility** | None (hardcoded) | Negotiated (flexible but complex) |
| **Configuration** | Declarative peers | Negotiation + SA policies (complex) |
| **Latency** | Low (minimal overhead) | Moderate (IKE negotiation, more processing) |
| **Standardization** | IETF RFC 9434 (adopted 2023) | IETF standards (1990s-2000s) |
| **Adoption** | Growing (Linux kernel, mobile) | Enterprise standard (ingrained) |
| **Enterprise alignment** | Emerging | Mature (compliance, audit familiarity) |

WireGuard is simpler and faster for new deployments. IPSec remains entrenched where algorithm flexibility or regulatory requirements mandate it.

### WireGuard vs. OpenVPN

| Aspect | WireGuard | OpenVPN |
|--------|-----------|---------|
| **Transport** | UDP (thin protocol) | UDP/TCP (flexible) |
| **Encryption** | ChaCha20-Poly1305 (hardcoded) | OpenSSL (negotiated, updates follow OpenSSL versions) |
| **Latency** | Very low | Higher (userspace, more overhead) |
| **Code** | Auditable kernel module | Large userspace daemon (OpenSSL + OpenVPN logic) |
| **Configuration** | Simple | Moderate (more options) |
| **Portability** | Linux kernel (native), userspace on others | Userspace everywhere |
| **Adoption** | Rapidly growing | Established (privacy-focused VPN providers) |

OpenVPN is more flexible (supports TCP fallback, configurable ciphers); WireGuard is modern and lean.

## Performance & Security Considerations

### Advantages

- **Simplicity:** Fewer attack surfaces, easier to audit
- **Speed:** Kernel-space performance, minimal crypto overhead (ChaCha20-Poly1305 is fast on modern CPUs)
- **Connection migration:** Built-in roaming across networks
- **Scalability:** Stateless peers allow thousands of connections on a single server
- **Modern cryptography:** Curve25519, ChaCha20, BLAKE2s are current best practice

### Limitations & Trade-offs

- **Algorithm inflexibility:** If Curve25519 or ChaCha20 are broken, a new WireGuard version is non-trivial; you can't just swap algorithms like with IPSec
- **No backward compatibility guarantee:** WireGuard reserves the right to change primitives in major updates
- **Peer management at scale:** Every peer must know every other peer for mesh topologies (solved by Tailscale/Headscale but adds operational complexity)
- **No forward secrecy by default:** Sessions are tied to static keys (mitigated by policy to rotate keys periodically)
- **UDP-only:** No TCP fallback if UDP is blocked; requires explicit relay solutions (DERP)

## Deployment Patterns

1. **Site-to-site VPN:** Two offices, each runs a WireGuard endpoint, routes traffic between office subnets
2. **Remote access VPN:** Employees connect from home via WireGuard to corporate network (kernel module or app)
3. **Mesh networking:** Tailscale/Headscale for device-to-device connectivity (laptops, servers across locations)
4. **Zero-trust overlay:** Needing encrypted tunnels between every service:service connection without a service mesh
5. **Embedded networking:** WireGuard's minimal footprint suits IoT and embedded systems

## See Also

- [networking-vpn-protocols](./networking-vpn-protocols.md)
- [security-network.md](./security-network.md) — IPSec, firewalls
- [networking-tls-handshake.md](./networking-tls-handshake.md) — TLS vs. Noise handshakes
- [infrastructure-service-discovery.md](./infrastructure-service-discovery.md) — Coordination servers in service discovery