# Accessible Design Patterns — WCAG, Component Implementation & User Flows

## WCAG and the POUR Principles

The Web Content Accessibility Guidelines (WCAG) organize accessibility requirements around four principles — POUR — that govern how accessible patterns are built. Understanding these principles clarifies why specific patterns matter and when exceptions are questionable.

### Perceivable

Information must be presentable to users in ways they can perceive. Not everyone perceives the same way; the requirement is that at least one perceivable pathway exists for every piece of information.

Perceivable considerations for patterns:

- Text alternatives for non-text content (images, icons, SVGs)
- Captions and transcripts for multimedia
- Color must not be the only way to convey meaning (use pattern, text, or icons alongside)
- Sufficient contrast between foreground and background (WCAG AA: 4.5:1 for body text, 3:1 for large text)

### Operable

Users must be able to interact with interface components and navigate using their input method of choice. An interface that works only with a mouse excludes keyboard users, voice control users, switch users, and others.

Operable patterns require:

- All functionality reachable from keyboard (not mouse-only)
- Adequate time to read and interact (no automatic page refreshes, time limits disclosed)
- Predictable behavior across interactions and navigation
- Navigation mechanisms (skip links, landmarks, headings) for finding content
- No content that causes seizures or physical reactions (no flashing >3Hz)

### Understandable

Information must be understandable. Legible text at appropriate reading level, consistent patterns, and clear error messages — all reduce cognitive load and support users with language, learning, or attention differences.

Understandable patterns:

- Text is readable and clear
- Navigation appears consistently across pages
- Components behave predictably (form submission doesn't randomly relocate focus)
- Forms identify required fields, provide clear error messages, and suggest corrections

### Robust

Content must work across current and future user agents, including assistive technologies. Robust implementation means valid HTML, proper semantic markup, and programmatic access to names, roles, and values.

## Focus Management and Keyboard Navigation

Keyboard navigation is foundational. Not every user can or wants to use a mouse; some use keyboards exclusively, others use switches or voice control that navigates via tab order.

### Focus Visibility

Every interactive element must have a visible focus indicator. Default browser outlines (`:focus`) are often styled away; replacements must be:

- Visible (sufficient contrast, not obscured)
- Clear (shape and size obvious)
- Not removed from view (don't hide focused elements behind fixed headers)

The `:focus-visible` pseudo-class distinguishes keyboard focus from mouse clicks, allowing different styling:

```css
button:focus {
  outline: 2px solid transparent; /* Removes default */
}
button:focus-visible {
  outline: 3px solid #0066CC; /* Visible for keyboard only */
}
```

### Tab Order and Logical Sequence

Tabbing through a page should follow logical order (normally top-to-bottom, left-to-right). The HTML source order determines tab order; inline `tabindex="1", "2", etc.` is an anti-pattern because it's fragile and hard to maintain.

Use `tabindex="-1"` to remove elements from tab order (e.g., decorative buttons) or to make off-screen content focusable without including it in natural flow. `tabindex="0"` makes non-interactive elements interactive (use for custom components if necessary).

### Skip Links

A skip link is a hidden anchor placed early in the page (often as the first focusable element) that jumps to main content, skipping repetitive navigation. It's typically hidden but visible on focus `:focus`.

Usage:

```html
<a href="#main-content" class="skip-link">Skip to main content</a>
<!-- Navigation, sidebars, etc. -->
<main id="main-content">
  <!-- Page content -->
</main>
```

Skip links drastically improve navigation efficiency for keyboard users and screen reader users who don't want to hear site navigation repeated on every page.

## Modal Dialogs and Focus Trapping

Modals interrupt workflow and must manage focus carefully to prevent users from accidentally interacting with background content.

### Focus Trap Implementation

When a modal opens:

1. Save the element that had focus before the modal opened
2. Move focus to the modal or to the first focusable element inside it
3. Trap focus within the modal (when Tab reaches the last focusable element, cycle back to the first)
4. When the modal closes, restore focus to the previously focused element

```javascript
const focusableElements = modal.querySelectorAll(
  'button, [href], input, textarea, select, [tabindex]:not([tabindex="-1"])'
);
const firstElement = focusableElements[0];
const lastElement = focusableElements[focusableElements.length - 1];

modal.addEventListener('keydown', (e) => {
  if (e.key !== 'Tab') return;
  if (e.shiftKey && document.activeElement === firstElement) {
    e.preventDefault();
    lastElement.focus();
  } else if (!e.shiftKey && document.activeElement === lastElement) {
    e.preventDefault();
    firstElement.focus();
  }
});
```

### Semantics and ARIA

Modals should use `role="dialog"` and have an accessible label:

```html
<div role="dialog" aria-labelledby="dialog-title" aria-modal="true">
  <h2 id="dialog-title">Confirm Action</h2>
  <p>Are you sure?</p>
  <button>Cancel</button>
  <button>Confirm</button>
</div>
```

The `aria-modal="true"` attribute signals to screen readers that background content is inert (not currently available).

## Combobox and Autocomplete

A combobox (select with autocomplete or free-form input) requires careful coordination of keyboard and mouse interaction, focus management, and screen reader announcements.

### Implementation Checkpoints

- **Double interaction**: Users can click the trigger to open a suggestions list or type to filter
- **Keyboard navigation**: Arrow keys move through suggestions; Enter selects; Escape closes
- **Dynamic announcements**: Screen readers announce the number of suggestions and the current selection
- **Filtering logic**: Filter as you type or navigate with arrows (different UX)
- **Selection handling**: Selected value updates both input and combobox state

```html
<div class="combobox">
  <input 
    type="text"
    role="combobox"
    aria-expanded="false"
    aria-autocomplete="list"
    aria-controls="suggestions"
  />
  <ul id="suggestions" role="listbox" hidden>
    <li role="option" aria-selected="false">Suggestion 1</li>
    <li role="option" aria-selected="false">Suggestion 2</li>
  </ul>
</div>
```

### Screen Reader Announcements

When suggestions filter: `aria-live="polite"` on the listbox announces the count update without interrupting. When selection changes, update `aria-selected` and move `aria-selected="true"` to the current option.

## Data Tables

Tables are notoriously difficult to make accessible because their structure is both visual (rows/columns) and semantic (header/data relationships).

### Header Structure

Data tables must have `<th>` headers with explicit scope:

```html
<table>
  <thead>
    <tr>
      <th scope="col">Name</th>
      <th scope="col">Email</th>
      <th scope="col">Role</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Alice</td>
      <td>alice@example.com</td>
      <td>Admin</td>
    </tr>
  </tbody>
</table>
```

`scope="col"` tells assistive tech that this header applies to all cells below it. `scope="row"` for row headers. Nested tables or complex multi-level headers require `<colgroup>` and explicit associations.

### Accessibility Issues

- **No captions or summaries**: Add `<caption>` or `aria-label` to describe table purpose
- **No row headers**: In tables without row headers, screen readers can't anchor cell meaning (hard to know which row a value belongs to)
- **Sorting and pagination**: If sorting is available, announce which column is sorted and in what direction (`aria-sort="ascending"`). If paginating, provide context about total rows and current page
- **Responsive tables**: Stacked mobile layouts (one row per column) need careful restructuring to maintain semantic meaning

## Form Validation and Error Recovery

Forms must clearly associate labels with inputs, indicate required fields, and provide helpful error messages.

### Label Association

```html
<label for="email">Email</label>
<input type="email" id="email" required aria-required="true" />
```

The `for` attribute links the label to the input by ID. `aria-required="true"` is redundant with HTML5 `required`, but reinforces it for older assistive technology.

### Required Field Indication

Mark required fields both visually and semantically:

```html
<label for="email">Email <span aria-label="required">*</span></label>
<input type="email" id="email" required />
```

The `*` is perceived by sighted users; `aria-label` makes it available to screen reader users.

### Error Messages

Connect error messages to inputs with `aria-describedby`:

```html
<input type="email" id="email" aria-describedby="email-error" />
<div id="email-error" role="alert">
  Invalid email format. Please use example@domain.com
</div>
```

`role="alert"` causes screen readers to announce the error immediately (not wait for focus change). Keep error text concise and actionable.

### Error Recovery

- Submit form does not auto-scroll away from the error
- Input retains focus at error (so user can immediately edit)
- Error message disappears when the input becomes valid (not just when user focuses on it)

## Motion Sensitivity and Reduced Motion

The `prefers-reduced-motion` media query detects users who have requested reduced animation in OS settings (important for vestibular disorders and motion sensitivity).

```css
/* Default: animated transition */
.slide {
  transition: transform 0.3s ease-out;
}

/* Respected: instant change */
@media (prefers-reduced-motion: prefer-reduced) {
  .slide {
    transition: none;
  }
}
```

Apply this to all animations: transitions, transforms, infinite animations. Reducing motion doesn't mean removing it entirely — a brief fade-in can be acceptable; a 3-second spinning loading indicator is not.

## Cognitive Load and Complexity

Many accessibility patterns overlap with good UX for everyone under cognitive load:

- **Clear headings**: Section structure aids users with attention or memory differences
- **Consistent navigation**: Reduces wayfinding cognitive cost
- **Plain language**: Supports users with language differences or disabilities
- **Progressive disclosure**: Reducing initial information load prevents overwhelm
- **Confirmation dialogs**: Confirm destructive actions before executing

These patterns aren't "extras for disabled users" — they're foundational UX practices that benefit everyone.

## See Also

- web-accessibility.md — Broader accessibility principles and semantic HTML
- design-color-typography.md — Contrast and readability considerations
- design-systems.md — Component design including accessibility requirements