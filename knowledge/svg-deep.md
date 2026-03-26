# SVG for Developers: Deep Dive

## Coordinate System and ViewBox

SVG renders on an infinite canvas using a coordinate system distinct from HTML's pixel grid.

### ViewBox: Logical Viewport

The `viewBox` attribute maps SVG coordinates to display size:

```html
<svg viewBox="0 0 200 100" width="400" height="300">
  <!-- Content drawn in 0,0 → 200,100 logical space -->
  <!-- Displayed at 400×300 physical pixels -->
  <!-- 2:1 horizontal scaling, 3:1 vertical scaling -->
</svg>
```

**`viewBox="x y width height"`**: Defines the logical rectangle to display. The SVG coordinate system (0,0) is top-left; x increases rightward, y downward (same as HTML canvas).

- If viewBox aspect ratio differs from display size, content scales non-uniformly (squashes/stretches). Use `preserveAspectRatio` to override (default: `xMidYMid meet`).
- `meet`: Scale uniformly to fit inside container (letterbox)
- `slice`: Scale uniformly to cover container (crop)
- `xMinYMin`, `xMidYMid`, `xMaxYMax`: Nine alignment options

```html
<svg viewBox="0 0 100 100" width="200" height="600" preserveAspectRatio="xMidYMid meet">
  <!-- Uniform 2:6 scaling would distort. "meet" scales uniformly to 2:2, centering content. -->
</svg>
```

Interactive resizing doesn't require viewBox recalculation; width/height alone can change without altering coordinates.

## SVG Elements and Shapes

SVG shapes are drawn in the order they appear (painters model: later elements on top).

### Basic Shapes

```html
<circle cx="50" cy="50" r="40" fill="blue" stroke="black" stroke-width="2" />

<rect x="10" y="20" width="80" height="60" rx="5" ry="5" fill="red" />
<!-- rx, ry: corner radius -->

<polygon points="50,10 90,90 10,90 50,50" fill="green" />
<!-- points: comma/space-separated x,y pairs -->

<polyline points="10,10 20,20 30,15" fill="none" stroke="black" stroke-width="2" />
<!-- Open path; polygon closes automatically, polyline doesn't -->

<line x1="0" y1="0" x2="100" y2="100" stroke="black" stroke-width="2" />

<ellipse cx="100" cy="50" rx="80" ry="40" fill="yellow" />
```

### Path Element: The Workhorse

`<path>` draws arbitrary shapes using a **path data string** (commands + coordinates):

```html
<path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" fill="blue" />
<!-- M: moveto; L: lineto; Z: closepath (straight line back to start) -->
```

#### Path Commands

| Command | Params | Description |
|---------|--------|------------|
| `M x y` | (x, y) | **Move** to coordinates (start new subpath) |
| `L x y` | (x, y) | **Line** to coordinates |
| `H x` | (x) | **Horizontal** line to x (y unchanged) |
| `V y` | (y) | **Vertical** line to y (x unchanged) |
| `C x1 y1 x2 y2 x y` | 6 params | **Cubic Bezier curve** (2 control points) |
| `S x2 y2 x y` | 4 params | **Smooth cubic** (reflects previous control point) |
| `Q x1 y1 x y` | 4 params | **Quadratic Bezier** (1 control point) |
| `T x y` | (x, y) | **Smooth quadratic** |
| `A rx ry x-axis-rotation large-arc-flag sweep-flag x y` | 7 params | **Arc** (elliptical) |
| `Z` | none | **Close** path with line to start |

Uppercase = absolute coordinates; lowercase = relative to current position.

Example combining commands:

```html
<!-- Heart shape -->
<path d="M 150 50
         C 150 20, 120 0, 90 0
         C 60 0, 30 20, 30 50
         C 30 80, 60 110, 90 140
         C 100 130, 110 120, 120 110
         C 150 80, 150 80, 150 50 Z" 
      fill="red" />
```

#### Bezier Curves Explained

**Cubic Bezier** (`C x1 y1 x2 y2 x y`): Curve from current point to (x, y), influenced by two control points (x1, y1) and (x2, y2). Control points act as "magnets" pulling the curve toward them.

**Quadratic Bezier** (`Q x1 y1 x y`): Single control point, simpler/faster to render. Often sufficient for icons.

