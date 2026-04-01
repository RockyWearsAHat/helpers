# WebGL/WebGPU: GPU Rendering, Shaders & Compute

## Overview

WebGL and WebGPU are two GPU-accelerated graphics APIs for the web. **WebGL** (built on OpenGL ES) is mature, widely supported, and handles 3D rendering via rasterization. **WebGPU** is the newer standard, offering lower-level GPU access, compute shaders, and better performance across platforms.

The key mental model: WebGL is a **fixed rendering pipeline** (vertex → fragment stages) suitable for 3D scenes. WebGPU is a **flexible GPU abstraction** supporting both graphics and general-purpose compute, with more explicit control over memory and synchronization.

---

## WebGL: Mature 3D Graphics API

### Architecture

WebGL renders 3D geometry through a fixed pipeline:

1. **Vertex Shader** — Transforms vertices (positions, normals, UVs); runs per-vertex
2. **Rasterization** — Hardware-accelerated triangle scan-conversion
3. **Fragment Shader** — Computes per-pixel color; runs per-fragment (after depth/stencil tests)
4. **Framebuffer** — Render target (screen or texture)

```glsl
// Vertex Shader (GLSL)
#version 300 es
in vec3 position;
in vec3 normal;
uniform mat4 uModel, uView, uProj;

out vec3 vNormal;

void main() {
  gl_Position = uProj * uView * uModel * vec4(position, 1.0);
  vNormal = normalize((uModel * vec4(normal, 0.0)).xyz);
}

// Fragment Shader (GLSL)
#version 300 es
in vec3 vNormal;
uniform vec3 uLightDir;

out vec4 outColor;

void main() {
  float diffuse = max(dot(vNormal, uLightDir), 0.0);
  outColor = vec4(vec3(diffuse), 1.0);
}
```

### WebGL Workflow

1. Create program from vertex + fragment shaders
2. Bind vertex buffers (positions, normals, UVs)
3. Set uniforms (matrices, lighting)
4. Call `drawArrays()` or `drawElements()`
5. GPU renders to framebuffer

**Strengths:**
- Universally supported (WebGL 1.0 → 2.0 across all browsers, older devices)
- Well-understood pipeline; rich ecosystem (three.js, babylon.js, Cesium)
- Mature tooling; shader compilation cached by browser
- Good for 3D scenes, games, visualizations

**Limitations:**
- **Explicit state machine.** Verbose setup (bind textures, framebuffers, programs)
- **Synchronous CPU-GPU communication.** Reading pixels blocks; GPU stalls common
- **Limited compute.** No compute shaders; must hack with render targets
- **Texture compressions platform-dependent.** ASTC, S3TC support varies
- **Extension brittleness.** Features behind extensions; inconsistent across devices

### Shaders & GLSL ES

WebGL uses GLSL ES (OpenGL Shading Language, embedded systems version):

- **Precision qualifiers:** `lowp`, `mediump`, `highp` (mobile GPU optimization)
- **Built-in varyings:** `gl_Position`, `gl_FragCoord`, `gl_FrontFacing`
- **No dynamic branching.** Control flow costs on mobile
- **Texture lookup:** `texture2D()`, `textureCube()`

Common optimization patterns:
- Move expensive math to vertex shader
- Bake lighting into vertex colors or textures
- Use texture atlasing to batch draw calls
- Instancing for repeated geometry

---

## WebGPU: Next-Generation GPU API

### Design Philosophy

WebGPU is a modern GPU abstraction targeting modern graphics architectures (Metal, Vulkan, DX12). Design goals:

1. **Lower-level GPU control** — Explicit synchronization, resource binding, memory management
2. **Cross-platform consistency** — Same code across macOS (Metal), Windows (DX12), Linux (Vulkan)
3. **Compute-first** — Compute shaders are first-class; not bolted on
4. **Performance** — Less driver overhead; optimal resource usage per-platform

### Architecture

WebGPU models the GPU as a **command queue** + **resources** (buffers, textures, bind groups). Code describes a computation, then submits it to the GPU:

