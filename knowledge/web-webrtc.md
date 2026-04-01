# WebRTC — Real-Time Communication Principles

## Overview

WebRTC (Web Real-Time Communication) enables peer-to-peer audio, video, and data transfer directly between browsers and devices without requiring plugins or intermediary servers for the media path itself. The architecture addresses a fundamental tension: achieving low-latency communication while navigating the realities of NAT, firewalls, heterogeneous networks, and varying device capabilities.

The protocol suite builds on decades of VoIP and teleconferencing engineering, packaging concepts from SIP, RTP, ICE, and DTLS into a browser-native API surface.

## The Signaling Problem

WebRTC deliberately leaves signaling — the mechanism by which peers discover each other and negotiate connection parameters — unspecified. This is an intentional design decision, not an oversight.

### Why Signaling Is Out of Scope

| Concern               | Implication                                                                       |
| --------------------- | --------------------------------------------------------------------------------- |
| Application diversity | Chat apps, telehealth, gaming, and IoT have wildly different discovery needs      |
| Authentication        | Who can call whom is an application-level policy decision                         |
| Presence and routing  | Whether users are online, busy, or reachable depends on application semantics     |
| Protocol choice       | WebSocket, HTTP polling, XMPP, SIP — each has different infrastructure trade-offs |

### What Signaling Must Exchange

Before a peer connection can be established, both sides need to exchange:

- **Session descriptions** (SDP offers and answers) — what media capabilities each peer has
- **ICE candidates** — network address candidates for connectivity checks
- **Application-level metadata** — room IDs, user identity, call intent

The signaling channel itself needs no special latency guarantees. It carries relatively small control messages — the high-bandwidth media flows over the peer connection once established.

### Signaling Topologies

```
Centralized:    Peer A ←→ Signal Server ←→ Peer B
Federated:      Peer A ←→ Server A ←→ Server B ←→ Peer B
Serverless:     Peer A ←→ (manual exchange, QR code, etc.) ←→ Peer B
```

Most production deployments use centralized signaling for simplicity, though federated approaches exist in contexts where no single entity controls the infrastructure.

## NAT Traversal: ICE, STUN, and TURN

The majority of devices sit behind Network Address Translation (NAT), which means their local IP addresses are not directly reachable from the public internet. Establishing peer-to-peer connections through NAT is one of the harder problems WebRTC solves.

### Why Direct Connections Are Hard

- **Symmetric NATs** create unique mappings per destination, making hole-punching unreliable
- **Firewalls** may block unsolicited inbound UDP traffic
- **Carrier-grade NAT (CGNAT)** adds additional translation layers
- **IPv4 exhaustion** means multiple layers of NAT are increasingly common
- **Enterprise networks** often restrict outbound protocols to HTTP/HTTPS only

### ICE (Interactive Connectivity Establishment)

ICE is a framework for finding the best network path between peers. It gathers multiple candidate addresses and systematically tests connectivity.

**Candidate types, ordered by preference:**

| Type             | Source                   | Latency | Reliability                   |
| ---------------- | ------------------------ | ------- | ----------------------------- |
| Host             | Local network interface  | Lowest  | Only works on same LAN        |
| Server-reflexive | Discovered via STUN      | Low     | Depends on NAT type           |
| Peer-reflexive   | Discovered during checks | Low     | Opportunistic                 |
| Relay            | Allocated via TURN       | Higher  | Works through almost all NATs |

ICE performs connectivity checks using STUN binding requests on all candidate pairs, prioritizing lower-latency paths. The process is:

1. Gather local host candidates
2. Query STUN servers for server-reflexive candidates
3. Optionally allocate TURN relay candidates
4. Exchange all candidates via signaling
5. Perform connectivity checks on candidate pairs
6. Select the best working pair

**Trickle ICE** allows candidates to be sent incrementally as they're discovered, rather than waiting for all gathering to complete. This reduces connection setup time significantly.

### STUN (Session Traversal Utilities for NAT)

STUN servers are lightweight — they simply reflect back the public IP and port a client's packets appear to come from. This lets a peer behind NAT learn its external address.

STUN alone succeeds in roughly 80-85% of cases. It fails when both peers are behind symmetric NATs or restrictive firewalls.

