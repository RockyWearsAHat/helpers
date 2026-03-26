# CSS Performance — Rendering Optimization and Paint Efficiency

CSS performance determines time to interactive and visual stability. Modern optimization focuses on reducing render-blocking resources, lowering paint complexity, and enabling the compositor to work independently.

## Render-Blocking CSS

All CSS blocks rendering by default. The browser must parse CSS and apply styles to the DOM before painting, even if some CSS only applies at larger viewports or in print contexts.

### Render-Blocking Problem

```html
<link rel="stylesheet" href="styles.css">
<link rel="stylesheet" href="print.css">
<link rel="stylesheet" href="animation-heavy.css">
```

All stylesheets block rendering. The page waits for all three to download and parse before renderer proceeds.

### Solution: Media Query-Gated CSS

```html
<link rel="stylesheet" href="styles.css">
<link rel="stylesheet" href="print.css" media="print">
<link rel="stylesheet" href="responsive.css" media="(min-width: 768px)">
```

Stylesheets with non-matching media queries don't block rendering:
- `print.css` doesn't block screen rendering
- `responsive.css` doesn't block rendering on mobile

### Critical Path CSS

Extract essential styles for above-the-fold content:

```html
<style>
  /* Critical CSS: immediate paint */
  body { font: 16px sans-serif; }
  .header { background: white; padding: 1rem; }
  .hero { font-size: 2rem; }
</style>

<link rel="stylesheet" href="full-styles.css">
```

Inline critical CSS to enable First Contentful Paint (FCP) immediately. Load remaining styles asynchronously.

## Paint Worklet (CSS Paint API)

Paint worklet allows custom paint operations via JavaScript:

```css
.custom-shape {
  background-image: paint(custom-gradient);
}
```

Implemented in JavaScript:
```javascript
registerPaint('custom-gradient', class {
  paint(ctx, size) {
    ctx.fillStyle = '#007bff';
    ctx.fillRect(0, 0, size.width / 2, size.height);
  }
});
```

Reduces SVG or image overhead for custom shapes and patterns.

## Content-Visibility

`content-visibility` tells the browser content is off-screen and can defer layout, style, and paint calculations:

### `content-visibility: auto`

```css
.cards {
  content-visibility: auto;
  contain-intrinsic-size: auto 200px;
}
```

Off-screen `.cards` don't receive layout/paint until entering the viewport. The browser reserves 200px height, preventing layout shift as items scroll into view.

**Effect**: Reduces render time for long lists or tall pages by orders of magnitude.

### `content-visibility: hidden`

```css
.hidden-section {
  content-visibility: hidden;
}
```

Content is completely skipped during layout and paint, though it remains in the accessibility tree. Equivalent to `display: none` but faster for toggle patterns.

### `contain-intrinsic-size`

```css
.card {
  content-visibility: auto;
  contain-intrinsic-size: auto 150px;
}
```

Reserves 150px for layout calculations before paint. If actual height differs, layout shift occurs after paint (not ideal, but acceptable for performance trade-off).

## CSS Containment (`contain` Property)

Declare what aspects of an element are independent, allowing the browser to optimize:

```css
.isolated-component {
  contain: layout style paint;
}
```

### Containment Types

| Type | Effect |
|------|--------|
| `layout` | Element's layout is independent; descendants don't affect siblings |
| `style` | Scoped styles; `var()` doesn't leak, containment scope applies |
| `paint` | Element's paint is independent; clipping doesn't affect siblings |
| `size` | Element's size is independent; no layout recalculations for content changes |
| `content` | All of the above (equivalent to `layout style paint`) |
| `strict` | Both `layout` and `style` |

### Practical Example: Card Grid

```css
.card {
  contain: layout style paint;
  width: 300px;
  height: 300px;
}
```

Each card's layout, style calculation, and painting are independent. Changes inside one card don't trigger recalculation of siblings.

## Will-Change: Compositor Hints

`will-change` hints to the browser that an element will animate or change, enabling optimizer to prepare:

```css
.animated-box {
  will-change: transform, opacity;
  animation: slideIn 1s;
}
```

The browser may promote the element to a compositor layer, enabling GPU acceleration.

### Caution: Overuse Harms Performance

