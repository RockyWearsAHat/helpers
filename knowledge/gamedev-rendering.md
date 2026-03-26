# Real-Time Rendering — Rasterization, PBR, Ray Tracing & Global Illumination

Real-time rendering converts game state into pixels at 30–120 fps. Modern rendering balances **visual fidelity**, **performance**, and **predictability**. The pipeline has evolved from fixed-function hardware to programmable shaders to hybrid approaches mixing rasterization and ray tracing.

## Rasterization Pipeline

The **rasterization pipeline** is the foundational real-time rendering approach: geometry → screen-space raster → final pixels.

### Stages

```
Vertex Shader → Rasterizer → Fragment Shader → Blend/Output
```

1. **Vertex Shader**: Per-vertex transformation. Inputs: vertex position, normal, UV, bone weights. Outputs: screen position, interpolated attributes (normal, UV, tangent).
2. **Rasterizer (Fixed Hardware)**: Determines which pixels each triangle covers; generates fragments (potential pixels) with interpolated attributes.
3. **Fragment Shader (Pixel Shader)**: Per-fragment computation. Evaluates material properties, lighting, textures. Outputs: color, depth (optional).
4. **Blend/Output**: Combines fragment color with framebuffer using blend modes.

### Limitations

- **Single pass**: Lighting information must be available during fragment evaluation.
- **Overdraw**: Transparent objects require careful sorting; overlapping geometry on screen rasterizes all fragments (wasteful).
- **Shadow quality**: Real-time shadows require pre-computed shadow maps or expensive per-light evaluation.

## Shader Types

### Vertex Shader
Transforms per-vertex data (position, normal, UV) to screen or world space. Skeletal animation (bone deformation) happens here; high vertex count penalties ripple through the pipeline.

### Fragment Shader (Pixel Shader)
Evaluates per-pixel/fragment color. Most visual quality lives here: normal mapping, parallax mapping, material properties, lighting calculations.

### Compute Shader
General-purpose GPU computation. Used for particle updates, cloth simulation, image processing, deferred rendering light culling. Not part of the classic graphics pipeline; executed independently.

### Geometry Shader
Optional stage *between* vertex and rasterizer. Generates, modifies, or culls primitives. Examples: billboarding (camera-facing quads), silhouette extrusion for shadow volumes. High latency; rarely used in modern performance-conscious engines.

### Tessellation (Displacement & Control Shaders)
Subdivides patches (not triangles) at runtime, enabling smooth silhouettes without pre-computed high-poly meshes. Excellent for terrain; must be calibrated carefully (displacement can hide triangles or cause T-junctions).

## Forward vs Deferred Rendering

### Forward Rendering
Each object accumulated with contributions from all lights.

```
For each object:
  For each light:
    Accumulate light contribution
  Render object
```

**Strengths**: Simple, handles transparency naturally, AA is straightforward.  
**Weakness**: O(lights × objects); becomes expensive at 10+ dynamic lights.

### Deferred Rendering
Geometry pass writes materials (position, normal, albedo, roughness); light pass shades all pixels with all lights.

```
Geometry Pass: For each object → write G-buffer (position, normal, albedo, etc.)
Light Pass: For each light → shade all pixels using G-buffer
```

**Strengths**: O(lights) is decoupled from geometry complexity; scales to many lights.  
**Weaknesses**: MSAA is non-trivial (requires per-sample data or resolve complexity); transparency requires fallback to forward; bandwidth to G-buffer can stall memory.

### Hybrid Approach
Forward for opaque, deferred for lights, special handling for transparency and reflections. Most modern AAA engines use this.

## Physically Based Rendering (PBR)

**PBR** is a framework for material representation that produces consistent, predictable results across lighting conditions. The metallic workflow is standard industry practice.

### Core Parameters (Metallic Workflow)

| Parameter | Range | Meaning                                    |
| --------- | ----- | ------------------------------------------ |
| Albedo    | 0–1   | Base color (diffuse for non-metals)        |
| Normal    | 0–1   | Per-pixel surface normal (encoded as RGB)  |
| Metallic  | 0–1   | 0 = dielectric, 1 = conductor              |
| Roughness | 0–1   | 0 = mirror, 1 = diffuse                    |
| AO        | 0–1   | Ambient occlusion (baked/real-time)        |

