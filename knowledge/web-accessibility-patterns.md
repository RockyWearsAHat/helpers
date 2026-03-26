# Web Accessibility Patterns — Focus Management, Keyboard Navigation & ARIA

## Focus Management

Focus represents the single interactive element receiving keyboard input. The browser's default focus indicator (often a blue outline) is semantic — it unambiguously shows what keyboard input will affect. Many developers hide this indicator without replacing it, creating keyboard navigation that appears broken to users.

### Focus Trap and Release

Modal dialogs must trap focus — keyboard Tab cycles within the modal, never reaching background content. Implement by:

1. On modal open: store the previously focused element
2. Trap Tab/Shift+Tab inside modal using event listeners
3. On modal close: restore focus to the previously focused element

This pattern ensures screen reader users and keyboard navigators don't get "lost" navigating into inaccessible background content.

**Focus visible pseudoclass**: Use `:focus-visible` to style focus only when keyboard input triggers it, not mouse clicks. Allows hiding focus on mouse users (reducing visual noise) while preserving it for keyboard users:

```css
button:focus-visible { outline: 2px solid blue; }
```

### Focus restoration

When dynamic content loads or views change, move focus to the new content area. A common pattern: on page navigation within an SPA, announce the page title and set focus to a container or the main heading. Prevents screen reader users from being silently placed at the top of a page they didn't navigate to.

## Keyboard Navigation

Not all keyboard input is Tab-based navigation. Many components (comboboxes, menus, tree views) use arrow keys, Home/End, and other modifiers. Implement these conventions consistently.

### Expected conventions (per ARIA Authoring Practices)

- **Tab/Shift+Tab**: Move between focusable elements
- **Arrow keys (Up/Down/Left/Right)**: Navigate within a menu, listbox, tree, tab group, or slider
- **Enter**: Activate button or confirm input
- **Space**: Toggle checkbox, activate button (alternative to Enter)
- **Escape**: Close popup, cancel dialog
- **Home/End**: Jump to first/last item in list or set slider to min/max

### Roving tabindex pattern

In complex components (toolbars, menus), only one child is in the tab order (tabindex="0"). Arrow keys programmatically move focus between siblings, adjusting tabindex values. User presses Tab once to enter, arrow keys to navigate, Tab again to exit. Reduces keyboard stops and makes large lists navigable.

Example: A toolbar with 12 buttons. Without roving tabindex, Tab passes through all 12. With roving tabindex, Tab enters at the first button, arrow keys move between buttons, Tab exits the toolbar.

## ARIA Landmarks and Semantic HTML

Landmarks allow screen readers to quickly jump between major page sections. HTML5 semantics (nav, main, article, aside) provide landmarks automatically. ARIA role="region" with aria-label creates custom landmarks.

Using semantic HTML is almost always preferable — avoid `<div role="button">` when `<button>` exists. But for legacy or highly custom interfaces, ARIA landmarks fill the gap.

Common landmarks:
- `<nav>` or `role="navigation"`: Site navigation
- `<main>` or `role="main"`: Primary content
- `<aside>` or `role="complementary"`: Sidebar, related links
- `<form>` or `role="search"`: Search bar
- `<section aria-label="...">` or `role="region"`: Custom groupings

## Live Regions

Live regions announce dynamic changes to screen readers without requiring focus movement. Prefix a live container with `aria-live`, and any text updates within trigger an announcement.

- `aria-live="polite"`: Announce after current speech finishes (less intrusive)
- `aria-live="assertive"`: Interrupt current speech (alerts, errors)
- `aria-live="off"`: Default, no announcement

Use cases:
- Form validation errors (assertive)
- Real-time stock price updates (polite)
- Search result counts (polite)

Related attributes:
- `aria-atomic="true"`: Announce the entire region, not just the changed text
- `aria-relevant="additions text"`: Specify what changes trigger announcements

## Modal and Dialog Patterns

Modals must:
1. Have a clear accessible name (heading, aria-label, aria-labelledby)
2. Trap focus (Tab cycles within modal only)
3. Have a clearly labeled close button
4. Announce their presence (role="dialog" or role="alertdialog")

