# CSS Grid Deep Dive

## Core Model: Grid Lines, Tracks, and Gaps

CSS Grid is a 2D layout system on a coordinate grid. Unlike Flexbox's single-axis flow, Grid explicitly positions items across rows and columns.

### Grid Terminology

- **Grid lines**: Named boundaries between tracks (1-indexed). A 3-column grid has 4 vertical lines (1, 2, 3, 4).
- **Grid tracks**: Space between lines. A `grid-template-columns: 100px 200px` creates 2 tracks (100px, 200px) and 3 lines (start, mid, end).
- **Grid cells**: Intersection of a row track and column track. The smallest unit.
- **Grid area**: Rectangle spanning one or more cells.
- **Gutter (gap)**: Space between tracks. `grid-gap: 20px` adds 20px between all items.

```css
.container {
  display: grid;
  grid-template-columns: 100px 200px 150px;  /* 3 tracks, 4 lines */
  grid-template-rows: auto 50px 1fr;
  grid-gap: 15px;  /* shorthand for grid-column-gap + grid-row-gap */
}
```

This creates a 3×3 grid. Items placed explicitly via `grid-column` / `grid-row` or auto-placed in order.

## Explicit Placement: Grid Lines and Spans

Position items by line numbers or spans:

```css
.item-a {
  grid-column: 1 / 3;  /* Start line 1, end line 3 = spans columns 1–2 */
  grid-row: 1 / 2;     /* Row 1 only */
}

.item-b {
  grid-column: 2 / span 2;  /* Start line 2, span 2 = end line 4 */
  grid-row: 2 / 4;          /* Rows 2–3 */
}
```

Line numbering:
- Positive: 1-indexed from start (1, 2, 3, ...)
- Negative: 1-indexed from end (-1, -2, -3, ... where -1 = last line)

`span N` means "cover N tracks starting from current position."

## Named Lines and Grid Template Areas

Names make grid code self-documenting and refactor-proof.

### Named Lines

```css
.container {
  grid-template-columns: 
    [sidebar-start] 200px 
    [sidebar-end content-start] 1fr 
    [content-end];
  grid-template-rows: 
    [header-start] 60px 
    [header-end content-start] auto 
    [content-end footer-start] 40px 
    [footer-end];
}

.sidebar {
  grid-column: sidebar-start / sidebar-end;
  grid-row: content-start / content-end;
}

.header {
  grid-column: content-start / content-end;
  grid-row: header-start / header-end;
}
```

Multiple names on one line (e.g., `[sidebar-end content-start]`) are both assigned to the same line, useful for column/row boundaries. Reference any name:

```css
grid-column: sidebar-start / span 2;  /* Start at named line, span 2 tracks */
```

### Grid Template Areas

ASCII diagram syntax for intuitive layout:

```css
.container {
  display: grid;
  grid-template-columns: 1fr 2fr;
  grid-template-rows: 60px auto 40px;
  grid-template-areas: 
    "header header"
    "sidebar main"
    "footer footer";
  gap: 10px;
}

.header { grid-area: header; }
.sidebar { grid-area: sidebar; }
.main { grid-area: main; }
.footer { grid-area: footer; }
```

Each string represents a row; space-separated words are columns. Named areas span all cells with that name. `.` (dot) represents unused cells.

Areas must form rectangles; `zig-zag` doesn't work. Areas automatically determine grid size (3 rows × 2 columns inferred above).

Refactoring with named areas is trivial—rearrange the diagram, and elements follow:

```css
grid-template-areas: 
  "header sidebar"
  "main sidebar"
  "footer footer";
```

## Track Sizing: Fixed, Flexible, and Auto

Track width/height controls how space is distributed.

### Fixed Sizes
```css
grid-template-columns: 100px 200px 150px;
```

Total = 450px (regardless of container size).

### Flexible Units: `fr` (Fraction)

`fr` divides remaining space proportionally. `1fr` = 1 share; `2fr` = 2 shares.

