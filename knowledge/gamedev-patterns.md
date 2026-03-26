# Game Development Engineering Patterns

Game development sits at the intersection of real-time systems, graphics programming, physics simulation, and interactive design. The engineering patterns that emerge from this domain reflect a constant negotiation between performance, expressiveness, and maintainability — with frame-time budgets imposing hard constraints that most application domains never face.

## The Game Loop

Every interactive application revolves around a loop that processes input, updates state, and renders output. The structure of this loop has profound implications for simulation correctness, visual smoothness, and CPU utilization.

### Fixed vs Variable Timestep

| Approach          | Mechanism                                        | Strengths                                      | Weaknesses                                             |
| ----------------- | ------------------------------------------------ | ---------------------------------------------- | ------------------------------------------------------ |
| Fixed timestep    | Update logic runs at a constant dt (e.g., 1/60s) | Deterministic simulation, reproducible physics | Decoupled from display refresh; may need interpolation |
| Variable timestep | dt = time since last frame                       | Naturally adapts to hardware speed             | Non-deterministic; physics instability at large dt     |
| Semi-fixed        | Fixed update + variable rendering                | Deterministic core with smooth visuals         | Implementation complexity; interpolation overhead      |

The **accumulator pattern** bridges fixed-timestep logic with variable frame rates. The loop accumulates elapsed real time, then drains it in fixed-size steps:

```
accumulator += elapsed_time
while accumulator >= FIXED_DT:
    update_simulation(FIXED_DT)
    accumulator -= FIXED_DT
alpha = accumulator / FIXED_DT
render(interpolate(previous_state, current_state, alpha))
```

The interpolation factor `alpha` produces smooth rendering between discrete simulation states. Without it, objects visually stutter even when the simulation is stable.

### Frame Pacing Considerations

Consistent frame delivery matters more than raw frame rate for perceived smoothness. A game running at a locked 30fps often feels smoother than one fluctuating between 40-60fps. Approaches to frame pacing include vertical sync (vsync), adaptive sync protocols, and frame queuing strategies — each trading latency against visual consistency.

## Entity-Component-System (ECS)

Traditional object-oriented game architectures tend toward deep inheritance hierarchies — `GameObject → Character → Enemy → FlyingEnemy`. ECS emerged as a data-oriented alternative that favors composition over inheritance.

### Core Concepts

- **Entity**: An identifier (often just an integer) with no behavior or data of its own
- **Component**: A plain data container attached to an entity — position, velocity, health, sprite
- **System**: Logic that operates on entities possessing specific component combinations

```
Entity 42:  [Position, Velocity, Sprite]       → processed by MovementSystem, RenderSystem
Entity 43:  [Position, Velocity, Sprite, AI]    → processed by MovementSystem, RenderSystem, AISystem
Entity 44:  [Position, Sprite]                  → processed by RenderSystem only
```

### Architectural Trade-offs

| Dimension           | Inheritance Hierarchies    | ECS                                     |
| ------------------- | -------------------------- | --------------------------------------- |
| Adding behavior     | New class or mixin         | Attach component + system queries it    |
| Data layout         | Scattered across objects   | Can be tightly packed by component type |
| Runtime flexibility | Limited by class hierarchy | Entities reconfigured at runtime        |
| Debuggability       | Familiar object inspection | Component state spread across tables    |
| Learning curve      | Familiar OOP patterns      | Requires rethinking object identity     |

ECS does not universally outperform OOP designs. Small-scale games with clear hierarchies may find traditional approaches simpler. ECS tends to shine when entity types are fluid, entity counts are large, or cache performance is critical.

### Archetype vs Sparse Set Storage

ECS implementations differ in how they store component data. **Archetype storage** groups entities sharing the same component set into contiguous arrays — excellent cache behavior for iteration, costly when components are added/removed. **Sparse set storage** maps entity IDs to component indices — faster structural changes, potentially worse iteration locality.

## Data-Oriented Design

ECS often aligns with broader data-oriented design (DOD) principles, which prioritize how data flows through the CPU cache hierarchy over how code is conceptually organized.

### Why Cache Coherence Matters

Modern CPUs fetch memory in cache lines (typically 64 bytes). Accessing one byte pulls the entire line into L1 cache. Sequential access patterns that touch adjacent memory exploit prefetching hardware. Random access patterns — common in pointer-heavy OOP designs — trigger cache misses costing 100+ cycles each.

For a game updating 10,000 entities per frame at 60fps, the difference between cache-friendly and cache-hostile layouts can be the difference between meeting and missing the 16.6ms frame budget.

**Structure of Arrays (SoA)** vs **Array of Structures (AoS)**:

- AoS: `[{pos, vel, health}, {pos, vel, health}, ...]` — groups all data per entity
- SoA: `{[pos, pos, ...], [vel, vel, ...], [health, health, ...]}` — groups all data per component type

When a system only needs position and velocity (not health), SoA avoids loading irrelevant health data into cache lines.

## Scene Graphs and Spatial Partitioning

