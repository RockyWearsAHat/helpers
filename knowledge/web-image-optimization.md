# Web Image Optimization — Formats, Responsive Images, Lazy Loading & Performance

## Image Optimization Fundamentals

Images often represent 50-80% of web page bytes. Optimizing images has the highest return on bandwidth investment.

**Optimization layers:**
1. **Format & compression**: Choose correct format, adjust quality
2. **Responsive images**: Serve correct size per device
3. **Lazy loading**: Defer off-screen images
4. **Placeholders**: Show low-quality while loading
5. **CDN delivery**: Edge caching, transformation

Each layer compounds benefits.

## Image Formats

### JPEG (Joint Photographic Experts Group)

**Characteristics:**
- Lossy compression (quality 0-100)
- Best for photographs, natural scenes
- 8-bit RGB (no transparency)
- Supported everywhere

**Trade-off:** Quality vs. file size. Lower quality = smaller file but visible artifacts.

**Modern JPEG tools:** MozJPEG, libjpeg-turbo improve compression 5-10% over standard JPEG.

### PNG (Portable Network Graphics)

**Characteristics:**
- Lossless compression (all detail preserved)
- 8-bit indexed or 32-bit RGBA
- Transparency support (alpha channel)
- Larger than JPEG for photos

**Use cases:** Screenshots, diagrams, text-heavy graphics, icons (when not SVG).

**Optimization:** PNG crushing tools (pngquant, optipng) reduce file size 10-30% without quality loss.

### WebP (Google)

**Characteristics:**
- Lossy and lossless modes
- ~25-30% smaller than JPEG at same quality
- Supports transparency (alpha channel)
- Supported in all modern browsers (except IE)

**Trade-off:** older browser support requires fallback to JPEG.

**Usage:**
```html
<picture>
  <source srcset="image.webp" type="image/webp">
  <img src="image.jpg" alt="...">
</picture>
```

### AVIF (AV1 Image Format)

**Characteristics:**
- Next-generation format (royalty-free codec)
- 20-50% smaller than JPEG (depending on content)
- Supports lossy, lossless, animation, transparency
- Modern browser support: Chrome 85+, Firefox 93+, Safari 16+

**Trade-off:** Slower encoding than WebP, limited tool support.

**Adoption:** Growing but not yet universal. Use with fallbacks.

### JPEG XL (JXL)

**Characteristics:**
- Advanced format with exceptional compression
- 30-40% better than JPEG
- Progressive decode (resolution improves as bytes arrive)
- Transparency support

**Status:** Emerging standard. Apple dropped Safari support (2023), reducing adoption momentum.

### SVG (Scalable Vector Graphics)

**Characteristics:**
- Vector format (scales infinitely)
- Text-based (CSS/JS manipulable)
- Lossless

**Use cases:** Icons, logos, illustrations, diagrams.

**Advantages:** Perfect for all screen densities, animatable, can be inline.

**Disadvantages:** Slow for complex/photorealistic images.

## Modern Format Selection

**Decision tree:**
- Photograph → AVIF (with WebP fallback, JPEG for older browsers)
- Illustration/diagram → SVG if possible, else AVIF
- Icon → SVG
- Screenshot with text → PNG (or WebP if low transparency) → JPEG fallback
- Animation → Video (MP4, WebM) if large, else animated WebP/AVIF

**File size hierarchy (relative):**
- SVG: smallest for vectors
- WebP lossy: baseline
- AVIF: -30-40%
- JPEG: baseline (larger than WebP by ~25%)
- PNG: largest

## Responsive Images

### Image Sizing Problem

A single image URL doesn't work across devices:
- 1920px desktop: 300KB image
- 768px tablet: 150KB image sufficient
- 375px mobile: 50KB image sufficient

Serving 300KB image to mobile wastes bandwidth.

### Solution: Picture Element + srcset

