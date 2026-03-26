# CSS Architecture — BEM, CSS Modules, CSS-in-JS, Utility-First & Modern Selectors

CSS architecture is the practice of organizing stylesheets so they scale with application size. Problems emerge early: style conflicts, naming collisions, difficulty tracking which CSS is used, maintainability across teams.

## The Core Problems CSS Architecture Solves

1. **Naming conflicts** — Two developers create `.button` in different files; one overwrites the other
2. **Over-specificity** — Selectors become increasingly complex (`div.container > section.main article.post .title { }`)
3. **Dead code** — CSS remains in production long after it's unused; it's hard to delete
4. **Single Responsibility Principle** — CSS files mix layout, typography, colors, interactions; changes ripple unexpectedly
5. **Coupling** — Styles tightly coupled to HTML structure; refactoring breaks styling

## BEM: Block Element Modifier

BEM is a **naming convention** that addresses conflicts through explicit, lengthy names.

### The Three-Level Hierarchy

- **Block** — A standalone, reusable component (`.card`, `.button`, `.navigation`)
- **Element** — A part of a block, semantically tied to it (`.card__title`, `.button__icon`, `.navigation__item`)
- **Modifier** — A variant or state (`.button--primary`, `.card--featured`, `.navigation__item--active`)

### Example

```css
/* Block */
.button { }

/* Elements (always belong to a block) */
.button__text { }
.button__icon { }

/* Modifiers (variants) */
.button--primary { background: blue; }
.button--secondary { background: gray; }
.button--disabled { opacity: 0.5; }
.button__icon--left { margin-right: 8px; }

/* State (a type of modifier) */
.button--loading { animation: spin 1s linear infinite; }
```

### HTML

```html
<button class="button button--primary">
  <span class="button__icon button__icon--left">→</span>
  <span class="button__text">Next</span>
</button>
```

### Strengths

- **No specificity wars** — Everything is one class, specificity is flat
- **Explicit relationships** — The name encodes the structure
- **Reusable** — Blocks can be moved, copied, renamed without being bound to markup structure

### Weaknesses

- **Verbose** — Class names are long (`is-loading` vs `button--loading` vs `btn-loading`)
- **Fragile** — Names are conventions, not enforced; a developer can ignore the pattern
- **Markup pollution** — Classes leak HTML abstraction; every element needs a class
- **Doesn't scale well with composition** — A block that uses another block creates ambiguity

## CSS Modules: Scoped Styling via Tooling

**CSS Modules** lever bundlers to scope CSS to a file. Each module's selectors are automatically renamed to prevent collisions, and the CSS is imported as JavaScript.

### How It Works

`Button.module.css`:
```css
.button {
  background: blue;
  padding: 8px 16px;
  border: none;
}

.primary {
  background: darkblue;
}
```

`Button.jsx`:
```javascript
import styles from './Button.module.css';

export function Button({ variant = 'default' }) {
  return (
    <button className={`${styles.button} ${styles[variant]}`}>
      Click me
    </button>
  );
}
```

At build time, the bundler:
1. Scopes `.button` to a unique name (e.g., `Button__button__a1b2c`)
2. Exports `styles` as `{ button: 'Button__button__a1b2c', primary: 'Button__primary__d3e4f' }`
3. Rewrites the CSS with the scoped names

Result: **CSS is locally scoped by default**. No naming collisions, no cascading conflicts.

### Strengths

- **Automatic scoping** — No naming discipline required
- **Dead code detection** — Unused CSS modules are easily removed (unused imports)
- **Composition** — Combine multiple modules; class names are explicitly imported

### Weaknesses

- **No shared styles easy** — Common utilities require careful module organization
- **Object syntax in JSX** — `className={styles.button}` is verbose; `className="button"` can't work
- **Limited cross-file reuse** — Sharing component styles requires composition patterns
- **Bundler dependency** — Only works if bundler supports CSS Modules

## CSS-in-JS: Styles as Component Code

**CSS-in-JS** libraries (styled-components, Emotion) co-locate styles with components. Styles are JavaScript objects or template literals.

### styled-components (Template Literal Syntax)

```javascript
import styled from 'styled-components';

const StyledButton = styled.button`
  background: blue;
  padding: 8px 16px;
  border: none;
  cursor: pointer;

  &:hover {
    background: darkblue;
  }

  ${props => props.primary && css`
    background: darkred;
    color: white;
  `}
`;

export function Button({ primary }) {
  return <StyledButton primary={primary}>Click me</StyledButton>;
}
```

### Emotion (Object Syntax)

```javascript
import { css } from '@emotion/react';
import styled from '@emotion/styled';

const buttonStyle = css({
  background: 'blue',
  padding: '8px 16px',
  border: 'none',
  '&:hover': { background: 'darkblue' }
});

const StyledButton = styled.button(props => ({
  ...buttonStyle,
  ...(props.primary && { background: 'darkred', color: 'white' })
}));
```

### Strengths

- **Colocation** — Styles live with components; no CSS file to hunt for
- **Dynamic styles** — Props directly influence styling without class-name mangling
- **Tree-shaking** — Unused component styles can be removed by bundlers
- **Type-safety** — Theme values are TypeScript types
- **Automatic vendor prefixes** — Polished() or built-in support

### Weaknesses

