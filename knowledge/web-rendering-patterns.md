# Web Rendering Patterns: CSR, SSR, SSG, ISR, RSC

## Overview

"Rendering" is the process of converting application code into HTML sent to browsers. Where this happens — client, server, build time, on-demand — determines user experience, performance characteristics, SEO, and infrastructure costs. Modern web stacks combine multiple patterns in a single application. This is not a "choose one forever" decision but a per-route consideration.

## Client-Side Rendering (CSR)

Server sends an empty HTML shell with JavaScript. The browser downloads and runs JavaScript, which queries APIs, fetches data, and renders UI.

**Flow**: Browser loads index.html → downloads bundle.js → executes JavaScript → calls `/api/data` → renders UI.

**Pros**: Highly interactive, dynamic, fast navigation between routes (no full page load), natural rich interactions, frontend and backend can scale/deploy independently.

**Cons**: Slower initial page load (bundle download + JavaScript parse + first API call), blank page before render (poor perceived performance), worse SEO (search engines may not wait for JS execution to crawl), entire app bundle is browser-side.

**Use cases**: Dashboards, web apps with many interactive states, intranet tools, real-time collaboration (figma-style).

**Performance reality**: CSR is fast if bundles are small (<60kb gzipped) and API latency is low. Large bundles + slow APIs = slow first paint. Progressive React aims to mitigate this.

## Server-Side Rendering (SSR)

Server executes application code on each request, renders to HTML, sends complete HTML to browser. Browser receives fully formed DOM and can display it immediately.

**Flow**: Browser requests `/page` → server runs React/Vue/etc, fetches data, renders to HTML string → sends HTML + hydration bundle → browser displays HTML instantly → JavaScript hydrates for interactivity.

**Pros**: Fast first contentful paint, good SEO (full HTML sent), progressive enhancement possible (page works before JS loads), better for slow networks or low-end devices.

**Cons**: Slower response time (server must wait for data, render), server CPU cost scales with traffic, typically slower navigation (full page reloads or re-render per route), harder to cache (every request might be unique based on URL), stateful server is complex.

**Hydration cost**: Browser receives HTML, then downloads and runs JavaScript to make it interactive. During hydration window, user sees clickable UI but nothing happens until JS loads and attaches event listeners. Mismatch (server rendered "Hello alice" but client renders "Hello bob") causes hydration errors and UI flicker.

**Next.js with SSR**: `getServerSideProps()` runs on each request, data is fetched server-side, component is rendered on server, sent as HTML, then hydrated client-side.

## Static Site Generation (SSG)

HTML is pre-generated at build time. Every user receives the same HTML.

**Flow**: Build process runs routes, fetches data, renders to static HTML files → on deploy, files are uploaded to CDN → users download pre-rendered files.

**Pros**: Extremely fast (serve from CDN/cache), minimal server cost (no computation), excellent SEO, perfect cacheability, trivial scaling.

