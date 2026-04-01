# Web Font Performance: FOIT, FOUT, and Loading Strategies

## Font Rendering Phases: FOIT vs FOUT

When a web font loads asynchronously, the browser must choose: show invisible text (FOIT) or show fallback (FOUT) until the font arrives.

### Flash of Invisible Text (FOIT)

The browser waits for the font, rendering nothing for 3 seconds (default timeout). If font arrives within timeout, it renders. If timeout elapses, fallback displays.

```
Timeline: 0ms (request) ——— 3000ms (timeout) ——— 2500ms (font loaded, too late)
Result: 3 second blank, then fallback renders
```

Pros: No jarring visual shift (no layout reflow when swapping fonts)
Cons: Users see blank content; perceived slowness increases

### Flash of Unstyled Text (FOUT)

The browser immediately renders fallback font, then swaps to web font when ready (no timeout).

```
Timeline: 0ms (fallback renders) ——— 500ms (font loaded) ——— swap to webfont
Result: Quick content visible, smooth swap
```

Pros: Content visible immediately; better perceived performance
Cons: Brief visual difference if fallbacks differ in width/height

## Font-Display: Controlling Rendering Behavior

`font-display` property (in `@font-face`) explicitly chooses the strategy:

```css
@font-face {
  font-family: "MyFont";
  src: url("myfont.woff2") format("woff2");
  font-display: swap;  /* Immediate fallback, swap when ready */
}
```

### Font-Display Values

| Value | Behavior | Use Case |
|-------|----------|----------|
| `auto` | Browser decides (usually FOIT with 3s timeout) | Default; no optimization |
| `block` | 3s FOIT, then fallback forever (no swap) | Rare; only if font is critical |
| `swap` | Immediate FOUT, swap when ready | **Recommended for most fonts** |
| `fallback` | 100ms FOIT, then FOUT, swap within 3s | Balanced; slower than swap |
| `optional` | 100ms FOIT, then FOUT, don't swap | Font is truly optional (best perceived perf) |

**Recommendation**: Use `font-display: swap` for fonts critical to design (headings), `optional` for supplementary fonts (decorative).

```css
/* Heading font: prioritize visible content */
@font-face {
  font-family: "Heading";
  src: url("heading.woff2");
  font-display: swap;
}

/* Supplementary icon font: don't block render */
@font-face {
  font-family: "Icons";
  src: url("icons.woff2");
  font-display: optional;
}
```

## Preloading and Preconnect

Optimize font loading latency with resource hints:

### Preload: Fetch Early

```html
<link rel="preload" href="/fonts/myfont.woff2" as="font" type="font/woff2" crossorigin>
```

Tells browser to fetch font as high-priority resource immediately, before CSS parsing. Reduces font download latency by ~100-200ms in HTTP/1.1 and HTTP/2.

### Preconnect: Warm Up Connection

```html
<link rel="preconnect" href="https://fonts.googleapis.com" crossorigin>
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
```

Establishes TCP + TLS handshake to font server early, reducing connection setup time.

**Avoid preloading too many fonts** (> 3-4). Each preload is a top-level request with overhead. Prioritize critical fonts only.

### Resource Hints Guideline

```html
<!-- Best practice for third-party fonts -->
<link rel="preconnect" href="https://fonts.googleapis.com" crossorigin>
<link rel="preload" href="https://fonts.gstatic.com/s/roboto/v30/KFOmCnqEu92Fr1Mu4mxK.woff2" 
      as="font" type="font/woff2" crossorigin>
```

## Font Subsetting: Unicode-Range and Pyftsubset

Web fonts often include glyphs for many languages, bloating file size. Subsetting removes unused characters.

### Unicode-Range: Browser-Based Subsetting

`unicode-range` tells the browser when to use a font:

```css
@font-face {
  font-family: "RobotoLatin";
  src: url("roboto-latin.woff2");
  unicode-range: U+0-10FFFF;  /* All Unicode */
}

@font-face {
  font-family: "RobotoLatin";
  src: url("roboto-latin-ext.woff2");
  unicode-range: U+0100-017E;  /* Latin Extended-A */
}

/* Browser loads roboto-latin by default, adds -ext only if needed */
```

Define multiple `@font-face` rules for the same family with different unicode-range values:

```css
@font-face {
  font-family: "Roboto";
  src: url("roboto-400-latin.woff2");
  font-weight: 400;
  unicode-range: U+0020-007E;  /* ASCII + common symbols */
}

@font-face {
  font-family: "Roboto";
  src: url("roboto-400-latin-ext.woff2");
  font-weight: 400;
  unicode-range: U+0100-017F;  /* Latin Extended-A + B */
}
```

### Pyftsubset: Offline File Reduction

Tools like `pyftsubset` (from fonttools) cut fonts offline:

```bash
pip install fonttools
pyftsubset myfont.woff2 --unicodes=U+0020-007E,U+0100-017F
pyftsubset myfont.woff2 --unicodes=U+0020-007E --flavor=woff2 --output-file=myfont-subset.woff2
```

Reduces file size by 50-80% for most projects (removing CJK, less-common scripts).

## Variable Fonts: Axes and Instances

Variable fonts encode multiple font weights/styles in one file, using named axes:

```css
@font-face {
  font-family: "Roboto Flex";
  src: url("RobotoFlex[opsz,wdth,wght].woff2");
  font-weight: 100 900;  /* Supports range 100–900 */
  font-style: normal italic;
}

.text {
  font-weight: 500;  /* Interpolated from variable axis */
  font-variation-settings: "wdth" 100, "opsz" 16;  /* Custom axis control */
}
```