**Arc** (`A rx ry rotation large-arc sweep x y`): Arc of an ellipse.
- `rx, ry`: Ellipse radii
- `rotation`: X-axis rotation of ellipse (degrees)
- `large-arc-flag`: 0 = small arc (< 180°), 1 = large arc
- `sweep-flag`: 0 = counterclockwise, 1 = clockwise
- `x, y`: End point

## Transforms and Coordinate Systems

Transform functions operate on SVG elements, altering coordinate systems for children:

```html
<g transform="translate(50, 30) rotate(45) scale(1.5)">
  <circle cx="0" cy="0" r="30" />
</g>
<!-- Circle drawn at origin (0,0), then transformed -->
```

Available transforms: `translate(x, y)`, `rotate(angle, cx, cy)`, `scale(sx, sy)`, `skewX(angle)`, `skewY(angle)`, `matrix(a b c d e f)`.

Rotation angle in degrees (not radians). `rotate(45, cx, cy)` rotates around point (cx, cy); default center is (0, 0).

**Order matters**: `translate(50, 0) rotate(45)` differs from `rotate(45) translate(50, 0)`. Transforms apply left-to-right.

CSS transforms work on SVG elements too:

```css
.icon {
  transform: rotate(90deg) translateX(10px);
}
```

## Filters: Blur, Color, Distortion

SVG filters are GPU-accelerated effects defined as reusable components:

```html
<defs>
  <filter id="blur">
    <feGaussianBlur in="SourceGraphic" stdDeviation="5" />
  </filter>
</defs>

<circle cx="50" cy="50" r="40" filter="url(#blur)" />
```

### Common Filter Primitives

| Filter | Effect |
|--------|--------|
| `feGaussianBlur` | Blur by stdDeviation |
| `feColorMatrix` | Color shifts (grayscale, brightness, hue) |
| `feOffset` | Shift element offset |
| `feBlend` | Blend two inputs (multiply, screen, etc.) |
| `feMorphology` | Erode or dilate shapes |
| `feComposite` | Combine two inputs (arithmetic operations) |
| `feTurbulence` | Perlin noise for texture effects |
| `feDropShadow` | Shadow effect shorthand |

#### Color Matrix Example

```html
<feColorMatrix type="saturate" values="0" />
<!-- values: 0 = grayscale, 1 = normal, 2 = oversaturated -->

<feColorMatrix type="hueRotate" values="45" />
<!-- Rotate hue by 45 degrees -->

<feColorMatrix type="matrix" values="
  1 0 0 0 0
  0 1 0 0 0
  0 0 1 0 0
  0 0 0 1 0" />
<!-- 5×4 matrix; each row is [R G B A offset] -->
```

**Performance caveat**: Complex filters (turbulence, displacement) can be expensive on large surfaces. Use sparingly or apply to small elements.

## Clipping and Masking

### Clip-Path: Hard Boundaries

Clip-path cuts pixels outside a defined region:

```html
<defs>
  <clipPath id="clip-circle">
    <circle cx="50" cy="50" r="40" />
  </clipPath>
</defs>

<image href="photo.jpg" x="0" y="0" width="100" height="100" clip-path="url(#clip-circle)" />
<!-- Image visible only within circle; outside is transparent -->
```

### Mask: Transparency Gradients

Mask uses luminance (brightness) to control transparency:

```html
<defs>
  <mask id="fade-mask">
    <rect x="0" y="0" width="100" height="100" fill="white" />
    <!-- White = opaque, black = transparent -->
    <rect x="50" y="0" width="50" height="100" fill="url(#gradient)" />
  </mask>
</defs>

<image href="photo.jpg" mask="url(#fade-mask)" />
<!-- Right half fades out based on gradient -->
```

Clip-path is fast; mask is slower (requires alpha compositing per pixel).

## SMIL Animation: SVG Native Transitions

SMIL (Synchronized Multimedia Integration Language) animates SVG properties declaratively, without JavaScript or CSS:

```html
<circle cx="50" cy="50" r="40" fill="blue">
  <animate 
    attributeName="r" 
    from="40" 
    to="80" 
    dur="2s" 
    repeatCount="infinite" 
    fill="freeze" />
  <!-- Radius oscillates 40→80 forever -->
</circle>
```

### SMIL Elements

| Element | Purpose |
|---------|---------|
| `animate` | Animate a single property |
| `animateTransform` | Animate transform (rotate, scale, translate) |
| `animateMotion` | Move along path |
| `set` | Set property instantly (no interpolation) |