### Scene Graphs

A scene graph organizes objects hierarchically based on spatial relationships. A character entity might parent arm, leg, and head entities — transforming the parent applies to all children. Scene graphs simplify articulated objects, UI layouts, and relative positioning but can impose overhead when hierarchies are deep or frequently restructured.

### Spatial Partitioning Structures

Efficiently answering "what objects are near point X?" is fundamental to collision detection, rendering culling, and AI queries.

| Structure | Dimensionality | Splitting Strategy             | Best Suited For                      |
| --------- | -------------- | ------------------------------ | ------------------------------------ |
| Quad-tree | 2D             | Recursive quadrant subdivision | Tile-based games, 2D collision       |
| Octree    | 3D             | Recursive octant subdivision   | Open-world 3D environments           |
| BSP tree  | 2D/3D          | Hyperplane partitioning        | Static geometry, indoor environments |
| Grid      | 2D/3D          | Uniform cell division          | Evenly distributed objects           |
| BVH       | 2D/3D          | Bounding volume hierarchy      | Ray tracing, dynamic objects         |
| R-tree    | 2D/3D          | Minimum bounding rectangles    | Database-style spatial queries       |

The choice depends on how dynamic the scene is, how uniformly objects are distributed, and whether the structure needs frequent rebuilding.

## Physics Simulation

### Rigid Body Dynamics

Physics engines typically simulate rigid bodies — objects that don't deform. Each body has mass, inertia tensor, position, orientation, linear velocity, and angular velocity. At each timestep, forces and torques are integrated to produce new velocities, then velocities are integrated to produce new positions.

The **symplectic Euler** integrator (update velocity first, then position using new velocity) provides reasonable stability for game physics without the cost of higher-order methods.

### Collision Detection

Collision detection typically proceeds in two phases:

**Broad phase** — quickly eliminates pairs that cannot possibly collide:

- Axis-Aligned Bounding Box (AABB) overlap tests
- Sweep-and-prune along sorted axes
- Spatial hashing into grid cells

**Narrow phase** — determines exact contact between candidate pairs:

- GJK (Gilbert-Johnson-Keerthi) algorithm for convex shapes
- SAT (Separating Axis Theorem) for convex polytopes
- Mesh-level triangle intersection for complex geometry

### Constraint Solvers

After detecting contacts, a constraint solver prevents interpenetration and applies friction. Sequential impulse solvers iterate through contact points, applying corrective impulses. More iterations improve accuracy at higher computational cost. The trade-off between solver iterations and frame budget is a constant tension in game physics.

## Rendering Pipeline

Modern real-time rendering follows a pipeline from geometry to pixels.

### Pipeline Stages

1. **Vertex processing** — Transforms vertices from model space through world, view, and projection matrices. Vertex shaders may also compute lighting, skinning, or procedural deformation.

2. **Primitive assembly & clipping** — Groups vertices into triangles, clips against the view frustum, discards off-screen geometry.

3. **Rasterization** — Determines which pixels (fragments) each triangle covers. Interpolates vertex attributes (color, texture coordinates, normals) across the triangle surface.

4. **Fragment processing** — Fragment shaders compute per-pixel color using textures, lighting models, shadows, and material properties. The most computationally expensive stage for visually complex scenes.

5. **Output merge** — Depth testing, stencil testing, and blending combine fragment results into the final framebuffer.

### Forward vs Deferred Rendering

| Aspect           | Forward Rendering          | Deferred Rendering                                    |
| ---------------- | -------------------------- | ----------------------------------------------------- |
| Light scaling    | O(objects × lights)        | O(objects) + O(lights × screen pixels)                |
| Memory           | Low — single pass          | High — G-buffer stores position, normal, albedo, etc. |
| Transparency     | Natural alpha blending     | Requires separate forward pass                        |
| Anti-aliasing    | MSAA works naturally       | MSAA costly; post-process AA preferred                |
| Material variety | Unlimited shader variation | G-buffer format constrains material data              |
| Bandwidth        | Lower                      | Higher — multiple render targets                      |

Many engines use hybrid approaches — deferred for opaque geometry, forward for transparent objects and special materials.

### Level of Detail (LOD)

Objects far from the camera contribute fewer pixels. LOD systems swap high-detail meshes for simpler versions based on distance, screen-space coverage, or importance. Approaches include discrete LOD (distinct mesh versions), continuous LOD (progressive mesh simplification), and impostor systems (billboard sprites for distant objects). The challenge lies in avoiding visible "popping" during transitions.

## Input Handling

### Polling vs Event-Driven

- **Polling**: Check input state each frame. Simple, predictable timing, but can miss brief inputs between frames.
- **Event-driven**: Process input events from a queue. Captures all inputs, but event timing relative to simulation steps requires care.

Most game input systems combine both — events fill a buffer, which is polled during the update phase.

### Input Buffering and Dead Zones

**Input buffering** stores recent inputs to forgive imprecise timing. Fighting games commonly buffer attack inputs for several frames so that a player pressing "punch" slightly before landing still triggers the move.