```css
grid-template-columns: 200px 1fr 1fr;  /* 200px fixed, remaining space split equally */
/* In 900px container: 200px + 350px + 350px */

grid-template-columns: 1fr 2fr 1fr;  /* 1 : 2 : 1 ratio */
/* In 900px: 225px + 450px + 225px */
```

`fr` applies *after* fixed/auto sizes consume their space, so mixing units works naturally.

### Auto Sizing

`auto` adapts to content:

```css
grid-template-columns: auto 1fr auto;  /* Fit first/last to content, middle takes remaining */
```

### Min-Content and Max-Content

- `min-content`: Smallest size without wrapping content (longest unbreakable unit, e.g., word)
- `max-content`: Largest size allowing content to reflow naturally

```css
grid-template-columns: min-content 1fr max-content;
```

### Minmax(): Responsive Flexibility

`minmax(min, max)` constrains track size to a range:

```css
grid-template-columns: minmax(100px, 1fr) minmax(150px, 300px) auto;
```

First track: minimum 100px, grows up to 1fr of available space. Second: minimum 150px, caps at 300px. Enables fluid, responsive grids without media queries.

## Auto-Placement Algorithm

If items don't have explicit placement, the browser auto-places them in reading order (left-to-right, top-to-bottom by default).

```css
.container {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  grid-auto-rows: minmax(100px, auto);
}

/* Items placed in order: 1→2→3→4... */
/* After row 1 fills, row 2 auto-created */
/* grid-auto-rows: minmax(...) sizes auto-created rows */
```

`grid-auto-flow: column` changes to column-first (left-to-right, top-to-bottom traversal switches to top-to-bottom, left-to-right). `grid-auto-flow: row dense` attempts to fill holes (greedy packing).

## Auto-Fill vs Auto-Fit in Repeat

Both `repeat(auto-fill, minmax(...))` and `repeat(auto-fit, minmax(...))` create responsive grids without media queries:

```css
grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
```

Creates as many 200px columns as fit the container; extras wrap.

### Difference

- **`auto-fill`**: Creates tracks even if empty. If 5 items fit in a 6-track row, track 6 remains empty.
- **`auto-fit`**: Collapses empty tracks. Same scenario, 6th track collapses to 0 width.

```css
/* Rarely matters, but collapse-empty is auto-fit behavior */
grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));  /* May have empty rightmost columns */
grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));   /* Collapses empty */
```

In practice, use `auto-fit` for cleaner layouts when items shouldn't have forced spacing.

## Subgrid: Nesting Alignment

Subgrid allows a child grid to inherit the parent grid's lines, enabling vertical alignment across nested layouts—a game-changer for card layouts, form fields, etc.

```css
.container {
  display: grid;
  grid-template-columns: 200px 1fr 100px;
  grid-template-rows: auto auto;
}

.card {
  display: grid;
  grid-template-columns: subgrid;  /* Inherit parent's 3 columns */
  grid-template-rows: subgrid;      /* Inherit parent's 2 rows */
}

.card-title {
  grid-column: 1 / 2;  /* Aligns with parent's first column */
  grid-row: 1 / 2;
}
```

Without subgrid, nested grids create independent coordinate systems; images and text in different cards won't align vertically. Subgrid solves this.

Browser support: Chrome/Edge 115+, Firefox 115+, Safari 17+. Older browsers ignore subgrid (falls back to normal grid nesting).

## Alignment: Justify & Align

Grid alignment operates on two axes:

- **`justify-*`** (row axis, left-right): Controls column alignment
- **`align-*`** (column axis, top-bottom): Controls row alignment

### Track Alignment (Space Distribution)

- `justify-content`: Position all columns + gaps within container (start, end, center, space-between, space-around, space-evenly)
- `align-content`: Position all rows + gaps within container

```css
.container {
  height: 500px;
  justify-content: center;  /* Center columns horizontally */
  align-content: space-between;  /* Space rows vertically */
}
```