```javascript
// Create render pipeline
const pipeline = device.createRenderPipeline({
  layout: 'auto',
  vertex: {
    module: device.createShaderModule({ code: vertexShaderCode }),
    entryPoint: 'main',
    buffers: [{ arrayStride: 12, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x3' }] }]
  },
  fragment: {
    module: device.createShaderModule({ code: fragmentShaderCode }),
    entryPoint: 'main',
    targets: [{ format: navigator.gpu.getPreferredCanvasFormat() }]
  },
  primitive: { topology: 'triangle-list' }
});

// Create bind group (uniforms, textures, samplers)
const bindGroup = device.createBindGroup({
  layout: pipeline.getBindGroupLayout(0),
  entries: [
    { binding: 0, resource: { buffer: uniformBuffer } },
    { binding: 1, resource: sampler },
    { binding: 2, resource: textureView }
  ]
});

// Record draw commands
const encoder = device.createCommandEncoder();
const pass = encoder.beginRenderPass({
  colorAttachments: [{ view: canvasContext.getCurrentTexture().createView(), loadOp: 'clear', clearValue: [0, 0, 0, 1], storeOp: 'store' }]
});
pass.setPipeline(pipeline);
pass.setBindGroup(0, bindGroup);
pass.setVertexBuffer(0, vertexBuffer);
pass.draw(vertexCount, 1, 0, 0);
pass.end();

device.queue.submit([encoder.finish()]);
```

### WGSL: WebGPU Shading Language

WGSL (WebGPU Shading Language) replaces GLSL. Design improvements:

- **Type safety.** Strong typing; no type coercion bugs
- **Explicit memory layout.** `@align(16)`, `@size(4)` attributes control padding
- **Compute shaders.** `@compute @workgroup_size(8, 8, 1)` blocks parallel work
- **Early validation.** Shader errors caught at creation time, not draw time

```wgsl
// Uniform block
struct Camera {
  view: mat4x4<f32>,
  proj: mat4x4<f32>
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var textureSampler: sampler;
@group(0) @binding(2) var textureData: texture_2d<f32>;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) normal: vec3<f32>
}

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) normal: vec3<f32>
}

@vertex
fn vertexMain(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  output.position = camera.proj * camera.view * vec4(input.position, 1.0);
  output.normal = input.normal;
  return output;
}

@fragment
fn fragmentMain(input: VertexOutput) -> @location(0) vec4<f32> {
  let color = textureSample(textureData, textureSampler, vec2(0.5, 0.5));
  return color;
}
```

### Compute Shaders

WebGPU's killer feature: **compute shaders** are first-class. Run arbitrary parallel work on the GPU:

```wgsl
@group(0) @binding(0) var<storage, read_write> data: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn computeMain(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let idx = global_id.x + global_id.y * 8u;
  data[idx] = data[idx] * 2.0; // Double all values in parallel
}
```

Use cases: physics simulation, image processing, machine learning inference, particle systems, pathfinding.

### Advantages

- **Explicit synchronization.** Know when GPU/CPU coordinate
- **Compute parity.** General-purpose GPU programming, not just graphics
- **Lower overhead.** Less GPU state tracking; optimal per-platform translation
- **Texture compression transparent.** Browser handles ASTC, BC, ETC2
- **Buffer mapping.** Efficient async read-back without stalls

### Limitations (Current)

- **Emerging standard.** Chrome/Edge stable; Firefox/Safari partial support
- **API evolution.** Features still being standardized (e.g., indirect dispatch, sparse textures)
- **Smaller ecosystem.** Fewer libraries; three.js + babylon.js adding WebGPU backends now

---

## Comparative Analysis

| Aspect | WebGL | WebGPU |
|--------|-------|--------|
| **Maturity** | Stable, 12+ years | Emerging (W3C spec, Chrome stable Q1 2024) |
| **Browser Support** | ~98% | ~70% (Chrome, Edge stable; Safari/Firefox partial) |
| **Abstraction** | Fixed graphics pipeline | Flexible GPU commands |
| **Compute** | Via render-to-texture hacks | Native compute shaders |
| **Synchronization** | Implicit GPU submissions | Explicit command buffers |
| **State Management** | Global mutable state | Explicit binding groups |
| **Performance** | Mature optimizations | Potentially better (less driver overhead) |
| **Learning Curve** | Moderate (lots of tutorials) | Steep (new concepts, explicit memory) |

---

## Frameworks & Libraries

### WebGL Frameworks

