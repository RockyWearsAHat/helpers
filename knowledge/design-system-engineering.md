# Design System Engineering — Component APIs, Theming, Documentation & Adoption

## The Problem: Fragmentation Without Systems

Organizations without design systems face compounding costs. Teams build similar components separately. Designers maintain parallel visual specs in Figma + Miro + Confluence. Developers maintain similar button logic across web, iOS, Android, with drift in behavior and appearance. Changes take months to propagate. Bug fixes in one platform don't reach others. Onboarding new team members means learning inconsistent patterns.

Design systems exist to make these costs linear instead of exponential. A shared component library (web + native), unified design tokens, and clear governance allow multiple products to evolve in lockstep.

## Component API Design

A component's interface (props, callbacks, default values) determines how efficiently developers can use it and how it evolves.

**API philosophy:**

- **Composition over configuration**: Hundreds of boolean props is a code smell. Instead, prefer composable elements. Example: `<Select>{options.map(opt => <Option value={opt.id}>{opt.label}</Option>)}</Select>` beats `<Select options={[]} optionLabelKey="label" ... />`.
- **Sensible defaults**: Most instances of a button are primary, unstyled, medium. Props override, not define. Never require 5 props for a simple button.
- **Explicit is better than magic**: If content can be passed as `children` or via a `label` prop, choose one path. Trying to support both (with fuzzy fallbacks) confuses consumers.
- **Backward compatibility**: Once you ship an API, changing it breaks consuming products. Design APIs you can live with. Deprecate carefully; provide migration paths. Semver: breaking changes = major version bump.

**Common API patterns:**

- **Controlled vs. Uncontrolled**: Form components (Select, Checkbox, Input) support both. Uncontrolled (`<Input defaultValue="Jim" />`) is simpler for one-off forms; controlled (`<Input value={name} onChange={setName} />`) aids complex validation. Document both clearly.
- **Render props + composition**: Instead of `<Table renderRow={...} renderCell={...} />`, expose `<Table><Row><Cell /></Row></Table>`. Enables nesting, cleaner JSX, and dynamic content.
- **Status prop explicitly**: Instead of `isLoading && <Spinner />` logic inside Button, make it `<Button status={isLoading ? 'loading' : 'idle'} />`. Separates concerns; consumers see all valid states upfront.

**Layout and sizing:**

- Define a consistent sizing scale (`xs`, `sm`, `md`, `lg`, `xl`) and use it across components.
- Flex properties (`flex`, `grow`, `shrink`) should be limited or non-standard. Prefer constraints (min-width, max-width, aspect ratio).
- Spacing should use design tokens (see below), not arbitrary padding. `padding: 12px` becomes `padding: var(--space-md)`.

## Design Tokens and Theming

Design tokens are atomic values (colors, spacing, typography, motion) that encode design decisions. Theming is the ability to swap token values (light mode, dark mode, brand variants, regional customization).

**Token hierarchy:**

