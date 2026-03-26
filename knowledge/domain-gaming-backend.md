# Game Server Backend — Architecture, Authentication, Matchmaking & State Management

## Overview

Game server backends differ from traditional web backends in their **real-time synchronization demands**, **consistency vs availability trade-offs**, and **player-centric data modeling**. A game server orchestrates player lifecycle (authentication → matchmaking → session → persistence), maintains authoritative game state, detects and prevents cheating, and handles thousands of concurrent connections with latency expectations measured in milliseconds.

## Core Responsibilities

### Player Authentication & Session Management

Game players authenticate once per game session (not per-request like HTTP APIs). Authentication typically involves:

- **Device-level identity** — Account ID, player name, device fingerprint
- **Stat signing** — Client detects cheating via stat validation; client-submitted scores should never be trusted
- **Session tokens** — Short-lived JWT or opaque tokens in game client state
- **Account verification** — Cross-platform linking (Steam, Epic, Xbox, PlayStation, Apple)

Session management differs from web apps: game clients maintain persistent sockets (no stateless request-response). A player has one active session per device; logging in elsewhere invalidates prior sessions or queues disconnect + reconnect.

**Stat signing / anti-cheat at authentication layer:** The server must verify stats came from legitimate gameplay. Pre-validate score deltas (a player's health cannot jump from 10 to 1000 in one frame). Some games compute a hash of scorecard events and verify the hash matches; hash tampering on the client breaks validation.

### Matchmaking

Matchmaking assigns players to games based on skill, latency, preferences, and queue times. Two primary models:

**Centralized matchmaker** — All players join a global queue. A scheduler evaluates all pending matchups using scoring algorithms (minimize latency sum, balance skill ratings, minimize queue time variance). Works for turn-based games and slower-paced online games; reduces queue fragmentation.

**Distributed peer-assisted** — Players form parties, query region-specific servers, and select matches directly. Works for large-scale MMOs; reduces central bottleneck but increases player responsibility for server selection.

**Latency awareness** — Group players by geographic region or ping distance. Use RTT measurements from prior matches or lookups (e.g., GeoIP + latency database Maxmind, ping services). Don't force cross-region matches unless queue time exceeds threshold.

**Rating systems** — Elo, TrueSkill, or Glicko rates player skill. A match's rating delta should reflect expected outcome; upsets grant larger rating swings. Matchmakers pair players with similar ratings to ensure competitive balance.

**Queue positioning** — Communicate queue depth, estimated wait time, and any pending cancellations. Prevent false hope; if 10,000 players are queued ahead, say so.

### Inventory & Economy Systems

Player inventories store items, currency, equipment, cosmetics. Economy systems model supply/demand via pricing, crafting, trading, and progression.

**Inventory architecture:** Typically a document per player (MongoDB, DynamoDB) with categories:

```
player_id: 12345
inventory:
  currency: { gold: 1000, gems: 50 }
  equipment: [{ id: "sword_1", rarity: "rare", durability: 0.8 }, ...]
  cosmetics: [{ id: "skin_legendary_01" }, ...]
  materials: { wood: 100, iron: 50 }
```

**Economy loops:**
- **Earn:** Winning matches, quests, daily bonuses, battle pass progression
- **Spend:** Item purchases, crafting recipes, gacha rolls, upgrades
- **Trade:** Player-to-player markets (if enabled); requires escrow and fraud detection
- **Caps:** Hard limits (e.g., max 10,000 gold) prevent exploit stacks

**Pricing models:** In-game currency (earned through gameplay) vs real-money currency (purchased). Real-money currency often cannot be "farmed" to prevent RMT exploits.

**Durability / upkeep:** Some games require periodic item maintenance to incentivize ongoing engagement and spending. Repair costs create sinks for surplus currency.

### Leaderboards & Rankings

Most games rank players globally or by region. Leaderboards are read-heavy: millions of rank lookups per day but updates only when scores change. Redis sorted sets are the standard approach:

```
ZADD leaderboard:global:season_1 12500 player_123
ZADD leaderboard:global:season_1 12000 player_456
ZREVRANGE leaderboard:global:season_1 0 99  → top 100 players
ZREVRANK leaderboard:global:season_1 player_123  → player's rank
```

**Partitioning:** Split by region (leaderboard:us, leaderboard:eu), season, or game mode to keep datasets manageable. Seasonal resets (monthly, quarterly) prevent permanent stagnation.

**Cheating detection:** Monitor for impossible score jumps. If a player's rating increases 5000 points overnight, flag for review. Cross-check against replay data.

**Decay & resets:** Some games reset leaderboards seasonally or apply Elo decay to inactive players, forcing engagement to maintain rank.

## Anti-Cheat & Replay Systems

### Anti-Cheat Strategies

**Server authority:** The server is always the source of truth. Client inputs → server validates → server computes state → server broadcasts outcome. Never trust client calculations.

**Stat validation:** Impossible events (damage exceeding weapon stats, healing without a ability) indicate tampering.

**Behavior analysis:** Detect inhuman reaction times, perfect aim trajectories, impossible resource accumulation. Machine learning models flag suspicious patterns.

**Replay verification:** Record tick-by-tick inputs and state. Replay the same inputs on a server and compare outcomes. Mismatches = desync or cheat.

**Encryption + signing:** Sign critical game state (player position, health) with a server private key. Client cannot forge valid signatures.

**Challenge-response:** Server periodically sends random challenges (e.g., "compute SHA256(secret + timestamp)"). Client must respond within latency window. Slow responses indicate proxy attack or lag.

### Replay Systems

Replays store the **input stream** (keyboard/mouse) and initial state, allowing deterministic re-execution. They serve three purposes:

1. **Proof for disputes** — Player claims they won but server disagreed? Replay proves the outcome.
2. **Cheat investigation** — Did a replay produce impossible game state? Evidence of hacking.
3. **Player review** — Let players study their own gameplay; social feature (share replays with friends).

**Storage:** Store compressed input deltas (only frame-by-frame changes) rather than full state snapshots. A 30-minute game at 60 FPS = 108,000 frames; storing full state would require gigabytes. Deltas compress to megabytes.

## Game State Persistence & Databases

Game backends must persist:

- **Player profiles** — Name, level, stats, cosmetics, settings
- **Match results** — Winner, duration, kills, economy delta
- **Progression** — Battle pass, achievement, quest state
- **Inventory & currency** — As discussed above

**Database choice:**
- **SQL (PostgreSQL, MySQL)** — Good for relational quests ("player completed quest_5, can now unlock event_7")
- **NoSQL (MongoDB, DynamoDB)** — Schemaless inventory and cosmetics (each item type varies)
- **Hybrid** — SQL for structured data, NoSQL for denormalized player summaries

**Consistency model:** Player operations within a session are linearizable (reads always get latest writes within that session). Cross-session operations are eventually consistent (profile updates take seconds to propagate).

**Write throughput:** Buffer rapid micro-updates (health, ammo) in memory; flush periodically (every 30 seconds or on match end) to avoid write amplification. A player losing 1 HP per frame = 60 damage writes per second; coalescing reduces to 1 write per 30 seconds.

## Real-Time vs Turn-Based Architecture

### Real-Time Games

Player actions have **immediate effect** and **latency is critical**. Requirements:

- **Server tickrate** — Typically 60-120 Hz (ticks per second). Each tick processes inputs, updates state, broadcasts changes.
- **Client-side prediction** — Client assumes next move will succeed, displays it immediately, then corrects if server disagrees. Reduces perceived latency.
- **Lag compensation / rollback** — Some games (fighting games) compute outcomes based on frame-perfect input and roll back state if a player's action invalid.
- **Bandwidth optimization** — Send only state deltas, use delta compression (e.g., Raknet, custom protocols). HTTP/REST is too slow; use UDP or WebSockets.

### Turn-Based Games

Players take sequential actions with **no strict latency requirements**. Turn-based allows:

- **HTTP-based** — Each action is a REST POST; no persistent connection needed
- **Larger computations** — Server can run AI, pathfinding, or complex physics per turn
- **Longer TTL** — Players can think for 60+ seconds; no real-time constraints
- **Defer state broadcast** — Send result when turn ends, not per-keystroke

**Hybrid designs** — Some games combine both (e.g., "real-time match setup, turn-based voting").

## Scaling Patterns

**Game server per match** — Each match (10 players) runs on one server instance. Match discovery via load balancer or matchmaker API. When match ends, server recycled. Works up to ~1M concurrent players with 100-server cluster.

**Horizontal sharding** — Large MMOs partition players across "shards" (server clusters). Shard 1 = players 1-100k, Shard 2 = players 100k-200k. Players in different shards never interact unless teleporting.

**Cross-region replication** — For global games, replicate leaderboards and economy data to regional caches. Use consistent hashing (Memcached, Redis) to avoid hot spots.

**Observer mode** — Don't let spectators join the authoritative server; send them a data stream instead. Spectators don't need 0-latency; 1-2 second delay is acceptable.

## Typical Technology Stack

- **Game server runtime:** Custom C++ (Unreal, custom), Erlang (Elixir GenServer for turn-based), Go (low-latency)
- **Matchmaker:** Custom logic or managed service (AWS GameLift, PlayFab)
- **Leaderboards:** Redis sorted sets, DynamoDB with GSI
- **Inventory:** MongoDB, DynamoDB, PostgreSQL JSONB
- **Persistence:** PostgreSQL (with conn pooling), CockroachDB (global scale)
- **Message queue:** Kafka for economy events (purchase → inventory → leaderboard update)
- **In-memory cache:** Redis for session tokens, hot inventory

## See Also

- [gamedev-networking.md](gamedev-networking.md) — Client-server synchronization, prediction, lag compensation
- [gamedev-ecs.md](gamedev-ecs.md) — Entity-component systems for game state
- [scaling-load-balancing.md](scaling-load-balancing.md) — Load balancing match servers
- [api-rate-limiting.md](api-rate-limiting.md) — Protecting matchmaker API from spam
- [security-rate-limiting-protection.md](security-rate-limiting-protection.md) — Anti-bot matchmaking