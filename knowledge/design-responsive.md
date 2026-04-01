# Responsive Design — Media Queries, Container Queries, Fluid Layouts & Modern CSS

## From Adaptive to Responsive to Container-Based

Web design has evolved through paradigms as CSS capabilities expanded:

- **Desktop-first (2000s)**: Build for desktop, bolt on mobile with `display: none`
- **Responsive (2010s)**: One codebase, flexible grids, media queries at breakpoints (Ethan Marcotte, 2010)
- **Mobile-first (2010s-2020s)**: Start with mobile constraints, enhance for larger screens
- **Intrinsic web design (2020s)**: Components respond to context (viewport, container, user preferences) rather than breakpoints
- **Container queries (2023+)**: Components size based on their container size, not viewport — recursive responsive design

Each paradigm remains relevant. Container queries don't replace media queries; they complement them.

## Media Queries and Breakpoints

Media queries respond to viewport and device characteristics: screen width, height, color depth, orientation, reduced motion preference.

### Breakpoint Strategy

There's no universal set of "correct" breakpoints. Strategies vary:

**Pixel-based breakpoints** (most common):
```css
@media (min-width: 768px) { /* tablet */ }
@media (min-width: 1024px) { /* desktop */ }
@media (min-width: 1440px) { /* large desktop */ }
```

Breakpoints should align with design intent, not device inventory. Breakpoints change as devices do; rather than "iPhone sizes," use breakpoints where your design actually breaks.

**Content-based breakpoints**:
```css
@media (min-width: 40em) { /* layout shifts here */ }
```

Some teams choose breakpoints based on where content naturally reflows (e.g., when a two-column layout collapses to one column). This requires testing in browser, not guessing from device specs.

**Range queries (modern browsers)**:
```css
@media (width >= 768px) and (width < 1024px) { }
/* Instead of: */
@media (min-width: 768px) and (max-width: 1023px) { }
```

Clearer, less error-prone, now widely supported (2024+).

### Mobile-First vs. Desktop-First

**Mobile-first**: Base stylesheet is mobile, then add media queries for larger screens:
```css
.card { /* mobile layout */ }
@media (min-width: 768px) {
  .card { /* tablet/desktop */ }
}
```

**Desktop-first**: Base stylesheet is desktop, media queries remove behavior for smaller screens:
```css
.card { /* desktop layout */ }
@media (max-width: 767px) {
  .card { /* mobile */ }
}
```

Mobile-first forces thoughtful reduction and works well with progressive enhancement. Desktop-first is sometimes more intuitive if the design starts there but can lead to bloated mobile CSS if not disciplined.

## Flexible Grids and Layouts

Responsive layouts rest on flexible foundations, not fixed dimensions.

### CSS Grid for Responsive Layout

```css
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 1rem;
}
```

`auto-fit` creates as many columns as fit in the container. `minmax(300px, 1fr)` ensures columns are at least 300px but grow to fill available space. No breakpoints needed — layout responds fluidly.

### Flexbox for Linear Flows

```css
.container {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}
.item {
  flex: 1 1 300px; /* grow, shrink, basis */
}
```

`flex: 1 1 300px` means: grow equally, shrink equally, minimum width 300px. Items wrap when space runs out.

## Fluid Typography

Text size should adjust to viewport size, not jump at breakpoints. `clamp()` enables fluid scaling without media queries.

```css
.heading {
  font-size: clamp(1.5rem, 5vw, 3rem);
}
```

This reads: "At least 1.5rem, preferably 5% of viewport width, but never more than 3rem." Text scales smoothly between breakpoints.

### Typographic Scale

A typographic scale uses consistent ratios between font sizes (commonly 1.5× or 1.125× ratio):

```
Body: 1rem (16px)
Heading 3: 1.25rem (1rem × 1.25)
Heading 2: 1.5625rem (1.25 × 1.25)
Heading 1: 1.953rem (1.5625 × 1.25)
```

This creates visual harmony without arbitrary sizes scattered throughout CSS.

## Container Queries — A Paradigm Shift

Container queries are the most significant responsive design feature since Grid. They enable components to adapt based on their container's size, not the viewport.

### Establishing a Query Container

```css
.card {
  container-type: inline-size; /* query against width, not height */
}
```

`inline-size` queries the logical width (physical width in LTR, height in vertical writing). `size` queries both width and height (rarely needed).

### Container Query Units

Inside a component, use container query units:

```css
@container (width >= 400px) {
  .card-content {
    display: grid;
    grid-template-columns: 1fr 1fr; /* two-column layout */
  }
}

@container (width < 400px) {
  .card-content {
    display: block; /* single column */
  }
}
```

Container query units (`cqw`, `cqh`, `cqi`, `cqb`):
- `1cqw` = 1% of container width
- `1cqh` = 1% of container height
- `cqmin/cqmax` for aspect-ratio calculations

### Design System Implications

Container queries make components truly reusable. A card component works in a sidebar narrow container and a full-width grid without modification:

```css
.card {
  container-type: inline-size;
}
.card-image {
  width: 100%;
}
@container (width >= 300px) {
  .card-image {
    float: left;
    width: 40%;
  }
}
```

The component responds to its context, not viewport. Impossible before container queries.

## Responsive Images

Responsive images serve appropriately sized assets based on viewport, pixel density, and format support.

### srcset for Resolution and Size

```html
<img 
  src="image-400w.jpg"
  srcset="
    image-400w.jpg 400w,
    image-600w.jpg 600w,
    image-800w.jpg 800w"
  sizes="
    (max-width: 600px) 90vw,
    (max-width: 1200px) 60vw,
    800px"
  alt="Description"
/>
```

`srcset` lists images and their widths. `sizes` tells the browser which image to request based on viewport/device. The browser picks the smallest image that adequately fills the space — not always the largest available.

### picture for Art Direction

When different aspect ratios or compositions are needed for different viewports:

```html
<picture>
  <source media="(min-width: 1024px)" 
          srcset="desktop-wide.jpg 1200w">
  <source media="(min-width: 768px)" 
          srcset="tablet.jpg 800w">
  <img src="mobile.jpg" alt="Description" />
</picture>
```

Use `picture` for significant composition changes, not just size. Otherwise, `srcset` is simpler.

## Modern Viewport and Sizing Units

CSS viewport units have expanded beyond `vw`/`vh`:

- `vw/vh` — Viewport width/height (100vw = full width)
- `dvw/dvh` — Dynamic viewport (accounts for mobile browser UI that appears/disappears)
- `svw/svh` — Small viewport (minimum size when browser UI is visible)
- `lvw/lvh` — Large viewport (maximum size when browser UI is hidden)

On mobile, the address bar appears and disappears; `100vh` could exceed the visible area. Use `dvh` for full-screen layouts. `svh` ensures content fits regardless of browser UI state.

```css
.hero {
  min-height: 100dvh; /* Full screen including dynamic viewport */
}
```

## Viewport Meta Tag and Device Pixel Ratio

```html
<meta name="viewport" content="width=device-width, initial-scale=1" />
```

This tells mobile browsers: "Assume a 1-to-1 ratio between CSS pixels and device pixels, render at device width." Without it, mobile browsers scale the page down.

Device pixel ratio (DPR) — physical pixels ÷ CSS pixels — determines which srcset image loads. A 1080px device with DPR 2 requests 540px images (2 pixels per CSS pixel). Omit this calculation; let the browser choose via `sizes`.

## Intrinsic Web Design Principles

Modern responsive design embraces constraints given by the web's nature (fluid medium, user preferences, multiple devices) rather than fighting them:

1. **Fluid and fixed typography**: Use both; choose based on purpose
2. **Truly fluid layouts**: Modern CSS (Grid, Flexbox) eliminates fixed-width column layouts
3. **Responsive images and media**: Art-directed and size-appropriate
4. **Contextual responsive behavior**: Container queries for components; media queries for page-level changes
5. **Respect user preferences**: `prefers-reduced-motion`, `prefers-color-scheme`, `prefers-contrast`

## Responsive Data Tables

Data tables are problematic on mobile: full-width tables overflow; column stacking obscures structure.

### Stack-on-Mobile Pattern

```css
@media (max-width: 768px) {
  table, thead, tbody, tr, th, td {
    display: block;
  }
  thead {
    display: none; /* Hide header row */
  }
  tr {
    border: 1px solid #ddd;
    margin-bottom: 1rem;
  }
  td::before {
    content: attr(data-label);
    font-weight: bold;
    display: block;
  }
}
```

```html
<td data-label="Email">user@example.com</td>
```

Each cell displays its label (via `data-label` and `::before`), creating a key-value list on small screens.

### Horizontal Scroll (Less Ideal)

Some tables scroll horizontally on mobile — acceptable for complex data but worse for accessibility and usability than restructuring.

## See Also

- design-color-typography.md — Readability at various sizes
- design-systems.md — Token-based responsive values
- web-browser-rendering.md — How viewport and device pixels interact