1. **Global tokens** (design's perspective): `color-blue-600`, `space-16`, `font-family-sans`. These are rarely used directly.
2. **Semantic tokens** (product's perspective): `color-primary`, `color-success`, `space-component-padding`, `font-body`. Semantic tokens *reference* global tokens. A theme change (light to dark) remaps semantics, not globals.

Example:
```
Global: color-blue-600 = #0066CC
Semantic light: color-primary -> color-blue-600
Semantic dark: color-primary -> color-blue-400

When user switches theme, color-primary automatically remaps.
```

**Token serialization and sync:**

- **Single source**: Tokens live in one place (Figma Tokens, Design Tokens Community Group JSON, or a custom registry).
- **Multi-format output**: Token processor generates CSS custom properties (`--color-primary: #0066CC`), JS exports (`export const colorPrimary = '#0066CC'`), iOS constants (`let colorPrimary = UIColor(...)`), Android resources (`<color name="colorPrimary">#0066CC</color>`).
- **Tools**: Token Studio (Figma plugin), Style Dictionary (open source, generates multiple formats), Tokens.studio, ConfigCat.

**CSS custom properties:**

```css
:root {
  --color-primary: #0066CC;
  --space-md: 16px;
  --font-body: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

.button {
  padding: var(--space-md);
  background-color: var(--color-primary);
  font-family: var(--font-body);
}
```

Allows runtime theme switching (no rebuild). Dark mode:

```css
@media (prefers-color-scheme: dark) {
  :root {
    --color-primary: #4C9AFF;
  }
}
```

**Multi-platform tokens:**

- Web: CSS custom properties.
- React Native: JavaScript objects (`export const tokens = { colorPrimary: '#0066CC' }`).
- iOS: Swift package with colors, fonts, dimensions.
- Android: XML resources or Kotlin data classes.

Syncing across platforms requires automated tooling. Manual duplication breaks quickly.

## Documentation: Storybook, Chromatic, and Playgrounds

Documentation is not pretty component showcases — it's the developer's first experience with your components.

**Storybook (primary tool):**

- Browser-based component explorer. Run `npm run storybook` locally; view all components, all variants, interact with props in real-time.
- Stories are code (`<Button size="lg" onClick={...} />`), not screenshots. Stories evolve with components.
- Addons layer on functionality: Docs (auto-generated markdown from stories), Accessibility checker (axe integration), Interactions (play testing), Visual (regression detection).
- Page structure:
  - **Overview**: What is this component? When to use it. Link to design spec or issue.
  - **Examples**: Common use cases with stories. "Primary button", "Disabled state", "Loading state".
  - **Interactive playground**: Spin props, see changes in real-time. Props table auto-populates from TypeScript/PropTypes.
  - **Accessibility**: Test with axe; note keyboard behavior, screen reader announcements.
  - **API**: Parameter table (props, types, defaults).
  - **Migration**: For version bumps, show before/after code examples.

**Chromatic (cloud deployment):**

- Snapshot testing for visual regression. Push stories to Chromatic; it captures visual snapshots and flags changes.
- CI integration: PR preview URLs, automated visual diff comments on PRs.
- Alerts on broken stories or accessibility failures.
- Collaborative review: Designers sign off on visual changes before merge.

**Custom playgrounds:**

- For complex components (data table, rich editor, chart), embed playgrounds directly in docs so developers can experiment.
- Example: Rich editor with live code editor + preview pane. Users configure settings and see live rendering.

## Versioning and Migration

Design systems are products. They have versions, deprecations, and breaking changes.

**Semver:**

- **Major** (1.0 → 2.0): Breaking changes (prop removed, behavior changed, API incompatible).
- **Minor** (1.2 → 1.3): New features, new optional props. Backward compatible.
- **Patch** (1.2.3 → 1.2.4): Bug fixes. Backward compatible.

Example: Adding an optional `variant` prop is minor. Removing the `type` prop is major.

**Migration paths:**

- **Deprecation warnings**: Before removing a prop, warn users. `console.warn('Button: `type` prop deprecated as of v2.0. Use `variant` instead.')`.
- **Codemod support**: For major versions, provide an automated codemod (using jscodeshift or similar) to help consumers upgrade. `npx @design-system/codemods@latest v1-to-v2`.
- **Changelog + migration guide**: Document what changed, why, and how to upgrade. Include before/after code examples.
- **Deprecation timeline**: Remove stable APIs only after 2-3 major releases. Give teams time to upgrade.

## Cross-Platform Consistency

Web + native components share semantics but differ in implementation.

**Alignment:**

- Component names and concepts are identical (Button, Card, Modal, Input).
- Props translate (React props → Swift `@State`, Kotlin function parameters).
- Behaviors are normalized for platform conventions (iOS nav differs from web nav; modal animation differs).
- Design tokens flow to all platforms (colors, spacing, typography).

**Reality:** Perfect parity is impossible and undesirable. Native apps should feel native. A bottom sheet (iOS) is a dialog (web). Gestures (touch, swipe) are platform-specific. System APIs (keyboard, haptics) differ.

Consistency means: shared vocabulary, shared tokens, shared design principles. Not identical code.

**Tools:**

- **Shared TS types**: `@design-system/types` exports interfaces (`ButtonProps`, `ModalState`) usable across web + RN projects.
- **Component stubs**: Native teams implement `IButton` interface; web teams do the same. Cross-platform tooling provides shared tokens via TypeScript.
- **Pattern documentation**: Document the reasoning (why a Card has X padding) so native teams can apply it to their implementation.

## Adoption Metrics and Governance

Design systems succeed or fail based on adoption. A beautiful system used nowhere has negative ROI.

**Adoption metrics:**

- **Coverage**: Percentage of UI components in the system. "70% of buttons in the app use DesignSystem.Button." Aim for >90% on common components.
- **Velocity**: Time to ship a new feature. Systems should reduce this. Measure before/after.
- **Consistency**: Manual or automated checks for off-system components. Tooling (linters) can flag `<div className="custom-button">` usage.
- **Satisfaction**: Developer and designer NPS. Survey teams: "Is the design system easy to use?"

**Governance:**

- **Single source of truth**: One repo, one deployment pipeline. No forks (teams customizing locally diverge from the system).
- **Contribution model**: Clear process for proposing new components or changes. Requires design + eng review.
- **Stewardship**: Design systems are owned by a small, funded team (not a volunteer side project). Stewards prioritize component requests, manage versions, maintain documentation.
- **Release cadence**: Predictable releases (monthly, every 2 weeks). No surprise breaking changes.
- **Opt-in adoption**: Teams can upgrade at their own pace within reason (don't force upgrades that break their code).

**Anti-patterns:**

- Owning design system as a committee (everyone owns it, no one owns it).
- Treating it as free labor (stewards staff it as a side project).
- Abandoning it when initial adoption is slow (adoption takes 6-12 months).
- Refusing breaking changes out of fear (stagnation is worse than breakage).

## See Also

- [Design Systems — Tokens, Components, Governance & Handoff](design-systems.md)