### BRDF (Bidirectional Reflectance Distribution Function)

A PBR shader evaluates the BRDF given view direction, light direction, and surface properties. The split-sum approximation precomputes two lookup tables:

1. **Irradiance map**: Indirect diffuse.
2. **Specular map + Fresnel-Schlick LUT**: Indirect specular by roughness.

This makes real-time image-based lighting (IBL) feasible.

### Fresnel Effect

Surfaces are more reflective at grazing angles. The **Fresnel-Schlick** approximation:

```
F = F₀ + (1 - F₀) × (1 - cos θ)^5
```

where F₀ is the surface's reflectivity at normal incidence (typically 0.04 for dielectrics, varies for metals).

## Real-Time Ray Tracing (RTX, DXR)

**Ray tracing** follows light paths through the scene, enabling physically accurate reflections, refractions, and shadows at high cost.

### Hardware (NVIDIA RTX, AMD RDNA, Intel Xe)

Dedicated RT cores accelerate BVH (Bounding Volume Hierarchy) traversal and ray–triangle intersection. RTX makes 1–4 rays/pixel feasible in real time.

### Hybrid Approach (Practical)

```
Rasterization Pass: G-buffer + depth
Ray Tracing Pass: 1–2 rays/pixel for reflections + shadows (quarter or half resolution)
Temporal Upsampling: Reprojection + AI denoise (DLSS, FSR, XeSS)
```

**Denoisers**: Temporal and spatial filtering reduce ray noise. NVIDIA OptiX, Intel Open Image Denoise, shaders-based bilateral filtering.

### Use Cases

- **Reflections**: Mirror-like surfaces with correct distortion, parallax.
- **Shadows**: Soft shadows from area lights; replaces shadow maps.
- **Global illumination**: Indirect lighting bounces; replaces lightmaps and light probes.
- **Refractions**: Glass, water with correct volume interactions.

### Costs

- **Latency**: BVH traversal is non-uniform; some rays hit complex geometry, others empty space.
- **Bandwidth**: Texture lookups for hit positions; cache misses ripple.
- **Power**: High-end GPU utilization; mobile feasibility is limited.

## Global Illumination

Indirect light (non-direct from sun/lamps) defines scene ambience and realism. Approaches:

### Lightmaps (Offline)
Bake indirect light into texture atlases. Fast at runtime, static. Standard for mobile and PS4-era console.

### Light Probes (Hybrid)
Sample indirect light at key positions. Interpolate at runtime. Cheap, but grid-dependent; visible seams if density is low.

### Voxel Cone Tracing
Voxelize the scene into a sparse octree; trace cones to approximate diffuse GI. Good for dynamic scenes; moderate cost. Older technique (Cryengine 3).

### Screen-Space Global Illumination (SSGI)
Reproject previous frames to infer indirect light. Works only for on-screen geometry; off-screen light leaks.

### Real-Time Ray Tracing GI
Fire rays from striking points; trace to find emitters. Most physically correct; highest cost. Modern approach with denoisers.

## LOD and Culling

### Level of Detail (LOD)
Distant objects use lower-poly meshes; nearby use high-poly. Smooth transitions reduce popping.

```
Distance < 10m → 50k tris
10m < Distance < 50m → 10k tris
50m < Distance → 1k tris
```

### Frustum Culling
Exclude objects outside camera view.

### Occlusion Culling
Exclude objects hidden behind other geometry. Requires pre-computed occlusion meshes or conservative rasterization tricks.

### Dynamic Resolution
Render at lower resolution, upsample. Adaptive based on GPU time budget.

## Performance Hot Spots

- **Draw call submission**: CPU upload to GPU; batching amortizes.
- **Memory bandwidth**: Vertex/index/texture fetches; cache coherence matters. L1 is ~64 bytes/cycle; miss penalties are high.
- **Overdraw**: Transparent blending; forward rendering with many lights.
- **Shader complexity**: Complex fragment shaders on high-poly scenes.

## See Also

- gamedev-engine-architecture.md (rendering pipeline integration)
- gamedev-physics.md (integration with simulation)
- hardware-gpu-computing.md (GPU architecture)