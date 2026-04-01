# Entity Component System (ECS) — Data-Driven Composition, Storage & Query Patterns

Entity-Component-System (ECS) inverts the class hierarchy paradigm. Instead of representing game objects as monolithic classes, ECS separates **identity** (entity IDs), **state** (components), and **behavior** (systems). This decoupling enables data-oriented design, cache-coherent memory layouts, and straightforward parallelism.

## The Core Model

### Entities
An entity is a unique ID—a handle with no inherent data. In implementations like Bevy and EnTT, entities are often `(generation, index)` pairs:
- **Generation**: Incremented when an old index is reused (detects "use after free")
- **Index**: Stable slot in the entity array

This design catches bugs early (using a deleted entity raises an error) and enables efficient recycling of slots.

### Components
Components are **pure data structures** attached to entities. No methods, no behavior. Examples:
```
Position { x: f32, y: f32 }
Velocity { vx: f32, vy: f32 }
Health { hp: i32, max: i32 }
```

The principle of "data without behavior" is enforced. Logic lives in systems, creating a clean separation.

### Systems
Systems operate on entities with specific component combinations. A system is a function (or closure) that iterates over all entities matching a query:
```
// Pseudocode
for entity in world.query::<(&mut Position, &Velocity)>() {
    entity.position.x += entity.velocity.vx * dt;
    entity.position.y += entity.velocity.vy * dt;
}
```

Systems are **data transformers**—they read inputs, transform them, and write outputs. They're independent and can run in parallel if their component sets don't overlap.

## Storage Strategies

The choice of storage dramatically affects query performance and cache locality.

### Archetype-Based (Dense Storage)

Entities with identical component sets are grouped into **archetypes**. Each archetype is a struct-of-arrays (SoA) layout:
```
Archetype: [Position, Velocity, Sprite]
Positions:   [p0, p1, p2, p3, ...]
Velocities:  [v0, v1, v2, v3, ...]
Sprites:     [s0, s1, s2, s3, ...]
```

**Strengths:**
- **Contiguous iteration**: All positions are adjacent in memory; CPU cache prefetcher is highly effective.
- **Branch prediction**: Tight loops over homogeneous data require minimal branching.
- **SIMD-friendly**: Data layout naturally suits vectorization.

**Weaknesses:**
- **Structural changes**: Adding a component to an entity moves it to a different archetype, requiring copy and reindexing.
- **Fragmentation**: Many small archetypes waste memory and reduce cache benefit.

**Implementations:** Bevy, Unity DOTS, Flecs.

### Sparse Sets (Ragged Arrays)

A sparse set maintains a reverse mapping: for each entity, which component does it have? Queries iterate the sparse array pseudorandomly.

```
Entity Index:  0  1  2  3  4  5
Has Position:  T  T  F  T  F  T
Latest Dense:  0  1  _  2  _  3
```

**Strengths:**
- **Cheap structural changes**: Adding/removing components doesn't move the entity; just update the sparse array.
- **Flexible queries**: Easy to add/remove components dynamically.

**Weaknesses:**
- **Cache misses**: Jumping between sparse indices causes CPU cache misses.
- **Branch misprediction**: Iteration order is irregular.

**Implementations:** Older Bevy ECS, EnTT (optionally).

### Hybrid Approaches

Modern ECS libraries use a hybrid: archetype-based for iteration (performance-critical) and sparse indexing for queries (flexibility). Bevy's recent redesign (0.14+) emphasizes archetype storage with efficient query-to-archetype mapping.

## Query Systems

A query specifies which component combinations to iterate over. Implementations vary:

### Compile-Time Queries (Bevy style)
```
// Rust
for (entity, pos, vel, sprite) in query.iter() { ... }
```

The compiler knows the exact components at build time. The ECS can precompute which archetypes match the query, making iteration a pointer walk through archetypes.

### Runtime Queries (C++ style)
```
// Pseudocode
auto view = registry.view<Position, Velocity>();
for (auto entity : view) { ... }
```

Less type safety, but more flexible for dynamically-typed engines or scripting.

### Query Filtering
Most systems support filters:
- **With**: Include entities that have component X
- **Without**: Exclude entities with component Y
- **And/Or**: Compound logic

Filters act as a second-level archetype discrimination without materializing all combinations.

## Component Lifecycle and Removal

Removing components requires careful bookkeeping:

1. **Swap-and-pop**: Archetype maintains a dense array. To remove the i-th entity's component, swap it with the last entity in the archetype, then pop.
2. **Index updates**: The swapped entity's index changes; update all references.
3. **Deferred removal**: Queue removals and process at frame boundaries to avoid iterator invalidation.

Most production ECS libraries defer structural changes (add/remove) to the end of the current frame, ensuring systems don't race.

## Event Handling in ECS

Events (collisions, input, damage) are typically handled as:

### Event Queues
Store events in a queue; systems read them:
```
// Physics system outputs CollisionEvents
// Damage system reads CollisionEvents and applies health changes
```

Events typically persist for one frame. After all systems finish, the queue clears.

