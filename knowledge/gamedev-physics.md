# Game Physics — Rigid Body Simulation, Collision Detection & Constraints

Game physics engines simulate motion under gravity, friction, and collision. They must balance **accuracy**, **performance**, and **determinism** within a 2–5ms budget per frame.

## Rigid Body Simulation Loop

```
For each rigid body:
  1. Apply forces (gravity, user input, wind resistance)
  2. Integrate velocity and position (Euler, RK4, or Verlet)
  3. Solve collisions
  4. Update transform
```

### Integration Methods

| Method         | Formula                           | Stability | Accuracy | Cost                 |
| -------------- | --------------------------------- | --------- | -------- | -------------------- |
| Explicit Euler | v_new = v + a×dt                  | Poor      | O(dt²)   | Very cheap           |
| Semi-implicit  | v_new = v + a×dt; p_new = p + v  | Good      | O(dt²)   | Very cheap (standard)|
| RK4            | 4-stage weighted average of a(t) | Excellent | O(dt⁴)   | 4× evaluation cost   |
| Verlet         | p_new = 2p - p_old + a×dt²        | Good      | O(dt²)   | Implicit constraint  |

**Semi-implicit (velocity Verlet)** is industry standard: stable even at large dt, no explicit velocity storage, correct energy dissipation.

### Damping

**Friction** and **air resistance** dissipate energy:

```
v_new = v × (1 - damping × dt)  // per-frame decay
```

Linear damping ~0.01–0.05; angular damping similar. High damping = sluggish, unrealistic bounce.

## Collision Detection

Collision detection is split into **broad phase** and **narrow phase**.

### Broad Phase (Spatial Partitioning)

Quickly reject pairs that cannot collide. Approaches:

#### AABB Trees (Bounding Volume Hierarchies)
Organize objects in a tree via bounding box overlap. Query: insert object, traverse tree, collect candidates at leaf collisions.

- **Strengths**: Fast insertion; good cache locality after tree rebuild.
- **Weakness**: Static trees become unbalanced as objects move; requires periodic rebuilding.

#### Grid / Spatial Hash
Divide space into cells. Objects register in cells they occupy. Query: hash cell and neighbors.

- **Strengths**: Fast insert/remove; dynamic.
- **Weakness**: Overhead for tall/thin objects; tuning cell size is crucial.

#### Quadtree / Octree
Hierarchical spatial partitioning. Leaf nodes contain objects within volume.

- **Strengths**: Adaptive to object density.
- **Weakness**: Insertion/removal requires tree rebalancing.

### Narrow Phase (Exact Collision)

Given two candidates, determine **if** and **where** they collide.

#### GJK (Gilbert-Johnson-Keerthi)
Iteratively finds the closest point between two convex shapes using the Minkowski difference.

- **Best for**: Convex shapes (boxes, spheres, capsules, arbitrary convex polyhedra).
- **Output**: Distance; if ≤ 0, objects collide. Separation vector available.
- **Cost**: O(iterations), typically 5–20 per pair.

#### SAT (Separating Axis Theorem)
If two convex shapes don't collide, there exists an axis where their projections don't overlap.

- **Best for**: Polygons in 2D; polyhedra in 3D (but expensive). Also triangles vs. everything.
- **Cost**: O(faces + edges), typically 10–50 tests per pair.

#### Ray Casting
Test if a ray (line segment) intersects a shape. Used for sweeping (continuous collision) and queries.

### Continuous Collision Detection (CCD)

Discrete collision detection misses fast-moving objects passing through thin geometry.

```
Discrete: Object at t, then t+dt. If both positions miss, collision missed.
Continuous: Sweep object from t to t+dt; check every intermediate position.
```

**Approach**: Treat swept volume as capsule; test vs. stationary geometry. Cost: 2–3× narrow-phase time.

**When to enable**: Bullets, fast impacts. Disable for slow-moving objects (too expensive).

