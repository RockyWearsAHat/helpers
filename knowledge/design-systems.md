# Design Systems — Tokens, Components, Governance & Handoff

## What a Design System Is

A design system is a shared language between design and engineering: a documented collection of reusable components, design tokens (atomic values for color, spacing, typography), and patterns that teams apply consistently across products. It differs from a component library in scope — a component library is just code; a design system includes governance, documentation, evolution strategy, and alignment mechanisms between disciplines.

Design systems exist because consistency is expensive to achieve without it. Without a system, teams make identical decisions repeatedly, designers and developers maintain parallel definitions of "what is a button," and visual debt accumulates as products diverge.

## Design Tokens as the Foundation

Tokens are the operational unit of design systems: semantic, reusable values that encode design decisions. They move beyond static design files to become programmable assets shared between Figma, code, and other tooling.

### Token Categories

- **Color tokens**: Brand colors, semantic colors (success/error/warning), neutral scales. Modern systems use tokens organized hierarchically (`brand-blue`, `feedback-error`, `surface-default`).
- **Spacing tokens**: Baseline increments (typically 4px or 8px multiples) defining gutters, padding, margins. Semantic naming (`space-xs`, `space-md`, `space-2xl`) scales better than numeric.
- **Typography tokens**: Font family, size, weight, line height, letter spacing. Can include complete typography scales (e.g., `display-large`, `heading-1`, `body`).
- **Effect tokens**: Shadows, blur, border width. Often neglected but critical for layering and depth.
- **Motion tokens**: Duration, easing functions. Prevent random animation speeds across products.

### Token Serialization

Tokens must exist in formats both tools and code understand. JSON Schema (Design Tokens Community Group format) enables:

```
{
  "color": {
    "brand": {
      "primary": { "value": "#0066CC", "$type": "color" }
    }
  }
}
```

This single source flows to CSS (via `--color-brand-primary`), iOS (via Swift enums), Android (via resource files), and Figma (via Variables), keeping definitions in sync. Tools like Token Studio and partial sync to design systems collective manifest files automate this.

## Component Libraries and Variation Systems

Components in design systems are reference implementations: a button is not just a `.button` class but a pattern documenting interactions, keyboard behavior, states (hover, focus, disabled, loading), and accessibility requirements.

### Atomic Design Model

Components organize hierarchically:

- **Atoms**: Irreducible elements (button, input, label, icon)
- **Molecules**: Simple compositions (search bar = input + button + icon)
- **Organisms**: Complex sections (navbar, card with metadata, data table)

This taxonomy helps teams reason about component scope and dependencies.

### Variants and Configuration

Modern component systems expose variance through props, not class combinatorics. A button might include:

- `variant`: "primary" | "secondary" | "tertiary"
- `size`: "sm" | "md" | "lg"
- `state`: "default" | "loading" | "disabled"
- `icon`: optional icon slot

This compositional approach reduces component sprawl. Combinatorics explode quickly (3 variants × 3 sizes × 4 states = 36 possible buttons), so most systems cap variation or use compound components to nest complexity.

## Design-to-Development Handoff

The handoff between design and engineering is the most brittle point in design systems. Misalignment leads to reimplementation, bugs, and angry back-and-forth conversations.

### Figma as Specification

Modern practice treats Figma as the source of truth: designers use Figma Variables (2023+) to define tokens in the same file where components are built. Components in Figma are structured using Auto Layout (responsive constraints baked into design) to match CSS flexibility.

Plugins like Storybook CLI and Chromatic integrate Figma components directly into code review, showing design and implementation side-by-side.

### From Design to Code

The gap remains substantial. Common handoff bridges:

- **Design tokens exported**: JSON/CSS variables synced via GitHub Actions to code repos
- **Figma URLs in component libraries**: Engineers reference design specs directly
- **Storybook as shared documentation**: Shows both design intent and working code, updated by developers
- **Component snapshots**: Figma to code comparison detecting regressions

Some teams use code-first approaches (engineers implement components in code, design specs them in Figma). Others generate code from design (Figma plugins that export React components) — a premature promise that rarely works without heavy manual refinement.

## Theming and Dark Mode

