# Web Graphics: SVG, Canvas 2D, WebGL & Choosing Between Them

## Overview

The web platform provides three distinct graphics rendering APIs:

1. **SVG (Scalable Vector Graphics)**: DOM-based, vector graphics, CSS-styleable, accessible
2. **Canvas 2D**: Imperative pixel-based API, immediate-mode rendering, high performance
3. **WebGL/WebGPU**: GPU-accelerated 3D graphics, shaders, compute context

Each occupies a different performance/expressiveness trade-off. Choosing among them requires understanding their fundamental architectures and use cases.

## SVG: DOM-Based Vector Graphics

### Core Model

SVG is an XML-based vector graphics format. In HTML, SVG elements are DOM nodes:

```html
<svg width="200" height="200">
  <circle cx="100" cy="100" r="50" fill="blue" />
  <path d="M 10 10 L 100 100" stroke="red" stroke-width="2" />
</svg>
```

SVG shapes (`<circle>`, `<rect>`, `<path>`, `<g>`, `<text>`) are DOM elements. They participate in:
- CSS styling (no shadows or filters in SVG elements)
- JavaScript event listeners (`click`, `mouseover`)
- ARIA attributes (accessible by default)
- Querying via `querySelector`

### Rendering and Performance

SVG rendering goes through the layout pipeline:
1. SVG DOM parsed
2. Style computed
3. Bounding boxes calculated
4. Rasterization to screen pixels

Advantages:
- Infinite zoom without pixelation (vectors scale arbitrarily)
- Accessibility: text is real text, selectable and screen-reader accessible
- Interactivity natural: each element responds to events
- CSS integration: colors, sizes, transforms use standard CSS

Disadvantages:
- Reflow/repaint overhead: changing any element triggers recalculation
- Large numbers of elements (10,000+) become expensive
- Complex filters or transforms slow; GPU not typically used
- Not ideal for pixel-specific manipulation

### Common Use Cases

- **Icons and logos**: Crisp at any size
- **Diagrams and graphs**: Interactive, zooming
- **Complex illustrations**: Blend of interactivity and fidelity
- **Infographics**: Animated data visualization

### Animation Techniques

CSS animations work natively:
```css
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

circle { animation: spin 2s infinite; }
```

JavaScript-driven animation via `requestAnimationFrame` and attribute updates:
```javascript
function animate() {
  const angle = (Date.now() / 1000) * 360;
  circle.setAttribute('transform', `rotate(${angle})`);
  requestAnimationFrame(animate);
}
```

Performance: CSS animations are GPU-accelerated. JavaScript attribute updates are not; each triggers reflow.

## Canvas 2D: Imperative Immediate-Mode Rendering

### Core Model

Canvas is a pixel canvas element. You draw to it imperatively using a 2D context:

```html
<canvas id="myCanvas" width="200" height="200"></canvas>

<script>
  const canvas = document.getElementById('myCanvas');
  const ctx = canvas.getContext('2d');
  ctx.fillStyle = 'blue';
  ctx.fillRect(10, 10, 50, 50);
  ctx.stroke();
</script>
```

Drawing is **immediate-mode**: you issue draw commands in sequence. Canvas doesn't retain a scene graph—it rasterizes to a bitmap. Once drawn, you can't select or inspect individual shapes.

### Alpha Blending and Compositing

Canvas supports advanced compositing operations:

```javascript
ctx.globalAlpha = 0.5;  // 50% transparent
ctx.globalCompositeOperation = 'multiply';  // Blend mode
ctx.fillRect(0, 0, 100, 100);
```

Composite operation modes (source-over, destination-in, lighten, screen, multiply, overlay, etc.) enable sophisticated visual effects.

### Pixel Access

Canvas allows direct pixel manipulation via `getImageData`:

```javascript
const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
const pixels = imageData.data;  // Uint8ClampedArray [R, G, B, A, R, G, B, A, ...]

// Invert colors
for (let i = 0; i < pixels.length; i += 4) {
  pixels[i] = 255 - pixels[i];      // R
  pixels[i + 1] = 255 - pixels[i + 1];  // G
  pixels[i + 2] = 255 - pixels[i + 2];  // B
}

ctx.putImageData(imageData, 0, 0);
```

Use case: real-time image processing, video frame manipulation.

### Performance Characteristics

Canvas is fast for moderate amounts of dynamic drawing:
- No layout overhead
- Bitmap targets GPU texture; drawing is GPU-accelerated
- Per-pixel manipulation is CPU-bound (slow on large images)
- Redraws entire frame each cycle (no dirty region tracking)

Trade-off: performance for loss of scene model. You must manually manage what to redraw each frame.

### Common Use Cases

- **Games**: Background, sprites, particle effects
- **Data visualization**: Line charts, scatter plots (faster than SVG for thousands of points)
- **Real-time effects**: Video filters, live drawing
- **Procedural graphics**: Fractals, noise-based imagery

## WebGL and WebGPU: GPU 3D Graphics

### WebGL (OpenGL on the Web)

