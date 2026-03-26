# CSS Layout Systems — Concepts & Evolution

## The Foundational Model: Normal Flow

Every CSS layout system builds on or departs from _normal flow_ — the default algorithm browsers use to position elements without explicit layout instructions. Understanding normal flow is prerequisite to understanding everything that followed.

In normal flow, elements participate in one of two formatting contexts:

| Context           | Behavior                                                               | Elements                                |
| ----------------- | ---------------------------------------------------------------------- | --------------------------------------- |
| Block formatting  | Elements stack vertically, each occupying the full available width     | `div`, `p`, `section`, headings         |
| Inline formatting | Elements flow horizontally within a line, wrapping when space runs out | `span`, `a`, `em`, `strong`, text nodes |

The interaction between these two contexts produces the fundamental document flow that HTML has used since the 1990s. Block-level elements generate _block boxes_ that participate in a block formatting context. Inline-level elements generate _inline boxes_ that participate in an inline formatting context.

### Block Formatting Context Creation

A block formatting context (BFC) is an independent layout region where the internal layout of elements doesn't affect the outside, and vice versa. BFCs are created by several conditions:

- The root element of the document
- Floated elements
- Absolutely or fixed positioned elements
- Elements with `overflow` values other than `visible`
- Elements with `display: flow-root` (the explicit, purpose-built trigger)
- Flex and grid items
- Table cells and captions

BFC creation matters because it controls float containment, margin collapsing boundaries, and layout independence. Many "clearfix hacks" throughout CSS history were indirect ways of triggering BFC creation before `display: flow-root` existed.

### Margin Collapsing

Adjacent vertical margins between block-level elements in the same BFC collapse — the larger margin wins rather than both being applied. This behavior surprises developers but follows a coherent logic: margins express _minimum desired spacing_, not additive spacing. A paragraph wanting 20px below it and a heading wanting 30px above it need 30px between them, not 50px.

Margin collapsing does not occur in flex containers, grid containers, or across BFC boundaries — which is one reason these newer layout modes feel more predictable.

## Float-Based Layouts — Emergence and Limitations

Floats were designed for a narrow purpose: wrapping text around images, mirroring print layout conventions. When CSS lacked any real layout mechanism, developers repurposed floats as a general-purpose layout tool — a creative misuse that dominated web layout for over a decade.

### How Floats Work

A floated element is removed from normal flow and shifted to the left or right edge of its containing block. Subsequent content flows around it. Floats do not contribute to the height of their parent — the "collapsing container" problem — which led to the necessity of _clearing_.

```css
/* The classic two-column float layout */
.sidebar {
  float: left;
  width: 25%;
}
.main {
  float: left;
  width: 75%;
}
.footer {
  clear: both;
} /* forces footer below both floats */
```

### Why Floats Were Inadequate for Layout

The fundamental issue: floats solve a content-wrapping problem, not a space-distribution problem. Consequences included:

- **No vertical centering** — floats have no concept of alignment along the cross axis
- **No equal-height columns** without hacks (faux columns via background images, or `display: table`)
- **Source order dependency** — float layout is tightly coupled to DOM order
- **Fragile clearfix requirements** — forgetting to clear floats produced cascading layout breakage
- **No intrinsic sizing negotiation** — floats couldn't express "take remaining space" or "share space equally"

The industry iterated through progressively clever workarounds: the clearfix hack, negative margin techniques, `display: table-cell` for faux columns, and the `inline-block` approach with its whitespace sensitivity.

## Flexbox — One-Dimensional Layout

Flexbox introduced a fundamentally different mental model: instead of positioning individual elements, you describe how a _container_ should distribute space among its _items_.

### The Axis Model

Flexbox operates along two axes:

```
Main Axis (flex-direction)
─────────────────────────────►
│
│  Cross Axis
│  (perpendicular to main)
▼
```

