# CSS Container Queries — Responsive Components

Container queries enable components to respond to their container's size, not the viewport size. Introduced in CSS Containment Level 3 spec, they represent a shift from viewport-based to context-based responsive design.

## Problem: Why Container Queries?

Media queries query the viewport:
```css
@media (min-width: 768px) {
  .card { grid-template-columns: 1fr 1fr; }
}
```

But components often appear in different contexts:
- Same card in a 400px sidebar layout
- Same card in a 600px main column
- Same card in a 900px full-width grid

Media queries force components to change at fixed breakpoints regardless of their actual space. A card at 500px inside a 1200px viewport can't adapt differently if placed in a 400px sidebar. **Container queries solve this: components query their own container size, enabling truly reusable, context-aware layouts.**

## Container Type: Establishing Query Targets

Before querying a container, designate it as queryable with `container-type`:

```css
.card-container {
  container-type: inline-size;
}
```

### Container Types

| Type | Queries |
|------|---------|
| `inline-size` | Logical width (physical width in LTR, height in vertical writing) |
| `size` | Both width and height |
| `normal` | No containment (default) |

**Practical guidance**: Use `inline-size` for most responsive layouts (quicker than two-dimensional queries). Use `size` rarely — it's needed only for aspect-ratio-dependent layouts.

### Container Names: Multiple Query Targets

In complex layouts, distinguish between multiple containers:

```css
.sidebar {
  container-type: inline-size;
  container-name: sidebar;
}

.main {
  container-type: inline-size;
  container-name: main;
}

@container sidebar (width < 300px) {
  .sidebar-item { display: none; }
}

@container main (width >= 800px) {
  .main-grid { grid-template-columns: 1fr 1fr 1fr; }
}
```

`container-name` is optional; if omitted, queries target the nearest queryable ancestor.

## Container Query Syntax

### Size Queries

```css
@container (width >= 400px) {
  .card-content { display: grid; grid-template-columns: 1fr 1fr; }
}
```

Supported operators: `>` (greater than), `<` (less than), `>=`, `<=`, `=`.

### Logical Operators

```css
@container (width >= 400px) and (width < 800px) {
  /* single-column wide enough for side-by-side text */
}

@container (width < 300px) or (height < 400px) {
  /* compact layout for narrow OR short containers */
}

@container not (width < 200px) {
  /* everything except very small */
}
```

### Style Queries

Query computed values of elements inside the container (advanced, limited browser support as of 2024):

```css
@container style(color: red) {
  /* Only applies if a parent has computed color: red */
}
```

Style queries enable conditional logic based on inherited properties, but support is incipient.

## Container Query Units

Inside a container query block, use container-relative units:

| Unit | Meaning |
|------|---------|
| `cqw` | 1% of container's width |
| `cqh` | 1% of container's height |
| `cqi` | 1% of container's inline size |
| `cqb` | 1% of container's block size |
| `cqmin` | Smaller of `cqi` or `cqb` |
| `cqmax` | Larger of `cqi` or `cqb` |

Example:
```css
@container (width >= 400px) {
  .card-title { font-size: 5cqw; } /* scales with container */
}
```

Container query units make component internals respond proportionally to their context, enabling truly scalable designs.

## Container Queries vs. Media Queries

| Aspect | Media Queries | Container Queries |
|--------|---------------|-------------------|
| What they query | Viewport, device characteristics | Container's size |
| Use case | Global layout decisions | Component-level responsiveness |
| Can nest | No | Yes (query any ancestor) |
| Dependency | External context | Local context |
| Recursive | No | Yes (containers can query parents) |

**Container queries don't replace media queries.** Use both:
- **Media queries**: Adapt overall page layout to viewport and device (light/dark mode, reduced motion)
- **Container queries**: Adapt components to their specific context

## Practical Patterns

### Reusable Card Component

```css
.card-wrapper {
  container-type: inline-size;
}

.card { padding: 1rem; }

@container (width < 300px) {
  .card {
    display: block;
    padding: 0.75rem;
  }
  .card-footer { display: none; }
}

@container (width >= 300px) and (width < 600px) {
  .card {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 1rem;
  }
}

@container (width >= 600px) {
  .card {
    display: grid;
    grid-template-columns: auto 2fr 1fr;
    gap: 1rem;
  }
}
```

This card adapts independently in a 200px sidebar, 500px main column, or 900px grid without viewport media queries.

### Nested Container Queries

```css
.outer-container { container-type: inline-size; }
.inner-container { container-type: inline-size; }

@container outer-container (width >= 800px) {
  .item { flex: 1 1 300px; }
}

@container inner-container (width < 200px) {
  .item { font-size: 0.875rem; }
}
```

Containers can query their nearest ancestor container, enabling multi-level responsive logic.

## Browser Support and Caveats

Container queries shipped in all major browsers (2023–2024):
- Chrome 105+ (2022)
- Firefox 110+ (2023)
- Safari 16+ (2022)

**Important caveats:**
1. **Container establishes containment**: `container-type` enables CSS containment, which means the container is a stacking context, establishes block formatting context, and limits layout calculations to descendants inside the container.
2. **Circular logic prevented**: Can't query a container from within itself; queries target ancestors only.
3. **Performance**: Queries are efficient because layout calculations are scoped to the container.

## Migration from Media Queries

Existing layouts using media queries can gradually adopt container queries:

```css
/* Old: viewport-dependent */
@media (min-width: 768px) {
  .card { grid-template-columns: 1fr 1fr; }
}

/* New: context-dependent, works at any viewport size */
@container (width >= 400px) {
  .card { grid-template-columns: 1fr 1fr; }
}
```

Prefer container queries for **component layouts** and media queries for **page layouts and global settings**.

## See Also

- [design-responsive.md](design-responsive.md) — Media queries, breakpoints, fluid typography
- [web-css-layout.md](web-css-layout.md) — Grid, flexbox, layout modes
- [architecture-design-systems.md](architecture-design-systems.md) — Component systems