**Dead zones** define a threshold below which analog stick displacement is treated as zero, preventing drift from imprecise hardware. Dead zone shape (circular vs axial) affects diagonal sensitivity.

## State Machines for Game Entities

Finite state machines (FSMs) model entity behavior as discrete states with transitions:

```
Idle → [detect player] → Chase → [in range] → Attack → [health low] → Flee
  ↑                                                                      |
  └──────────────────── [player lost] ←──────────────────────────────────┘
```

### Hierarchical State Machines

Flat FSMs grow unwieldy as states multiply. Hierarchical state machines (HFSMs) nest states — an "InCombat" superstate might contain "Melee," "Ranged," and "Dodge" substates, sharing common transitions like "TakeDamage."

### Behavior Trees

Behavior trees offer an alternative that scales better for complex AI. Nodes represent actions or decisions, composed via sequences (do all in order), selectors (try until one succeeds), and decorators (modify child behavior). The tree is traversed each tick, naturally supporting interruption and priority-based behavior.

## Network Multiplayer Concepts

### Architecture Models

- **Client-server**: One authoritative server validates all state. Clients send inputs, receive state updates. Resistant to cheating but requires server infrastructure.
- **Peer-to-peer**: Each peer simulates locally, shares state with others. Lower infrastructure cost but harder to prevent cheating and handle asymmetric latency.
- **Relay server**: A lightweight server forwards messages without simulation authority. Combines some benefits of both models.

### Latency Compensation

Network latency between client input and server response creates perceived lag. Techniques to mitigate this:

- **Client-side prediction**: The client simulates its own inputs immediately, then reconciles with server corrections. Mispredictions cause "rubber-banding."
- **Server reconciliation**: When the server confirms a past input, the client replays subsequent inputs from that confirmed state.
- **Entity interpolation**: Remote entities are rendered slightly in the past, smoothly interpolating between received positions.
- **Rollback netcode**: The simulation can rewind to a past state, apply a late-arriving input, and re-simulate forward. Common in fighting games where frame-accurate input matters.

### State Synchronization

Full state snapshots consume bandwidth. Delta compression sends only what changed. Interest management limits updates to entities relevant to each client. Quantization reduces precision to compress numeric values. The trade-off is always bandwidth versus consistency.

## Asset Loading and Resource Management

Games load textures, meshes, audio, animations, and level data — often gigabytes total. Strategies differ based on target platform and game structure:

- **Streaming**: Load assets progressively as needed, often tied to player position. Open-world games depend on this.
- **Bundling**: Package related assets together to reduce I/O operations. Bundle granularity balances load time against memory waste.
- **Reference counting**: Track how many systems use each asset; unload when the count reaches zero.
- **Async loading**: Load assets on background threads to avoid frame hitches. Requires synchronization when assets become available.

## Memory Allocation Strategies

General-purpose allocators (malloc/new) introduce fragmentation and unpredictable latency — problematic when every microsecond of a 16.6ms frame budget counts.

### Custom Allocator Patterns

| Allocator              | Mechanism                        | Use Case                                     |
| ---------------------- | -------------------------------- | -------------------------------------------- |
| Pool allocator         | Pre-allocated fixed-size blocks  | Particles, bullets, entities of uniform size |
| Frame/linear allocator | Bump pointer, reset each frame   | Per-frame temporary data                     |
| Stack allocator        | LIFO allocation/deallocation     | Scoped temporary allocations                 |
| Double-buffered        | Two frame allocators alternating | Current-frame and previous-frame data        |
| Free list              | Linked list of freed blocks      | Variable-size allocations with reuse         |

These patterns trade generality for predictable performance. A frame allocator that simply resets a pointer each frame has effectively zero allocation cost but requires that all allocations are truly frame-scoped.

## Cross-Cutting Concerns

### Determinism

Replay systems, lockstep multiplayer, and automated testing all require deterministic simulation — identical inputs must produce identical outputs. Floating-point non-determinism across platforms, hash map iteration order, and thread scheduling all threaten determinism. Fixed-point arithmetic, ordered containers, and single-threaded simulation updates are common countermeasures.

### Profiling and Frame Budgets

GPU and CPU profiling tools measure where time is spent within a frame. A typical 60fps budget of 16.6ms might be allocated:

- Input processing: < 1ms
- Physics simulation: 2-4ms
- AI and gameplay logic: 2-4ms
- Rendering submission: 3-5ms
- GPU rendering: 8-12ms (overlapped with next frame's CPU work)

When the budget is exceeded, something must be simplified — fewer physics iterations, lower LOD thresholds, reduced draw distance, or simpler shaders. These are engineering trade-offs, not failures.

### Hot Reloading

Iterating on gameplay requires rapid feedback. Hot reloading systems allow code, shaders, or data to be modified while the game runs. This introduces complexity — serializing/deserializing live state, handling schema changes, maintaining system invariants across reloads — but dramatically accelerates development iteration.
