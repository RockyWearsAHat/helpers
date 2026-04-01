# Web Design Tokens — Naming, Types, Formats & Multi-Platform Theming

## What Are Design Tokens

Design tokens are design decisions captured as **data** rather than hardcoded values. Examples: `color-primary: #0066cc`, `spacing-lg: 24px`, `font-size-heading-1: 32px`. Tokens bridge design and engineering by creating a single source of truth across platforms.

Tokens differ from CSS variables because they're **format-agnostic**: the same token can output CSS, JSON, Swift, Kotlin, or design system documentation without duplication. This enables consistent theming across web, mobile, and design tools.

## Token Types

### Color Tokens
- **Primitive/Global**: Brand palette (`blue-50`, `blue-900`), neutrals, system colors
- **Semantic**: Tokens mapped to meaning (`color-primary`, `color-success`, `color-error`)
- **Component**: Component-specific overrides (`button-primary-bg`, `input-focus-ring`)

Semantic tokens enable theming: switching from light to dark mode changes `color-primary` once, not dozens of individual color references.

### Spacing Tokens
- **Scale**: `spacing-xs` (4px), `spacing-sm` (8px), `spacing-md` (16px), `spacing-lg` (24px), `spacing-xl` (32px)
- **Use cases**: margins, padding, gaps, gutters
- Foundation for consistent whitespace. Prevents arbitrary spacing like `margin: 13px`.

### Typography Tokens
- **Font families**: `font-family-sans`, `font-family-mono`, `font-family-serif`
- **Font sizes**: `font-size-xs` (12px), `font-size-sm` (14px), `font-size-base` (16px), `font-size-lg` (18px), `font-size-2xl` (32px)
- **Font weights**: `font-weight-light` (300), `font-weight-normal` (400), `font-weight-bold` (700)
- **Line heights**: `line-height-tight` (1.2), `line-height-normal` (1.5), `line-height-relaxed` (1.75)
- **Letter spacing**: `letter-spacing-tight`, `letter-spacing-normal`, `letter-spacing-wide`

Typography tokens often bundle related properties: `heading-1: { size: 32px, weight: 700, line-height: 1.2 }`

### Elevation / Shadow Tokens
- **Shadow depth**: `shadow-sm`, `shadow-md`, `shadow-lg` for Z-axis hierarchy
- Applied to cards, modals, tooltips
- Foundation for visual hierarchy

### Border Tokens
- **Widths**: `border-width-thin` (1px), `border-width-medium` (2px)
- **Radii**: `border-radius-sm` (4px), `border-radius-md` (8px), `border-radius-lg` (16px), `border-radius-full` (9999px)
- **Styles**: `border-style-solid`, `border-style-dashed`

### Duration & Easing Tokens (Animation)
- **Durations**: `duration-fast` (100ms), `duration-base` (200ms), `duration-slow` (300ms)
- **Easing functions**: `ease-in`, `ease-out`, `ease-in-out`, `ease-linear`
- Prevents inconsistent animation speeds across components

### Opacity Tokens
- **Levels**: `opacity-0`, `opacity-20`, `opacity-50`, `opacity-80`, `opacity-100`
- Used for disabled states, overlays, hover effects

## Naming Conventions

### Hierarchical Naming (Recommended)
`[category]-[subcategory]-[state]-[property]`

Examples:
- `color-primary-hover` — primary action color on hover
- `spacing-sm` — small spacing in scale
- `font-size-heading-1` — heading level 1 font size
- `shadow-elevated` — shadow for elevated surfaces

**Benefits:**
- Self-documenting
- Predictable organization
- Reduces naming conflicts
- Easy to discover related tokens

### Semantic vs. Primitive
- **Primitive**: `blue-500` — describes the actual value
- **Semantic**: `color-primary` — describes the meaning

Design systems layer semantic tokens over primitives. Semantic tokens change with theme; primitives remain constant.

### Theming with Token Aliases
```
// Primitive tokens (always present)
color-blue-500
color-gray-50

// Light theme
color-primary: $color-blue-500
color-bg-default: $color-gray-50

// Dark theme
color-primary: $color-blue-400  // lighter variant for dark
color-bg-default: $color-gray-900
```

Changing theme only updates aliases, not all token usages.

## Token Formats & Standards

### W3C Design Tokens Format
Official standard for representing design tokens as JSON. Structure:
```json
{
  "color": {
    "primary": {
      "$value": "#0066cc",
      "$type": "color"
    }
  },
  "spacing": {
    "md": {
      "$value": "{spacing.base} * 4",
      "$type": "dimension"
    }
  }
}
```

