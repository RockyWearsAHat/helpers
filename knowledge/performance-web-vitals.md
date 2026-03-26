# Performance: Web Vitals — Core Metrics, RUM, and Loading Optimizations

Web performance directly impacts user experience and business outcomes. Google's Core Web Vitals define a standard baseline; RUM (Real User Monitoring) captures actual user experience.

## Core Web Vitals (CWV)

Google's 2020 initiative standardized three metrics representing distinct aspects of user experience.

### Largest Contentful Paint (LCP)

LCP measures when the largest visible element (text block, image, video) renders on the page. The user sees the page is loading.

**Threshold:** ≤ 2.5 seconds is "good"

**Why it matters:** Users perceive the page as "done loading" when they see the main content. A slow LCP feels like the page is hanging.

**Common causes:**
- Large images not optimized (4MB JPG instead of 50KB WebP)
- Render-blocking JavaScript (script tags in <head> block parsing)
- Slow server response time (TTFB > 500ms)
- Suboptimal fonts (downloading before render, blocking text display)
- Render-blocking CSS

**Optimization strategies:**
1. **Image optimization**: Use WebP/AVIF; resize; lazy-load below-fold images
2. **Code splitting**: Load only critical JavaScript first; defer non-essential
3. **Server-side caching**: Reduce TTFB via CDN or origin optimization
4. **Preload critical resources**: `<link rel="preload" href="font.woff2" as="font">`
5. **Reduce JavaScript parse/compile time**: use event-driven or streaming approaches

### Interaction to Next Paint (INP) or First Input Delay (FID)

INP (new in 2024, replaces FID) measures the latency from user interaction (click, tap) to visual response. Captures max interaction latency, not average.

**Threshold:** ≤ 200ms is "good"

**Why it matters:** Users expect immediate feedback. A 1-second delay between click and response feels broken, even if average latency is 200ms.

**Common causes:**
- Long JavaScript execution (heavy computations on main thread)
- Main thread blocking (long-running tasks prevent event handlers from running)
- Layout thrashing (repeated read-then-write of DOM, forcing layout recalculations)

**Optimization strategies:**
1. **Break up long tasks**: Use `requestIdleCallback()` or setTimeout to yield to the browser
2. **Web Workers**: Move computational work off the main thread
3. **Reduce JavaScript bundle size**: Less code = faster parsing and execution
4. **Debounce/throttle event handlers**: Prevent cascading handler invocations
5. **Avoid layout thrashing**: Queue DOM reads, then batch DOM writes

### Cumulative Layout Shift (CLS)

CLS measures unexpected visual shifts after page load. The user starts reading, the page jumps, and they lose focus.

**Threshold:** ≤ 0.1 is "good"

**Common causes:**
- Images without explicit width/height (size unknown until load)
- Ads injected into the page (late-loaded, shifting content)
- Fonts swapping (layout shifts when web font loads and swaps with fallback)
- Animations without containment (transforms cause reflow)

**Optimization strategies:**
1. **Reserve space**: Set width/height on images and media
2. **Load ads asynchronously**: Inject after page is interactive
3. **Font-display: swap**: Use fallback immediately; swap to web font (prevents layout shift from CLS perspective)
4. **Avoid animating properties that trigger layout**: Use `transform` and `opacity` instead of `width` and `height`
5. **Use `contain` CSS**: Limit layout recalculations to specific elements

## Related Metrics

### Time to First Byte (TTFB)

TTFB is the elapsed time from the request starting until the first byte of the response arrives. It's dominated by network latency + server processing time.

**Breakdown:**
- DNS lookup: ~10-50ms (cached locally)
- TCP connection: ~50-100ms
- TLS handshake: ~50-100ms (if HTTPS)
- Server processing: 0ms-2000ms
- Network transmission: depends on bandwidth

**Optimization:**
1. **DNS prefetch**: `<link rel="dns-prefetch" href="https://cdn.example.com">`
2. **Preconnect**: `<link rel="preconnect" href="https://cdn.example.com">`
3. **Server-side caching**: Django cache, Redis, CDN origin shield
4. **Edge computing**: Run logic geographically closer to users

### First Contentful Paint (FCP)

FCP is when the first element (text, image, SVG) renders. Earlier than LCP because it includes first text/image, not the largest.

**Use case:** Diagnostic; not a Core Web Vital. If FCP is good but LCP is bad, images are the bottleneck.

## Resource Loading and Optimization

### Critical Path Optimization

The **critical path** is the chain of resources needed before the page is interactive:

1. HTML → (download & parse)
2. Discover CSS/JS in `<head>`
3. Download CSS (blocks rendering)
4. Parse CSS
5. Download JavaScript (blocks parsing if not async)
6. Parse and execute JavaScript
7. Page is interactive

Each dependency adds latency (RTT = round-trip time).

**Optimization:**
- Minimize dependencies (combine resources)
- Parallelize (HTTP/2 multiplexing, multiple domains)
- Preload critical resources
- Defer non-critical resources

### Preload, Prefetch, Preconnect

**Preload**: High-priority resource needed soon after load start:
```html
<link rel="preload" href="font.woff2" as="font" crossorigin>
```

Signals the browser to download early, without blocking page rendering.

**Prefetch**: Low-priority resource needed for a future navigation:
```html
<link rel="prefetch" href="/next-page.js">
```