| Property          | Controls                                               |
| ----------------- | ------------------------------------------------------ |
| `flex-direction`  | Which axis is "main" (row, column, and their reverses) |
| `justify-content` | Distribution along the main axis                       |
| `align-items`     | Alignment along the cross axis                         |
| `align-self`      | Per-item override of cross-axis alignment              |

This axis abstraction is powerful because changing `flex-direction` from `row` to `column` rotates the entire layout model — `justify-content` and `align-items` swap their visual effect without changing their semantic meaning.

### Flex Factor Distribution

The `flex` shorthand (`flex-grow`, `flex-shrink`, `flex-basis`) controls how items negotiate for space:

```css
.item {
  flex: 1 1 0;
} /* grow equally, shrink equally, start from zero */
.item {
  flex: 0 0 auto;
} /* don't grow, don't shrink, use content size */
.item {
  flex: 2 1 200px;
} /* grow at 2x rate, shrink normally, start at 200px */
```

The algorithm:

1. Lay out items at their `flex-basis` size
2. Calculate remaining space (or overflow)
3. Distribute remaining space proportionally to `flex-grow` ratios
4. If overflow, shrink proportionally to `flex-shrink` ratios (weighted by basis)

The distinction between `flex-basis: 0` and `flex-basis: auto` is subtle but consequential. With `0`, all space is distributed proportionally — items with `flex: 2` get exactly twice the space of `flex: 1`. With `auto`, content size is accounted for first, and only _remaining_ space is distributed proportionally.

### Where Flexbox Fits and Where It Struggles

Flexbox excels at one-dimensional distribution problems:

- Navigation bars, toolbars, button groups
- Centering (both axes) with minimal effort
- Card rows that distribute space or wrap
- Input groups with fixed-width labels and flexible inputs

Flexbox becomes awkward when the layout is inherently two-dimensional — when you need items in different rows to align to the same column boundaries. Flex items in one row have no knowledge of items in another row, because flexbox models each line independently.

## Grid — Two-Dimensional Layout

CSS Grid introduced track-based thinking — defining a layout in terms of rows and columns as a unified coordinate system.

### The Track Model

Grid defines layout through _tracks_ (rows and columns) that create _cells_, which combine into _areas_:

```css
.container {
  display: grid;
  grid-template-columns: 200px 1fr 1fr; /* 3 column tracks */
  grid-template-rows: auto 1fr auto; /* 3 row tracks */
}
```

The `fr` unit represents a fraction of the available space — similar to flex-grow ratios but operating in two dimensions simultaneously. `1fr 2fr` allocates one-third and two-thirds of remaining space respectively.

### Explicit vs Implicit Grids

The _explicit grid_ is what you define with `grid-template-*` properties. The _implicit grid_ is what the browser creates to accommodate items that fall outside the explicit grid.

```css
.container {
  grid-template-columns: repeat(3, 1fr); /* explicit: 3 columns */
  grid-template-rows: 100px; /* explicit: 1 row */
  grid-auto-rows: minmax(50px, auto); /* implicit rows: at least 50px */
}
```

If you place 9 items in a 3-column grid with 1 explicit row, the browser generates 2 additional implicit rows. The `grid-auto-rows` and `grid-auto-columns` properties control the sizing of these implicit tracks.

### Named Lines and Areas

Grid supports semantic naming that decouples layout definition from placement:

```css
.container {
  grid-template-areas:
    "header  header  header"
    "sidebar content aside"
    "footer  footer  footer";
}

.header {
  grid-area: header;
}
.sidebar {
  grid-area: sidebar;
}
```

This provides a visual map of the layout directly in CSS — the template string mirrors the visual structure. Reorganizing the layout becomes a matter of editing the template rather than recalculating line numbers.

### Subgrid

Subgrid allows nested grids to inherit track definitions from their parent grid, solving the alignment problem that arises when independently-sized grids are nested. Without subgrid, a card grid where each card has a header, body, and footer cannot align all headers across cards — each card's internal grid is independent. Subgrid makes the inner grid participate in the outer grid's track sizing.