## Constraint Solvers

Constraints enforce relationships: joint limits, contact forces, ropes, hinges.

### Sequential Impulse (Industry Standard)

Iteratively apply impulses to satisfy constraints.

```
For iterations 1 to N:
  For each constraint:
    Compute required impulse to satisfy constraint
    Apply impulse to both bodies
```

**Convergence**: Typically 4–8 iterations for typical game scenarios. Higher = more realistic but slower.

**Determinism**: Same order of iteration always produces same result (unlike parallel solvers; beware of floating-point rounding).

### Projected Gauss-Seidel
Sequential impulse variant. More stable than naive sequential, faster convergence.

### Warm Starting
Cache and reuse last frame's impulses as initial guess. Improves convergence; critical for stability.

## Common Physics Engines

| Engine  | Integration      | Broad Phase        | Narrow Phase | Constraint | Notes                         |
| ------- | ---------------- | ------------------ | ------------ | ---------- | ----------------------------- |
| Bullet  | Open-source C++  | AABB trees         | GJK, SAT     | Sequential | Industry standard, free       |
| PhysX   | Proprietary C++  | AABB trees + grid  | GJK          | TGS            | NVIDIA; console standard      |
| Havok   | Proprietary C++  | Optimized spatial  | GJK/SAT      | Iterative  | AAA studios; expensive        |
| Rapier  | Rust             | AABB trees, grid   | GJK, SAT     | Sequential | Modern, good docs, safe       |

## Verlet Integration

**Verlet** stores position at t-1 and t; no explicit velocity:

```
p(t+dt) = 2×p(t) - p(t-dt) + a×dt²
```

**Advantages**:
- Automatically energy-preserving.
- Velocity implicit; no need to store it.
- Soft body / cloth is trivial (constraints pull particles together).

**Disadvantages**:
- Damping requires position adjustment (not intuitive).
- Friction is harder to implement.
- Less control over velocity directly.

Used in **soft body** and **cloth** simulation.

## Soft Body Physics

Cloth, jello, deformable meshes. Represented as particle network:

- Particles: position, velocity, mass.
- Constraints: distance constraints keep particles at fixed distance (cloth seams).
- Collision: particles collide with rigid bodies; drag simulation handles wind.

**Verlet** is natural for soft body (above). Collision: check each particle against environment.

## Fluid Simulation

Real-time fluid (water, smoke, lava) is GPU-intensive. Approaches:

### Shallow Water Equation (Heightfield)
Represents water as 2D grid of heights. Good for oceans, rivers.

### SPH (Smoothed Particle Hydrodynamics)
Particles represent fluid. Per-particle density and pressure computed from neighbors. Forces: viscosity, surface tension, gravity.

- **Cost**: O(particles × neighbors), typically 30–50 neighbors per particle.
- **GPU**: Compute shaders with spatial hashing for fast neighbor lookup.

### Voxel-Based (MAC Grid)
3D Eulerian grid stores velocity. Incompressibility enforced via Poisson solver (iterative).

## Determinism and Networking

Physics must be **deterministic** for replays and multiplayer synchronization.

**Challenges**:
- Floating-point rounding is platform-dependent.
- Parallel solvers are non-deterministic (order-dependent).
- Time step inconsistency causes drift.

**Solutions**:
- Fixed timestep only (never variable dt).
- Single-threaded solver or careful ordering.
- Quantize positions/velocities on sync boundaries (server reconciliation).

## Performance Hot Spots

- **Broad phase**: Too many candidates = slow narrow phase; too sparse = miss collisions.
- **Narrow phase**: GJK iterations; SAT face/edge enumeration.
- **Constraint solver**: O(constraints × iterations); warm starting critical.
- **Continuous collision**: Expensive; enable selectively.

## See Also

- gamedev-patterns.md (game loop integration)
- gamedev-engine-architecture.md (simulation module architecture)
- gamedev-networking.md (determinism in multiplayer)