# Game Engine Architecture — Design Patterns, Systems Integration & Evolution

Game engine architecture determines what problems a developer can solve efficiently. Engines must balance **performance**, **expressiveness**, **maintainability**, and **iteration speed**. The architectural choices made early cascade through the entire engine lifetime.

## Entity-Component-System (ECS)

The **Entity-Component-System** pattern decouples data layout from behavior, enabling high-performance iteration, data-driven design, and dynamic composition without inheritance hierarchies.

### Core Model

- **Entity**: An ID, nothing more. Purely a handle.
- **Component**: Pure data (position, velocity, health, rotation). No methods. Densely packed in memory.
- **System**: Logic that reads/writes components of a specific type. Operates on contiguous arrays.

### Strengths

- **Cache locality**: All components of one type live adjacent in memory. The CPU cache becomes vastly more effective than with object-oriented inheritance.
- **Composition over inheritance**: No deep class hierarchies. Mix components freely at runtime.
- **Parallelism**: Systems operate independently. Multiple threads can safely iterate different component types simultaneously.
- **Iteration speed**: New behaviors are new systems; no recompilation of unrelated classes.

### Weaknesses and Mitigations

- **Query complexity**: Finding entities with specific component combinations requires indexing. Solutions: archetype-based lookup (group entities by component set), sparse sets, or hybrid schemes.
- **Debugging obscurity**: Logic is dispersed across many small systems. Requires robust logging and time-stepping tools.
- **Cascading effects**: One system's output feeds into another's input, creating temporal dependencies. Order matters; non-determinism is a trap.

### Variants

- **Monolithic ECS** (Unity, Unreal): Engines provide a built-in ECS with predefined component types.
- **Plugin ECS** (standalone: EnTT, Bevy): Developers assemble an engine from ECS + additional systems.
- **Hybrid**: Traditional OOP objects with ECS components for performance-critical subsystems.

## Scene Graph

The **scene graph** organizes spatial relationships hierarchically. A node's transformation is relative to its parent; global position is computed by walking up the tree.

```
Root (identity)
├─ Player (pos=5,0)
│  └─ LeftHand (local_pos=1,0)     [global ≈ 6,0]
├─ Environment (pos=0,0)
│  ├─ Building (pos=10,0)
│  └─ Light (pos=20,5)
```

### Observations

- **Culling**: Frustum-culling traversals skip entire subtrees if the parent is outside the view.
- **Batching**: Siblings often share materials or can be batched together, reducing draw calls.
- **Manipulation**: Moving the parent automatically repositions all children; convenient but can hide inefficiencies if applied carelessly.

### Pitfalls

- **Overly deep hierarchies**: Each frame, traversing to compute world positions becomes costly.
- **Dynamic reparenting**: Expensive during frame; batted against stability.
- **Implicit ordering**: Depth sorting for transparency must account for hierarchy, not just position.

## Asset Pipeline

The **asset pipeline** converts raw source files (3D models, textures, audio, configs) into engine-native formats for efficient loading and streaming.

### Stages

1. **Import**: Parse source format (FBX, PNG, WAV, JSON).
2. **Processing**: Compress, optimize (decimation, LOD generation, baking).
3. **Packaging**: Bundle into engine formats (.asset, .bundle, or database blobs).
4. **Streaming**: Load incrementally; unload when out of view.

### Considerations

- **Compression**: XCF→PNG trade-off (disk space vs. load time vs. quality).
- **LOD cascades**: Generate multiple detail levels offline; runtime selects by distance.
- **Cook-on-demand**: Regenerate assets when source changes; cache results.
- **Streaming**: Prioritize visible/near-camera content; stream high-detail versions in background.

## Rendering Pipeline Integration

The rendering pipeline sits at the intersection of game state and graphics API (OpenGL, Vulkan, DirectX 12).

### Flow

```
Game State → Culling → Sorting → Material Batching → GPU Submission → Presentation
```

- **Culling**: Remove entities outside camera frustum, behind occluders.
- **Sorting**: By material to minimize state changes; by depth for transparency.
- **Batching**: Group draws with identical materials into single GPU command.
- **Submission**: Build command buffers; dispatch to GPU.

### Overhead Points

- **Draw call submission**: CPU→GPU synchronization is expensive; batching amortizes cost.
- **Shader switching**: Each unique shader incurs state changes.
- **Texture binding**: Atlasing nearby textures reduces binding overhead.

## Scripting Systems

**Scripting** enables designers and tools to drive behavior without recompilation.

### Language Choice

| Language   | Integration     | Performance | Iteration | Common Use                |
| ---------- | --------------- | ----------- | --------- | ------------------------- |
| Lua        | Lightweight VM  | Moderate    | Fast      | Unity, Roblox, embedded   |
| C#         | JIT compiled    | Good        | Fast      | Unity standard            |
| GDScript   | Godot native    | Good        | Very fast | Godot (in-engine tool)    |
| C++        | Native          | Excellent   | Slow      | Performance-critical core |
| ECS-based  | Data definition | Excellent   | Very fast | Bevy, Flecs (entity data) |

### Hot Reloading

Recompiling scripts without restarting the engine saves iteration time. Implementations:

- **VM reload**: Unload old bytecode; load new; migrate live state (tricky but possible).
- **Restart simulation**: Clear all instances; reload script; recreate from serialized state.
- **Editor mode only**: Hot reload in the editor; not during shipping builds.

## Module Dependency Patterns

Healthy engines decouple major systems:

- **Rendering module**: Independent of physics or scripting.
- **Physics module**: Can be swapped (PhysX, Bullet, Havok).
- **Audio module**: Pluggable; many games disable it in dedicated server builds.
- **Scripting module**: User-facing; game code doesn't depend on engine internals.

Well-defined interfaces between modules grant flexibility.

## Profiling and Hot Spots

Real-time budgets (16ms @ 60fps, 33ms @ 30fps) enforce discipline. Common bottlenecks:

- **Vertex transform**: Transform hierarchies, skinning.
- **Culling**: Especially on CPU if not parallelized.
- **Material setup**: Shader compilation at runtime (vs. precompiled).
- **Memory allocation**: Frame-time spikes; use object pools or frame arenas.

## Engine Comparison: Unreal, Unity, Godot

| Dimension             | Unreal 5          | Unity 2023+       | Godot 4           |
| --------------------- | ----------------- | ----------------- | ----------------- |
| Architecture          | Pure C++          | C#/C++ hybrid     | GDScript, C#, C++ |
| Rendering             | Nanite + Lumen   | Built-in, custom  | Vulkan/OpenGL     |
| ECS adoption          | Emerging (MASS)   | Entities (opt-in) | Native            |
| Scripting iteration   | Slow (C++)        | Fast (C#)         | Very fast (reload)|
| Mobile performance    | Good              | Good              | Good              |
| Learning curve        | Steep             | Moderate          | Gentle            |
| Cost                  | Free (royalties)  | Freemium          | Free (MIT)        |

## See Also

- gamedev-patterns.md (game loop, frame timing)
- gamedev-rendering.md (pipeline integration details)
- gamedev-physics.md (simulation integration)
- architecture-patterns.md