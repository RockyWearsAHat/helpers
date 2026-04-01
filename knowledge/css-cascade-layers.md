# CSS Cascade Layers — Managing Style Priority

CSS cascade layers provide explicit control over stylesheet authorship priority, solving long-standing CSS architecture problems around specificity wars and third-party style conflicts. Introduced in CSS Cascade Level 4, layers let authors define a priority order for style sources.

## The Problem: Specificity and Import Order

CSS has always resolved conflicts with two rules:
1. **Specificity**: Higher-specificity selectors override lower-specificity ones
2. **Source order**: Later styles override earlier ones

This creates problems:
- Third-party CSS often uses `!important` to ensure overrides, making it difficult to adjust
- Utility frameworks (Tailwind) need high specificity or `!important` to override component styles
- Resets and normalizers must come first, but sometimes they use `!important`
- Author styles need to override third-party CSS, but specificity battles escalate

**Cascade layers solve this by making authorship intention explicit, independent of specificity or source order.**

## Declaring Layers

Layers establish a priority order. They can be declared implicitly or explicitly:

### Explicit Layer Declaration

```css
@layer reset, base, theme, components, utilities;
```

This declares the priority order, lowest to highest. Styles in `utilities` always override `components`, regardless of specificity or file load order.

### Implicit Layer Declaration via @layer

```css
@layer reset {
  * { margin: 0; padding: 0; }
}

@layer base {
  body { font-family: sans-serif; }
}

@layer components {
  .card { padding: 1rem; }
}

@layer utilities {
  .m-0 { margin: 0; }
  .p-1 { padding: 0.25rem; }
}
```

### Layering Imports

```css
@import url('reset.css') layer(reset);
@import url('tailwind.css') layer(utilities);
@import url('my-component.css') layer(components);
```

Imported files automatically belong to their specified layer, guaranteeing priority regardless of load order.

## Unlayered Styles

Styles **not inside any layer** (unlayered) have the highest priority, above all layers:

```css
@layer base {
  .heading { font-size: 2rem; }
}

/* This unlayered style overrides the layered one */
.heading { font-size: 3rem; }
```

**Practical implication**: Layer your entire codebase to maintain predictability. Unlayered styles should be exceptional (e.g., inline overrides).

## Specificity Within Layers

Specificity still matters within the same layer:

```css
@layer components {
  .card { padding: 1rem; }           /* specificity: 0,1,0 */
  .card.elevated { padding: 2rem; }  /* specificity: 0,2,0 — overrides */
}
```

But specificity inside a higher layer overrides ANY specificity in a lower layer:

```css
@layer components {
  .card { padding: 1rem; }
}

@layer utilities {
  .p-2 { padding: 0.5rem; } /* specificity: 0,1,0 */
}

/* .p-2 overrides .card padding, even though .card has higher specificity
   because utilities layer is higher than components layer */
```

**Key principle**: Layer priority (`utilities` > `components` > `base`) always wins over specificity.

## `!important` in Layers

`!important` reverses the cascade within its context. For layered styles:

```css
@layer base {
  body { background: white !important; }
}

@layer utilities {
  .bg-black { background: black; } /* Can't override the !important base style */
}
```

`!important` in a lower layer beats non-`!important` in a higher layer:

```css
@layer base {
  .heading { font-size: 1.5rem !important; }
}

@layer utilities {
  .text-3xl { font-size: 3rem; } /* Doesn't override due to !important in base */
}
```

**Guidance**: Minimize `!important` even in layers. It should be rare, reserved for truly non-overridable values (e.g., system font fallbacks).

## Practical Architecture: Third-Party CSS

Layers excel at managing third-party CSS:

```css
/* Declare layer priority */
@layer reset, thirdparty, base, components, utilities, overrides;

/* Third-party library */
@import url('external-framework.css') layer(thirdparty);

/* Your own layers */
@layer reset { /* CSS reset */ }
@layer base { /* Base typography, colors */ }
@layer components { /* Your components */ }
@layer utilities { /* Utility classes */ }

/* Override third-party if needed */
@layer overrides {
  .external-library-class { /* your override */ }
}
```

Now your styles predictably override third-party CSS without specificity wars or `!important` pollution.

## Nested Layers

Layers can nest:

```css
@layer framework {
  @layer reset {
    * { margin: 0; }
  }

  @layer base {
    body { font-family: sans-serif; }
  }
}

@layer app {
  @layer components { /* ... */ }
  @layer utilities { /* ... */ }
}
```

Nested layers inherit priority from their parent layer. `framework.reset` is always lower priority than `app.utilities`, even though both are nested.

## Migration Strategy: Adopting Layers

Existing codebases can adopt layers incrementally:

### Phase 1: Explicit Priority Order
```css
@layer reset, base, components, utilities;
```

### Phase 2: Migrate by Category
- Move resets to `@layer reset`
- Move base styles (typography, colors) to `@layer base`
- Move components to `@layer components`
- Move utilities to `@layer utilities`

### Phase 3: Third-Party Management
```css
@import url('tailwind.css') layer(utilities);
@import url('bootstrap.css') layer(thirdparty);
```

### Phase 4: Remove Specificity Hacks
Once layers are in place, reduce unnecessary specificity:
```css
/* Before: fighting specificity */
.card.card.card { padding: 1rem; }

/* After: rely on layer priority */
@layer components {
  .card { padding: 1rem; }
}
```

## Browser Support

Cascade layers shipped in all major browsers (2022–2023):
- Chrome 99+ (2022)
- Firefox 97+ (2022)
- Safari 15.4+ (2022)

Older browsers ignore `@layer` and fall back to normal cascade rules (specificity + source order).

## Common Patterns

### Reset + Framework Integration
```css
@layer reset, framework, app;

@layer reset {
  * { margin: 0; padding: 0; box-sizing: border-box; }
}

@import url('bulma.css') layer(framework);

@layer app {
  .hero { /* app-specific override */ }
}
```

### Token-Driven Design System
```css
@layer tokens, defaults, components, utilities;

@layer tokens {
  :root {
    --color-primary: #007bff;
    --spacing-base: 1rem;
  }
}

@layer components {
  .button { padding: var(--spacing-base); }
}

@layer utilities {
  .btn-primary { background-color: var(--color-primary); }
}
```

## Caveats and Traps

1. **Unlayered styles beat all layers**: Be intentional. Layer your entire stylesheet to avoid unexpected overrides.
2. **`@layer` in HTML `<style>` tags**: Each `<style>` tag is a separate source; layers don't span across multiple `<style>` tags in HTML.
3. **Nesting doesn't isolate specificity**: Specificity still compares globally; nesting just affects priority order.
4. **Verbose for small projects**: For single-file stylesheets without third-party CSS, layers add complexity. Use them when managing multiple sources or teams.

## See Also

- [web-css-architecture.md](web-css-architecture.md) — BEM, CSS Modules, CSS-in-JS, architecture patterns
- [design-design-systems.md](design-design-systems.md) — Token systems, consistency, scalability
- [api-versioning.md](api-versioning.md) — Managing compatibility (similar problem space)