WebGL gives direct GPU access via GLSL shaders:

```javascript
const canvas = document.getElementById('canvas');
const gl = canvas.getContext('webgl2');

// Vertex shader (runs per vertex)
const vertexShader = `
  attribute vec2 position;
  void main() {
    gl_Position = vec4(position, 0.0, 1.0);
  }
`;

// Fragment shader (runs per pixel)
const fragmentShader = `
  precision mediump float;
  void main() {
    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);  // red
  }
`;
```

Workflow:
1. Create vertex and fragment shaders
2. Compile and link program
3. Upload data to GPU buffers
4. Set uniforms and attributes
5. Draw call

Advantages:
- True 3D: perspective transforms, depth testing, lighting
- Massive parallelism: GPU processes millions of pixels per frame
- Low-level control: blend modes, stencil operations, multiple render targets

Disadvantages:
- Steep learning curve: graphics pipeline complexity
- Error messages opaque: shader compilation failures hard to debug
- Browser compatibility: older devices may not support WebGL 2

### WebGPU (Next Generation)

WebGPU is the modern successor, designed from scratch for the web:

```javascript
const canvas = document.getElementById('canvas');
const context = canvas.getContext('webgpu');

const device = await adapter.requestDevice();

const pipeline = await device.createRenderPipeline({
  layout: device.createPipelineLayout({ bindGroupLayouts: [] }),
  vertex: { module: device.createShaderModule({ code: wgsl }), entryPoint: 'main' },
  fragment: { module: device.createShaderModule({ code: wgsl }), entryPoint: 'main' },
  primitive: { topology: 'triangle-list' },
});

// Render pass
const commandEncoder = device.createCommandEncoder();
const passEncoder = commandEncoder.beginRenderPass({ /* ... */ });
passEncoder.setPipeline(pipeline);
passEncoder.draw(3, 1, 0, 0);
passEncoder.end();
device.queue.submit([commandEncoder.finish()]);
```

Advantages over WebGL:
- Lower CPU overhead (explicit GPU commands)
- Compute shaders (general-purpose GPU computing)
- Better error messages and validation
- Modern language (WGSL or SPIR-V)

Status: Still specification-in-progress; shipping in Chrome, Safari; Firefox support in progress.

### Use Cases

- **3D visualization**: CAD models, scientific data, architectural walkthroughs
- **Game engines**: Babylon.js, Cesium, Three.js, Oimo.js use WebGL
- **Data processing**: GPGPU compute for ML, simulations
- **Video effects**: Real-time neural style transfer, object detection

## Choosing Between Them

| Axis | SVG | Canvas 2D | WebGL/WebGPU |
|------|-----|-----------|--------------|
| **Complexity** | Simple markup | Scripted, imperative | Very complex |
| **Scale** | 100s-1000s of shapes | Thousands of pixels | Millions of pixels |
| **Interactivity** | Native event handling | Custom hit testing | Custom (complex) |
| **Accessibility** | Native text, ARIA | Not accessible (opaque bitmap) | Not accessible |
| **Performance** | Good for static, poor for frequent updates | Good for dynamic 2D | Excellent for 3D and compute |
| **Zoom/scaling** | Vector quality | Pixelated on zoom | Depends on content |
| **Learning curve** | Shallow | Moderate | Steep |
| **Data access** | Full DOM | Pixel-level only | None (GPU-side) |

**Decision tree:**
1. Is it interactive, zooming, text-heavy, accessible? **→ SVG**
2. Is it 2D, dynamic, involving hundreds of shapes? **→ Canvas 2D**
3. Is it 3D or compute-heavy? **→ WebGL/WebGPU**

## Animation Techniques Across All Three

### SVG Animation
- CSS keyframes (GPU-accelerated if animating transforms)
- SVG `<animate>` element (declarative, limited)
- requestAnimationFrame + attribute updates (CPU-bound)

### Canvas 2D Animation
- requestAnimationFrame redraw loop (clear + draw each frame)
- Sprite sheets and frame indices

### WebGL/WebGPU Animation
- Per-frame render calls with updated uniforms
- Vertex transformations in shaders

Performance comparison for animating 1000 elements:
- **SVG**: Reflow overhead, slow without GPU transforms
- **Canvas 2D**: Fast, GPU-backed texture, one composite call
- **WebGL**: Fastest, entire scene in one draw call

## Common Pitfalls

- **Choosing SVG for data with 10,000+ points**: Switch to Canvas 2D or WebGL
- **Drawing to Canvas without clearing**: Previous frame persists; forget `ctx.clearRect()`
- **Hit testing on Canvas**: No built-in; must track bounding boxes manually
- **WebGL without understanding clip space**: Vertices outside [-1, 1]³ are clipped invisibly
- **Mixing coordinate systems**: SVG top-left origin vs. Canvas top-left vs. WebGL center with Y-up
- **Not using offscreen Canvas**: Rendering UI and game logic separately, then compositing

For interactive graphics prioritizing responsiveness, prefer SVG. For performance-critical animation or 3D, prefer Canvas 2D or WebGL.