### Grid vs Flexbox — Complementary, Not Competing

| Dimension         | Flexbox                                  | Grid                                                        |
| ----------------- | ---------------------------------------- | ----------------------------------------------------------- |
| Layout model      | One-dimensional (row OR column)          | Two-dimensional (rows AND columns)                          |
| Content vs layout | Content determines sizing; layout adapts | Layout defines structure; content fills it                  |
| Sizing approach   | Items negotiate space among themselves   | Tracks (rows/columns) define space, items are placed within |
| Wrapping behavior | New flex lines are independent           | All rows/columns share the same track definitions           |

Both models coexist naturally. Grid handles page-level structure while flexbox handles component-level alignment within grid cells. Choosing between them depends on whether the layout problem is fundamentally one-dimensional or two-dimensional.

## Stacking Contexts and Z-Index

Z-index confusion stems from a common misconception: that z-index creates a single global stacking order. In reality, z-index values are scoped within _stacking contexts_ — independent layering scopes that nest hierarchically.

### The Mental Model

Think of stacking contexts as transparent sheets stacked on a table. Each sheet can contain sub-sheets. An element's global stacking position depends on both its z-index _within its stacking context_ and that context's position in the parent context.

```
Root stacking context
├── Element A (z-index: 1000)           ← Forms its own stacking context
│   └── Child of A (z-index: 999999)    ← Constrained within A's context
├── Element B (z-index: 2)              ← B is above A, so B's children
│   └── Child of B (z-index: 1)           beat A's children regardless
```

Child of B (effective z-index: low) paints above Child of A (z-index: 999999) because B's stacking context sits above A's in the parent context. No z-index value on A's children can escape A's context.

### Stacking Context Triggers

Stacking contexts are created by conditions beyond just `z-index`:

- `z-index` with non-`auto` value on positioned elements
- `opacity` less than 1
- `transform`, `filter`, `backdrop-filter`, `perspective`
- `will-change` with certain values
- `isolation: isolate` (the explicit, purpose-built trigger)
- Flex/grid items with non-`auto` z-index

The `isolation: isolate` property exists specifically to create a stacking context without side effects — analogous to `display: flow-root` for BFC creation.

## Container Queries — Component-Centric Responsiveness

Media queries respond to the _viewport_ — the browser window dimensions. Container queries respond to the dimensions of a _containing element_, enabling components that adapt based on the space available to them rather than the screen size.

### The Conceptual Shift

```css
/* Viewport-centric: same component, different screen sizes */
@media (min-width: 768px) {
  .card {
    flex-direction: row;
  }
}

/* Component-centric: same component, different container sizes */
@container (min-width: 400px) {
  .card {
    flex-direction: row;
  }
}
```

This shift matters because components are reused in contexts that viewport queries cannot distinguish — a card component in a full-width area vs the same card component in a narrow sidebar both see the same viewport width.

### Containment Requirements

Container queries require establishing _containment_ — the browser must be able to determine the container's size independently of its contents:

```css
.card-wrapper {
  container-type: inline-size; /* contain inline dimension for queries */
  container-name: card; /* optional: named container for targeting */
}
```

Containment creates a boundary: the container's size cannot depend on its children's sizes, and container queries allow children to respond to the container's size. This breaks the circular dependency that would otherwise make container queries uncomputable.

## The Cascade — A Conflict Resolution System

The cascade is often taught as a set of rules to memorize. A more productive mental model treats it as a coherent conflict resolution system for when multiple rules try to set the same property on the same element.

### Resolution Order

When multiple declarations compete for the same property, the cascade resolves the conflict through ordered criteria:

| Priority    | Criterion             | Effect                                                     |
| ----------- | --------------------- | ---------------------------------------------------------- |
| 1 (highest) | Origin and importance | User-agent → user → author; `!important` reverses this     |
| 2           | Specificity           | ID > class/attribute/pseudo-class > element/pseudo-element |
| 3           | Scope proximity       | Closer `@scope` rules win (newer addition)                 |
| 4           | Order of appearance   | Later declarations win                                     |

