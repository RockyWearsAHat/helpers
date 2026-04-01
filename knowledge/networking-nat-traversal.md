# NAT Traversal — Reaching Peers Behind Firewalls

## Overview

Network Address Translation (NAT) rewrites source/destination IP addresses at network boundaries, enabling multiple internal hosts to share a single external IP. This saves IPv4 addresses but blocks inbound connections: externally, there's no way to address a machine behind NAT.

NAT traversal techniques enable peers separated by NAT to discover each other and establish direct communication paths — essential for P2P applications, real-time media, and decentralized systems. The challenge: NAT rules are implementation-specific, and some (symmetric NAT) are fundamentally hostile to P2P.

## NAT Behavior Classification

### Cone NAT (Full, Address, Port-Restricted)

The NAT creates a **mapping** from internal address:port to external address:port. Once created, the mapping permits any external peer to send packets back through it (full cone) or packets from the IP that originally received one (address-restricted) or same IP:port (port-restricted).

**Behavior**: Predictable. If I send a packet from internal `192.168.1.10:5000` to external peer `203.0.113.5:5001`, the NAT maps my traffic to external `88.77.66.55:6000` (assigned). Any packet sent back to `88.77.66.55:6000` reaches my internal address.

### Symmetric NAT

The NAT assigns a **new external port for each unique destination IP:port pair**. Mapping internal `192.168.1.10:5000` → external `88.77.66.55:6001` for destination `203.0.113.5:5001` is distinct from mapping the same internal port to a different destination `203.0.113.6:5002` (which might get external `88.77.66.55:6002`).

**Implication**: Peer discovery mechanisms fail. I can learn my external address by contacting a STUN server, but that address is only valid for communicating with that specific server. Contacting a different peer requires a different external port — which I cannot predict.

**Prevalence**: Common in carrier-grade NAT (ISPs, 4G networks, corporate firewalls). Makes P2P without relays impractical.

## STUN — Session Traversal Utilities for NAT

**STUN** (RFC 5389) is a lightweight protocol to discover your external address and NAT behavior. A client sends binding requests to a STUN server; the server reflects back the source IP:port. Client compares internal address to reflected address, inferring NAT type and external binding.

### Discovery Flow

```
Client              NAT                 STUN Server
  |                  |                       |
  |--BINDING-REQUEST-|--BINDING-REQUEST---→ |
  |                  |                       |
  |← BINDING-RESPONSE-|← BINDING-RESPONSE--- |
  |-- (addr: external-ip, port: external-port)
```

### Learning NAT Type

Client sends binding requests to the STUN server from different local addresses/ports:

| Test            | Maps to same external port? | NAT Type           |
|-----------------|-----------------------------|--------------------|
| Request 1 & 2   | Yes                         | Full cone or address-restricted |
| (same src:port) | No                          | Port-restricted or symmetric |
| Request 3       | ✓                           | Port-restricted cone |
| Request 3       | ✗                           | Symmetric NAT (problem) |

**Limitation**: Symmetric NAT cannot reliably predict future mappings. STUN alone is insufficient.

## ICE — Interactive Connectivity Establishment

**ICE** (RFC 8445) is the comprehensive framework for discovering and validating connectivity candidates. It builds on STUN and TURN, adding candidate gathering and validation.

### Candidates

An ICE candidate is an address where a peer might be reachable: local IP, external IP from STUN, or relay address from TURN. Each candidate is triple: IP, port, and protocol (UDP/TCP).

### Gathering and Connectivity Checks

1. **Gather candidates**: Local addresses + STUN (reflexive) + TURN (relay)
2. **Exchange candidates**: Send SDP (Session Description Protocol) offer/answer between peers (out-of-band, e.g., WebSocket signaling)
3. **Connectivity checks**: Each peer sends STUN binding requests to every candidate of the other peer, in priority order
4. **Success**: First successful bidirectional exchange is selected; media flows through that candidate pair
5. **Failover**: If primary fails, try next candidate

**Prioritization**: Local candidates preferred (lowest latency), then reflexive (STUN), then relay (TURN). Reduces latency and relay load.

### ICE Lite vs. Full

