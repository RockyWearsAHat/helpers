# Technical SEO — Crawling, Indexing, Core Web Vitals & Rendering Strategies

## Crawling and Indexing Fundamentals

Search engines discover pages by following links from known pages. A crawler (bot) fetches HTML, extracts links, and queues them for crawling. Indexing is the process of parsing content and storing it in the search engine's database.

### Crawl budget

Search engines allocate a limited number of requests per domain (crawl budget). For small sites, this is not a constraint. For large sites or frequently updated content, crawl budget becomes a bottleneck. Wasting crawl budget on non-essential pages (duplicate content, staging URLs, old versions) means fewer essential pages get crawled.

Patterns to preserve crawl budget:
- **Canonicalize duplicate content**: Use `<link rel="canonical">` to point to the primary version
- **Block low-value pages**: Use `robots.txt` to exclude staging, admin, or internal search pages
- **Fix crawl errors**: Broken redirects, infinite redirect loops, 404s drain budget
- **Leverage sitemaps**: Explicitly list important pages so the crawler prioritizes them

### User-Agent and robots.txt

`robots.txt` is a protocol file at the domain root that tells bots which paths to crawl:

```
User-agent: *
Disallow: /admin/
Disallow: /private/
Allow: /public/

Crawl-delay: 10
```

`User-agent: *` matches all bots. Specific rules for `Googlebot`, `Bingbot`, etc., override the `*` rule.

**Important**: `robots.txt` is advisory. It doesn't prevent access; a malicious bot ignores it. Use authentication for genuinely private content.

## Sitemaps

XML sitemaps (site priority, last modified date, change frequency) help search engines prioritize crawling:

```xml
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/page1</loc>
    <lastmod>2025-03-25</lastmod>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
```

Submit sitemaps via `robots.txt` or directly to Google Search Console / Bing Webmaster Tools.

For large sites with dynamic content (e-commerce), generate sitemaps dynamically and refresh them on schedule.

## Canonical URLs

Canonical URLs resolve URL duplication without harming SEO. Multiple URLs can represent the same content:
- `example.com/page` and `example.com/page?utm_source=google`
- `example.com` and `www.example.com`
- `https` and `http` versions
- `/product?id=1` and `/products/widget/`

Use `<link rel="canonical">` to declare the primary URL:

```html
<link rel="canonical" href="https://example.com/widget">
```

