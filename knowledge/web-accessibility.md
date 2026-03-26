# Web Accessibility — Principles, Patterns & Inclusive Design

## Accessibility as a Spectrum

Accessibility is not a binary pass/fail condition — it exists on a continuum. An interface can be fully accessible to one group while presenting barriers to another. Disabilities span a wide range of experiences:

| Category  | Permanent                                      | Temporary                      | Situational                     |
| --------- | ---------------------------------------------- | ------------------------------ | ------------------------------- |
| Visual    | Blindness, low vision, color vision deficiency | Eye infection, dilated pupils  | Bright sunlight on screen       |
| Motor     | Limb difference, tremor, paralysis             | Broken arm, RSI                | Holding a child, wearing gloves |
| Auditory  | Deafness, hard of hearing                      | Ear infection                  | Noisy environment               |
| Cognitive | Learning differences, memory impairments       | Concussion, medication effects | Sleep deprivation, distraction  |

This spectrum means accessibility is not about a separate population of "disabled users." The same interface decisions affect everyone at different times and in different contexts. Designing for permanent disabilities often produces solutions that benefit the temporary and situational cases — a phenomenon known as the curb-cut effect.

## The Curb-Cut Effect

Sidewalk curb cuts were mandated for wheelchair users. They turned out to benefit parents with strollers, delivery workers with hand trucks, travelers with luggage, cyclists, and anyone with temporary mobility limitations. The same dynamic plays out in digital interfaces:

- **Captions** designed for deaf users benefit anyone in a noisy or quiet environment
- **Keyboard navigation** designed for motor-impaired users benefits power users and developers
- **Clear hierarchy and plain language** designed for cognitive accessibility benefits everyone under cognitive load
- **High-contrast text** designed for low-vision users benefits anyone on a low-quality display or in bright light

Accessibility constraints consistently produce broadly better design — not as a side effect, but because they force attention to clarity, structure, and robustness.

## The POUR Principles

The Web Content Accessibility Guidelines (WCAG) organize accessibility around four principles:

### Perceivable

Information and interface components must be presentable in ways users can perceive. This does not require that every user perceive content in the same way — it requires that at least one perceivable pathway exists for each user.

Key considerations:

- Text alternatives for non-text content (images, icons, charts)
- Captions and transcripts for audio/video content
- Sufficient color contrast between foreground and background
- Content that doesn't rely solely on color to convey meaning
- Content that can be presented in different ways without losing structure

### Operable

Interface components and navigation must be operable through various interaction methods. An interface that works exclusively with a mouse excludes keyboard users, switch users, voice control users, and many others.

Key considerations:

- All functionality available from a keyboard
- Users have enough time to read and interact with content
- Content doesn't cause seizures or physical reactions
- Users can navigate, find content, and determine where they are
- Input methods beyond keyboard are supported

### Understandable

Information and interface operation must be understandable. Perceivable and operable content that users cannot comprehend still presents a barrier.

Key considerations:

- Text is readable and understandable at appropriate levels
- Content appears and operates in predictable ways
- Users are helped to avoid and correct mistakes
- Consistent navigation and identification patterns

### Robust

Content must be robust enough to be interpreted by a wide variety of user agents, including assistive technologies. This is the durability principle — content should work not just with current tools but with reasonable future tools.

Key considerations:

- Valid, well-structured markup
- Programmatic name, role, and value for all interface components
- Status messages conveyed to assistive technologies without focus changes

## Semantic HTML as Foundation

The most impactful accessibility decision is often the simplest: using appropriate HTML elements for their intended purpose.

### Why Semantics Matter

Browsers and assistive technologies extract meaning from HTML structure. A `<button>` carries built-in semantics, keyboard behavior, and accessibility properties that a `<div onclick="...">` does not:

| Native `<button>`                       | `<div>` with click handler                |
| --------------------------------------- | ----------------------------------------- |
| Announced as "button" by screen readers | Announced as generic container            |
| Focusable via Tab key by default        | Not focusable without `tabindex`          |
| Activatable via Enter and Space         | Only responds to click without extra code |
| Included in form submission             | No form integration                       |
| Disabled state via `disabled` attribute | Requires custom implementation            |

Replacing a semantic element with a generic one and rebuilding its behavior through JavaScript and ARIA is rarely an improvement. Each reimplemented behavior is an opportunity for edge cases, inconsistencies, and omissions.

### Common Semantic Choices

| Need                     | Appropriate Element             | Common Mistake                         |
| ------------------------ | ------------------------------- | -------------------------------------- |
| Navigation section       | `<nav>`                         | `<div class="nav">`                    |
| Page heading hierarchy   | `<h1>` through `<h6>` in order  | Choosing heading level for visual size |
| List of items            | `<ul>`, `<ol>`, `<dl>`          | Series of `<div>` elements             |
| Tabular data             | `<table>` with `<th>`           | Grid of `<div>` elements               |
| Standalone content block | `<article>`                     | `<div class="article">`                |
| Complementary content    | `<aside>`                       | `<div class="sidebar">`                |
| Form field labels        | `<label>` associated with input | Placeholder text as sole label         |