### TURN (Traversal Using Relays around NAT)

TURN servers act as media relays when direct connectivity is impossible. All traffic flows through the TURN server, adding latency and bandwidth cost but guaranteeing connectivity.

**Trade-offs of TURN:**

- Adds a network hop, increasing latency by 10-50ms typically
- The TURN server bears the bandwidth cost of relaying media
- Operational expense scales with concurrent sessions and bandwidth
- Provides the fallback that makes "it always works" possible

In practice, 10-20% of connections in diverse network environments require TURN relay. Enterprise and mobile networks tend toward higher TURN usage.

## SDP: Capability Negotiation

Session Description Protocol (SDP) is the format used to describe multimedia session parameters. In WebRTC, SDP documents are exchanged as "offers" and "answers" during the negotiation process.

### What SDP Contains

```
Session-level metadata (timing, origin)
├── Media description (audio)
│   ├── Codec list with parameters
│   ├── RTP payload type mappings
│   ├── RTCP feedback mechanisms
│   ├── Bandwidth limits
│   └── Encryption parameters (DTLS fingerprint)
├── Media description (video)
│   ├── Codec list and profiles
│   ├── Resolution/framerate capabilities
│   └── Simulcast layers
└── Data channel description
    └── SCTP parameters
```

### The Offer/Answer Model

1. **Offerer** generates an SDP describing its capabilities and preferences
2. SDP is transmitted via the signaling channel
3. **Answerer** examines the offer, intersects with its own capabilities, generates an answer
4. The answer is sent back via signaling
5. Both peers now have a mutually agreed media configuration

Renegotiation can occur mid-session — adding/removing tracks, changing codecs, or adjusting bandwidth constraints triggers a new offer/answer exchange.

### SDP Munging

Applications sometimes modify SDP between generation and transmission to enforce policies (e.g., removing unwanted codecs, setting bandwidth caps). This practice is fragile — SDP syntax is complex and modifications can introduce subtle interoperability issues. The WebRTC community has moved toward APIs like `RTCRtpTransceiver` to make many SDP manipulations unnecessary.

## Media Capture and Constraints

### Constraint System

Media capture uses a constraint model where applications express preferences and requirements:

```
Constraints:
  Required ("exact"):   Width = 1920   → Fail if not achievable
  Ideal:                FrameRate = 30  → Best effort
  Range:                Width 640-1920  → Any value in range acceptable
```

The constraint solver attempts to find device configurations satisfying all requirements while optimizing for ideal values. When no perfect solution exists, the engine may relax ideal constraints or reject the request entirely if required constraints cannot be met.

### Capture Considerations

- **Permission models** vary across platforms — some require per-session consent, others remember grants
- **Device enumeration** reveals hardware fingerprint information, creating privacy trade-offs
- **Echo cancellation** requires careful coordination between capture and playback
- **Platform audio routing** (Bluetooth, speaker, earpiece) involves OS-specific behavior
- **Screen capture** has different permission requirements than camera/microphone

## Codec Negotiation

### Audio Codecs

| Codec | Typical Bitrate | Latency  | Characteristics                                             |
| ----- | --------------- | -------- | ----------------------------------------------------------- |
| Opus  | 6-510 kbps      | 2.5-60ms | Mandatory-to-implement, versatile, handles speech and music |
| G.711 | 64 kbps         | 0.125ms  | Legacy telephony interop, no compression                    |
| G.722 | 48-64 kbps      | ~4ms     | Wideband telephony                                          |

Opus dominates WebRTC audio due to its adaptability — it dynamically switches between speech and music coding modes and adjusts bitrate to network conditions.

### Video Codecs

| Codec | Profile                  | Considerations                                                   |
| ----- | ------------------------ | ---------------------------------------------------------------- |
| VP8   | Mandatory in early specs | Broad support, reasonable efficiency                             |
| VP9   | Optional                 | Better compression ratio, higher CPU cost                        |
| H.264 | Mandatory-to-implement   | Hardware acceleration widely available, licensing considerations |
| AV1   | Emerging                 | Best compression, significant encode cost                        |

Codec selection involves trade-offs between compression efficiency (bandwidth), computational cost (CPU/battery), hardware acceleration availability, and interoperability with existing infrastructure.

