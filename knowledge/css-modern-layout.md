# CSS Modern Layout — Subgrid, Nesting, :has(), and Beyond

Recent CSS additions enable grid-aware nesting, relational selectors, and advanced UI primitives. These features shift CSS from declarative property lists toward compositional, context-aware layout and styling.

## Subgrid: Grid-Aware Nesting

Subgrid connects nested grid items to their parent grid, enabling aligned multi-level layouts.

### Traditional Grid Nesting

Nested grids are isolated:
```css
.parent-grid {
  display: grid;
  grid-template-columns: 1fr 2fr 1fr;
}

.card {
  display: grid;
  grid-template-columns: 1fr 1fr; /* Independent grid */
}
```

The card's 2-column grid doesn't align with the parent's 3-column grid, causing misalignment.

### Subgrid Solution

```css
.parent-grid {
  display: grid;
  grid-template-columns: 1fr 2fr 1fr;
}

.card {
  display: grid;
  grid-template-columns: subgrid; /* Inherit parent's columns */
}

.card-item {
  grid-column: span 1; /* Aligns to parent's columns */
}
```

Now `.card-item`s align to the parent grid's 3-column structure, enabling consistent alignment across multiple nested grids.

### Practical Example: Design System Grid

```css
.page-layout {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  gap: 1rem;
}

.sidebar {
  display: grid;
  grid-template-columns: subgrid;
  grid-column: span 3;
}

.sidebar-card {
  grid-column: span 12; /* Spans all 12 parent columns within sidebar */
}
```

**When to use subgrid**: Multi-level layouts where inner grids must align to outer grid structure. Enables cohesive design systems where internal layouts inherit the design grid.

### Subgrid for Rows

```css
.parent {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  grid-template-rows: auto 1fr auto;
}

.item {
  display: grid;
  grid-template-columns: subgrid;
  grid-template-rows: subgrid;
}
```

Align both rows and columns to the parent grid.

## CSS Nesting: Style Composition

CSS nesting allows selectors to nest within parent rules, reducing repetition and improving readability.

### Basic Nesting

```css
.card {
  padding: 1rem;
  background: white;

  .card-header {
    font-weight: bold;
  }

  .card-body {
    margin-top: 0.5rem;
  }
}
```

Expands to:
```css
.card { padding: 1rem; background: white; }
.card .card-header { font-weight: bold; }
.card .card-body { margin-top: 0.5rem; }
```

### Pseudo-Classes and Pseudo-Elements

```css
.button {
  padding: 0.5rem;

  &:hover {
    background-color: #007bff;
  }

  &:focus {
    outline: 2px solid #007bff;
  }
}
```

The `&` refers to the parent selector.

### Using & in Complex Selectors

```css
.card {
  padding: 1rem;

  & > .header {
    font-size: 1.5rem;
  }

  &.elevated {
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
  }

  @media (max-width: 768px) {
    & {
      padding: 0.5rem;
    }
  }
}
```

### Nesting Complex Selectors

```css
.form {
  display: flex;

  & input,
  & textarea {
    padding: 0.5rem;
  }

  & input[type="submit"] {
    background-color: blue;
  }
}
```

**When to nest**: Keep nesting 2–3 levels deep. Deep nesting reduces readability despite reducing repetition.

## `:has()` Selector: Relational Selection

The `:has()` pseudo-class selects elements that contain specific descendants or siblings (parent selector).

### Basic Parent Selection

```css
.card:has(> .warning) {
  border-color: orange;
}
```

"Select `.card` if it contains a direct child with class `.warning`."

### Sibling Selection

```css
input:has(+ .error-message) {
  border-color: red;
}
```

"Select `input` if it's followed by an element with class `.error-message`."

### Conditional Component Styling

```css
.form-group:has(input:invalid) {
  background-color: #ffe0e0;
}

.card:has(img) {
  grid-template-columns: auto 1fr; /* Adjust layout if image present */
}

ul:has(> li > input[type="checkbox"]:checked) {
  background: #f0f0f0;
}
```

