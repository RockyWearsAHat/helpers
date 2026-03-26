# Web Components — Custom Elements, Shadow DOM & Encapsulation

## Overview

Web Components are a collection of web platform specifications enabling the creation of reusable, encapsulated custom HTML elements. They consist of four core APIs:

1. **Custom Elements**: Define new HTML tag names with custom behavior
2. **Shadow DOM**: Encapsulate styling, markup and behavior
3. **HTML Templates**: Define inert markup templates to be cloned
4. **HTML Imports** (deprecated): Were intended for component packaging; now modules handle this

The philosophy is component encapsulation: a component owns its styles, DOM structure, and behavior without polluting the page's global scope.

## Custom Elements: Lifecycle and Behavior

### Defining a Custom Element

Custom elements inherit from `HTMLElement` or specialized subclasses like `HTMLButtonElement`:

```javascript
class MyButton extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.shadowRoot.innerHTML = `
      <style>button { color: blue; }</style>
      <button><slot></slot></button>
    `;
  }
}

customElements.define('my-button', MyButton);
```

Usage in HTML:
```html
<my-button>Click me</my-button>
```

### Lifecycle Callbacks

A custom element fires callbacks at specific moments:

| Callback | Fires | Use Case |
|----------|-------|----------|
| `constructor()` | Element created (not yet in DOM) | Initialize properties, avoid layout reads |
| `connectedCallback()` | Element inserted into DOM | Setup Shadow DOM, event listeners, fetch data |
| `disconnectedCallback()` | Element removed from DOM | Cleanup listeners, timers, subscriptions |
| `attributeChangedCallback(name, oldVal, newVal)` | Watched attribute changes | React to `<my-el color="red">` → `color="blue"` |
| `adoptedCallback()` | Element moved to new document | Rare; triggered by `document.adoptNode()` |

**Key semantic**: `attributeChangedCallback` only fires for attributes listed in `observedAttributes`:

```javascript
static get observedAttributes() {
  return ['color', 'size'];
}

attributeChangedCallback(name, oldVal, newVal) {
  if (name === 'color') {
    this.style.color = newVal;
  }
}
```

Mistake: changing a property doesn't trigger `attributeChangedCallback`. Only attribute mutation does. Conflating properties and attributes is the first confusion point.

### Properties vs. Attributes

HTML attributes are strings on the element tag. JavaScript properties are values on the object.

```html
<my-input disabled=""></my-input>
```

```javascript
// Attribute: string or absent
element.getAttribute('disabled');  // "" or null

// Property: typically boolean, synced to attribute
element.disabled = true;  // Sets attribute
element.disabled;  // true
```

Best practice: map attributes to properties and keep them in sync:

```javascript
get color() {
  return this.getAttribute('color') || 'black';
}

set color(value) {
  this.setAttribute('color', value);
}
```

## Shadow DOM: Encapsulation and Styling

### Shadow DOM Basics

Attaching a shadow root to an element creates a private DOM subtree:

```javascript
const shadow = element.attachShadow({ mode: 'open' });
shadow.innerHTML = '<p>Shadow DOM content</p>';
```

The `mode` is critical:

| Mode | Access | Use Case |
|------|--------|----------|
| `'open'` | `element.shadowRoot` accessible from outside | Component wants external introspection |
| `'closed'` | Only internal code accesses shadow root | Truly isolated; light DOM can't peer in |

The light DOM and shadow DOM are separate trees. Querying from outside finds light DOM only. Querying from inside finds shadow DOM only.

### Styling Isolation

Styles in shadow DOM do not leak out. Styles outside do not leak in (with exceptions):

```javascript
// Inside shadow root:
<style>
  p { color: red; }  // Only affects shadow <p>, not light DOM
  ::slotted(*) { margin: 0; }  // Styles light DOM content in slots
</style>

// Outside:
<style>
  p { color: blue; }  /* Light DOM <p> only; shadow <p> unaffected */
</style>
```

**CSS custom properties** pierce the boundary by design:

```html
<style>
  my-component {
    --component-color: blue;
  }
</style>

<!-- Inside shadow DOM -->
<style>
  p { color: var(--component-color); }  /* reads blue */
</style>
```

Inherited properties (color, font-size, line-height) also pierce to shadow DOM unless explicitly overridden.

### Slots and Distribution

Slots define where light DOM content appears in the shadow tree:

```html
<!-- Shadow templates -->
<div class="header">
  <slot name="title">Default title</slot>
</div>

<!-- Light DOM usage -->
<my-card>
  <h1 slot="title">My Title</h1>
  <p>Body content goes to unnamed slot</p>
</my-card>
```

Key points:
- Unnamed `<slot>` catches all content not assigned to named slots
- Slotted elements remain in light DOM; they're not moved
- CSS can target `.header p` from shadow styles, but `p` in `::slotted(p)` targets only assigned content
- Overflowing slots (more content than slots) are silently ignored

### Declarative Shadow DOM

Typically shadow DOM is created imperatively in `connectedCallback()`. Declarative Shadow DOM (new spec) embeds it in HTML:

```html
<my-element>
  <template shadowroot="open">
    <style>p { color: red; }</style>
    <p>Styled content</p>
  </template>

  <p>Light DOM fallback (semantic HTML)</p>
</my-element>
```

Advantage: shadow DOM is available before JavaScript runs, improving performance and accessibility. Disadvantage: unclear browser support (still shipping).

## Framework Integration (Lit, Stencil, React Interop)

### Lit: Lightweight Web Components

Lit simplifies reactive rendering and template syntax:

```javascript
import { LitElement, html, css } from 'lit';

class MyGreeting extends LitElement {
  static properties = { name: { type: String } };
  static styles = css`
    p { color: blue; }
  `;

  render() {
    return html`<p>Hello, ${this.name}!</p>`;
  }
}

customElements.define('my-greeting', MyGreeting);
```

Lit compiles template literals to efficient DOM updates. Reactivity is automatic.

### Stencil: Build Toolchain

Stencil compiles TypeScript to web components with generation of framework bindings:

```typescript
@Component({
  tag: 'my-component',
  styleUrl: 'my-component.css',
})
export class MyComponent {
  @Prop() name: string;
  @State() count: number = 0;

  render() {
    return <div>Hello, {this.name}!</div>;
  }
}
```

Stencil generates React, Vue, Angular wrappers automatically, easing interop.

## Accessibility Challenges

Web Components don't automatically inherit semantic HTML benefits:

- **ARIA**: Custom elements may need explicit `role`, `aria-label`, `aria-describedby` attributes
- **Keyboard navigation**: Shadow DOM content doesn't participate in tab order unless explicitly managed
- **Focus**: `focus()` on custom element works; focus management inside shadow DOM is invisible to assistive tech by default
- **Form association**: Custom form elements need `ElementInternals` API for proper form participation (still experimental)

```javascript
class MyCheckbox extends HTMLElement {
  constructor() {
    super();
    this.internals = this.attachInternals();
  }

  connectedCallback() {
    this.addEventListener('click', () => {
      this.checked = !this.checked;
      this.internals.setFormValue(this.checked ? 'on' : '');
    });
  }
}
```

Best practice: start with semantic HTML templates in shadow DOM, layer custom behavior on top.

## Interoperability with Frontend Frameworks

### React Interop

React treats custom elements as unknown, forwarding all attributes as strings:

```jsx
<my-button disabled>Click</my-button>  // Works
// React passes disabled as string, not boolean property
```

Workaround: wrap custom element or use `dangerouslySetInnerHTML`:

```jsx
<my-button ref={el => el && (el.disabled = true)}>Click</my-button>
```

Vue and Angular handle custom elements more naturally; they preserve properties.

### Framework Slots Conflict

Frameworks have virtual slot syntaxes that can collide with Web Components slots:

```jsx
// React: children prop
function MyComponent({ children }) {
  return <div>{children}</div>;
}

// vs. Web Component slot
<my-component>
  <span slot="title">Title</span>
</my-component>
```

Framework-generated wrappers handle this translation.

## Common Use Cases and Limitations

### Where Web Components Excel

- **Design systems**: Encapsulated component libraries shipped across teams
- **Lazy-loaded microfrontends**: Custom elements with deferred script loading
- **Framework-agnostic plugins**: Embed in any app (WordPress, Vue, React simultaneously)
- **Third-party widgets**: Ad networks, analytics using custom elements

### Limitations

- **Slow adoption in frameworks**: React still discouraged; more natural in Vue/Angular
- **Tooling immaturity**: Bundling, lazy-loading, SSR less straightforward than framework components
- **Testing complexity**: Shadow DOM hidden from queries; special test utilities needed
- **Serialization**: Custom elements don't automatically serialize to HTML string for SSR

Web Components excel at component isolation and framework independence; they're less ideal for framework-integrated applications where virtual DOM synchronization is needed.