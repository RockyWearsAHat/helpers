# Color Theory for Engineers — Color Spaces, Gamma, Perceptual Uniformity, Accessibility

## Color Spaces: Representation and Purpose

Color spaces are mathematical models for representing colors as tuples of values. Each space makes different aspects of color manipulation convenient or efficient.

### RGB (Red-Green-Blue)

**RGB** is the most common digital representation: three channels (R, G, B), each 0–255 in 8-bit or 0.0–1.0 normalized. It reflects hardware directly: screens emit red, green, and blue light, mixed additively.

RGB is **device-dependent** — the same RGB(255, 0, 0) appears different on two monitors with different phosphor spectra. Modern workflow: specify colors in an absolute color space (sRGB, Display P3, Rec. 2020), then convert to device RGB at display time.

**Limitations**: Non-uniform perceptually — a step of 10 in red differs visually from a step of 10 in blue or green. No separation of brightness from hue, making adjustments unintuitive.

### HSL and HSV

**HSL (Hue-Saturation-Lightness)** and **HSV (Hue-Saturation-Value)** are cylindrical models derived from RGB. Both separate hue (0–360°) from intensity.

- **H (Hue)**: 0–360°, the dominant wavelength. Conceptually a wheel: red → yellow → green → cyan → blue → magenta → red.
- **S (Saturation)**: 0–100%, intensity of the hue. 0% = gray (no color), 100% = pure hue.
- **L/V**: **Lightness** (HSL) or **Value** (HSV). HSV = percentage of white mixed with the hue. HSL = percentage of brightness independently.

**Difference**: HSV(100%, 100%, 100%) is white; HSL(*, *, 100%) is always white. HSL's thirds (0%, 50%, 100%) are black, mid-tone, and white respectively, making it slightly more intuitive for designers. However, HSL's *saturation* becomes unintuitive near the extremes of lightness.

**Use case**: Intuitive for pickers and manipulations in design tools. Rarely used in serious color work because the perceptual uniformity is still poor.

### Lab and LCH

**Lab** (CIELAB) separates color into three orthogonal dimensions:

- **L (Lightness)**: 0–100, perceptually uniform in human vision. L=50 is perceived as halfway between black and white.
- **a (Axis)**: Green (negative) to red (positive).
- **b (Axis)**: Blue (negative) to yellow (positive).

Lab is **device-independent** and **perceptually uniform** — equal steps in Lab ≈ equal perceptual differences. This makes Lab ideal for algorithmic color manipulation, interpolation, and accessibility calculations.

**LCH** (cylindrical Lab): L as above, C (Chroma) = saturation-like, H (Hue) = angle 0–360°. Easier to reason about than rectangular Lab. C=0 is always gray, H is undefined at C=0.

**Modern context**: CSS supports Lab and LCH via `color(lab ...)` and `color(lch ...)`. Modern color contrast calculations (APCA) use Lab-derived metrics.

### Standard Color Spaces: sRGB, Display P3, Rec. 2020

Colors are specified relative to a **color gamut** — the set of real colors achievable by a display.

**sRGB**: The web standard. Defined by IEC 61966-2-1. Smaller gamut: covers ~33% of visible color space. Assumed by browsers unless explicitly overridden. **Use**: legacy web content, guaranteed compatibility.

**Display P3**: Larger gamut (~50% of visible). Common on mid-range phones and newer monitors. More saturated colors possible, especially greens and reds. **Use**: modern web content, Apple ecosystem.

**Rec. 2020 (BT.2020)**: Ultra-wide gamut used in UHDTV broadcasting. Covers ~75% of visible colors. **Use**: professional broadcast, cinema (less common in web).

**Conversion**: A color specified in one gamut can be mapped to another via **gamut mapping** — typically chroma reduction (desaturate) or lightness adjustment. The mapping is lossy; out-of-gamut colors can't be exactly represented.

**CSS context**: Use `color()` function: `color(display-p3 0.8 0.2 0.4)`. Browsers fall back to sRGB if P3 isn't supported. Use `@supports (color: ...)` to detect capability.

## Gamma Correction

**Gamma** is the power-law relationship between pixel code (e.g., 0–255) and the actual light output of a display.