**Power of `:has()`**: Enables state-driven styling without modifying HTML. Replaces JS-based state classes in many cases.

### Complex `:has()` Queries

```css
.container:has(> .error, > .warning) {
  border-left: 4px solid orange;
}

article:has(img) {
  display: grid;
  grid-template-columns: 200px 1fr;
}
```

**Caveat**: `:has()` is computationally expensive for high-frequency selectors (e.g., `:has(*)`). Use specific selectors.

## Individual Transform Properties

Instead of the combined `transform` property, separate properties enable partial updates:

```css
.element {
  translate: 10px 20px;
  rotate: 45deg;
  scale: 1.5;
}
```

Equivalent to:
```css
.element {
  transform: translate(10px, 20px) rotate(45deg) scale(1.5);
}
```

### Practical Benefits

1. **Easier animation**: Animate only `rotate` without resetting `translate`
2. **CSS overrides**: Media queries can update one property
3. **Readability**: Intent is clearer

```css
.element {
  translate: 10px 20px;
  rotate: 0deg;
  transition: rotate 0.3s;
}

.element:hover {
  rotate: 45deg;
}
```

## Scroll-Driven Animations (CSS Scroll Animations)

Animations triggered by scroll position (experimental):

```css
@supports (animation-timeline: view()) {
  .fade-in {
    animation: fadeIn 1s linear;
    animation-timeline: view();
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
}
```

Elements fade in as they scroll into view, without JavaScript intersection observers.

## View Transitions API

Coordinate CSS transitions across page navigations (for SPAs and single-document transitions):

```css
::view-transition-old(root) {
  animation: slideOut 0.3s;
}

::view-transition-new(root) {
  animation: slideIn 0.3s;
}

@keyframes slideOut {
  from { transform: translateX(0); }
  to { transform: translateX(100%); }
}

@keyframes slideIn {
  from { transform: translateX(-100%); }
  to { transform: translateX(0); }
}
```

JavaScript initiates: `document.startViewTransition(() => updateDOM())`.

## Dialog and Popover Enhancements

### Dialog Styling

```css
dialog {
  padding: 2rem;
  border: none;
  border-radius: 8px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
}

dialog::backdrop {
  background: rgba(0, 0, 0, 0.5);
}
```

### Popover API Styling

```css
[popover] {
  position: absolute;
  padding: 1rem;
  background: white;
  border: 1px solid #ccc;
}

[popover]:popover-open {
  /* Applied when popover is shown */
}
```

## Anchor Positioning (CSS Anchor Position)

Position elements relative to **anchors** rather than fixed coordinates (emerging):

```css
.tooltip {
  position: absolute;
  anchor-default: --anchor;
  top: anchor(bottom);
  left: anchor(left);
  margin-top: 0.5rem;
}
```

Tooltips automatically position relative to their anchor element, useful for dynamic UI.

## @scope: Style Scope Isolation

Limit selector scope to a specific subtree (emerging):

```css
@scope (.card) to (.card .footer) {
  .heading { font-size: 1.5rem; }
}
```

`.heading` inside `.card` (but not inside `.card .footer`) gets the style. Reduces naming conflicts without BEM.

## Browser Support Summary

| Feature | Chrome | Firefox | Safari |
|---------|--------|---------|--------|
| Subgrid | 90+ | 71+ | 16+ |
| CSS Nesting | 120+ | 117+ | 17.5+ |
| `:has()` | 105+ | 121+ | 15.4+ |
| Individual Transforms | 104+ | 72+ | 14.1+ |
| Scroll Animations | 115+ | Not supported | 18+ |
| View Transitions | 111+ | Not supported | 18+ |
| Popover API | 114+ | 125+ | 17.6+ |

## See Also

- [web-css-layout.md](web-css-layout.md) — CSS Grid, Flexbox, layout fundamentals
- [design-responsive.md](design-responsive.md) — Media queries, container queries, responsive patterns
- [web-animation.md](web-animation.md) — Keyframes, transitions, performance
- [web-web-components.md](web-web-components.md) — Scoped styles, encapsulation