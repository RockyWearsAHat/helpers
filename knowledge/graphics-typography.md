# Typography for Developers — Web Fonts, Variable Fonts, Loading, OpenType

## Typeface Anatomy and Classification

### Typeface Anatomy

Understanding the structure of type enables intentional font selection and design.

**Baseline**: The imaginary line on which most letters sit. Descenders (g, p, q, y) drop below.

**x-height**: Height of lowercase letters like 'x'. A key metric affecting readability at small sizes. Fonts with large x-heights appear larger at the same point size.

**Cap height**: Height of uppercase letters. Usually higher than x-height.

**Ascenders** (b, d, h, k, l): Rise above x-height but not to cap height (typically 80–90% of cap height).

**Descenders** (g, j, p, q, y): Drop below baseline.

**Stroke**: Thin or thick parts of letter forms. The overall thickness variation is **contrast**.

**Serifs**: Small lines extending from letter ends. Serifs vs. sans-serifs define broad families.

**Kerning**: Horizontal spacing between specific letter pairs (e.g., AV, To). Adjusted to account for optical spacing differences based on letter shapes.

**Tracking**: Uniform spacing across all characters, adjusted globally.

**Leading** (often spelled "leading" from "lead" metal spacers in print): Vertical distance between lines (baseline to baseline). Larger leading aids readability, especially for body text and small x-heights.

### Type Classification

**Serif**: Fonts with finishing lines. Associated with print and tradition. Examples: Times New Roman, Garamond, Georgia.
- **Old Style** (e.g., Garamond): Low contrast, angled serifs, humanistic proportions.
- **Transitional** (e.g., Baskerville): Medium contrast, straighter serifs, more regularity.
- **Didone** (e.g., Bodoni): High contrast, thin horizontal serifs, geometric.

**Sans-serif**: No serifs. Modern, clean. Examples: Arial, Helvetica, Inter, Roboto.
- **Humanist** (e.g., Gill Sans, Segoe UI): Organic feel, varied stroke widths.
- **Geometric** (e.g., Futura, Montserrat): Mathematical forms, limited stroke variation.
- **Monospace** (e.g., Courier, IBM Plex Mono): Fixed width, each character occupies equal space. Essential for code.

**Monospace considerations**: Line height is critical. Proportional spacing in terminals makes code harder to scan. Most monospaces are humanist or geometric, and weights range widely; choose based on code context (terminal vs. highlighted in docs).

## Web Fonts: Format and Loading

### Font Formats

**TrueType (.ttf)**: Quadratic outlines, hinting instructions. Widely supported but larger file sizes.

**OpenType (.otf)**: Extends TrueType with cubic outlines and advanced typographic features. Most modern fonts are OpenType variants.

**WOFF (Web Open Font Format)**: Compressed TrueType/OpenType wrapper. Reduces file size by ~25–30%. Widely supported. **de facto standard** for web.

