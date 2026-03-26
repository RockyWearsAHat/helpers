# Game Networking — Synchronization, Prediction & Lag Compensation

Game networking synchronizes state across client and server machines connected by networks with **variable latency**, **packet loss**, and **bandwidth constraints**. The core challenge: present a responsive, consistent experience despite 50–200ms latency.

## Architectures

### Client-Server

One authoritative server; clients connect to it. Server validates all actions (cheating prevention).

```
Client: Input → Predicted state (optimistic) → Render
        ↓ (send input)
Server: Validate input → Simulate → Broadcast state
        ↓ (send state update)
Client: Receive state → Reconcile prediction error
```

**Strengths**: Anti-cheat; authoritative; scales to many clients on one server.  
**Weaknesses**: Server compute cost; single point of failure; client latency always felt.

### Peer-to-Peer (P2P)

Clients communicate directly; one peer is host/authority (or voting scheme).

```
Client A: Action → Broadcast to B, C, D
Client B: Receive → Validate locally → Simulate
```

**Strengths**: No server cost; lower latency (direct path).  
**Weaknesses**: P2P connectivity hard (NAT, firewalls); easier to cheat; one player's lag affects all.

### Hybrid (Client-Server + Relay)

Server manages state; clients use P2P for low-latency interaction (input prediction, fast collision feedback). Server is source of truth.

## State Synchronization

### Continuous Full State Sync
Every frame, server sends complete world state. Simple; expensive bandwidth (megabytes/sec for complex worlds).

### Delta (Differential) Sync
Server sends only changed properties since last ack. Exponentially reduces bandwidth.

```
Frame 100: Player A moved, health changed → Send {id: A, pos, health}
Frame 101: Nothing changed for A → Send {}
Frame 102: Player A took damage → Send {id: A, health}
```

**Implementation**: Track dirty flags per entity; batch updates; compress on wire.

### Importance Culling (Interest Management)

Distant objects updated less frequently or not at all.

```
Close (<50m): 60 Hz
Medium (50–200m): 20 Hz
Far (>200m): 5 Hz or not at all
```

Bandwidth scales O(visible objects), not O(total objects). Critical for MMOs.

## Client-Side Prediction

The client doesn't wait for server confirmation; it **predicts** locally and renders immediately.

### Flow

```
Frame N:
1. Client inputs movement
2. Client predicts: pos_new = pos + velocity × dt
3. Client renders predicted position
4. Client sends input to server

Server processes, responds with authoritative position

Frame N+K (latency = K frames):
5. Client receives server position
6. If prediction matches: excellent
7. If prediction is wrong: snap/lerp to server position
```

**Strengths**: Instant feedback; responsive feel.  
**Weaknesses**: Prediction errors create visible jitter on snap; prediction must match server logic exactly or des sync.

### Extrapolation

Predict opponent motion based on last known velocity:

```
pos_predicted = last_pos + velocity × (current_time - last_update_time)
```

Breaks on sudden direction changes (dodge, jump); player appears to teleport.

## Server Reconciliation

When server sends authoritative state, client reconciles prediction error.

### Resimulation (Replay)
1. Client receives server state for frame N.
2. Client replays all inputs from N onward using server's logic.
3. Client redraws from new predicted state.

If client logic matches server, no visual skip. Requires **deterministic input→state mapping**.

### Lerp
1. Client smoothly interpolates predicted state toward server state over next frame(s).
2. Hides prediction error; less jarring than snap.

Trade-off: smooth but slightly delayed visuals.

## Lag Compensation (Hitbox Rewinding)

In a shooter, the client fires at where they *see* the target (latency: target is actually ahead). Without compensation, shots miss due to lag.

### Server-Side Rewinding
1. Client fires; sends shot + timestamp.
2. Server rewinds all player positions to time of shot.
3. Server checks collision at rewound positions.
4. Server moves time back to present.

Result: players can hit despite latency.

```
Server time: T_now
Client fires at T_now - latency, targeting player at position P_client_sees

Server rewinds:
  For each player: pos = interpolate(state_history, T_now - latency)
  
Check hitbox collision at rewound position → Result: hit

Restore positions to T_now
```

**Implementation**: Store state history (circular buffer); interpolate at arbitrary times.

## Rollback Netcode (GGPO)

**Rollback** (used in fighting games, RTS) trades simplicity for lower latency.

### Concept

All clients simulate all players locally. On input arrival, roll back to when input was sent; resimulate from that point forward.

```
Client A at Frame 50 (wall-clock 100ms)
Client B at Frame 50 (wall-clock 110ms, slower machine)

A sends input: "attack at Frame 50"
Latency: 30ms

B receives input at Frame 52 (110 + 30 = 140ms wall-clock)
B rewinds to Frame 50; resimulates frames 50, 51, 52 with new input
```

**Strengths**: Extremely low perceived latency (no wait for server).  
**Weaknesses**: Resim is expensive; determinism is **absolute** requirement; older clients resync constantly.

**Determinism requirement**: Same inputs → same state, always, down to floating-point bit level. Typically requires:
- Lock timestep to fixed Hz.
- Disable async tasks.
- Quantize physics (integer or fixed-point).

## Lockstep

All clients wait for all inputs before advancing frame.

```
Frame N:
  Client A sends input
  Client B sends input
  (Whoever finishes first waits)
  All clients receive both inputs
  All clients advance to Frame N+1 (identical state)
```

**Requirements**: Deterministic simulation; network is lossless or retry.  
**Cost**: Frames delay by max(latencies); slow anyone = stalls everyone. Breaks down at 150ms+ latency.

Use: RTS games (classic approach).

## Net Serialization

Compact wire formats to reduce bandwidth.

### Variable-Length Encoding
```
int health:
  1 byte: [tag=health | compressed_value]
float position:
  3 bytes: [quantized_x | quantized_y | quantized_z]
  (16-bit per component; ~0.01 unit precision)
```

### Delta Compression
Only transmit fields that changed.

### Quantization
Reduce precision: float (32-bit) → half-float (16-bit) or quantized int (8–16 bit) with offset/scale.

## Packet Loss & Reliability

### Unreliable (UDP)
Fire and forget. Loss is acceptable for frequent updates (position every frame rebounded by next frame anyway).

### Reliable (TCP-like)
Sequence numbers; acknowledgments; retransmit on timeout. Used for critical state (login, spawn).

### Selective Reliability
Position updates unreliable (high frequency); gear pickups reliable (low frequency but critical).

## See Also

- gamedev-patterns.md (game loop, frame timing)
- gamedev-physics.md (deterministic simulation)
- networking-protocols.md (TCP, UDP, latency, jitter)