Tokens enable theming by composing multiple token sets. A dark mode isn't a separate design; it's a different assignment of color tokens to the same semantic names.

### Semantic Naming

Good: `color-surface-default`, `color-text-primary` (meaning: apply this to surfaces, text respectively)
Bad: `color-navy`, `color-lightgray` (locked to specific palettes, doesn't support theming)

Semantic tokens reference primitive tokens: `color-text-primary` might be `color-gray-900` in light mode and `color-gray-100` in dark mode.

### Responsive Tokens and Container Queries

Modern systems support different token values based on context:

- Viewport size: button padding larger on desktop, smaller on mobile
- Container size: component adjusts based on available space, not viewport
- High contrast mode: respecting accessibility preferences
- Reduced motion: animations disabled for motion sensitivity

Container queries enable tokens to respond to local context, not just global breakpoints — a significant shift in how responsive design tokens function.

## Versioning and Governance

Design systems must balance stability (so implementations don't break) with evolution (so the system improves). Semantic versioning applies:

- **Major**: Breaking changes (component API renamed, token deleted)
- **Minor**: Additive (new token, new component variant, non-breaking API extension)
- **Patch**: Fixes (bug fix, clarification in documentation)

Governance practices include:

- **Deprecation windows**: New tokens are encouraged; old ones deprecated but functional for a release cycle
- **Mandatory contribution**: Design or engineering changes that affect the system must update tokens/components first
- **RFC process**: Major changes driven by request-for-comment cycles involving both disciplines
- **Change log discipline**: Every version documents what changed and why

## Documentation and Adoption

Comprehensive documentation is necessary but not sufficient for adoption. Teams adopt design systems when:

1. The system solves a real problem (consistency felt, redesigns are faster)
2. It's easier to use the system than circumvent it (good defaults, searchable components)
3. The system earns trust through stability and responsiveness to feedback

Documentation should include:

- **Token reference**: Interactive token explorer showing values, how to access them, when to use each
- **Component specs**: Visual specs, code examples (React, Vue, etc.), accessibility notes, interaction patterns
- **Usage examples**: Real-world implementations, anti-patterns (what not to do), contextual guidance
- **Storybook**: Live component playground with knobs/args showing all variants and states
- **Getting started**: Quick onboarding for new projects (install, configure, import tokens)

## Architecture and Evolving Systems

Large systems often split into:

- **Core tokens**: Primitive colors, spacing, typography — rarely change
- **Semantic tokens**: Higher-level abstractions using core tokens — adjust with product evolution
- **Component variants**: Specific configurations — change frequently with feature requests

This layering prevents token proliferation while allowing local adaptation. Teams sometimes maintain "extensions" for product-specific needs (e.g., marketing site tokens extending brand tokens).

## Cross-Platform Considerations

Design systems spanning web, iOS, Android, and other platforms must share semantic concepts while respecting platform idioms:

- Color tokens are universal
- Spacing principles (but the base unit may differ: 4pt on iOS, 4px on web)
- Typography can't be identical (platform fonts differ, metrics differ) but semantic scales (heading-1, body) translate
- Component APIs adapt to platform conventions (Swift properties vs. React props)

Shared token formats (JSON-based token standards) help, though complete parity is rarely the goal — platforms are different by design.

## Challenges and Evolution

Design systems succeed or fail based on adoption, not completeness. Common friction points:

- **Customization pressure**: Every team wants an exception; systems need strength to say no or provide extension mechanisms
- **Stale documentation**: Components ship; docs don't get updated; nothing is more damaging to trust
- **Over-abstraction**: Trying to support every use case creates bloated components that solve no problem well
- **Tool fragmentation**: Figma + Storybook + code repo all drift; keeping them synchronized is ongoing work
- **Adoption momentum**: Early teams drive network effects; initial adoption is the hardest phase

Modern systems are increasingly recognizing that design systems are not finished products but living codebases that require ongoing investment, cross-discipline collaboration, and readiness to evolve.

## See Also

- web-accessibility.md — Accessibility patterns essential for component design
- web-browser-rendering.md — How components render and why pixel-perfect specs are myths
- architecture-patterns.md — Architectural principles that organize systems thinking