**srcset with density descriptors:**
```html
<img 
  src="image.jpg"
  srcset="image.jpg 1x, image@2x.jpg 2x, image@3x.jpg 3x"
  alt="..."
>
```

`1x` = normal density (96 DPI)  
`2x` = retina displays (192 DPI)  
`3x` = ultra-high density

Browser chooses based on device pixel ratio.

**Limitation:** Only handles retina, not device width.

### srcset with Width Descriptors (Recommended)

```html
<img
  src="image-small.jpg"
  srcset="
    image-small.jpg 640w,
    image-medium.jpg 960w,
    image-large.jpg 1280w
  "
  alt="..."
>
```

`640w` = image is 640px wide. Browser compares screen width and device pixel ratio, chooses appropriate file.

**Example:**
- 375px mobile @ 2x density = effective 750px viewport
- Browser picks `image-large.jpg` (1280w), which is larger than needed but best match

**sizes attribute:** Hints to browser what rendered width will be:
```html
<img
  srcset="image-small.jpg 640w, image-large.jpg 1280w"
  sizes="(max-width: 600px) 100vw, 50vw"
  alt="..."
>
```

`sizes` says: on small screens, image is 100% viewport width; on larger screens, 50%.

Browser now knows rendered width before fetching, picks optimal image size.

### Picture Element + Source

Best for art direction (cropping changes per device) or format selection:

```html
<picture>
  <source media="(min-width: 960px)" srcset="image-wide-large.webp">
  <source media="(min-width: 600px)" srcset="image-medium.webp">
  <source srcset="image-small.webp">
  
  <!-- Fallback -->
  <img src="image-small.jpg" alt="...">
</picture>
```

Media queries pick source. Allows different aspect ratios or crops per breakpoint.

### Responsive image tools:

**sharp** (Node.js): Generate multiple sizes from one source  
**imagemin**: Batch image compression  
**imgproxy**: On-the-fly image resizing (self-hosted)

## Lazy Loading

Off-screen images delay loading until needed. Saves bandwidth and accelerates initial page load.

### Native Lazy Loading

```html
<img src="..." loading="lazy" alt="...">
```

Browser loads image when it approaches viewport (~1-2 screen lengths before visible).

**Browser support:** Most modern browsers. Fallback graceful (ignores attribute, loads immediately in older browsers).

### Intersection Observer API

Fine-grained control:
```javascript
const images = document.querySelectorAll('img[data-src]');
const observer = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      const img = entry.target;
      img.src = img.dataset.src;
      observer.unobserve(img);
    }
  });
});
images.forEach(img => observer.observe(img));
```

HTML:
```html
<img data-src="image.jpg" src="placeholder.jpg" alt="...">
```

### Lazy Loading Trade-offs

**Benefits:** Faster initial page load, saves bandwidth for unseen images.

**Risks:** 
- Images below fold load late; perceived performance worse if user scrolls immediately
- CLS (Cumulative Layout Shift) if image dimensions aren't reserved

**Best practice:** Reserve space for images with container aspect ratio.

## Blur-Up / LQIP Placeholders

**Low Quality Image Placeholder (LQIP)**: Show tiny blurred placeholder while real image loads.

**Perceived performance:** Looks less broken, feels faster.

### Techniques

**Inline JPEG preview:**
```html
<img
  src="image.jpg"
  style="background-image: url('data:image/jpeg;base64,/9j/4AAQSk...')"
  alt="..."
>
```

Embed tiny JPEG (40-100 bytes) as data URI. Decoded immediately.

**CSS blur + filter:**
```css
img {
  filter: blur(20px);
}
img[data-src] {
  /* while loading */
}
img[data-loaded] {
  filter: blur(0);
  transition: filter 0.3s ease-out;
}
```

**Blurhash** (encoding library): Compact hash encoding image essence, decoded in browser as placeholder.

## Content-aware image resizing

**Aspect ratio preservation:** Cropping images to fixed ratios can lose important content.