Canonical can point to a different domain (e.g., a retailer pointing to a manufacturer's page). The canonical URL doesn't need to return a 2xx status code, though it should be publicly accessible.

Avoid self-referential canonicals or chains (A→B→C); point directly to the primary.

## hreflang for Multilingual Sites

`hreflang` tells search engines which page version is intended for which language/region:

```html
<link rel="alternate" hreflang="en" href="https://example.com/en/page">
<link rel="alternate" hreflang="fr" href="https://example.com/fr/page">
<link rel="alternate" hreflang="x-default" href="https://example.com/page">
```

The `x-default` version is a fallback for users whose language isn't explicitly listed.

Implemented in HTML `<head>`, XML sitemaps, or HTTP headers. Mismatch (saying an English page targets `fr`) confuses search engines; verify consistency.

## Structured Data and JSON-LD

Structured data helps search engines understand content semantics. JSON-LD (JavaScript Object Notation for Linked Data) is the modern standard:

```html
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "Article",
  "headline": "Understanding SEO",
  "author": {
    "@type": "Person",
    "name": "Jane Doe"
  },
  "datePublished": "2025-03-25",
  "image": "https://example.com/image.jpg",
  "description": "A guide to technical SEO"
}
</script>
```

Common types (schema.org):
- `Article` — News, blog posts
- `WebPage` — General pages
- `Product` — E-commerce items
- `LocalBusiness` — Stores, offices
- `Event` — Conferences, concerts
- `Recipe` — Cooking instructions
- `FAQPage` — Q&A content
- `VideoObject` — Embedded videos

Search engines use structured data for:
- **Rich snippets**: Star ratings, price, availability (e.g., "★★★★★ $9.99 In Stock")
- **Knowledge graph**: Entity information (people, companies, places)
- **Voice search**: Parsed FAQ content

Validate structured data with Google's Rich Results Test or Schema.org validators.

## Core Web Vitals

Core Web Vitals are user experience metrics that search engines use as ranking signals:

### Largest Contentful Paint (LCP)

Time when the largest visible element (often an image or text block) finishes rendering. Target: **2.5 seconds or less**.

Affected by:
- Server response time
- Render-blocking CSS/JavaScript
- Large unoptimized images
- Client-side rendering delays

Optimize: minimize critical path, defer non-critical JS, use responsive images, consider SSR/SSG.

### Cumulative Layout Shift (CLS)

Unexpected movement of page elements during load. Target: **score ≤ 0.1** (on a scale where 1.0 = completely unstable).

Common causes:
- Late-loading ads, embeds (don't reserve space)
- Unoptimized images or fonts without `width` / `height` attributes
- Dynamically injected content above fold
- Web fonts causing FOUT (Flash of Unstyled Text)

Optimize: reserve layout space (use `aspect-ratio` CSS or width/height attributes), load ads asynchronously below fold, prefer system fonts or preload web fonts.

### Interaction to Next Paint (INP)

Latency from user interaction (click, tap, key press) until the visual response appears. Target: **200ms or less**.

Affected by:
- Long-running JavaScript blocking the main thread
- Inefficient event handlers
- Render updates triggered by input

Optimize: break up long tasks (use `requestIdleCallback`), debounce/throttle input handlers, offload work to Web Workers, optimize render performance.

## Rendering Strategies: CSR, SSR, SSG & Impact

Different rendering strategies have SEO implications:

### Client-Side Rendering (CSR)

JavaScript runs in the browser, fetches data, and renders HTML. SEO challenges:
- Crawlers may not execute JavaScript (though Google's crawler does)
- Initial HTML is minimal; content appears later
- First paint is slow; LCP often poor
- Crawlers may timeout before rendering completes

Use when: Highly interactive apps (dashboards, collaboration tools). Provide a fallback `<noscript>` for no-JS users.

### Server-Side Rendering (SSR)

Server executes JavaScript, renders HTML, sends the fully-rendered page. SEO advantages:
- Crawlers receive complete HTML on first request
- Content is immediately visible (better LCP)
- Metadata (title, meta description, OG tags) are server-rendered and discoverable

Challenges: Increased server load, complexity managing hydration (client reattaching event listeners to server-rendered HTML).

### Static Site Generation (SSG)

Build process pre-renders pages to static HTML at deploy time. SEO advantages:
- Perfect for static content (documentation, blogs, marketing sites)
- Extremely fast (pure HTML + CDN)
- Crawlers always see complete content

Challenges: Stale content (pages only update on rebuild), not suitable for dynamic content (real-time data, per-user customization).

### Hybrid approaches

- **Incremental Static Regeneration (ISR)**: Generate static pages on-demand and cache them; invalidate on schedule
- **Streaming SSR**: Render page sections progressively; send initial shell to browser while data loads

For SEO, the choice depends on content velocity. High-frequency updates (real-time data) require CSR or SSR. Infrequently-changing content (blogs) profits from SSG.

## Meta Tags and Social Metadata

### Essential meta tags

```html
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="Brief page summary for SERP snippet">
<meta name="robots" content="index, follow">
<title>Page Title | Site Name</title>
```

Meta descriptions are suggested (search engines may rewrite them), but they influence click-through rates.

### Open Graph (OG) and Twitter Cards

OG tags control how content appears when shared on social platforms:

```html
<meta property="og:type" content="article">
<meta property="og:title" content="Article Title">
<meta property="og:description" content="Brief summary">
<meta property="og:image" content="https://example.com/image.jpg">
<meta property="og:url" content="https://example.com/article">
```

Twitter Card equivalents:

```html
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="Title">
<meta name="twitter:description" content="Summary">
<meta name="twitter:image" content="https://example.com/image.jpg">
```

Missing OG tags result in generic (often poor) previews on social platforms.

## URL Structure and Slugs

URLs signal content hierarchy and keywords:

- `example.com/blog/understanding-seo` (good — hierarchical, descriptive)
- `example.com/p/12345` (poor — opaque)
- `example.com/blog/understanding-seo?utm_source=twitter` (canonical to remove params)

Guidelines:
- Use hyphens (not underscores or spaces)
- Lowercase only
- Avoid query parameters for primary content (use canonical to manage variants)
- Keep URLs short; prioritize descriptiveness over brevity

## Robots.txt and X-Robots-Tag

`robots.txt` is global. For page-level control, use `<meta name="robots">` or `X-Robots-Tag` HTTP header:

```html
<meta name="robots" content="noindex, nofollow">
```

Options:
- `index` / `noindex`: Include in search results?
- `follow` / `nofollow`: Follow links on the page?
- `noarchive`: Don't show cached version
- `noimageindex`: Don't index images
- `max-snippet: 160`: Limit SERP snippet length

Use `noindex` for temporary pages (staging, previews, old versions) and `nofollow` for untrusted external links.

## Monitoring and Tools

- **Google Search Console**: Indexation status, coverage issues, Core Web Vitals
- **Bing Webmaster Tools**: Similar; useful for Bing-specific insights
- **PageSpeed Insights**: Core Web Vitals, lab data, recommendations
- **Lighthouse**: Automated audits (performance, accessibility, SEO)
- **Schema.org validator**: Verify structured data correctness

## See also

- [web-rendering-patterns.md](web-rendering-patterns.md) — CSR vs. SSR vs. SSG architectural trade-offs
- [performance-core-web-vitals.md](performance-core-web-vitals.md) — Detailed Core Web Vitals optimization
- [accessibility-engineering.md](accessibility-engineering.md) — Semantic HTML foundations for SEO