- **three.js** — High-level 3D scenes, utilities, materials. Abstracts WebGL complexity. Renderer supports WebGPU backend (experimental).
- **babylon.js** — Full-featured engine, rich editor, WebGPU backend in development
- **Cesium.js** — Geospatial 3D mapping
- **PixiJS** — 2D WebGL renderer (games, interactive graphics)
- **PlayCanvas** — Cloud-based editor + runtime

### WebGPU Libraries

- **three.js WebGPURenderer** — WebGPU backend for three.js (experimental)
- **babylon.js WebGPU engine** — Native WebGPU engine
- **Wgpu-rs** (Rust) — Compiles to WASM, WebGPU target (not JS, but Rust → WebGPU pipeline)
- **Native WebGPU** — Direct API usage for low-level control

---

## Canvas Rendering & OffscreenCanvas

### Canvas Context Types

```javascript
// 2D rendering
const ctx = canvas.getContext('2d');
ctx.fillStyle = 'blue';
ctx.fillRect(10, 10, 100, 100);

// WebGL
const gl = canvas.getContext('webgl2');
const program = gl.createProgram();
// ... shader, rendering pipeline

// WebGPU
const gpu = navigator.gpu;
const adapter = await gpu.requestAdapter();
const device = await adapter.requestDevice();
const context = canvas.getContext('webgpu');
context.configure({ device, format: gpu.getPreferredCanvasFormat() });
```

### OffscreenCanvas

Render to a canvas in a worker thread (no DOM access):

```javascript
// Main thread
const offscreen = canvas.transferControlToOffscreen();
worker.postMessage({ canvas: offscreen }, [offscreen]);

// Worker thread
self.onmessage = (e) => {
  const canvas = e.data.canvas;
  const gl = canvas.getContext('webgl2');
  // Render in worker, no main-thread blocking
};
```

Enables **frame rate decoupling** — rendering runs independently from DOM updates.

---

## GPU-Accelerated Computation

### Beyond Graphics: ML & Physics

WebGPU compute shaders enable:

- **Tensor operations** — Matrix multiplication, convolution for ML inference
- **Physics simulation** — N-body, cloth, fluid on GPU
- **Image processing** — Filters, transforms in parallel
- **Pathfinding** — Compute flow fields, navigation meshes
- **Data processing** — Sort, reduce, scan operations

Example: Parallel reduction (sum all values)

```wgsl
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(256, 1, 1)
fn reduce(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let gid = global_id.x;
  var sum = data[gid];
  
  // Synchronize within workgroup
  for (var stride = 128u; stride > 0u; stride = stride / 2u) {
    workgroupBarrier();
    if (gid % (stride * 2u) < stride) {
      sum += data[gid + stride];
    }
  }
  
  data[gid] = sum;
}
```

---

## Design Decisions: WebGL vs WebGPU

**Choose WebGL when:**
- Target legacy/lower-end devices (~2015+)
- 3D graphics pipeline sufficient (no compute needs)
- Quick iteration (mature ecosystem, many tutorials)
- iOS/Safari required (WebGPU support emerging)

**Choose WebGPU when:**
- Compute shaders needed (physics, ML, image processing)
- Performance critical (lower GPU overhead)
- Modern platforms targeted (desktop, recent mobile)
- Fine-grained GPU control required
- Building frameworks or high-performance systems

**Hybrid approach:** Use WebGPU where supported, fallback to WebGL on unsupported browsers. Feature-detect `navigator.gpu`.

---

## Common Gotchas

1. **GPU memory management.** WebGPU buffers must be destroyed; leaks freeze over time
2. **Workgroup size limits.** Different on mobile vs desktop; benchmark `maxComputeWorkgroupInvocations`
3. **Precision differences.** Texture samplers, rounding vary between Metal/Vulkan/DX12
4. **Shader compilation costs.** Avoid creating pipelines per-frame
5. **Read-back stalls.** GPU→CPU data transfer blocks; batch reads or compute asynchronously
6. **MSAA complexity.** Anti-aliasing multisample targets add complexity; consider post-process alternatives

See also: [gamedev-rendering.md](gamedev-rendering.md), [web-svg-canvas.md](web-svg-canvas.md), [hardware-gpu-computing.md](hardware-gpu-computing.md)