### The Physics

Displays emit light via phosphors or LEDs. For historical reasons (old CRT tubes), the perceived video level was encoded as voltage^(1/2.2) ≈ voltage^0.45. This exponential curve is called **gamma encoding** with exponent γ ≈ 2.2.

Modern displays still apply this curve. The encoding is intentional: it compresses bright values and expands dark ones, leveraging human vision's logarithmic sensitivity to light. Benefits: better perceptual quantization in 8-bit values, more discrimination in shadows.

### sRGB's Gamma Function

sRGB defines a piecewise function (not pure power-law):

- For linear input L in [0, 1], output V = 1.055 × L^(1/2.4) − 0.055 (for L ≥ 0.0031308)
- Linear segment for very dark values to avoid division by zero.

The effect: a 50% gray code (127 in 0–255) is NOT 50% brightness. To the human eye, it appears darker (~22% brightness). Conversely, to achieve 50% perceived brightness, use ~187 in 8-bit.

### Linear vs. Gamma-Corrected Workflows

**Gamma-corrected** (display space): Pixel values as displayed. Used in all image formats (JPEG, PNG, sRGB TIFF), web images. Computationally efficient for display.

**Linear** (light space): Pixel values proportional to actual light energy. Required for physically-correct rendering (lighting calculations, blending, color manipulation). Light-physics operations **must** happen in linear space.

**Workflow**: 
1. Load sRGB image → convert to linear (apply inverse gamma).
2. Perform lighting, blending, color math in linear.
3. Convert back to sRGB (apply gamma function).
4. Display or save.

**In shaders (WebGL/WGPU)**: Textures are often uploaded as sRGB and automatically converted to linear by the GPU. Output framebuffer should also be sRGB to auto-convert output back. Explicit: `glTexImage2D(..., GL_SRGB)` and `glBindFramebuffer(..., GL_SRGB8)`.

**In CSS/Web**: Blending in CSS happens in uncorrected sRGB space (gamma-corrected). `mix-blend-mode: lighten` is not physically correct but matches user expectations and historical behavior.

## Perceptual Uniformity

A color space is **perceptually uniform** if equal Euclidean distances in the space correspond to equal perceived differences in human vision.

### Why It Matters

In non-uniform spaces (RGB, HSV), Δ RGB(10, 0, 0) (a small red step) looks quite different from Δ RGB(0, 10, 0) (a small green step), even though both are distance 10 in 3D space. This breaks algorithms for color interpolation, palette generation, and contrast calculation.

**Lab and LCH** are approximately perceptually uniform (within ~2–5 JND — just-noticeable-difference — tolerance). They're reference spaces for human perception.

### Application: Color Interpolation

Interpolating between two colors in RGB produces muddy midtones. Interpolating in Lab produces visually balanced transitions. For example:

- RGB interpolation between red and blue passes through grayish-purple midpoints.
- Lab interpolation between the same red and blue produces saturated, vivid purples mid-path.

Best practice: Convert endpoints to Lab, interpolate, convert back to sRGB for display.

### Application: Palette Generation

Algorithms like HSL-based palettes (shift hue, vary lightness) are popular but unreliable. Lab-based algorithms (contours of constant lightness, varied hue and chroma) produce perceptually balanced palettes.

## Color Blindness and Accessibility

**Color blindness** (color vision deficiency, CVD) affects ~4–8% of males and ~0.4% of females (varies by ancestry). Most common: red-green color blindness (protanopia, protanomaly, deuteranopia, deuteranomaly).

### Types

- **Protanopia**: No L-cone (red). Red and yellow look olive; magenta looks blue.
- **Deuteranopia**: No M-cone (green). Red and green both appear brown; similar to protanopia but the red bias is reversed.
- **Tritanopia**: No S-cone (blue). Very rare. Blue looks pink; yellow looks pale pink.
- **Monochromacy**: Cone dysfunction; sees only in grayscale. Ultra-rare.

### Accessibility Principles

1. **Never rely solely on color** to distinguish information. Use color + pattern, color + icon, or color + text label.
2. **Test palette contrast** using simulation: tools like `coblis.org` show how your palette appears under different CVD types.
3. **Use sufficient luminance contrast** independent of hue. WCAG AA requires 4.5:1 for normal text, 3:1 for large text.