**Solutions:**
- `object-fit: cover` / `object-position`: CSS cropping, flexible
- Smart crop tools (AWS Rekognition, imgproxy): Detect faces/objects, crop around them
- Art direction: Different crops per breakpoint via `<picture>`

## CLS Prevention

**Cumulative Layout Shift (CLS)** metric: page elements moving during load. High CLS ruins user experience.

**Image-related CLS:**
```html
<!-- Bad: no declared dimensions, layout shifts when image loads -->
<img src="image.jpg" alt="...">

<!-- Good: aspect ratio reserved -->
<img src="image.jpg" alt="..." width="400" height="300">
```

Modern browsers support aspect ratio CSS:
```css
img { aspect-ratio: 400 / 300; }
```

Dimensions allow browser to reserve space before image loads. Zero layout shift.

## CDN Image Transformation

CDNs (Cloudinary, Imgix, AWS CloudFront + Lambda) optimize images on-the-fly:

```
https://cdn.example.com/image.jpg?w=400&h=300&q=80&fmt=webp
```

Parameters:
- `w`, `h`: resize dimensions
- `q`: quality (JPEG 0-100)
- `fmt`: output format (webp, avif, etc.)
- `fit`: fit strategy (cover, contain, crop)

**Benefits:**
- Single source image, infinite variations
- Format negotiation (serve WebP to Chrome, JPEG to IE)
- On-the-fly optimization

**Trade-off:** Vendor lock-in, costs per-request or per-GB.

## Video Instead of Animated Images

Animated GIFs are 10-50x larger than video:
- 1 sec GIF: ~500KB
- 1 sec MP4 video: ~50KB

For animations, use `<video>` instead:
```html
<video autoplay muted playsinline loop>
  <source src="animation.mp4" type="video/mp4">
  <source src="animation.webm" type="video/webm">
</video>
```

Renders like GIF, fraction of size.

## Image Compression Tools

**Node.js / CLI:**
- **sharp**: Fast, modern. Official Node.js image library.
- **imagemin**: Batch optimization (jpeg-progressive, pngquant, etc.)
- **squoosh-cli**: Google's CLI, supports WebP/AVIF

**Online:**
- TinyPNG / TinyJPG: UI, batch upload
- Squoosh: Browser-based, real-time preview

**Quality targets:**
- Photographs: JPEG 75-85 quality or AVIF 50-70
- Screenshots: PNG or WebP lossless
- Thumbnails: Aggressive (50-60 quality)

## Performance Metrics

**Core Web Vitals involvement:**
- **LCP (Largest Contentful Paint)**: Large images delay first paint
- **CLS (Cumulative Layout Shift)**: Unreserved image dimensions shift content
- **FID/INP**: Not directly affected by images (interactivity)

Optimized images improve LCP 20-40% on image-heavy sites.

## Common Pitfalls

### Over-optimization
JPEG quality 40 visible compression artifacts. Quality 85+ often imperceptible. Find balance; benchmark.

### Responsive image complexity
`srcset` syntax confusing. Tools like Next.js Image auto-handle responsive sizing.

### Missing alt text
Accessibility + SEO. Always include meaningful alt.

### Ignoring bandwidth constraints
Rural/mobile networks: 3G at 500KB/s means 1MB image = 2s download. Optimize aggressively.

### CLS from unsized images
Always declare dimensions or aspect ratio.

## Tools & Ecosystem

**Frameworks with built-in optimization:**
- Next.js Image component: auto-responsive, lazy load, format selection
- Nuxt Image: Vue equivalent
- Astro Image: static-only optimization

**Standalone tools:**
- sharp: production workhorse
- Squoosh: browser-based preview
- Imgproxy: self-hosted transformation service
- CloudFront + Lambda@Edge: serverless on-demand optimization

## See Also

- performance-web-vitals.md — LCP, CLS, Web Vitals
- web-performance.md — broader performance strategies
- design-responsive.md — responsive design principles
- web-browser-rendering.md — how images block rendering