# IPv6 — 128-Bit Addressing, Configuration, and Transition

## Overview

IPv6 (RFC 2460) expands the IP address space from 32 bits (IPv4) to 128 bits, solving address exhaustion. A 128-bit address yields $2^{128}$ ≈ 340 undecillion unique addresses — theoretically enough to assign a unique address to every atom on Earth. Beyond scale, IPv6 simplifies routing, eliminates the need for NAT, and integrates security and mobility features.

Adoption is slow (≈40% of internet traffic as of 2025), driven by installed IPv4infrastructure inertia and the success of NAT in extending IPv4 life. However, IPv6 is essential for future-scale IoT, carrier networking, and advanced applications.

## Address Space and Notation

### 128-Bit Representation

IPv6 addresses are 128 bits, written in hexadecimal with colons separating 16-bit groups:

```
2001:0db8:85a3:0000:0000:8a2e:0370:7334
```

**Compression**: Leading zeros in each group omitted; consecutive all-zero groups replaced with `::` (once per address):

```
2001:db8:85a3::8a2e:370:7334      ✓ (compact)
2001:db8::8a2e:370:7334           ✗ ambiguous (where do zeros end?)
2001:db8::1                        ✓ (trailing ::1 is loopback subnet)
::1                                ✓ loopback
::                                 ✓ all zeros (not routable; used in unspecified context)
```

### Prefix Notation

CIDR-style: `2001:db8:85a3::/48` is a /48 prefix (first 48 bits identify network; remaining 80 for hosts).

**Typical allocations**: /32 to ISP (298 trillion addresses per customer); /48 to end site (65,536 subnets of /64). Each /64 subnet holds 2^64 hosts.

## Address Types

### Unicast

A single host. Three variants:

**Global unicast**: Routable on the internet. Assigned by IANA/RIR/ISP. Typical allocation: `/32` to ISP, `/48` to site, `/64` to subnet.

**Unique local** (ULA, RFC 4193): Private, non-routable (like RFC 1918 in IPv4). Generated pseudo-randomly to minimize collisions if networks merge. Prefix `fc00::/7`.

**Link-local**: Auto-generated from interface MAC address (fe80::/10). Valid only on the local link; router cannot forward. Used for neighbor discovery, DHCP, etc. Every IPv6 interface has one, no configuration needed.

### Multicast

Send to multiple recipients (one-to-many). Prefix `ff00::/8`. 

**Key multicast addresses**:
- `ff02::1` — all nodes on link (like IPv4 address broadcast)
- `ff02::2` — all routers on link
- `ff02::1:ffXX:XXXX` — solicited-node multicast (used in neighbor discovery)

**Use**: Router discovery, DHCPv6, service discovery, media streaming.

### Anycast

Send to nearest member of a group (same address, multiple advertisers). Router uses BGP to route to closest instance. Used for geo-distributed services (DNS root servers, CDNs).

**Address**: Syntactically indistinguishable from unicast; semantics defined by routing.

## Network Configuration: SLAAC vs. DHCPv6

### Stateless Address Auto-Configuration (SLAAC)

Router announces a prefix (e.g., `2001:db8:1:1::/64`) via RA (Router Advertisement). Host appends 64-bit host identifier (usually derived from MAC via EUI-64, or random bits). Result: host configures own address without DHCP.

**Process**:

1. Host generates link-local address (fe80::MAC-derived)
2. Host sends RA request (or just listens for unsolicited RA)
3. Router advertises prefix + router address + hop limit
4. Host appends suffix to prefix → global address
5. Host performs DAD (Duplicate Address Detection) via multicast
6. Host is configured, no server needed

**Advantage**: Zero DHCPv6 infrastructure. **Disadvantage**: Limited to address auto-config (DNS, NTP, other options must be configured via other means or DHCPv6).

### DHCPv6

Similar to DHCPv4 but for IPv6. Server distributes prefixes, addresses, DNS, NTP, etc.

**Modes**:
- **RA + Stateless DHCPv6**: Router announces prefix (SLAAC); DHCPv6 provides only options (DNS, etc.)
- **Stateful DHCPv6**: DHCPv6 server assigns addresses (like DHCP in IPv4)
- **DHCPv6 Prefix Delegation**: Server delegates a prefix to a client (router). Indispensable for ISP-to-customer delegation.

**Prevalence**: SLAAC is simpler and prevalent in data-center/managed networks; DHCPv6 in ISP and enterprise.

## Neighbor Discovery (NDP)

**NDP** (RFC 4861) replaces IPv4 ARP and ICMP Redirect. Uses ICMPv6 multicast:

| Message Type         | Function                             |
|---------------------|--------------------------------------|
| Router Advertisement | Router announces prefix, hop limit  |
| Router Solicitation  | Host requests RA (on startup)       |
| Neighbor Advertisement | Host responds to address lookup    |
| Neighbor Solicitation | Host queries neighbor's MAC (like ARP) |
| Redirect            | Router directs host to better route |

**Example: Finding a neighbor's MAC**:

```
Host A → Multicast: "Who is 2001:db8::2?" (Neighbor Solicitation)
Host B → Unicast: "It's me; my MAC is 00:11:22:33:44:55" (Neighbor Advertisement)
```