## The Accessibility Tree

Browsers construct a parallel representation of the DOM called the _accessibility tree_. This tree is what assistive technologies actually interact with — not the visual rendering and not the raw DOM.

### How the Tree Is Built

The accessibility tree derives from the DOM but transforms it:

1. Each DOM element maps to an accessible object (or is pruned)
2. The accessible object has a **role** (button, link, heading, etc.)
3. It has a **name** (computed from content, labels, or ARIA attributes)
4. It has a **state** (expanded, checked, disabled, selected, etc.)
5. It has **relationships** (labelled-by, described-by, controls, etc.)

```
DOM:
<button aria-expanded="false" aria-controls="menu">
  <svg aria-hidden="true">...</svg>
  Menu
</button>

Accessibility tree node:
  Role: button
  Name: "Menu"
  State: collapsed
  Controls: "menu"
```

Elements with `aria-hidden="true"` or `display: none` are pruned from the accessibility tree. Purely decorative elements (role="presentation" or role="none") are also removed, though their text content may be preserved.

### Inspecting the Accessibility Tree

Browser developer tools expose the accessibility tree alongside the DOM. Inspecting this tree reveals what assistive technology users actually encounter — which elements have names, which are focusable, and what roles they carry. Discrepancies between visual appearance and tree representation are accessibility gaps.

## ARIA — The Double-Edged Sword

Accessible Rich Internet Applications (ARIA) provides attributes that modify the accessibility tree. ARIA can add semantics that HTML alone doesn't express. It can also introduce confusion and contradictions when misapplied.

### The First Rule of ARIA

If a native HTML element or attribute provides the semantics and behavior needed, use it instead of ARIA. ARIA adds _annotations_ to the accessibility tree but does not add _behavior_. Adding `role="button"` to a `<div>` announces it as a button but does not make it focusable, keyboard-activatable, or part of form submission.

### When ARIA Helps

ARIA is genuinely necessary for patterns that HTML doesn't natively support:

| Pattern                 | ARIA Approach                                                           |
| ----------------------- | ----------------------------------------------------------------------- |
| Custom tab interface    | `role="tablist"`, `role="tab"`, `role="tabpanel"`, `aria-selected`      |
| Live region updates     | `aria-live="polite"` or `aria-live="assertive"`                         |
| Current step indicator  | `aria-current="step"` or `aria-current="page"`                          |
| Expandable sections     | `aria-expanded`, `aria-controls`                                        |
| Combobox / autocomplete | `role="combobox"`, `aria-activedescendant`, `aria-autocomplete`         |
| Progress indication     | `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax` |

### When ARIA Causes Harm

ARIA misuse is a significant source of accessibility problems:

- **Incorrect roles** — `role="button"` on a link changes announcement but breaks link behavior expectations
- **Redundant ARIA** — `<button role="button">` or `<nav role="navigation">` adds noise without value
- **Missing required attributes** — `role="checkbox"` without `aria-checked` leaves state indeterminate
- **Broken relationships** — `aria-labelledby` pointing to a nonexistent or empty ID
- **Overriding native semantics** — `<h2 role="presentation">` removes heading semantics that navigation depends on

Automated accessibility audits frequently catch missing ARIA attributes but cannot catch _incorrect_ ARIA usage. A `role="menu"` on what is actually a navigation list passes automated checks but creates a confusing interaction model.

## Keyboard Navigation and Focus Management

Keyboard accessibility is foundational — it's the common denominator for users who rely on screen readers, switch devices, voice control, or simply prefer keyboard interaction.

### Tab Order and Focus

The default tab order follows DOM source order among _focusable elements_: links, buttons, form inputs, and elements with `tabindex`. This order should match the visual reading order.

| `tabindex` Value | Effect                                                             |
| ---------------- | ------------------------------------------------------------------ |
| Not set          | Default focusability (focusable only if natively interactive)      |
| `0`              | Added to tab order at its DOM position                             |
| `-1`             | Programmatically focusable but not in tab order                    |
| Positive values  | Inserted before natural tab order — almost universally problematic |

Positive `tabindex` values create a separate ordering tier that precedes all `tabindex="0"` and natural elements. This sounds useful but becomes unmanageable quickly — it couples focus order to explicit numbers rather than DOM structure.

### Focus Trapping

Modal dialogs, dropdown menus, and similar overlay patterns require _focus trapping_ — constraining Tab cycling within the component while it's active. Without trapping, a modal user can Tab behind the modal into invisible or irrelevant content.

