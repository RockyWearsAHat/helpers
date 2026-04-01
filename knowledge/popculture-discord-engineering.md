# Discord Engineering Lore: Real-Time Messaging at Scale

## Scaling from Go to Rust

In late 2020, Discord published "Why Discord is Switching from Go to Rust" (engineering blog). The move was surgical: not the entire codebase, but a specific bottleneck: **read state tracking**.

When a user reads a channel, Discord needs to:
1. Mark messages as read (state change)
2. Update the notification badge ("3 unread messages")
3. Sync across all the user's devices (desktop, mobile, web)
4. Respond sub-100ms so the UI feels instant

This state is distributed: millions of users × hundreds of channels they're in = hundreds of millions of read-state entries. On peak server times, Discord's infrastructure was handling 100k+ read state updates per second.

Discord's initial implementation used **Go with a garbage-collected heap**. Go is strongly typed, has good concurrency (goroutines), and compiles to a fast binary. But Go's garbage collector has a problem: **stop-the-world pauses**. Every few milliseconds, the GC freezes the entire runtime to mark and sweep memory. For a few milliseconds, no code runs. This is invisible in most applications but catastrophic for latency-sensitive services: if you're trying to respond to a read-state update in 50ms and the GC pauses for 5ms, you've lost 10% of your budget.

**Rust's advantage**: no garbage collector. Memory is managed at compile time via **ownership and borrowing**. Rust code runs with microsecond-scale predictability because there are no hidden pauses. For the read state service, Rust's deterministic latency was essential.

The trade-off: Rust is harder to write than Go. Rust's compiler is strict. But for a service that processes millions of events per second and must respond in microseconds, the extra development time (weeks vs. days) was worth it.

Discord rewrote the read state service in Rust and deployed it. P99 latency dropped from milliseconds to microseconds. Notification badges updated instantly. The user experience felt snappier.

This wasn't about Rust being "faster" universally; it was about **eliminating GC pauses for a specific latency-sensitive workload**. Discord still uses Go for many services; Rust is a surgical tool.

## Real-Time Messaging at Scale

Discord handles millions of concurrent users sending messages in real-time. This is a fundamentally different problem than HTTP request-response:

**HTTP request-response** (stateless):
- User sends `POST /message` with message content
- Server processes, stores in database
- Server responds with confirmation
- Connection closes
- Next request opens a new connection

**WebSocket** (stateful):
- User opens a persistent connection to the server
- Server keeps the connection open indefinitely
- User sends messages through the same connection
- Server broadcasts to all users in a channel through their connections
- When a new user connects, they get a catch-up of recent messages

Discord uses **sharded WebSocket servers**: the infrastructure is split so that:
- User 1 → Shard 1 (handles guild 1–1000)
- User 2 → Shard 2 (handles guild 1001–2000)
- etc.

This sharding reduces the number of connections any single server handles (thousands instead of millions per machine) and makes failure domains smaller: if Shard 1 crashes, only users in guilds 1–1000 are affected, not the entire platform.

Within each shard, Discord uses:
- **Event sourcing**: every message is an immutable log entry. Updates are appends, not mutations.
- **CRDT-inspired updates**: multiple users can edit the same channel settings (name, topic, permissions) concurrently. Discord resolves conflicts using last-write-wins plus timestamp-based ordering.
- **Presence tracking**: users have "online" status that must be broadcast quickly. Discord batches presence updates and sends them every few seconds to avoid spamming.

## Snowflake IDs (Again)

Discord uses Twitter's Snowflake ID scheme for messages, users, guilds, and roles. This is a deliberate architectural choice adopted across the industry because Snowflake solved the distributed ID generation problem so elegantly.

For Discord, Snowflake IDs enable:
- **Sortable messages**: a Discord channel's message history can be sorted by ID alone (no timestamp column needed).
- **Distributed generation**: Discord can issue IDs from any shard without coordination.
- **Embedded metadata**: the timestamp is in the ID itself, so you can roughly estimate when something was created without a database lookup.

## Bot Ecosystem as an API Economy

Discord's greatest engineering achievement may be its bot ecosystem. Discord provides:
- **Bot API**: authenticated access to read guilds, messages, users, send messages.
- **Webhooks**: lightweight endpoints that external services use to post messages (like GitHub notifications or error alerts).
- **Interactions**: slash commands, buttons, select menus—structured UX elements that bots can define and respond to.

This ecosystem creates a marketplace: bot developers build tools (moderation, music, games), users install bots to their servers, bot creators monetize through Patreon or in-app purchases. Discord gets network effects (more valuable the more bots exist) without building all features themselves.

The engineering required:

- **Rate limiting**: bots must be rate-limited to prevent abuse. Discord uses per-bot quotas (e.g., 50 messages per second per bot), enforced by the API gateway.
- **Webhook routing**: when a message is sent via webhook, Discord must route it to the origin bot's log without congestion.
- **Event delivery**: when a message is posted in a server with a bot registered to that event, Discord must deliver the event to the bot's webhook URL within seconds.

It's essentially an **event-driven architecture** where bots are external subscribers. Discord implemented a pub-sub system where server events (message posted, user joined, reaction added) are published to bot webhooks.

The bot ecosystem also shaped Discord's product roadmap: features like slash commands were added because the alternative (parsing prefix commands like `!help`) was brittle and unintuitive. By building structured interactions into the platform, Discord made bot development easier and the user experience more intuitive.

## Handling Popularity Spikes

Discord's traffic is highly variable:
- **Baseline**: millions of messages per day during normal use
- **Streamer going live**: hundreds of thousands of new users join a server to watch an event; concurrent traffic spikes 10x.
- **Game launch or major event**: a single server can gain 100k users in an hour.

Discord handles spikes with:
- **Overprovisioning**: Discord runs servers with 30–50% utilization during normal times, leaving headroom for spikes.
- **Load shedding**: if a shard is overloaded, new connections are temporarily rejected with a "Try again shortly" message.
- **Lazy loading**: when a user joins a server, Discord doesn't load the entire message history immediately. Instead, it streams recent messages and loads older ones on-demand.
- **Message deduplication**: if a user disconnects and reconnects, Discord doesn't resend messages they already have.

## Underrated: Message Ordering and Write Atomicity

In a distributed system where messages are replicated across shards, a user may see messages arrive out of order or in conflicts (e.g., two users edit the same message simultaneously).

Discord's approach:
- **Single-writer semantics per channel**: only one shard writes to a channel's message log. This ensures total ordering.
- **Replication to read replicas**: the message log is replicated asynchronously to other shards for redundancy.
- **Optimistic updates**: when you type a message, it appears instantly in your view (optimistic). If the shard rejects it (rate-limit, permission error), it's shown as failed and scrolls back out.

This design choice (sacrifice strong consistency for low latency) defines Discord's user experience: you see your own actions immediately, even if other users see a slight delay.

## Key Insights

- **Language choice matters profoundly for latency**: Go's GC pauses made it unsuitable for microsecond-scale latency. Rust's deterministic runtime solved a specific problem. This is not "Rust is better than Go universally"; it's "Rust solved this constraint."
- **Real-time messaging requires rethinking state management**: database transactions don't work at 100k ops/sec. Discord uses immutable event logs and eventual consistency instead.
- **Platform APIs are force multipliers**: Discord's bot ecosystem is worth 10x more in features and engagement than anything the Discord team could build in-house. The API is infrastructure.
- **User experience is built into the engineering**: Discord's choice of optimistic updates + eventual consistency means users feel snappy even though the backend is distributed and eventually consistent.