```html
<g>
  <circle cx="50" cy="50" r="10" fill="red" />
  <animateMotion dur="3s" repeatCount="infinite">
    <mpath href="#path-id" />
    <!-- Follows path defined by <path id="path-id"> -->
  </animateMotion>
</g>
```

**Deprecation risk**: SMIL is not actively developed; browser support remains but won't expand. New SVG animations use CSS or Web Animations API for future-proofing.

## SVG Sprites: Efficient Icons and Reuse

Sprite sheets group multiple icons in one SVG file:

```html
<!-- sprite.svg -->
<svg xmlns="http://www.w3.org/2000/svg" style="display: none;">
  <symbol id="icon-save" viewBox="0 0 24 24">
    <path d="...save icon path..." />
  </symbol>
  <symbol id="icon-delete" viewBox="0 0 24 24">
    <path d="...delete icon path..." />
  </symbol>
</svg>

<!-- Usage in HTML -->
<svg width="24" height="24">
  <use href="sprite.svg#icon-save" />
</svg>
```

Benefits:
- Single HTTP request for all icons
- Easy recoloring via CSS (`.icon { fill: blue; }`)
- Scalable (viewBox handles sizing)
- Small file size (SVG compresses well)

`<use>` elements don't download the sprite until referenced; lazy loading supported.

## Accessibility

SVG accessibility requires semantic markup:

```html
<svg role="img" aria-label="Play button">
  <title>Play</title>
  <circle cx="50" cy="50" r="40" />
  <polygon points="35,30 35,70 65,50" fill="white" />
</svg>

<!-- For decorative SVG, use aria-hidden -->
<svg aria-hidden="true">
  <circle cx="50" cy="50" r="40" />
</svg>
```

- Use `<title>` for hover tooltips
- Use `role="button"` for interactive SVG
- Use `role="img"` for icon/image SVG
- Ensure sufficient color contrast

## Optimization: SVGO and Manual Techniques

SVG files contain redundancy: metadata, unused attributes, verbose paths.

### SVGO: Automated Optimization

```bash
npm install -g svgo
svgo input.svg -o output.svg
```

- Removes metadata, unused defs
- Simplifies paths (reduces precision, smaller file)
- Converts element names to lowercase

### Manual Optimization

- **Remove fill/stroke if inherit**: `<circle><animate>` inherits parent fill
- **Use presentation attributes over inline styles**: Smaller bytes
- **Round large decimals**: `10.5432 → 10.5`
- **Use relative commands in paths**: `l 10 0` vs `L 50 0` if relative is shorter

Example:

```html
<!-- Before -->
<svg viewBox="0 0 100.0000 100.0000" xmlns="http://www.w3.org/2000/svg">
  <defs></defs>
  <circle cx="50.0000" cy="50.0000" r="40.0000" style="fill: blue; stroke: none;" />
</svg>

<!-- After (SVGO optimized) -->
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" fill="blue" />
</svg>
```

## SVG vs Canvas vs DOM

|  | **SVG** | **Canvas** | **DOM** |
|--|---------|-----------|--------|
| **Rendering** | Retained-mode (DOM-based) | Immediate-mode (pixel buffer) | Retained (DOM nodes) |
| **Interactivity** | Native (events on elements) | Manual (hit testing) | Native (click, hover, etc.) |
| **Animation** | Smooth (GPU, CSS/WAAPI/SMIL) | Smooth (rAF loop) | Smooth (CSS/WAAPI) |
| **Scalability** | Any resolution (vector) | Fixed resolution (rasterized) | Fixed (text remains sharp) |
| **Performance** | Good for <500 elements | Excellent for dense graphics | Depends on DOM size |
| **Text** | First-class (searchable) | Rasterized (no search) | Native (accessible) |
| **File size** | Compact (vector ops) | Large (pixel data) | Medium (markup) |

Use SVG for: icons, logos, diagrams, responsive graphics. Use Canvas for: games, real-time visualization, particle effects. Use DOM for: documents, UI.

## Summary

SVG is a scalable vector format with DOM integration, enabling interactive graphics with native events, CSS styling, and filters. Path commands (`M L C Q A`) draw arbitrary shapes; transforms and filters enable complex effects. Sprites reduce HTTP requests. SMIL animates natively but is deprecated; prefer CSS/WAAPI. Clip-path and masks control visibility. Accessible SVG uses semantic roles and titles. Optimize with SVGO. SVG excels at adaptive, interactive graphics that scale across devices while remaining searchable and accessible—advantages neither Canvas nor rasterized images match.