**Axes** (custom properties):
- `wght`: Weight (typically 100–900)
- `wdth`: Width (condensed–expanded)
- `opsz`: Optical size (adjustment for small vs large display)
- Custom axes defined by font designers

**Advantages**:
- Single file supports infinite weights/styles (smaller than 4-5 separate files)
- Smooth interpolation between instances
- Reduced HTTP requests

**Disadvantages**:
- Single file larger than most individual weights (but smaller than full family)
- Browser support: Good (95%+ modern browsers)

Example: Google Fonts variable font:

```html
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@100..900&display=swap" rel="stylesheet">

<style>
  body { font-family: Inter, sans-serif; }
  .light { font-weight: 300; }
  .bold { font-weight: 700; }
  .custom { font-variation-settings: "wght" 450; }  /* Any value, not just standard weights */
</style>
```

## Self-Hosting vs CDN

### Self-Hosting

Fonts served from your domain:

```css
@font-face {
  font-family: "MyFont";
  src: url("/fonts/myfont.woff2");
  font-display: swap;
}
```

**Pros**:
- Full control over caching headers, compression
- No third-party latency
- Works offline

**Cons**:
- Your infra handles requests and bandwidth
- No global CDN-based distribution
- More ops burden

### CDN (Google Fonts, Typekit)

```html
<link href="https://fonts.googleapis.com/css2?family=Roboto:wght@400;700&display=swap" rel="stylesheet">
```

**Pros**:
- Global CDN (lower latency for users worldwide)
- Caching benefits (shared across sites using same font)
- Automatic updates (new font versions deployed)

**Cons**:
- Third-party dependency
- Less control over headers
- Privacy: CDN logs font requests

**Hybrid**: Preconnect to CDN `googleapis.com`, preload specific font file from `gstatic.com` (CDN's static domain).

```html
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
```

## Fallback Font Matching and Size-Adjust

Font fallbacks can cause layout shift if widths/heights differ from web font. Minimize reflow with size-adjust:

```css
@font-face {
  font-family: "MyFont";
  src: url("myfont.woff2");
  font-display: swap;
  size-adjust: 90%;  /* Scale fallback to match web font dimensions */
}

body {
  font-family: "MyFont", Georgia, serif;  /* Georgia is fallback */
}
```

When Georgia renders temporarily, it's scaled to 90% to approximate MyFont's metrics. Reduces CLS (Cumulative Layout Shift) when swap occurs.

### Font Metrics Override

For fine-grained control, specify fallback metrics explicitly (CSS Fonts Module Level 5):

```css
@font-face {
  font-family: "MyFont";
  src: url("myfont.woff2");
  ascent-override: 110%;
  descent-override: 20%;
  line-gap-override: 0%;
}
```

Aligns ascent/descent/line-gap of fallback font with web font, reducing visible shift.

## Variable Font Axes and Instances

Instances are *saved styles* within a variable font:

```css
/* Roboto has instances: Thin, Light, Regular, Bold, Black */
@font-face {
  font-family: "Roboto";
  src: url("Roboto[wght].woff2");
  font-weight: 100 900;
}

/* Instance: Light = wght 300 */
.light { font-weight: 300; }

/* Custom interpolation */
.semi-bold { font-weight: 650; }  /* Between Bold (700) and Normal (400) */
```

If only specific instances are used, subsetting the variable font to include only those axes/ranges reduces file size further.

## Performance Metrics and Best Practices

### Web Vitals Impact

Fonts affect **CLS (Cumulative Layout Shift)** via fallback → webfont swap. Strategies to reduce CLS:

1. **Use `font-display: swap`**: FOUT perceived as less jarring than blank text
2. **Match fallback metrics**: Minimize width/height differences
3. **Preload critical fonts**: Ensure timely loading
4. **Subset: Reduce download time, faster swap

### Best Practice Checklist

- [ ] Use `font-display: swap` for displayed fonts, `optional` for supplementary
- [ ] Preload ≤ 2 critical fonts; preconnect to CDN
- [ ] Subset fonts to used character set (Latin only ≠ full Unicode)
- [ ] Use variable fonts if multiple weights/styles needed (1 file vs 4-5)
- [ ] Self-host if font is custom; use CDN for public fonts (caching benefit)
- [ ] Apply `size-adjust` or font metrics override to match fallback to webfont
- [ ] Monitor CLS; aim < 0.1
- [ ] Avoid preloading > 3 fonts (diminishing returns, request overhead)

### Example: Optimized Setup

```html
<head>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  
  <!-- Critical heading font -->
  <link rel="preload" href="https://fonts.gstatic.com/s/inter/v13/UcC73FwrK3iLTeHuS_fNSA.woff2" 
        as="font" type="font/woff2" crossorigin>
  
  <!-- Google Fonts stylesheet -->
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;700&display=swap" rel="stylesheet">
  
  <style>
    body { font-family: "Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    h1 { font-weight: 700; }
  </style>
</head>
```

## Summary

Web font performance balances rendering speed (FOIT/FOUT via `font-display`), loading latency (preload/preconnect), and file size (subsetting, variable fonts). Use `swap` to prioritize visible content; preload critical fonts; subset to used characters; prefer variable fonts for multiple weights. Minimize layout shift via size-adjust and fallback matching. Combine strategies for optimal CLS and perceived performance on all devices.