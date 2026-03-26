# Color and Typography in UI — Theory, Contrast, Readability & Implementation

## Color Theory Essentials

Color in UI serves both functional (status indication, grouping) and affective purposes (brand, emotion). Understanding color behavior prevents common mistakes.

### Hue, Saturation, Lightness

HSL (Hue, Saturation, Lightness) separates color into three dimensions:

- **Hue (0-360°)**: The color itself (red, blue, yellow). Perceived as going around a circle.
- **Saturation (0-100%)**: Purity of the color. 100% is vivid; 0% is gray.
- **Lightness (0-100%)**: Brightness. 0% is black; 100% is white.

HSL is more intuitive for design than RGB (Red, Green, Blue) because adjusting lightness or saturation is straightforward. `hsl(210, 100%, 50%)` is bright blue; `hsl(210, 50%, 50%)` is muted blue; `hsl(210, 100%, 70%)` is light blue.

However, HSL has perceptual limitations — changing lightness by 10% doesn't feel equally bright across hues. Red feels darker than blue at the same lightness.

### Perceptual Color Spaces

Modern web development increasingly uses perceptually uniform color spaces:

- **Lab**: Separates lightness (L) from color (a, b axes). Equal changes in Lab look equally changeful to human eyes.
- **LCH**: The cylindrical version of Lab (Lightness, Chroma, Hue). More intuitive than Lab cartesian coordinates.
- **Oklab/OkLCH**: Fixes some Lab perceptual issues; simpler math; increasingly supported in browsers (2024+).

sRGB (the web default) is device-centric, not human-centric. Switching to OkLCH for component generation and theming is becoming standard:

```css
/* In OkLCH, lightness changes feel uniform */
.button {
  background: oklch(60% 0.2 220); /* Blue */
}
.button:hover {
  background: oklch(50% 0.2 220); /* Darker blue, perceptually consistent darkening */
}
```

### Wide Color Gamut (P3)

Most web content targets sRGB, but displays increasingly support P3 (covers ~50% more colors). P3 can represent brighter, more saturated colors outside sRGB.

```css
@supports (color: color(display-p3 1 0 0)) {
  .brand {
    color: color(display-p3 1 0.2 0.3); /* Brighter, more saturated than sRGB equivalent */
  }
}
```

Use P3 for brand colors and gradients where the extra vibrancy matters. Fallback to sRGB for browsers that don't support it.

## Contrast Ratios — WCAG vs. APCA

Contrast is critical for readability, especially for low-vision users. The standard has evolved as our understanding improved.

### WCAG Contrast Ratio

WCAG defines contrast ratio as the formula: `(L1 + 0.05) / (L2 + 0.05)` where L is relative luminance (`L = 0.2126 * R + 0.7152 * G + 0.0722 * B`).

Thresholds (WCAG 2.1):

| Level | Body Text | Large Text (≥18px or ≥14px bold) |
|-------|-----------|----------------------------------|
| AA    | 4.5:1     | 3:1                              |
| AAA   | 7:1       | 4.5:1                            |

The formula treats all colors as equally important — green weighted 0.7, red 0.2, blue 0.07 — because human eyes are more sensitive to green differences.

### APCA — Advanced Perceptual Contrast Algorithm

WCAG ratios are based on research from the 1980s. APCA (part of WCAG 3.0 working draft, 2024) uses modern perceptual science and font-size awareness.

Key differences:

- **Font size matters**: Small text requires higher contrast; large text can use lower
- **Bidirectional**: Dark text on light differs from light on dark (not symmetric like WCAG ratios)
- **Polarity**: APCA uses Lc values ranging from roughly -108 (light text on black) to +108 (dark on white)

APCA levels:

| Lc    | Use                           |
|-------|-------------------------------|
| 90    | Body text, required for access |
| 75    | Medium text, secondary content |
| 60    | Large text, UI controls       |
| 45    | Graphical objects, icons      |
| 30    | Disabled, placeholder text    |

APCA is more nuanced: the same color pair might have a 4.5:1 WCAG ratio but Lc 75 in APCA (perfectly accessible for body text in one, not in the other depending on size).

### Which to Use?

**WCAG 2.1**: Established, widely required by regulation, understood by teams. Safe default.

**APCA**: More accurate, better for designers and developers who understand the science. Increasingly adopted by modern design systems. Not yet a legal requirement (WCAG 3.0 still working draft).

Many teams now publish contrast using both metrics. Tools like WebAIM Contrast Checker show both values.

## Color Blindness and Vision Differences

Approximately 8% of men and 0.5% of women have color vision deficiency. Designing for color blindness means:

1. **Don't use color alone** to convey meaning (red = error, green = success). Add icons, text, patterns.
2. **Choose accessible color pairs**: Some combinations are harder for color-blind users to distinguish.

Common types and their confusions:

- **Protanopia** (~1%): Red/green confusion; reds appear dark, greens appear yellow
- **Deuteranopia** (~1%): Red/green confusion; greens appear brownish, reds appear orange
- **Tritanopia** (rare): Blue/yellow confusion; blues appear pink, yellows appear light

Test designs with color-blind simulators (Coblis, Color Universal Design). A simple rule: if your dashboard's only red-green signals are "passed" (green) and "failed" (red), color-blind users can't tell them apart. Add icons or text: ✓ Passed, ✗ Failed.

## Typographic Scale and Vertical Rhythm

Typography without system feels arbitrary. Most disciplined designs use scales.