Downloaded in browser idle time. Useful for pagination or predictable user journeys.

**Preconnect**: Pre-establish TCP + TLS connection to a cross-origin server:
```html
<link rel="preconnect" href="https://analytics.example.com">
```

Reduces latency for subsequent requests (no new connection setup).

### HTTP/2 Server Push (Deprecated)

HTTP/2 Server Push allowed the server to proactively send resources. Now deprecated in HTTP/3 and largely ineffective (browser caches are already warm). Use preload instead.

## Image Optimization

Images often dominate page size (50-90% of bytes on news/e-commerce sites).

### Formats

**JPEG**: Lossy compression. Good for photos. ~50-75% smaller than PNG. Trade-off: quality loss.

**PNG**: Lossless. Good for graphics/logos. Larger file sizes.

**WebP**: Google's format; 25-35% smaller than JPEG/PNG. Supported in modern browsers. Fallback to JPEG for old browsers.

**AVIF**: Latest compression standard (AV1 video codec basis). 25-50% smaller than WebP but slower to encode. Limited browser support (now ~90%). Use as primary with WebP/JPEG fallbacks.

```html
<picture>
  <source srcset="image.avif" type="image/avif">
  <source srcset="image.webp" type="image/webp">
  <img src="image.jpg" alt="...">
</picture>
```

### Responsive Images

Serve different image sizes for different viewports:

```html
<img 
  srcset="small.jpg 480w, medium.jpg 800w, large.jpg 1200w"
  sizes="(max-width: 600px) 100vw, 50vw"
  src="medium.jpg"
  alt="..."
>
```

The browser downloads the appropriate size based on viewport width.

### Lazy Loading

Defer image loads until close to viewport:

```html
<img loading="lazy" src="image.jpg" alt="...">
```

Native browser support (Chrome 76+). Reduces initial page load by 20-50% for image-heavy sites.

## Font Loading

Fonts often block first paint because the browser waits for web fonts before rendering text.

### Font-Display Property

Controls how the font displays while the web font is loading:

```css
@font-face {
  font-family: 'MyFont';
  src: url('myfont.woff2') format('woff2');
  font-display: swap;
}
```

| Value | Behavior |
| --- | --- |
| `auto` | Browser default (usually block) |
| `block` | Hide text until font loads (up to 3 seconds) |
| `swap` | Show fallback immediately; swap when font loads |
| `fallback` | Show fallback; swap if font arrives within 100ms |
| `optional` | Show fallback; swap only if font arrives soon |

**Best practice:** `font-display: swap` avoids invisible text delay while still applying web fonts.

### Variable Fonts and Subsetting

**Variable fonts**: A single font file with multiple weights/widths. Smaller than shipping separate files (Normal.woff2 + Bold.woff2 + Italic.woff2).

**Font subsetting**: Include only glyphs used on the page. If the page is English, subset to Latin characters, cutting file size by 50%.

## JavaScript Performance

### Bundle Size Impact on LCP / INP

JavaScript parsing and execution happen on the main thread. Large bundles delay interactivity:

- **Parsing**: V8 (Chrome's JS engine) parses ~1MB of code per second
- **Execution**: Depends on code complexity; can add 1-5 seconds

A 3MB minified bundle takes ~3 seconds to parse, blocking interactivity.

**Optimization:**
1. **Code splitting**: Serve only critical code first; lazy-load page-specific code
2. **Tree-shaking**: Remove unused code (requires ES modules + compatible bundler)
3. **Polyfill loading**: Only load polyfills for older browsers
4. **Use dynamic imports**: `const module = await import('./heavy-lib.js')`

### Long Tasks

A **long task** is any JavaScript execution ≥ 50ms without yielding. Burns input responsiveness.

Detect and break up:
```javascript
// Bad: 100ms task
for (let i = 0; i < 10000000; i++) {
  processItem(i);
}

// Better: break into chunks
async function processItems() {
  for (let i = 0; i < 10000000; i++) {
    processItem(i);
    if (i % 1000 === 0) await new Promise(resolve => setTimeout(resolve, 0));  // yield
  }
}
```

## Measuring and Monitoring

### Lab vs Field Metrics

**Lab metrics** (Lighthouse, WebPageTest) run synthetic tests on a controlled device under consistent conditions. Reproducible but not representative of real users.

**Field metrics** (RUM — Real User Monitoring) capture actual user experiences. JavaScript running on real devices, real networks, real devices with varying resources.

Best practice: **Use both**. Lab metrics for diagnosis and regression detection. RUM for understanding real-world impact.

### Web Vitals Library

Google's `web-vitals` JavaScript library reports CWV metrics:

```javascript
import { getCLS, getFID, getFCP, getLCP, getTTFB } from 'web-vitals';

getCLS(console.log);   // Cumulative Layout Shift
getFID(console.log);   // First Input Delay (or INP)
getFCP(console.log);   // First Contentful Paint
getLCP(console.log);   // Largest Contentful Paint
getTTFB(console.log);  // Time to First Byte
```

Send to analytics backend for aggregation and alerting.

## See Also

- web-performance.md — broader web performance strategies
- performance-profiling.md — browser DevTools and tracing
- bundling-module-systems.md — code splitting and bundling
- web-rendering-patterns.md — rendering optimization techniques