### Specificity as Weight, Not Score

Specificity is often represented as a three-component tuple (ID, class, element). The comparison is lexicographic — 1 ID selector (1,0,0) beats any number of class selectors (0,N,0). This means specificity is not a single number; `(0,11,0)` does not beat `(1,0,0)`.

```css
#header .nav a     → (1, 1, 1)
.nav .nav .nav a   → (0, 3, 1)
/* The first wins — 1 ID beats any number of classes */
```

### Inheritance

Inheritance is separate from the cascade — it provides _default_ values for properties that aren't explicitly set. Some properties inherit by default (color, font properties, text properties), others don't (margin, padding, border, display). The logic: properties that typically should be consistent throughout a text block inherit; properties that define individual box characteristics don't.

### Cascade Layers

`@layer` introduces explicit precedence ordering for groups of styles, independent of specificity. This addresses the escalation pattern where specificity wars lead to increasingly specific selectors or `!important`:

```css
@layer reset, base, components, utilities;

@layer reset {
  * {
    margin: 0;
  }
}
@layer utilities {
  .hidden {
    display: none;
  }
}
```

Layers in later positions override layers in earlier positions, regardless of specificity within each layer. A simple class selector in the `utilities` layer beats an ID selector in the `reset` layer.

## Logical Properties — Writing Mode Independence

Traditional CSS properties like `margin-left`, `padding-top`, and `border-right` assume a left-to-right, top-to-bottom writing system. Logical properties abstract these into flow-relative terms:

| Physical      | Logical (horizontal-tb) | Meaning                    |
| ------------- | ----------------------- | -------------------------- |
| `margin-left` | `margin-inline-start`   | Start of the inline axis   |
| `padding-top` | `padding-block-start`   | Start of the block axis    |
| `width`       | `inline-size`           | Size along the inline axis |
| `height`      | `block-size`            | Size along the block axis  |

In a right-to-left context, `margin-inline-start` maps to `margin-right` instead of `margin-left`. This abstraction matters for internationalization and aligns with how flexbox and grid already think about layout — in terms of main and cross axes rather than absolute directions.

## Intrinsic Sizing Keywords

Beyond fixed units and percentages, CSS offers sizing keywords that express intent:

| Keyword              | Behavior                                                                  |
| -------------------- | ------------------------------------------------------------------------- |
| `min-content`        | The smallest size without overflow — longest word or widest fixed element |
| `max-content`        | The size at which no soft wrapping occurs                                 |
| `fit-content(limit)` | Uses available space up to the limit, then wraps like min-content         |
| `auto`               | Context-dependent; generally content-based with stretch behavior          |

These keywords enable layouts where sizing is expressed as relationships rather than fixed values:

```css
.sidebar {
  width: min-content;
} /* as narrow as content allows */
.dialog {
  width: fit-content(600px);
} /* content-sized, max 600px */
```

## The Evolution Pattern

CSS layout evolution follows a recognizable arc: each system addresses limitations of the previous one while introducing its own trade-offs.

| Era               | Primary Tool         | Strength                     | Core Limitation            |
| ----------------- | -------------------- | ---------------------------- | -------------------------- |
| Flow              | Normal flow          | Document structure           | No spatial layout control  |
| Float             | Floats + clearfix    | Text wrapping                | Not designed for layout    |
| Flexbox           | Flex containers      | One-dimensional distribution | Single-axis only           |
| Grid              | Grid containers      | Two-dimensional placement    | More complex mental model  |
| Container queries | Component containers | Context-aware components     | Requires containment setup |

None of these systems obsoletes the previous ones. Normal flow remains the correct choice for document-like content. Floats still serve their original purpose of wrapping content around embedded elements. The question is which tool matches the nature of the layout problem, not which is "newest."