### Modular Scale

A modular scale uses a ratio applied repeatedly. Common ratios:

- **1.125 (minor second)**: Subtle, for small projects
- **1.25 (major third)**: Classic, common
- **1.5 (perfect fifth)**: Bold, high contrast
- **1.618 (golden ratio)**: Mathematically pleasing, but rarely noticed

Starting from 16px body text with 1.25× ratio:

```
Body: 16px
Small: 16 × 1.25 = 20px
Normal: 16px
Large: 16 ÷ 1.25 = 12.8px (rounded: 13px)
H3: 20 × 1.25 = 25px
H2: 25 × 1.25 = 31.25px (rounded: 32px)
H1: 32 × 1.25 = 40px
```

The scale creates visual hierarchy without randomness. Designers often adjust slightly (bump h1 to 48px for presence), but the scale provides foundation.

### Vertical Rhythm

Vertical rhythm aligns all elements to a baseline grid (often 4px or 8px). Margins, padding, line height all use grid multiples.

```css
:root {
  --baseline: 8px;
}
body {
  line-height: 1.5; /* Usually 1.5× for body text readability */
}
.card {
  padding: calc(var(--baseline) * 2);
  margin-bottom: calc(var(--baseline) * 3);
}
```

Vertical rhythm creates calm, organized layouts. It's slightly less critical now that we have flexbox and grid, but still a useful discipline.

## Font Loading and Performance

How and when font files load affects performance and visual stability.

### FOIT vs. FOUT

- **FOIT (Flash of Invisible Text)**: Browser waits for custom font; renders nothing temporarily, then displays text. Avoids ugly fallback font but causes blank space.
- **FOUT (Flash of Unstyled Text)**: Browser shows fallback font immediately, swaps to custom font when loaded. Smooth experience, but users see the font change.

```css
body {
  font-family: 'Custom Font', Georgia, serif;
}
```

Browsers default to FOIT (2-3s timeout, then show fallback). CSS `font-display` property controls behavior:

```css
@font-face {
  font-family: 'Custom Font';
  src: url('font.woff2') format('woff2');
  font-display: swap; /* FOUT: show fallback immediately, swap when ready */
}
```

`swap` is usually best for performance (avoid blank text). `fallback` waits shorter (100ms), useful for decorative fonts.

### Variable Fonts

Variable fonts combine multiple weights/styles in one file. Instead of four files (regular, italic, bold, bold-italic), one file covers the range:

```css
@font-face {
  font-family: 'Inter Var';
  src: url('inter-var.woff2') format('woff2-variations');
}
body {
  font-family: 'Inter Var';
  font-weight: 400;
}
.bold {
  font-weight: 700;
}
```

Variable fonts reduce HTTP requests and file size, especially when using many weights. Most modern fonts offer variable versions; adoption is increasing.

### System Fonts as Strategy

Using system fonts (fonts already installed on the user's OS) eliminates download time:

```css
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica Neue', sans-serif;
}
```

The stack starts with OS-specific fonts (San Francisco on macOS, Segoe UI on Windows) and falls back to generic serif/sans-serif. Many products (GitHub, Slack) use system fonts for speed and native feel.

## Readability and Legibility

Readability (comprehension of content) and legibility (how easily you can distinguish letters) are distinct.

### Line Length

Very long lines (>100 characters) are harder to track; short lines require frequent wrapping. Optimal line length is 50-75 characters for body text.

```css
.article {
  max-width: 65ch; /* 65 characters at current font size */
}
```

Use `ch` units to define line length in characters (more semantic than arbitrary pixel widths).

### Line Height and Letter Spacing

Line height affects readability more than most realize:

- **Too tight** (1.2 or less): Dense, hard to track
- **Optimal** (1.4-1.6 for body): Readable, not spacious
- **Too loose** (2+): Disconnects lines

Letter spacing also matters — increased letter spacing helps low-vision and dyslexic readers:

```css
body {
  line-height: 1.6;
  letter-spacing: 0.02em;
}
```

### Alignment and Justification

- **Left-aligned**: Ragged right edge, natural rhythm, easier to read
- **Right-aligned**: Comparatively rare (headers, poetry)
- **Centered**: Use sparingly (headers, poetry, short texts only)
- **Justified**: Hyphenation creates even edges but adds complexity; not recommended for web (hyphens are fragile)

Justified text with orphans (single words wrapping to final line) or widows (single words at top of column) looks wrong and is harder to read.

## Dark Mode and Color Adaptation

`prefers-color-scheme` detects OS dark mode preference:

```css
@media (prefers-color-scheme: dark) {
  body {
    background-color: #1a1a1a;
    color: #f0f0f0;
  }
}
```

Dark mode isn't simply inverting colors:

- **Contrast changes**: White text on black (9:1 contrast) is high but can cause halation (perceived glow). Slightly reduced contrast (e.g., #e0e0e0 on #1a1a1a) is often more comfortable.
- **Saturation**: Colors feel oversaturated on dark backgrounds; desaturate slightly.
- **Accent colors**: Brand colors often need adjustment; blues brighter in dark mode, reds darker.

Test dark mode seriously — not a checkbox feature but a complete visual redesign using different contrast and saturation principles.

## See Also

- web-accessibility.md — Broader accessibility including contrast requirements
- design-systems.md — Token systems for managing color and typography consistently
- design-responsive.md — Responsive typography and fluid layouts