### Item Alignment Within Cells

- `justify-self`: Align item left-right within its cell
- `align-self`: Align item top-bottom within its cell
- `justify-items`: Set `justify-self` for all children
- `align-items`: Set `align-self` for all children

```css
.container {
  align-items: center;  /* Vertically center all items */
  justify-items: center;  /* Horizontally center all items */
}

.item-special {
  align-self: start;  /* Override: align to top */
}
```

## Masonry Layout and Gaps in Standards

CSS has no official masonry layout (Pinterest-style irregular columns). Workarounds:

### Column-Based Pseudo-Masonry

```css
.grid {
  column-count: 3;
  column-gap: 20px;
}

.item {
  break-inside: avoid;  /* Prevent item spanning columns */
}
```

Downside: Items flow top-to-bottom within columns, not left-to-right. Layout control is poor.

### Grid with Auto-Rows

Approximate masonry with dense packing:

```css
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  grid-auto-rows: 250px;  /* Fixed height: NOT true masonry */
  grid-auto-flow: dense;
}
```

Items don't reflow to fill gaps; only explicit placement changes help. True masonry is standardized in CSS but browser support is minimal (Chrome behind flag). Use JavaScript or wait for `layout: masonry` in CSS.

## Grid vs Flexbox Decision Tree

|  | **Grid** | **Flexbox** |
|--|----------|-----------|
| **Dimensionality** | 2D (rows + columns) | 1D (one axis) |
| **Item placement** | Explicit or auto-placed | Flow-based |
| **Use case** | Page layouts, card grids, form layouts | Navigation, component alignment, content distribution |
| **Nesting complexity** | Scales well (subgrid) | Harder (nested flex containers) |
| **Responsive** | `auto-fit`, `minmax` w/o media queries | Flexbox works, often needs media queries |

**Grid for:**
- Multi-column layouts (header, sidebar, main, footer)
- Grids of cards with consistent sizing
- Complex form layouts where fields should align
- Art direction in responsive design

**Flexbox for:**
- Navigation bars
- Centered content within containers
- Distributing space among items on a single axis
- Building blocks used inside grid items

Common pattern: Grid for page structure, Flexbox for component details.

## Responsive Patterns Without Media Queries

### Dynamic Column Count

```css
grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
```

On small screens, fewer columns auto-create; on large, more appear. No breakpoints needed.

### Intrinsic Sizing

```css
grid-template-columns: 
  min(1fr, 100%)
  clamp(200px, 50vw, 600px)
  max(auto, 100px);
```

- `min(A, B)`: Smaller of A or B
- `max(A, B)`: Larger of A or B
- `clamp(min, preferred, max)`: Constrain preferred between bounds

These respond to viewport and content without media queries.

### Container Queries Alternative

```css @container (min-width: 600px) {
  .grid {
    grid-template-columns: 1fr 2fr;
  }
}
```

Query *container* width, not viewport—better component encapsulation. Standard in modern browsers.

## Performance and Browser Considerations

- **Rendering**: Grid layout is GPU-friendly; browser engines optimize grid calculations. No performance penalty vs Flexbox.
- **Subgrid nesting depth**: Deep nesting (5+ levels) has marginal overhead but doesn't break.
- **`grid-gap` vs `gap`**: Both work; `gap` is the modern CSS standard, applies to Grid + Flexbox.
- **Implicit rows/columns**: Using `grid-auto-rows` / `grid-auto-columns` creates unlimited implicit tracks. Use sparingly for performance.

## Summary

CSS Grid provides powerful 2D layout control via explicit or auto-placement, named lines/areas, and responsive sizing. `subgrid` enables vertical alignment across nested layouts. `auto-fit` with `minmax()` replaces many media queries. Grid excels at page structure and multi-column designs; combine with Flexbox for component-level alignment. Named areas and template syntax make layouts self-documenting and refactor-safe.