**Cons**: Data is stale (only updates on rebuild/redeploy), slow iteration (rebuild can take minutes), only works for finite sets of pages (can't pre-generate `/users/:id` for millions of users).

**Use cases**: Blogs, documentation, marketing sites, product catalogs with finite items, any site where data changes infrequently.

**Next.js with SSG**: `getStaticProps()` runs at build time. Pages are generated once. Rebuilding is manual (CI trigger) or scheduled.

## Incremental Static Regeneration (ISR)

Hybrid of SSG and SSR. Initial deployment generates a subset of static pages. On request to a non-pregenerated page, server renders it and caches the result. At regular intervals, pages are refreshed in background.

**Flow**: `/products/1` pre-rendered at build time. `/products/999999` not pre-generated. User requests `/products/999999` → server renders on demand → cached → subsequent users get cached HTML → background job refreshes pages every N seconds/hours.

**Pros**: Combines SSG benefits (fast cache hits, CDN delivery) with SSR flexibility (handle dynamic pages on first request), pages refresh automatically in background, scales to infinite routes.

**Cons**: First request to new page has SSR latency, stale-while-revalidate window (old HTML served while refresh happens), revalidation must be configured correctly or data becomes stale.

**Next.js with ISR**: `getStaticProps()` with `revalidate: 60` (regenerate every 60 seconds). On-demand revalidation via `revalidateTag()` or `revalidatePath()`.

**Use case**: E-commerce sites (inventory changes hourly), news sites (stories update throughout day), social platforms (new user profiles created constantly).

## Streaming SSR

Server streams HTML to browser in chunks rather than waiting for the entire page to render.

**Traditional SSR**: Server waits for all data → renders full HTML → sends it → browser paints. Waterfall: if one slow data fetch blocks everything, entire page is delayed.

**Streaming SSR**: Server renders shell HTML and sends immediately. As data arrives, server sends component HTML incrementally. Browser paints shell, then fills in sections as they arrive.

**Flow**: Server starts rendering, sends `<html>`, body tags. `/api/slow-data` takes 5 seconds. Server sends HTML for page shell immediately, includes `<Suspense>` boundary. As data arrives, server sends that component's HTML and JavaScript to hydrate it inline.

**Pros**: Better perceived performance (something appears on screen immediately), reduces "time to interactive feel", leverages edge servers/CDNs for shell, flexible data loading.

**Cons**: Requires browser support (ReadableStream, TransformStream), more complex server implementation, debugging is harder (streaming state is implicit).

## React Server Components (RSC)

React components that run only on server and never on client. Unlike SSR (where components run on server then client), RSC components run once on server, send serialized output ("RSC payload") to client.

**Philosophy**: Server components are "free" — they don't bloat the JavaScript bundle, they can and run all database queries, secrets, file system access directly without API boundaries.

**Example**: A list of blog posts that queries the database directly (without API call) and filters. The component runs server-side, queries database, renders to server payload, sends to client. Client never runs the component code, only displays its output. No data refetching, no API layer needed.

**Rules**: Server components can't use hooks, event listeners, or browser APIs. Client components are marked `'use client'` at the top of the file and run in browser as usual. A server component can render client components (composition is flexible).

**Pros**: Smaller client bundles, less JavaScript sent, simpler mental model (component decides where it runs), zero API boilerplate for data fetching, edge/server cost is "free".

**Cons**: Brand new (Next.js 13+), unfamiliar to most teams, tooling/debugging still immature, mental model is non-obvious at first, async/await in component bodies is uncommon pattern.

**Next.js 15 default**: Server components are default; must opt-in to client components. Flips the 10-year React paradigm of "components run in browser."

## Partial Hydration & Islands Architecture

"Islands" are interactive components scattered in a sea of static HTML. Each island hydrates independently.

**Traditional hydration**: Send entire app's JavaScript. Browser runs *all* component code to attach event listeners, even for static components. Wasteful.

**Islands**: Static components are HTML only. Interactive components ("islands") are sent with JavaScript. Each island attaches its own event listeners.

**Example**: Blog post page. The article text is static HTML. The comment section is interactive (add reply, upvote). Comment section is an island sent with JavaScript. Article text is not.

**Implementations**: Astro (compiles to islands automatically), Fresh (Deno-based), Eleventy plugins.

**Pros**: Tiny JavaScript bundles for static-heavy pages, each island can be its own React/Vue/whatever app, scales to very large sites.

**Cons**: Multiple JavaScript runtimes on one page (memory/download overhead), cross-island communication requires workarounds, architectural overhead during development.

## Progressive Enhancement

Start with HTML that works without JavaScript. Enhance with CSS for style. Enhance with JavaScript for interactivity.

**Three layers**:
1. HTML — semantic markup, works in all browsers, navigation via links
2. CSS — styles, layout, animations
3. JavaScript — rich interactions, forms without page refresh, real-time updates

**Example**: A form sends data via `<form method="post">` to server. JavaScript enhances it with `preventDefault()` and AJAX. If JS doesn't load, form still works (slow full-page POST). If CSS doesn't load, form is still usable.

**vs modern SPA**: SPAs assume JavaScript always loads and is fast. Progressive enhancement assumes JavaScript is unreliable (network failure, slow parse, errors).

**Modern relevance**: Frameworks like SvelteKit and frameworks using RSC embrace progressive enhancement implicitly. Forms work even without JavaScript. Not a requirement but a nice fallback.

## Rendering Pipeline: Comparison

| Pattern | First Paint | Response Time | SEO | Cache Hit | Scalability | Use Case |
|---------|-------------|---|---|---|---|---|
| CSR | Slow (JS download) | Fast (client-side nav) | Poor | Good | High | Rich interactive apps |
| SSR | Fast | Slow (server render) | Good | Poor | Medium | User-specific pages |
| SSG | Very fast | — | Excellent | Excellent | Very high | Static content |
| ISR | Very fast (cache) / medium (first) | — | Excellent | Excellent | Very high | Dynamic + static mix |
| Streaming SSR | Fast (progressive) | Medium | Good | Fair | Medium | Large pages with sections |
| RSC | Fast | Fast | Good | Medium | High | Modern Next.js apps |
| Islands | Very fast | Varies | Good | Excellent | High | Static + interactive spots |

## Decision Framework

**New project, team experienced with React, real-time needs**: CSR or CSR + React Query for server state. Simple mental model.

**Content-driven (blogs, docs, marketing)**: Start with static generation (Astro, Hugo). Add interactivity with islands where needed.

**E-commerce, news, UGC platforms**: ISR. Pre-generate popular routes, handle tail on-demand, revalidate regularly.

**Enterprise SPA with teams**: CSR + GraphQL + Apollo. Explicit, scalable, well-understood by orgs.

**New team, modern stack**: Next.js with RSC (app router). Server components default, client where needed. Simplest mental model if team adapts to async/server paradigm.

**Performance-critical, complex UI**: Streaming SSR (Next.js 13+) with RSC + islands for components needing real-time updates.

Trade-off checklist:
- How much is UI interactive vs static?
- How fresh must data be?
- SEO requirements?
- Team expertise?
- Scale (traffic, routes, data freshness)?
- Deploy strategy (CDN, edge, server)?

See also: [web-performance.md](web-performance.md), [framework-nextjs.md](framework-nextjs.md), [framework-react.md](framework-react.md)

## References

- Next.js rendering: https://nextjs.org/docs/app/building-your-application/rendering
- React Server Components: https://react.dev/reference/rsc/server-components
- Astro islands: https://docs.astro.build/en/concepts/islands/
- Streaming SSR: https://www.patterns.dev/posts/streaming-ssr