### Event as Components
Some engines (Bevy) treat events as components on temporary entities:
```
// TransformChanged event is a component
// Systems subscribe to it via query
```

Temporary entities are reclaimed at frame end.

### Reactive Systems
A system can be marked to run only when certain events occur (versus always running). This optimization skips systems when irrelevant.

## OOP vs ECS: Conceptual Tradeoffs

### OOP Inheritance Model
```
GameObject
├─ Character (health, position, velocity)
├─ Item (position, owner)
└─ Particle (position, lifetime)

Character
├─ Player
├─ Enemy
└─ NPC
```

Problems emerge:
- **Diamond inheritance**: An object needing two behaviors suffers from multiple inheritance or awkward workarounds.
- **Cache thrashing**: Unrelated data packed in one object; iteration touches cold data.
- **Behavioral centralization**: Adding a new system (e.g., on-fire status) requires modifying base classes.

### ECS Model
```
Components:
- Position
- Velocity
- Health
- OnFire
- Sprite
- AI
- ...

Systems iterate component combinations:
- MovementSystem: [Position, Velocity]
- DamageSystem: [Health, OnFire]
- RenderSystem: [Position, Sprite]
```

Advantages:
- **True composition**: An entity can have any combination; no hierarchy gymnastics.
- **Cache efficiency**: Systems iterate contiguous data.
- **Decoupled iteration**: Adding a system doesn't touch existing code.

Limitations:
- **Complex queries**: Finding entities that should react to one another requires explicit queries or event handling.
- **Lost object identity**: In OOP, an Enemy has methods; in ECS, behaviors are in systems, making local reasoning harder.
- **Serialization**: Saving entity state is often complex because components are scattered across different systems' storage.

## Performance Characteristics

### Cache Locality
On modern CPUs (64-byte cache lines), a tight loop over SoA data achieves near-peak memory bandwidth (e.g., 10-25 GB/s on a CPU doing floating-point arithmetic). OOP object hierarchies often fit fewer data items per cache line.

### Parallelism
Systems operating on disjoint component sets can run in parallel without locking:
```
// Safe parallel execution
MovementSystem: [Position, Velocity] ← can run with
RenderSystem: [Position, Sprite]      ← these don't share writes

// Conflict: need synchronization
MovementSystem: [Position]  
PhysicsSystem: [Position]   ← both write Position
```

Bevy uses a dependency graph to schedule systems. Systems that don't conflict are parallelized; those that do are serialized.

### Iteration Speed
Walking a dense array is orders of magnitude faster than dereferencing pointers through a tree of objects. A system iterating 10,000 entities in a tight loop takes ~100μs (with prefetching and SIMD). OOP traversal might take 100× longer due to cache misses.

## Real-World Implementations

### Bevy ECS (Rust)
- Archetype-based storage with efficient query-to-archetype mapping
- Compile-time typed queries prevent runtime errors
- Parallelized system scheduling using a dependency graph
- Events as temporary entities or reusable event queues
- Hot-reload support during development

### Unity DOTS / Entities (C#)
- Chunk-based storage (16 KB chunks of tightly-packed components)
- Burst compiler (LLVM-based JIT) compiles systems to native code mid-game
- Hierarchical component data (data inherits from parent transforms)
- Netcode integration for multiplayer synchronization
- Still evolving; coexists with traditional MonoBehaviour approach

### Flecs (C)
- Hybrid archetype + sparse set storage
- Powerful query language (DSL-like)
- Entity relationships (e.g., "child of", "depends on")
- Systems with scoped iteration
- Language-agnostic; bindings to Rust, C++, Python, Zig, others

### EnTT (C++)
- Sparse-set by default; archetype queries available
- Header-only library; minimal overhead
- Static polymorphism (templates) for type safety
- Dense component storage on demand
- Lightweight; often used in custom engines

## Limitations and When NOT to Use ECS

- **Complex hierarchical state**: Deep inheritance with many sibling behaviors often feels more natural in OOP.
- **Turn-based systems**: ECS shines for continuous iteration; turn-based engines may benefit from event-driven patterns.
- **Cross-cutting concerns**: Transactions or global constraints (e.g., "no two entities in the same position") are awkward to express; require repeated queries.
- **Debugging**: Stepping through system execution is less intuitive than inspecting an object's methods.

## Hybrid Approaches

Many production engines use a hybrid:
- **High-performance rendering and physics**: Pure ECS
- **Scripted behaviors**: Event-driven scripting (Lua, GDScript) with ECS underneath
- **UI and tools**: Traditional OOP or immediate-mode pattern
- **Networking**: Hybrid; entity state synced via ECS, RPC calls handled separately

Hybrid designs balance simplicity (OOP for familiar parts) with performance (ECS for the hot loop).

## Related Concepts

See also: [gamedev-engine-architecture](gamedev-engine-architecture.md) (systems integration, scene graphs), [gamedev-patterns](gamedev-patterns.md) (game loops, event handling), [algorithms-hash-tables](algorithms-hash-tables.md) (entity indexing), [performance-cache-locality](performance-cache-locality.md) (cache efficiency).