### Simulcast and SVC

In multi-party scenarios, senders can produce multiple encodings of the same source:

- **Simulcast**: Multiple independent streams at different resolutions/framerates
- **SVC (Scalable Video Coding)**: Layered encoding where base + enhancement layers provide quality tiers

Both approaches allow intermediary servers to select appropriate quality levels per receiver without transcoding.

## Data Channels

WebRTC data channels provide arbitrary bidirectional data transfer over the peer connection, built on SCTP over DTLS.

### Characteristics

| Property    | Options                                                                       |
| ----------- | ----------------------------------------------------------------------------- |
| Ordering    | Ordered or unordered                                                          |
| Reliability | Reliable, partial reliability (max retransmits), or unreliable (max lifetime) |
| Priority    | Influences scheduling when bandwidth is constrained                           |
| Protocol    | Application-defined subprotocol label                                         |

### Use Cases and Trade-offs

- **Reliable ordered**: File transfer, chat messages — TCP-like semantics
- **Unreliable unordered**: Game state updates, sensor data — minimal latency, stale data is acceptable
- **Partial reliability**: Screen sharing annotations — recent data matters, old loss is tolerable

Data channels share the same NAT traversal and encryption infrastructure as media, avoiding the need for separate connectivity mechanisms.

## Architecture: Peer-to-Peer vs Server-Mediated

### Mesh (Pure P2P)

Every participant connects directly to every other participant.

```
Connections = N × (N-1) / 2

3 participants: 3 connections
5 participants: 10 connections
10 participants: 45 connections
```

**Strengths**: No server infrastructure for media, lowest latency between pairs, no single point of failure.

**Weaknesses**: Upload bandwidth scales linearly with participants (each sender encodes and transmits N-1 streams), CPU cost of multiple encodes, practical limit of ~4-6 participants for video.

### SFU (Selective Forwarding Unit)

Each participant sends media once to the server; the server forwards selected streams to each receiver.

```
Participant → SFU → Participant A
                  → Participant B
                  → Participant C
```

**Strengths**: Sender uploads once regardless of participant count, server can select quality layers per receiver, recording is straightforward, scales to dozens of participants.

**Weaknesses**: Requires server infrastructure and bandwidth, adds one network hop of latency, the server is a single point of failure and a scaling bottleneck.

### MCU (Multipoint Control Unit)

The server decodes all incoming streams, composites them into a single mixed stream, re-encodes, and sends one stream per receiver.

**Strengths**: Minimal client bandwidth (receives one stream), works with very constrained clients, consistent layout.

**Weaknesses**: Server bears enormous computational cost (decode + composite + encode per output), adds latency from processing pipeline, reduces layout flexibility, limits quality to server's encoding choices.

### Architecture Selection Considerations

| Factor                    | Mesh                | SFU                 | MCU               |
| ------------------------- | ------------------- | ------------------- | ----------------- |
| Max participants (video)  | ~4-6                | ~50-100             | ~20-50            |
| Server cost               | None                | Moderate            | High              |
| Client CPU                | High                | Low-moderate        | Low               |
| Client bandwidth (upload) | High                | Low                 | Low               |
| Latency added             | None                | Low                 | Moderate          |
| Recording                 | Hard                | Easy                | Easy              |
| Layout flexibility        | Full client control | Full client control | Server-determined |

Most production conferencing systems use SFU architecture as the pragmatic middle ground.

## Bandwidth Estimation and Adaptive Bitrate

### The Estimation Problem

Network conditions change continuously — available bandwidth, latency, and packet loss fluctuate as paths shift and competing traffic varies. WebRTC implementations must estimate available bandwidth in real-time and adapt media encoding accordingly.

### Estimation Approaches

- **Loss-based**: High packet loss indicates congestion; reduce sending rate
- **Delay-based**: Increasing one-way delay suggests queue buildup; reduce before loss occurs
- **Hybrid**: Combine loss and delay signals for more nuanced response

Delay-based estimation tends to be more responsive but requires accurate clock synchronization. Loss-based is simpler but reacts after congestion has already caused visible quality degradation.

### Adaptation Mechanisms

When bandwidth estimates change, the system can adjust:

- Video resolution (spatial adaptation)
- Video frame rate (temporal adaptation)
- Video codec quantization (quality adaptation)
- Audio codec bitrate
- Simulcast layer selection (at SFU)

The adaptation controller must balance responsiveness (adapting quickly to changes) against stability (avoiding oscillation between quality levels).

## Security Model

### Mandatory Encryption

WebRTC mandates encryption for all media and data. This is not optional or configurable — it is a core architectural requirement.

- **DTLS** (Datagram Transport Layer Security) secures the key exchange
- **SRTP** (Secure Real-time Transport Protocol) encrypts media
- **DTLS-SRTP** combines them: DTLS negotiates keys that SRTP uses for media encryption

### Trust Model

The security model protects against network eavesdroppers but depends on correct certificate fingerprint exchange during signaling. If the signaling channel is compromised, a man-in-the-middle can substitute certificate fingerprints.

Approaches to strengthen the trust model:

- Authenticate the signaling channel (TLS for WebSocket, etc.)
- Display certificate fingerprints for manual verification
- Use identity providers to bind peer identities to certificates

### End-to-End Encryption Considerations

Standard WebRTC encryption terminates at each peer (or at an SFU, which can access unencrypted media). True end-to-end encryption through an SFU requires additional mechanisms:

- **Insertable Streams / Encoded Transform**: APIs allowing applications to encrypt frame payloads before the SFU sees them
- **SFrame**: A proposed framing format for end-to-end encrypted media

These approaches prevent the SFU from accessing media content but limit its ability to perform quality adaptation, since it cannot inspect or modify stream content.

## Scalability Challenges

### Why Pure Mesh Degrades

The fundamental issue is combinatorial: each participant must encode and transmit a separate stream to every other participant. With video, this means:

- N-1 simultaneous video encodes (CPU-bound)
- N-1 × bitrate upload bandwidth
- N-1 incoming streams to decode and render

Even with hardware acceleration, client devices hit practical limits quickly. Mobile devices are particularly constrained.

### Server-Side Scaling

SFU architectures face their own scaling challenges:

- **Bandwidth**: Forwarding N inputs to N outputs requires N² bandwidth at the SFU
- **Geographic distribution**: Participants across regions benefit from cascaded SFUs, adding complexity
- **Session affinity**: Participants in the same room typically must connect to the same SFU cluster
- **Failover**: SFU failure drops all participants; state transfer for seamless failover is complex

### Large-Scale Patterns

For events with hundreds or thousands of viewers, hybrid architectures become common:

- Active speakers use WebRTC to an SFU
- The SFU re-publishes to a CDN for passive viewers via HLS/DASH
- Viewers who want to "raise hand" upgrade from CDN playback to a WebRTC connection

## Reliability vs Latency Trade-offs

Real-time communication exists on a spectrum between reliability and latency:

| Approach                       | Latency Impact          | Reliability                                |
| ------------------------------ | ----------------------- | ------------------------------------------ |
| No retransmission              | Lowest                  | Lossy — gaps when packets lost             |
| NACK-based retransmission      | Low-moderate            | Recovers recent loss, bounded by RTT       |
| FEC (Forward Error Correction) | Adds bandwidth overhead | Recovers loss without retransmission delay |
| Redundant coding               | Adds bandwidth overhead | Protects against burst loss                |
| Jitter buffer (large)          | Higher                  | Smooths arrival timing variance            |
| Jitter buffer (small)          | Lower                   | More sensitive to network jitter           |

Production implementations typically combine several mechanisms and tune aggressively based on measured network conditions. Voice traffic prioritizes low latency over perfect reconstruction (listeners tolerate brief glitches). Video can tolerate slightly more latency for smoother playback. Data channels offer explicit reliability configuration per channel.

### The Buffering Dilemma

Jitter buffers absorb network timing variation but add latency. Adaptive jitter buffers attempt to minimize buffer depth while maintaining smooth playout:

- Too small: frequent audio glitches, video freezes
- Too large: perceptible delay, awkward conversation dynamics
- Adaptive: adjusts based on measured jitter, but adaptation speed involves its own trade-offs

The target for conversational audio is typically under 150ms one-way delay — beyond that, natural conversation flow breaks down as participants begin talking over each other.