### Tools and Workflows

- **CVD simulators**: Coblis, Color Blind Simulation, Colorlab.
- **CSS media query**: `@media (prefers-contrast: more)` or `@media (prefers-color-scheme: dark)` for user-driven adjustments.
- **Programmatic checks**: Libraries like `wcag-contrast` calculate contrast in perceptually uniform space.

## WCAG Contrast and APCA

### WCAG AA/AAA Contrast Ratio

**WCAG 2.1 defines contrast** as the ratio of luminance of the lighter color to the darker:

- **Luminance = 0.2126 × R + 0.7152 × G + 0.0722 × B** (after gamma correction).
- **Ratio = (L1 + 0.05) / (L2 + 0.05)**, where L1 ≥ L2. Range: 1 (identical) to 21 (pure white/black).

**Levels**:
- **AA**: 4.5:1 (normal text), 3:1 (large text).
- **AAA**: 7:1 (normal), 4.5:1 (large).

**Criticism**: The formula is archaic (from the CRT era) and based on averaged human vision. It doesn't account for:
- Font size and weight (small fonts need higher contrast).
- Color categories (red-green is harder to distinguish than red-blue at the same ratio).
- Modern displays and visual impairments.

### APCA (Advanced Perceptual Contrast Algorithm)

APCA is a research-backed replacement in development (not yet WCAG standard). Key differences:

- **Bidirectional**: Treats text foreground and background separately. `APCA(foreground, background)` differs from `APCA(background, foreground)`.
- **Polarity**: Light content on dark vs. dark on light. Dark text on light achieves higher contrast scores than light text on dark at the same luminance difference.
- **Font-aware**: Adjusts for font size and weight (smaller fonts need higher scores).
- **Chroma correction**: Accounts for hue-based perceptual differences.

**Adoption**: Used by major projects (W3C WebAIM), but not mandated in WCAG 2.1 yet. Transitional: Use WCAG for compliance, APCA for better actual accessibility.

## CSS Color Manipulation: `color-mix()`

CSS provides `color-mix()` for blending and transforming colors declaratively:

```css
/* Interpolate between two colors in a specified space */
color-mix(in lab, red 50%, blue 50%)  /* Lab space, 50/50 blend */
color-mix(in lch, #f00 30%, #00f 70%)  /* 30% red, 70% blue in LCH */
color-mix(in srgb, var(--bg) 80%, black 20%)  /* Darken background */
```

**Syntax**: `color-mix(in <colorspace>, <color1> <percentage1>, <color2> <percentage2>)`.

**Interpolation spaces** (MDN live list):
- `srgb`, `display-p3`, `rec2020` (rectangular).
- `hsl`, `hwb`, `lch`, `oklch`, `lab`, `oklab` (polar or axis-aligned).

**Use cases**:
- **Tints and shades**: `color-mix(in lab, color 80%, white 20%)` for consistent tinting.
- **Hover states**: `color-mix(in lch, var(--primary) 90%, black 10%)` for darkening on hover.
- **Accessibility**: Dynamically adjust contrast via `@media (prefers-contrast: more)`.

**Fallback**: Older browsers don't support; use `@supports (color: ...)` feature detection and provide fallback hex colors.

## Modern Color in Web Development

**Best practices**:
1. Specify colors in sRGB, Display P3 (modern), or Lab/LCH (algorithmic).
2. Use Lab for color math; convert to sRGB for display compatibility.
3. Test with CVD simulators and WCAG/APCA contrast tools.
4. Avoid relying on hue alone for UX; add icons or patterns.
5. Provide `prefers-color-scheme` and `prefers-contrast` support.
6. Use `color()` and `color-mix()` with fallbacks for older browsers.

**Tooling**: Color.js, chroma.js, TinyColor2 (JavaScript). CSS preprocessors (SASS) handle older gamut limitations.

## See Also

- **design-color-typography.md** — UI color and typography applied.
- **accessibility-engineering.md** — WCAG framework and ARIA.
- **web-accessibility.md** — Inclusive web design.