**WOFF2**: Improved compression, ~30% smaller than WOFF. Supported on modern browsers (IE 11 and below don't support). Preferred for new projects.

**EOT (Embedded OpenType)**: Proprietary Microsoft format for older IE. Obsolete.

**Format selection for web**:
```css
@font-face {
  font-family: 'MyFont';
  src: url('myfont.woff2') format('woff2'),
       url('myfont.woff') format('woff');
}
```

Include WOFF2 first (preferred, modern), WOFF as fallback. Skip formats older than WOFF in new projects.

### Font-Display Property

The `font-display` descriptor controls how a font behaves while loading:

```css
@font-face {
  font-family: 'MyFont';
  src: url('myfont.woff2') format('woff2');
  font-display: swap;
}
```

**Values**:
- **auto** (default): Browser-dependent behavior. Usually waits up to 3 seconds before rendering fallback.
- **block**: Invisible text while font loads (up to 3 seconds). Then swaps to loaded font. Creates **FOIT** (flash of invisible text).
- **swap**: Fallback renders immediately. Swaps to loaded font when ready. Creates **FOUT** (flash of unstyled text) — usually preferred.
- **fallback**: Invisible briefly (~100ms), then fallback. Swaps if font arrives within ~3 seconds.
- **optional**: Invisible briefly, then fallback (like fallback). Doesn't swap after 3 seconds; uses fallback permanently if not loaded in time.

**Best practice for body text**: Use `font-display: swap`. Users see text immediately; slight redesign when font loads is acceptable.

**For display fonts** (headings): Use `font-display: block` if font is critical to branding.

## Variable Fonts

**Variable fonts** encode multiple font weights/widths/styles in a single file via **axes** — continuous ranges of variation.

### Axes

**Standard axes** (registered by OpenType):
- **wght (Weight)**: 100–900 (or custom range, e.g., 200–800).
- **wdth (Width)**: 50–200 (percentage of normal width).
- **ital (Italic)**: 0 (normal) to 1 (italic).
- **opsz (Optical Size)**: Font metrics optimized for different display sizes (e.g., 8-14pt, 14-72pt).
- **GRAD (Grade)**: Subtle weight changes without changing metrics (useful for responsive sizing).

**Custom axes** (user-defined): Foundries create domain-specific axes, e.g., Roboto Flex has XTRA (width), YTAD (y-transparent ascenders/descenders), etc.

### Advantages

- **Single file vs. multiple**: Instead of loading `font-400.woff2`, `font-700.woff2`, `font-700i.woff2` (3 files), one variable font covers the range.
- **Responsive**: Interpolate weights smoothly based on viewport or context.
- **File size**: Despite containing all weights, variable fonts are often smaller than 2-3 static counterparts.

### CSS Usage

```css
@font-face {
  font-family: 'MyVarFont';
  src: url('myfont[wght].woff2') format('woff2');
  font-weight: 100 900; /* Supported range */
}

/* Interpolate weight continuously */
h1 { font-weight: 600; }
h2 { font-weight: 550; }
p { font-weight: 400; }

/* Responsive weight */
@media (max-width: 640px) {
  body { font-weight: 500; } /* Slightly bolder for legibility on small screens */
}

/* Custom axis interpolation */
@supports (font-variation-settings: 'XTRA' 0) {
  .wide { font-variation-settings: 'XTRA' 20; }
}
```

**`font-variation-settings`**: Low-level access to axes. Use conservatively; prefer `font-weight`, `font-width`, `font-style` when available.

## OpenType Features

**OpenType features** (tags like `liga`, `smcp`, `ss01`) enable typographic refinements:

**Common features**:
- **liga**: Ligatures (fi, fl, ff combine into special glyphs for improved spacing and aesthetics).
- **dlig**: Discretionary ligatures (rarer, contextual: ct ligature).
- **smcp**: Small capitals (capitals at x-height; semantically different from scaled capitals).
- **c2sc**: Caps to Small Capitals (converts uppercase to small caps in all-caps text).
- **lnum**: Lining figures (numbers at cap height, no descenders).
- **onum**: Old-style figures (varying heights, descenders for some digits; traditionally used in body text).
- **tnum**: Tabular figures (monospaced numbers for tables/financial data).
- **ss01, ss02, ...**: Stylistic sets (font-specific alternatives, e.g., different ampersand styles).

### CSS Activation

```css
/* Standard properties */
body {
  font-feature-settings: 'liga' on, 'dlig' on;
}

p {
  font-variant-numeric: lining-nums;
}

/* New CSS syntax (preferred) */
body {
  font-variant-ligatures: common-ligatures;
  font-variant-numeric: oldstyle-nums;
}

/* Contextual alternates */
body {
  font-feature-settings: 'calt' on;
}

/* Stylistic sets */
h1 {
  font-feature-settings: 'ss01' on; /* Font-specific variant */
}
```

**Caution**: Not all fonts support all features. Test in real environments. Older browsers ignore unsupported properties gracefully (fallback to default).

## Font Loading Strategies

### Prioritization

**High priority** (loaded early):
- Body text font (used extensively).
- Primary sans-serif or serif for main navigation.

**Medium/low priority**:
- Display/decorative fonts for headings.
- Fallback fonts (system stack).

### Strategies

**1. System Font Stack** (fastest, no network)
```css
body {
  font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
}
```

Browsers substitute with native defaults. No web font delay, but less brand control.

**2. Preload + Swap**
```html
<link rel="preload" as="font" href="/fonts/myfont.woff2" type="font/woff2" crossorigin>
```

```css
@font-face {
  font-family: 'MyFont';
  src: url('/fonts/myfont.woff2') format('woff2');
  font-display: swap;
}
```

Network request starts immediately. Fallback renders while loading. Preferred for most cases.

**3. Font Loading API** (JavaScript control)
```javascript
const font = new FontFace('MyFont', 'url(/fonts/myfont.woff2)');
document.fonts.add(font);
font.load().then(() => {
  // Font ready, can now use it
  document.body.style.fontFamily = 'MyFont, sans-serif';
});
```

Enables complex logic: load only fonts needed for current viewport, defer secondary fonts, track loading metrics.

**4. Subset Distribution**
```css
@font-face {
  font-family: 'MyFont';
  src: url('myfont-latin.woff2') format('woff2');
  unicode-range: U+0000-U+00FF; /* Latin + extended */
}

@font-face {
  font-family: 'MyFont';
  src: url('myfont-cjk.woff2') format('woff2');
  unicode-range: U+4E00-U+9FFF; /* CJK */
}
```

Multiple `@font-face` rules, each covering a `unicode-range`. Browsers load only the required subset. Dramatically reduces file size for multilingual sites.

### Performance Metrics

- **FOUT duration**: Time between fallback render and font swap. ~100–500ms typical.
- **Font request size**: WOFF2 compression is critical. A 400-weight sans-serif is typically 12–25KB; a full variable font (400–700 weight) is 20–40KB.
- **Network latency**: 3G vs. 5G impacts significantly. Preload in `<head>` to start requests early.

## Fluid Typography

**Fluid typography** scales smoothly across viewport widths, avoiding discrete breakpoints.

### CSS Approaches

**1. Calc** (moderate support)
```css
body {
  font-size: calc(16px + (20 - 16) * ((100vw - 320px) / (1280 - 320)));
}
```

Scales from 16px at 320px viewport to 20px at 1280px.

**2. CSS Clamp** (modern, recommended)
```css
body {
  font-size: clamp(16px, 2.5vw, 24px);
}
```

Scales with 2.5% of viewport width, clamped between 16px (min) and 24px (max). Cleaner syntax.

### Variable Fonts + Fluid Typography

Combine optical size axis with adaptive viewport-based weight:

```css
body {
  font-size: clamp(14px, 2vw, 18px);
  font-weight: clamp(400, 200 + 300 * ((100vw - 320px) / (1280 - 320)), 700);
}
```

Larger screens get heavier text (improved readability at distance) and larger size simultaneously.

## Font Subsetting and Optimization

### Subsetting Strategies

**Language-based**: Load only glyphs for the target language. Latin (U+0000–U+00FF) is ~256 glyphs; full Unicode is millions. Subsetting to a single language reduces file size by 40–70%.

**Character-based**: Identify unique characters in content (via script analysis) and load only those. Extreme subsetting, but requires dynamic subsetting per page.

**Variable Font Subsetting**: Subset both glyphs AND font axes. Remove unused widths or weights.

### Tools

- **fonttools** (Python): `pyftsubset font.ttf --unicodes=U+0000-U+00FF`
- **Google Fonts API**: Automatically subsetting via `&subset=latin` query parameter.
- **Glyphhanger** (Node.js): Extract glyphs from HTML, generate optimized subset.

### Best Practice

1. **Subset to language**: Start with language-level subsetting (Latin, Cyrillic, Greek).
2. **Profile actual usage**: Measure which characters are displayed on your site.
3. **Variable fonts first**: Reduce number of faces before subsetting.
4. **Monitor file size**: Aim for single-weight, single-width font files under 30KB.

## Web Font Hosting and CDNs

**Google Fonts**: Provides free, pre-subsetted fonts, automatic format detection, and CDN delivery.
```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600&display=swap" rel="stylesheet">
```

**Self-hosted**: Download WOFF2 variants, serve from your CDN. More control, but manual subsetting and updates required.

**Trade-off**: Google Fonts = convenience + privacy trade (tracking); self-hosted = privacy + operational burden.

## Modern Web Typography Workflow

1. Select 2–3 font families (body, display, mono). Prefer open-source or licensed web-font providers.
2. Choose formats: WOFF2 (primary), WOFF (fallback).
3. Subset early: Language → character set.
4. Set `font-display: swap` for body, `block` for critical display fonts.
5. Use `preload` for above-the-fold fonts.
6. Apply OpenType features (`liga`, `lnum`) matching design intent.
7. Test on low-bandwidth connections (DevTools throttling).
8. Monitor Core Web Vitals; ensure CLS (cumulative layout shift) from font loading stays < 0.1.

## See Also

- **graphics-color-theory.md** — Color spaces and contrast.
- **design-color-typography.md** — UI typography applied.
- **performance-web-vitals.md** — CLS, LCP, font impact on metrics.
- **web-browser-rendering.md** — Font rendering pipeline.