- **ICE Full**: Gathers candidates, performs connectivity checks, handles candidate addition during call
- **ICE Lite**: Provides candidates but does not check connectivity. Used by servers that prefer simplicity (they're not behind NAT)

## TURN — Traversal Using Relays around NAT

**TURN** (RFC 5766) is a relay protocol: a TURN server forwards packets between peers when direct connectivity fails (symmetric NAT case).

### Mechanics

Client allocates a **relay address** on the TURN server. The server listens on that address. When another peer sends packets to the relay address, the server forwards them to the client's internal address.

```
Peer A              Relay Server              Peer B
  |                       |                      |
  |---Data to relay addr---→ |                    |
  |                       |---Data to A's internal→ |
  |                       |                      |
  |← Data from relay------- | ←---Data to relay---- |
```

**Cost**: Bandwidth is doubled (in and out through relay). CPU at relay server bounds scalability. Used as fallback, not primary path.

### Allocation

Client sends TURN Allocate request with credentials (username/password or temporary token via OAuth). Server creates allocation, returns relay address + port. Client can open **permissions** on the TURN server, specifying which peers are allowed to send to this relay (mitigates amplification attacks).

## Hole Punching — Symmetrical Connectivity

When two peers behind symmetric NATs (or port-restricted cones) coordinate, both can send packets _simultaneously_ to predict each other's external ports.

### Concept

1. Peer A sends packet to B's external address (guessed or from STUN relay)
2. Peer B sends packet to A's external address (guessed or from STUN relay)
3. NAT at A creates outbound mapping; NAT at B creates outbound mapping
4. Packets from A reach B (arriving from outside, but A previously sent outbound, so mapping exists)
5. Packets from B reach A (symmetric condition: same logic)

**Requirement**: Timing and prediction must align. If ports are too unpredictable (pure symmetric NAT), hole punching fails; TURN is fallback.

### UDP vs. TCP Hole Punching

- **UDP**: Stateless; simple to implement. Holes close after inactivity (typically 30-60 seconds).
- **TCP**: Harder; requires accurate port prediction or simultaneous connection establishment (both initiate, NAT accepts if ports match).

## WebRTC ICE Flow

**WebRTC** (used by video conferencing, file-sharing apps) bundles STUN, TURN, and ICE.

1. Browser contacts STUN servers → gathers reflexive candidates
2. Browser contacts TURN servers (if configured) → gathers relay candidates
3. JavaScript application sends offer (candidates + media capabilities) over signaling transport (WebSocket, etc.)
4. Peer receives offer, sends answer with its candidates
5. Browser performs ICE connectivity checks
6. First successful candidate pair is negotiated; media flows

**STUN/TURN servers can be attacker-controlled**: Attackers can observe traffic (if TURN unencrypted) or conduct amplification attacks (TURN server floods a target with relayed packets). Mitigate with credentials, rate-limiting, and DTLS encryption over TURN.

## Alternative Approaches

### UPnP / NAT-PMP

NAT device exposes a port forwarding API. Client can request the NAT to forward external port X to internal address/port Y. Works if:
- NAT supports UPnP (many home routers do; enterprise firewalls often disable)
- Client is on same subnet as NAT device

**Advantage**: No third-party server required; direct connectivity without relay possible. **Disadvantage**: ISP-grade NAT and corporate firewalls often disable UPnP.

### Tailscale / WireGuard Approach

Rather than traversing NAT, join an encrypted overlay network. Client establishes encrypted tunnel to a coordination server, which facilitates peer discovery. If direct connection possible, send encrypted traffic point-to-point; otherwise relay through coordination server.

**Implication**: All traffic is encrypted end-to-end. Scales better than public STUN/TURN infrastructure because coordination is application-specific.

## Limitations

- **Symmetric NAT is a hard limit**: If both peers are behind symmetric NATs with independent port assignment, direct P2P requires relay (TURN).
- **STUN servers are centralized**: Compromise of STUN infrastructure can leak IP addresses.
- **TURN amplification attacks**: Open relay can be exploited to amplify traffic against a target.
- **IPv6 reduces need**: IPv6 gives each device a globally routable address; NAT traversal becomes unnecessary.

## See Also

- [web-webrtc.md](web-webrtc.md) — WebRTC protocol suite and architecture
- [networking-tcp-ip.md](networking-tcp-ip.md) — IP addressing and routing
- [networking-firewall.md](networking-firewall.md) — Stateful firewalls and packet filtering