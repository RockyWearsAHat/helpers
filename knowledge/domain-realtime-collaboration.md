# Real-Time Collaboration — CRDTs, OT, Conflict Resolution & Shared State

## Overview

Real-time collaboration systems let multiple users edit shared documents (text, spreadsheet, whiteboard) simultaneously and see each other's changes instantly. The challenge: **conflict resolution**. When two users edit the same paragraph simultaneously, who wins? Real-time collaboration algorithms ensure eventual consistency (all users converge to same state) without requiring a central authority to arbitrate every keystroke.

Two families of algorithms exist: **Operational Transformation (OT)** and **Conflict-Free Replicated Data Types (CRDTs)**. Both aim for **convergence** (users agree on final state) and **causality preservation** (if edit A happened before edit B, it appears in that order everywhere).

## Core Problem: Consistency Without Coordination

**Naive approach (not viable):** Lock the document. Only one user can edit at a time. Serializes all edits; no parallelism. Bad UX (user A's keystroke blocks user B).

**Desired:** Decentralized. Each user applies edits locally immediately (local-first), then syncs with peers. Two users can edit simultaneously; edits eventually converge.

**The conflict:** User A inserts "hello" at position 0. User B inserts "world" at position 0. Both see different results:
- A's view: "hello[world]…"
- B's view: "world[hello]…"

How do they converge? Need a conflict resolution algorithm.

## Operational Transformation (OT)

**Concept:** Transform edits to account for concurrent operations. Each edit is an **operation** (insert, delete); operations compose.

### How OT Works

**Operations:**
- `Insert(position, text)` — insert text at position
- `Delete(position, length)` — delete length characters at position

**Example:**

```
Initial state: "abc"

User A: Insert(1, "x") → "axbc"
User B: Insert(3, "y") → "abcy"

Concurrent execution:
A inserts at position 1 (doesn't see B's insertion yet)
B inserts at position 3 (doesn't see A's insertion yet)

Convergence:
A receives B's operation: Insert(3, "y")
  But A already inserted at position 1, shifting positions
  Transform: Insert(3, "y") + existing Insert(1, "x") = Insert(4, "y")
  Result: "axbcy"

B receives A's operation: Insert(1, "x")
  Transform: Insert(1, "x") relative to B's Insert(3, "y")
  Result: "axby" (unchanged; A's insert doesn't affect B's position 3)
```

**Transform function:** `transform(operation_A, operation_B)` produces `operation_A'` that, when applied after `operation_B`, produces the same result as if both happened in order.

**Properties required:**
- **Commutativity:** Applying ops in any order produces same result (A then B = B then A)
- **Idempotence:** Applying same operation twice doesn't break convergence
- **Causality preservation:** If A's edit causes B's edit, the order is preserved

**Centralized OT:** Server is authority. All clients send operations to server; server transforms and broadcasts. Guarantees convergence but requires server roundtrip (higher latency).

**Decentralized OT (p2p):** More complex. Each peer must track **causal history** (which operations came before this one) to avoid conflicts. Requires vector clocks or similar.

### OT Strengths & Weaknesses

**Strengths:**
- Minimal overhead (each operation is small)
- Compatible with existing algorithms (editing, version control)
- Well-understood (Google Docs uses OT)

**Weaknesses:**
- Transform functions are complex to implement correctly (often buggy)
- Doesn't integrate well with p2p systems; decentralized OT is hard
- Difficult to extend (adding new operations requires new transform rules)

## Conflict-Free Replicated Data Types (CRDTs)

**Concept:** Assign each character a unique identifier (UUID + causal timestamp) so concurrent inserts never conflict. All users apply operations in the same order and converge.

### How CRDTs Work

**Each character gets:**
- Content (the character)
- Position identifier (never changes)

**Position identifiers:** Use fractional positions so any number of insertions can happen "between" two positions.

**Example:**

```
Initial state: "ab" with positions [(a, 1.0), (b, 2.0)]

User A inserts "x" between a and b: (x, 1.5) → "axb"
User B inserts "y" between a and b: (y, 1.3) → "ayb"

Both users end up with positions:
[(a, 1.0), (y, 1.3), (x, 1.5), (b, 2.0)] → "ayxb"
```

**Implementation:** Store document as sorted list of (position_id, content) pairs. Insert always appends to position space; can be left of or right of any existing position.

**Tombstones for deletion:** Don't remove characters; mark deleted. Why? If user A deletes and user B later inserts after that char, we need to know where insertion goes. Tombstones preserve causal history.

```
[(a, 1.0), (x_deleted, 1.5), (b, 2.0)]
Render: "ab" (skip deleted)
```

**Merging:** Combine two document states. Each peer keeps full causal history. When peers sync:
- User A has edits: [Op1, Op3, Op5]
- User B has edits: [Op1, Op2, Op4]
- After merge: sorted by position ID → deterministic order

### CRDT Strengths & Weaknesses

**Strengths:**
- Always correct; no transform functions to compose
- Garbage-free (no need to coordinate who deletes what)
- Works in fully p2p / offline-first environments
- Scales to millions of operations

**Weaknesses:**
- Metadata overhead: each character needs unique ID (position + replica ID + timestamp) = 16-32 bytes per character
- Large documents (1M+ characters) use significant RAM
- Deletion (tombstones) doesn't actually free storage until compaction

## Popular CRDT Libraries

**Y.js** — Most popular for web collaboration. CRDT implementation optimized for text, arrays, maps. Works with:
- `y-websocket` for server sync
- `y-webrtc` for p2p
- Providers: Yjs works with Notion, HocusPocus servers, custom backends

**Automerge** — JSON-CRDT for complex data structures. Native representations of nested objects, arrays. Focuses on version history and time-travel debugging. Slower than Y.js (pure Rust/JS, not optimized) but more flexible.

**Operational Transformation libraries:**
- Google Closure Library (used in Google Docs)
- ShareDB (OT + WebSocket)

## Presence Awareness & Cursor Tracking

Collaboration needs **awareness:** who's online? Where is cursor? What's their name?

**Presence state:**
```
{
  user_id: 123,
  name: "Alice",
  color: "#FF5733",
  cursor_position: 45,
  selection_start: 40,
  selection_end: 50,
  idle_since: "2024-03-25T14:22:00Z"
}
```

**Broadcast:** Presence updates are frequent (on keypress, mouse move) but not critical. Broadcast to all peers via broadcast channel or WebSocket. Don't persist; ephemeral.

**Rendering:**
- Draw other users' cursors in different colors
- Show selection ranges as colored backgrounds
- Display user name/avatar on hover

**Idle detection:** Stop broadcasting after 30s inactivity to reduce bandwidth.

**Offline handling:** When P2P network partitions, presence becomes stale. Show "(offline)" badge next to cursor.

## Version History & Time Travel

Documents should record edit history for versioning, blame, undo/redo.

**Checkpoint model:**
- Store full document snapshot at regular intervals (every 10 edits or 5 minutes)
- Store delta (incremental edits) between checkpoints
- To restore version at time T: load checkpoint before T, apply deltas up to T

**Example:**
```
Checkpoint t=0: "hello" (size 100 bytes)
Edits t=1-10: [+5 bytes, -3 bytes, +2 bytes...] (50 bytes total)
Checkpoint t=10: "hello world" (size 110 bytes)
Edits t=11-20: [+10 bytes...] (30 bytes)

To restore t=15: load checkpoint@t=10, apply edits 11-15
```

**Query:** "Who deleted line 5?" — Trace back through edit history, find delete operation, attribute to user.

**Time travel:** Allow users to scrub through document history. Show document state at any point in time. Useful for education (show step-by-step how document evolved) and debugging.

## Multiplayer Text Editing Challenges

**Latency:** User types, sees keystroke immediately (local-first), but remote user sees it after network roundtrip (100-500ms). Use optimistic UI: show my edits instantly, correct if server disagrees.

**Bandwidth:** Sending full document on every edit is wasteful. Send only delta (one keystroke = 1 operation, 50 bytes). At 5 keystrokes/sec = 250 bytes/sec per user.

**Mobile:** Network unreliable. Messages drop. Use acknowledgment + retry (if no ack after 5s, resend).

**Pagination:** For large documents (1000+ pages), sending full sync on connect is slow. Lazy-load visible region + buffer (forward 10 pages, backward 5 pages).

**Undo/redo:** With multiple users, undo is ambiguous. User A types "hello", User B inserts between. User A presses Ctrl+Z. Should undo:
- All of "hello"? (undo my entire edit)
- Last keystroke? (undo "o", now "hell")

Most systems do per-user undo: User A's undo only reverts A's edits, not B's.

## Collaborative Whiteboard

Whiteboard elements (lines, shapes, text) are objects with position, size, color, stroke style.

**CRDT representation:**
```
shape {
  id: unique_id,
  type: "line",
  x1, y1, x2, y2: float,
  stroke_color: string,
  stroke_width: float,
  created_by: user_id,
  created_at: timestamp,
  tombstone: boolean (deleted?)
}
```

**Challenge:** Drawings can have thousands of shapes. Sending each shape update (x1, y1, x2, y2 changed) per-mouse-move = 60 updates/sec × 1000 shapes = 60K operations/sec. Too much.

**Optimization:** Only send bounding-box updates when drawing finishes (user releases mouse). During active draw, render locally only. Send final shape on mouse-up.

**Gesture recognition:** Recognize drawn shapes (hand-drawn rectangle → clean rectangle, squiggle → arrow). Client-side ML model; also helps stabilize shaky drawings.

## Hybrid Architectures

Some systems use **collaborative cloud storage** (Google Drive, Notion):
- Central server is authority (OT or CRDT server-side)
- Clients send operations to server
- Server broadcasts to all clients
- Higher latency but guaranteed consistency

**P2P collaboration** (Figma with local-first mode, Automerge-based apps):
- Each peer has full replica
- Peers sync via WebRTC, WebSocket
- No central server (offline-first)
- Trade-off: higher complexity, potential consistency issues if peers network-partition long term

## Typical Tech Stack

| Component | Examples |
| --- | --- |
| CRDT/OT algorithm | Y.js, Automerge, ShareDB |
| Transport | WebSocket, WebRTC, QUIC |
| Persistence | IndexedDB (browser), SQLite (Electron), PostgreSQL (cloud) |
| Server sync | HocusPocus, Figma's backend, custom |
| UI framework | React + contenteditable, Monaco Editor, ProseMirror |
| DevTools | Y.js devtools, Automerge history viewer |

## Design Trade-offs

| Aspect | OT | CRDT |
| --- | --- | --- |
| Correctness | Complex (bugs possible) | Guaranteed |
| Latency | Depends on server response | Instant (optimistic UI) |
| P2P | Difficult | Natural |
| Metadata overhead | Small | Large (position IDs) |
| Learning curve | Medium | Steep |
| Integrations | Mature (Google Docs) | Growing (Figma, Obsidian) |

## See Also

- [distributed-data-consistency.md](distributed-data-consistency.md) — Consistency models, CAP theorem
- [gamedev-networking.md](gamedev-networking.md) — Client-server state sync, prediction
- [networking-websocket-patterns.md](networking-websocket-patterns.md) — WebSocket for real-time delivery
- [data-event-modeling.md](data-event-modeling.md) — Event-based architectures
- [architecture-event-sourcing.md](architecture-event-sourcing.md) — Event history as source of truth