```css
/* BAD: too many elements, wastes GPU memory */
.item {
  will-change: transform;
}
```

Use `will-change` sparingly, only on elements that genuinely animate:

```css
/* GOOD: specific elements */
.hero.animated {
  will-change: transform;
}
```

### When to Remove

```css
.button {
  will-change: transform;
}

.button:active {
  transform: scale(0.95);
  will-change: auto; /* Remove after animation ends */
}
```

Remove `will-change` after the animation completes to free resources.

## Compositor Layers and Transform Promotion

CSS properties that trigger GPU acceleration (don't cause layout/paint recalculation):

| Property | Impact |
|----------|--------|
| `transform` | No layout recalc; GPU-accelerated |
| `opacity` | No layout recalc; GPU-accelerated |
| `filter` | GPU-accelerated (expensive) |
| `width`, `height` | Layout recalc (avoid animating) |
| `left`, `top` | Layout recalc (avoid animating) |
| `background-color` | Paint recalc (fast, usually OK) |

Animate `transform` and `opacity`, not position or dimensions:

```css
/* GOOD: GPU-accelerated */
@keyframes slideIn {
  from { transform: translateX(-100%); }
  to { transform: translateX(0); }
}

/* BAD: causes layout recalc */
@keyframes slideInBad {
  from { left: -100%; }
  to { left: 0; }
}
```

## Font-Display: Controlling Font Blocking

How fonts block text rendering:

```css
@font-face {
  font-family: 'MyFont';
  src: url('my-font.woff2') format('woff2');
  font-display: swap; /* Use fallback, swap when ready */
}
```

| Value | Behavior |
|-------|----------|
| `auto` | Browser default (usually `block`) |
| `block` | Hide text until font loads (max 3s) |
| `swap` | Show fallback immediately, swap when font loads |
| `fallback` | Show fallback immediately, wait briefly for font (failsafe after 100ms) |
| `optional` | Show fallback immediately; swap only if font loads early (within 100ms) |

**Recommendation**: Use `font-display: swap` for critical fonts to enable text rendering immediately.

## CSS Containment and Layout Shifts

`contain: layout` prevents elements from causing layout shifts in distant siblings:

```css
.sticky-header {
  position: sticky;
  contain: layout;
}

.content {
  contain: layout;
}
```

Isolates each section's layout calculations, preventing cascading reflows.

## Animation Performance: Keyframes

Animate only properties that don't trigger layout or paint:

```css
/* GOOD */
@keyframes fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* GOOD */
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* BAD: triggers layout recalc every frame */
@keyframes grow {
  from { width: 100px; }
  to { width: 200px; }
}
```

## Filter Performance

Filters are GPU-accelerated but expensive:

```css
/* Relatively cheap */
.blur { filter: blur(5px); }
.brightness { filter: brightness(150%); }

/* Expensive; avoid animating */
.drop-shadow { filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.2)); }
```

Use `box-shadow` or `text-shadow` instead of `drop-shadow()` for better performance.

## Performance Measurement

Use browser DevTools to identify bottlenecks:

1. **Lighthouse**: Audit performance metrics (FCP, LCP, CLS)
2. **Performance tab**: Measure rendering time by phase
3. **Rendering tab**: View paint events and compositor layers
4. **Coverage tab**: Identify unused CSS that could be removed

## Practical Optimization Checklist

| Technique | Benefit |
|-----------|---------|
| Critical CSS inlining | Faster First Contentful Paint |
| Media-gated stylesheets | Reduced render-blocking resources |
| `content-visibility: auto` | Faster rendering for long lists |
| `contain` on components | Independent layout calculations |
| Animating `transform`/`opacity` | GPU acceleration |
| `will-change` on animated elements | Preparation for GPU layer |
| `font-display: swap` | Text renders immediately |
| Removing unused CSS | Faster stylesheet download/parse |

## See Also

- [web-performance.md](web-performance.md) — Core Web Vitals, performance measurement
- [web-browser-rendering.md](web-browser-rendering.md) — Layout, paint, compositor pipeline
- [web-animation.md](web-animation.md) — Transitions, keyframes, performance considerations
- [web-image-optimization.md](web-image-optimization.md) — Image formats, loading strategies