- **Runtime overhead** — Styles are computed and injected at runtime; slower than pre-computed CSS
- **Bundle bloat** — CSS library code runs in browsers
- **Debugging difficulty** — Styles are dynamically generated; DevTools shows generated class names (`.emotion_1a2b3c`)
- **No CSS cascade** — Lost organizational structure of CSS files
- **SEO limitations** — Some crawlers may struggle with dynamically injected styles

## Utility-First (Tailwind CSS): Pre-Defined Atomic Styles

**Utility-first CSS** provides a huge library of single-purpose classes (`.mt-4`, `.flex`, `.bg-blue-500`). Rather than writing styles, you compose them.

### Example

```html
<button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
  Click me
</button>
```

Each class does one thing: `.bg-blue-500` sets `background: rgb(59, 130, 246)`.

### Tailwind Configuration

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    colors: {
      blue: {
        500: '#3B82F6',
        600: '#2563EB'
      }
    },
    spacing: {
      4: '1rem',
      8: '2rem'
    }
  }
};
```

### Strengths

- **No naming** — No need to invent class names; utilities are pre-defined
- **Consistency** — Values come from a theme (all spacings use the same scale)
- **Small final output** — Only used utilities are included in the build
- **Fast iteration** — Change markup to adjust styles; no CSS file editing
- **Discoverability** — Editor autocomplete shows all available utilities

### Weaknesses

- **Markup pollution** — Class names become unwieldy (`class="flex items-center justify-between p-4 rounded-lg shadow-md bg-gray-50 hover:bg-gray-100 transition-colors duration-200"`)
- **Difficult refactoring** — Extracting repeated patterns requires `@apply` or component abstraction
- **Reduced portability** — HTML is tightly coupled to Tailwind's class names
- **Learning curve** — Must learn the entire utility library
- **Framework friction** — Works best with component systems; standalone HTML is verbose

### Extracting Patterns in Tailwind

Rather than inline classes, use components:
```javascript
function Button() {
  return (
    <button className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2">
      Submit
    </button>
  );
}
```

Or use Tailwind's `@apply`:
```css
@layer components {
  .btn-primary {
    @apply bg-blue-500 hover:bg-blue-600 text-white px-4 py-2;
  }
}
```

## Modern CSS Features: Reducing Architecture Complexity

Recent CSS additions reduce the need for external architecture systems.

### Cascade Layers (`@layer`)

Organize cascade priority explicitly:
```css
@layer reset, base, theme, utilities;

@layer  reset {
  * { margin: 0; padding: 0; }
}

@layer theme {
  .button { background: blue; }
}

@layer utilities {
  .m-0 { margin: 0; }
  .p-4 { padding: 16px; }
}
```

Layers define specificity priority independently of selector complexity. Utilities layer always wins over theme layer, regardless of selector specificity.

### CSS Nesting

Write nested selectors without preprocessing:
```css
.button {
  background: blue;
  padding: 8px 16px;

  &:hover {
    background: darkblue;
  }

  &--primary {
    background: darkred;
  }

  .icon {
    margin-right: 8px;
  }
}
```

Compiles to:
```css
.button { background: blue; padding: 8px 16px; }
.button:hover { background: darkblue; }
.button--primary { background: darkred; }
.button .icon { margin-right: 8px; }
```

### Container Queries (`@container`)

Style elements based on their container size, not viewport:
```css
@container (min-width: 400px) {
  .card { display: grid; grid-template-columns: 1fr 1fr; }
}
```

In markup:
```html
<div style="container-type: inline-size;">
  <div class="card">...</div>
</div>
```

If the container is >= 400px wide, the card uses the grid layout. This enables truly responsive components that adapt to their context, not just viewport size.

### `:has()` Selector (Conditional Styling)

Style parent/sibling elements based on children:
```css
/* Style form if it contains an invalid input */
form:has(input:invalid) {
  border: 2px solid red;
}

/* Style list if it has a focused item */
.list:has(.item:focus) {
  background: #f0f0f0;
}
```

Eliminates JS-based state management for styling in many cases.

### Subgrid

Align grid items across nested grids:
```css
.parent { display: grid; grid-template-columns: 1fr 1fr; }
.child { display: grid; grid-template-columns: subgrid; }
```

Child grid uses parent's column definitions, enabling pixel-perfect alignment across nested components.

## Choosing an Architecture

| Approach       | Best For                              | Philosophy                       | Trade-Off                    |
|----------------|---------------------------------------|----------------------------------|------------------------------|
| BEM            | Large teams, mixed languages          | Naming discipline                | Verbose, convention-based   |
| CSS Modules    | Component-based apps (React, Vue)    | Scoped by default                | Requires bundler, verbose JSX |
| CSS-in-JS      | Dynamic theming, runtime styling     | Co-locate styles with components | Runtime overhead, debugging  |
| Utility-First  | Rapid development, consistency       | Predefined atomic utilities      | Markup pollution             |
| Modern CSS     | New projects with modern browsers    | Cascade layers + nesting + :has  | Limited browser support      |

**Practical guidance:**
- New projects: **Utility-first (Tailwind)** or **CSS Modules** + TypeScript
- Large teams: **BEM** (low friction, no tool dependencies) or **CSS Modules**
- Complex theming: **CSS-in-JS** (Emotion, styled-components)
- High performance: **BEM** or **CSS Modules** (no runtime overhead)
- Rapid prototyping: **Utility-first**

Most projects benefit from **layering**: use utilities for spacing/sizing, CSS Modules for components, CSS-in-JS for dynamic behavior.