Focus trap implementation needs to handle:

- Moving focus into the component when it opens
- Cycling focus from last focusable element back to first (and reverse)
- Restoring focus to the trigger element when the component closes
- Closing on Escape key press

### Focus Indicators

The default browser focus outline exists for a reason. Removing it (`outline: none`) without providing an alternative focus indicator eliminates the primary way keyboard users track their position on the page.

Approaches to custom focus indicators involve balancing visibility against visual design:

```css
/* Visible to keyboard users, hidden from mouse users */
:focus-visible {
  outline: 2px solid currentColor;
  outline-offset: 2px;
}
```

The `:focus-visible` pseudo-class applies focus styles only when the browser determines focus was reached via keyboard (or similar non-pointer input), addressing the tension between needing visible focus indicators and not wanting focus rings on every mouse click.

## Color and Visual Design

### Contrast Requirements

Text legibility depends on the contrast ratio between foreground and background colors. WCAG defines contrast ratios as a measured relationship between relative luminance values:

| Text Size                           | Minimum Ratio (AA) | Enhanced Ratio (AAA) |
| ----------------------------------- | ------------------ | -------------------- |
| Normal text (< 18pt / 14pt bold)    | 4.5:1              | 7:1                  |
| Large text (≥ 18pt / 14pt bold)     | 3:1                | 4.5:1                |
| UI components and graphical objects | 3:1                | —                    |

These are _minimum_ thresholds. Higher contrast generally improves readability for all users, though extremely high contrast (pure black on pure white) can cause visual fatigue for some users with dyslexia or photosensitivity.

### Color as Sole Indicator

Using color alone to convey information — red for error, green for success — fails for users with color vision deficiencies (approximately 8% of males, 0.5% of females globally). Effective alternatives include:

- Icons alongside color coding (error icon + red, check icon + green)
- Text labels that don't require color perception
- Patterns or shapes that remain distinguishable without color
- Border or underline treatments in addition to color changes

### Motion Sensitivity

Animations and transitions affect users with vestibular disorders, who may experience dizziness, nausea, or disorientation from motion. The `prefers-reduced-motion` media query expresses this user preference:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

This is not about eliminating all motion — it's about providing an alternative experience that avoids large-scale movement, parallax effects, and auto-playing animations. Reduced-motion users may still benefit from subtle transitions that convey state changes.

## Screen Reader Interaction Models

Understanding how screen readers present content changes how developers think about structure:

### Reading Modes

Most screen readers operate in multiple modes:

| Mode                 | Behavior                                           | Triggered By                                |
| -------------------- | -------------------------------------------------- | ------------------------------------------- |
| Browse/document mode | Arrow keys move through content element by element | Default in content areas                    |
| Focus/forms mode     | Keys pass through to the focused element           | Entering a form field or interactive widget |
| Application mode     | All keys pass through to the application           | `role="application"` (use with caution)     |

In browse mode, screen reader users navigate by headings, landmarks, links, or other structural elements. The heading hierarchy is a primary navigation mechanism — skipping heading levels or using headings for visual styling rather than structure disrupts this navigation.

### Announcements and Live Regions

Screen readers announce content as it comes into focus or as users navigate to it. But how does a screen reader announce content that _changes_ without user navigation — a notification, a chat message, a search result count?

Live regions solve this:

```html
<div aria-live="polite" aria-atomic="true">3 results found</div>
```

| Attribute       | Values                                 | Effect                                                  |
| --------------- | -------------------------------------- | ------------------------------------------------------- |
| `aria-live`     | `off`, `polite`, `assertive`           | Controls interruption priority                          |
| `aria-atomic`   | `true`, `false`                        | Whether the entire region or just changes are announced |
| `aria-relevant` | `additions`, `removals`, `text`, `all` | Which types of changes trigger announcements            |

`polite` waits for the current speech to finish. `assertive` interrupts immediately — appropriate for urgent messages like errors, inappropriate for routine updates.

### Accessible Names

Screen readers announce elements by their _accessible name_ — computed through a priority chain:

1. `aria-labelledby` (references another element's content)
2. `aria-label` (string value directly on the element)
3. Associated `<label>` element (for form controls)
4. Element content (text within the element)
5. `title` attribute (lowest priority, inconsistent support)
6. `placeholder` (not a reliable accessible name)

Common issues with accessible names:

- Icon-only buttons without `aria-label` have no accessible name
- Multiple elements with the same accessible name (e.g., many "Read more" links) are indistinguishable
- `aria-label` on non-interactive elements is ignored by some assistive technologies

## Challenging Patterns

### Forms

Forms are one of the most frequent sources of accessibility failures:

- **Labels** — Every input needs a programmatically associated label. Placeholder text disappears on input and is not announced as a label by all assistive technologies
- **Error messages** — Errors need to be associated with their inputs via `aria-describedby` and announced via live regions when they appear
- **Required fields** — `aria-required="true"` or the HTML `required` attribute communicates required status; asterisks alone do not
- **Grouping** — Related inputs (radio buttons, address fields) need `<fieldset>` and `<legend>` to convey their relationship
- **Autocomplete** — The `autocomplete` attribute helps users with cognitive and motor disabilities by enabling browser autofill

```html
<div>
  <label for="email">Email address</label>
  <input
    id="email"
    type="email"
    required
    autocomplete="email"
    aria-describedby="email-error"
  />
  <p id="email-error" role="alert" hidden>Please enter a valid email address</p>
</div>
```

### Modal Dialogs

Modals require coordinating multiple accessibility concerns simultaneously:

1. Focus moves into the modal when it opens
2. Focus is trapped within the modal
3. Background content is inert (not navigable or interactive)
4. `role="dialog"` and `aria-modal="true"` convey modal semantics
5. `aria-labelledby` points to the modal's heading
6. Escape key closes the modal
7. Focus returns to the trigger element on close

The `<dialog>` element handles many of these concerns natively when opened with `showModal()`, including focus management, Escape handling, and background inertness. Custom implementations must replicate each behavior that the native element provides.

### Dynamic Content and Single-Page Applications

Single-page applications that update content without full page loads present specific challenges:

- **Route changes** — Screen readers don't automatically announce that the page has changed. Focus management and document title updates signal navigation
- **Loading states** — Asynchronous content loading needs `aria-busy` and live region announcements when content arrives
- **Infinite scroll** — Removes the landmark-based navigation that pagination provides. Keyboard users may be unable to reach content after the feed
- **Client-side rendering** — Content injected via JavaScript may not be in the accessibility tree if timing or structure is incorrect

### Data Visualization

Charts, graphs, and complex visualizations pose unique challenges:

- Text alternatives need to convey the data story, not just describe visual appearance
- Complex data may warrant both a summary (`alt`) and a detailed description (`aria-describedby` linking to a data table)
- Interactive visualizations need keyboard alternatives for hover-dependent information
- Color-blind-safe palettes are necessary when color encodes data

## Testing Approaches

Accessibility testing works across multiple methods, each catching different categories of issues:

| Method                           | Catches                                                           | Misses                                                              |
| -------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------- |
| Automated scanning               | Missing alt text, contrast failures, missing labels, invalid ARIA | Incorrect labels, confusing interaction patterns, logical structure |
| Keyboard testing                 | Missing focus management, unreachable elements, traps             | Visual-only issues, screen reader announcement quality              |
| Screen reader testing            | Announcement quality, navigation structure, live region behavior  | Visual design issues, motor accessibility                           |
| Manual expert review             | Complex interaction patterns, holistic experience quality         | Scaleability — expensive and slow                                   |
| User testing with disabled users | Real-world experience, unexpected barriers, workflow issues       | Coverage — each participant represents specific needs               |

No single method catches everything. Automated tools typically identify 20-30% of accessibility issues. The remainder requires human judgment about whether the experience is genuinely usable.

## Cognitive Accessibility

Beyond sensory and motor accessibility, cognitive and neurological differences affect how people process interfaces:

- **Consistent layout and navigation** — Predictable structure reduces cognitive load for everyone, and is essential for users with memory or learning differences
- **Clear language** — Plain language, short sentences, and defined terms benefit users with cognitive disabilities and non-native speakers alike
- **Error prevention** — Confirmation dialogs, undo functionality, and clear formatting hints reduce the impact of mistakes
- **Reduced complexity** — Progressive disclosure, clear visual hierarchy, and minimal required memory load
- **Timing flexibility** — Time limits on sessions or tasks exclude users who process information at different speeds

These considerations overlap significantly with general usability — the distinction between accessibility and usability becomes particularly blurred in the cognitive domain.

## Accessibility as Design Quality

Accessibility constraints share a property with other design constraints: they narrow the solution space in ways that tend to produce more robust, more thoughtful designs. Requirements to provide text alternatives force consideration of what information content actually conveys. Requirements for keyboard operability force explicit interaction models. Requirements for semantic structure force meaningful content hierarchy.

These constraints don't merely accommodate specific disabilities — they surface and require resolution of ambiguities that might otherwise go unnoticed. An interface that can be fully understood without seeing it is an interface whose information architecture has been thoroughly considered. An interface that can be fully operated without a mouse is an interface whose interaction model is explicit and complete.

The tension between accessibility requirements and visual design ambitions is real but often overstated. Most accessibility failures stem not from design trade-offs but from patterns that would be improved for all users — missing labels, ambiguous links, inconsistent navigation, and trapped focus are usability problems with accessibility consequences.