```html
<div role="dialog" aria-labelledby="modal-title" aria-modal="true">
  <h1 id="modal-title">Confirm Delete</h1>
  <p>Are you sure?</p>
  <button>Cancel</button>
  <button>Delete</button>
</div>
```

`aria-modal="true"` signals to assistive tech that content outside is hidden/inert. Many developers implement this visually but forget the ARIA attribute.

## Skip Links

Skip links are hidden anchor links that become visible on focus, allowing keyboard users to jump over repetitive navigation to main content. Typically labeled "Skip to main content":

```html
<a href="#main-content" class="skip-link">Skip to main content</a>
<nav><!-- Large navigation --></nav>
<main id="main-content"><!-- Content --></main>
```

CSS shows the link on focus:

```css
.skip-link { position: absolute; left: -9999px; }
.skip-link:focus { left: 0; }
```

Skip links are low-tech but effective for keyboard navigation efficiency.

## Form Validation and Error Announcements

Inline validation must be clearly associated with form fields:

1. **Use `aria-describedby`** to link error text to the input:
```html
<input id="email" aria-describedby="email-error">
<span id="email-error" role="alert">Invalid email format</span>
```

2. **Mark invalid fields** with `aria-invalid="true"` and `aria-errormessage`:
```html
<input aria-invalid="true" aria-errormessage="email-error">
```

3. **Announce form-wide errors** with `role="alert"` or live regions, not silent visual styling

4. **Don't remove required field indicators** — screen readers need to know which fields are mandatory

## Data Tables

Tables without accessibility markup are unusable by screen readers, which read all cells sequentially without structure.

Use `<th>` headers with `scope="col"` (field headers) and `scope="row"` (row headers):

```html
<thead>
  <tr>
    <th scope="col">Name</th>
    <th scope="col">Role</th>
  </tr>
</thead>
<tbody>
  <tr>
    <td>Alice</td>
    <td>Manager</td>
  </tr>
</tbody>
```

For complex tables, use `id` and `headers` attributes:

```html
<th id="tbl-name">Name</th>
<td headers="tbl-name">Alice</td>
```

Provide a caption above the table describing its purpose and any sortable/interactive features.

## Disclosure (Expand/Collapse) Patterns

Disclosure widgets (accordions, collapsible sections) use `aria-expanded` to signal state:

```html
<button aria-expanded="false" aria-controls="section1">
  Show Details
</button>
<div id="section1" hidden><!-- Content --></div>
```

Related attributes:
- `aria-expanded="true|false"`: Current state
- `aria-controls`: ID of the disclosure content
- `hidden`: CSS-controlled or removed when expanded

On click, toggle both `aria-expanded` and the visibility of the controlled element.

## Carousel (Image Carousel) Patterns

Carousels are notoriously inaccessible. Best practice: provide a static thumbnail list as an accessible alternative, not the carousel itself. If a carousel is required:

1. **Make it keyboard operable**: Previous/Next buttons, arrow key navigation
2. **Don't auto-rotate**: Users expect static content; auto-rotation surprises screen reader users
3. **Use ARIA roles and labels**: `role="region" aria-label="Product images" aria-live="polite"`
4. **Announce slide count**: "Slide 3 of 5"
5. **Stop animation on focus**: Users shouldn't lose their place while navigating

Many design patterns work better without carousels — consider tabs, lists, or separate pages.

## Tooltip Patterns

Tooltips must be keyboard-accessible — not shown on hover alone, which excludes keyboard users. Patterns:

- **Dismiss on Escape**: Pressing Escape closes the tooltip
- **Visible on focus**: Tooltip appears when the target element receives focus (not just on hover)
- **aria-describedby or aria-label**: Associate the tooltip content with the target element

```html
<button aria-describedby="tooltip">Save</button>
<div id="tooltip" role="tooltip" hidden>
  Save this document
</div>
```

Avoid tooltips for essential information — they're often missed or hidden on mobile.

## See also

- [design-accessibility-patterns.md](design-accessibility-patterns.md) — WCAG principles and component implementation
- [accessibility-engineering.md](accessibility-engineering.md) — WCAG 2.2, ARIA, screen readers, legal compliance
- [testing-accessibility.md](testing-accessibility.md) — Automated and manual accessibility testing strategies