**Multicast listener discovery**: Used to track active multicast groups on a link (replaces IGMP from IPv4).

**Security gap**: NDP is unauthenticated by default. Can be spoofed. Mitigated with ND (Neighbor Discovery) authentication extension or SEND (Secure Neighbor Discovery, rarely deployed).

## Dual-Stack — IPv4/IPv6 Coexistence

Most internet infrastructure today runs **dual-stack**: systems support both IPv6 and IPv4. Enables transition without flag-day cutover.

### Client Behavior

When a host resolves `example.com`, it receives both A (IPv4) and AAAA (IPv6) records. Host attempts IPv6 first; if it fails (timeout or refused), falls back to IPv4.

**Happy Eyeballs** (RFC 8305): If IPv6 connection doesn't complete quickly (e.g., 250ms), start IPv4 in parallel. Use whichever succeeds first. Prevents IPv6 brokenness from blocking all connectivity.

### DNS Records

| Record | Protocol | Address |
|--------|----------|---------|
| A      | IPv4     | 32-bit, dotted decimal (192.0.2.1) |
| AAAA   | IPv6     | 128-bit, colon hex (2001:db8::1) |
| PTR    | Reverse lookup | IP-to-hostname |

DNS64 (RFC 6146): Synthesizes AAAA records from A records by wrapping IPv4 addresses in a prefix. Enables IPv6-only clients to reach IPv4-only servers (requires NAT64 gateway).

## IPv6 Transition Mechanisms

### 6to4

Automatically tunnels IPv6 packets inside IPv4, using a reserved prefix `2002:wwxx:yyyz:z/16` (where `wwxx:yyyy:zz` is the IPv4 address). Client with IPv4 connectivity can communicate over IPv6 globally via 6to4 relays.

**Limitation**: Deprecated (RFC 6751). Poor reliability due to centralized relays and lack of incremental deployment path.

### Teredo

Tunnels IPv6 over UDP/443 (NAT-friendly). Enables IPv6 connectivity for hosts behind NAT without ISP support. Used by Windows Vista/7/8 as fallback.

**Limitation**: High latency (tunneling overhead); poor for real-time applications. Largely obsolete with broader IPv6 adoption.

### 6in4 (Manual Tunneling)

Administrator configures an explicit IPv4 tunnel to an IPv6 gateway. More reliable than 6to4 (static config) but requires manual setup.

### NAT64 / DNS64

**NAT64** is a gateway that translates IPv6 packets to IPv4 and vice versa. **DNS64** synthesizes IPv4 addresses as IPv6 addresses in a well-known prefix (`64:ff9b::/96` by RFC).

**Use**: IPv6-only clients can access legacy IPv4-only servers via NAT64.

**Process**:

1. IPv6 client queries DNS for `example.com` (IPv4-only server)
2. DNS64 synthesizes AAAA: `64:ff9b::192.0.2.1`
3. Client connects to `64:ff9b::192.0.2.1` → NAT64 gateway
4. NAT64 extracts IPv4 address, connects to `192.0.2.1`, proxies traffic

**Limitation**: Stateful translation; TCP/UDP translation required. Not suitable for all protocols.

## Security Considerations

### IPv6 Firewall

Unlike IPv4, IPv6 bypasses NAT by default (each host is globally routable). Firewalling is essential; misconfiguration can expose internal services.

**Common mistake**: Enable IPv6 but forget to configure firewall rules. Hosts become unexpectedly reachable.

### ICMPv6

IPv6 depends on ICMPv6 (Neighbor Discovery, Path MTU, etc.); cannot be blocked entirely like IPv4 ICMP. Firewalls must allow specific ICMPv6 types.

### Privacy Extensions (RFC 4941)

By default, SLAAC derives host bits from MAC, making it trivial to track devices across networks (MAC-based identifiers). Privacy Extensions randomize host bits, generating temporary addresses that change periodically.

**Modern systems**: Most enable this automatically. Results in multiple addresses per interface (link-local + global + temporary).

### Extension Headers

IPv6 allows optional headers (routing, fragmentation). Can be used for attacks (scanning, DoS via resource consumption). Firewalls often block extension headers or rate-limit them.

## Adoption Status

- **Data centers**: Nearly 100% dual-stack; IPv6 often preferred for internal traffic
- **Mobile networks**: Carriers deploy IPv6; mobile clients typically dual-stack
- **Residential ISPs**: ~50% supporting IPv6 (varies by region)
- **Content delivery**: Major CDNs (Akamai, Cloudflare) fully IPv6-capable
- **Enterprise**: Lagging; many corporate networks IPv4-only

**Barriers**: Vendor support lags (some networking equipment not IPv6-native), training/expertise gaps, no burning business case (NAT works; IPv4 not fully exhausted in all regions).

## See Also

- [networking-tcp-ip.md](networking-tcp-ip.md) — TCP/IP model and IPv4 addressing
- [infrastructure-dns-architecture.md](infrastructure-dns-architecture.md) — DNS systems and records
- [security-network.md](security-network.md) — Network security and firewalling