**Advantages:**
- Language-agnostic
- Supports references and math (`{spacing.base} * 4`)
- Standardized across tools
- Enables ecosystem interoperability

### Style Dictionary (Amazon-backed)
Most popular transformer for design tokens. Converts W3C JSON into platform-specific outputs:
- CSS custom properties
- JavaScript objects
- SCSS variables
- Android XML
- iOS Swift
- Figma tokens
- JSON

A single token definition produces outputs for web, mobile, and design tools simultaneously.

### Figma Tokens Plugin
Exports tokens directly from Figma to JSON. Enables designers to define tokens, engineers to consume. Bridges design → engineering.

## Token Pipelines & Generation

### Typical Workflow
1. **Design ownership**: Designer defines tokens in Figma or token editor
2. **Export**: Tokens exported to version-controlled repository (YAML/JSON)
3. **Transform**: Style Dictionary or similar tool converts to platform formats
4. **Distribution**: Generated files shipped in NPM package, CSS-in-JS library, or design system package
5. **Consumption**: Applications import and use (CSS variables, JavaScript objects, etc.)

### CI/CD Integration
- Token changes trigger regeneration on every commit
- Generated code version-controlled or artifact-only
- Linting: validate token naming, detect orphaned tokens
- Breaking change detection: flag removed or renamed tokens

### Token Documentation Generation
Tools auto-generate design system documentation from token definitions, showing values, usage, examples. Keeps docs in sync automatically.

## Multi-Platform Token Management

### Web
- CSS custom properties (native)
- CSS-in-JS objects (Styled Components, Emotion)
- SCSS/LESS variables
- Tailwind config object

### Mobile (Native)
- Android: Color resources, dimension resources, styles XML
- iOS: Asset catalog, SwiftUI Color/Font definitions
- React Native: CSS-like object

### Design Tools
- Figma tokens plugin
- Design tokens in Sketch libraries
- Storybook integration for preview

**Challenge:** Keeping tokens synchronized across all platforms. Solution: Single source (W3C JSON) + automated generation per platform.

## Theming Implementation

### Light/Dark Mode
Organize tokens into theme files:
```
tokens/
  ├── colors-primitives.json
  ├── themes/
  │   ├── light.json
  │   └── dark.json
  └── spacing.json
```

Merge theme tokens at build time or runtime:
- **Build time**: Generate separate CSS for each theme, user picks via stylesheet
- **Runtime**: CSS custom properties override based on `data-theme` or `prefers-color-scheme`

### Multi-Brand Tokens
Enterprise apps serving multiple brands:
```
tokens/
  ├── base/
  │   ├── spacing.json
  │   └── typography.json
  └── brands/
      ├── acme/
      │   ├── colors.json
      │   └── logos.json
      └── globex/
          ├── colors.json
          └── logos.json
```

Each brand inherits base, overrides colors/logos. Single codebase, multiple visual languages.

### Density Tokens (Compact/Spacious)
Some platforms (desktop, enterprise) support density variants:
```
spacing:
  compact:
    md: 8px
  normal:
    md: 16px
  spacious:
    md: 24px
```

User preference switches density, CSS variables or component props adjust layout.

## Common Pitfalls

### Over-Tokenization
Not every value needs a token. Tokens add maintenance overhead. Limit to values reused 3+ times or core design decisions.

### Naming Drift
Without governance, teams create inconsistent tokens: `color-primary-focus` vs `color-primary-hover-state`. Enforce naming conventions in CI.

### Orphaned Tokens
Removing usage but leaving token defined. Code review and automated detection prevents bloat.

### Version Management
Tokens are APIs: renaming or removing is a breaking change. Semantic versioning and deprecation cycles matter.

### Platform-Specific Values
Some tokens vary per platform (font sizes differ between iOS and Android). Document exceptions, use platform-conditional tokens.

## Tools & Ecosystem

**Figma tokens plugin**: Designer-first token definition  
**Style Dictionary**: Most popular transformer  
**Storybook**: Token preview and documentation  
**Electro (GitHub)**: Google's design token system  
**Pollen**: UI library design tokens example  
**Starkly**: Token management SaaS  
**Zero Height**: Design system documentation with tokens

## See Also

- design-systems.md — tokens as part of broader system  
- design-system-engineering.md — component theming  
- design-color-typography.md — color theory, contrast  
- performance-web-vitals.md — CSS custom property performance