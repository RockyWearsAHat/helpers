# Astro

## Overview

Astro is a content-focused web framework that introduced the islands architecture — ship zero JavaScript by default, hydrate only the interactive components. It achieves near-perfect Lighthouse scores for content sites by sending HTML with no client JS unless you opt in.

## Core Architecture

### Content-First

Astro is optimized for content-heavy sites: blogs, marketing, docs, portfolios. It's not designed for highly interactive SPAs (use Next.js/Remix for those). The mental model: every page starts as static HTML, and you add interactivity surgically.

### Islands Architecture (Partial Hydration)

An "island" is an interactive UI component in a sea of static HTML:

```astro
---
import Header from '../components/Header.astro';   // static (no JS)
import SearchBar from '../components/Search.tsx';   // React island
import Sidebar from '../components/Sidebar.vue';    // Vue island
---
<Header />
<SearchBar client:load />       <!-- hydrate immediately -->
<Sidebar client:visible />      <!-- hydrate when scrolled into view -->
```

### Client Directives

| Directive                           | When Hydrated                            | Use Case                         |
| ----------------------------------- | ---------------------------------------- | -------------------------------- |
| `client:load`                       | Immediately on page load                 | Critical interactive elements    |
| `client:idle`                       | After page is idle (requestIdleCallback) | Non-critical interactive         |
| `client:visible`                    | When scrolled into viewport              | Below-fold content               |
| `client:media="(max-width: 768px)"` | When media query matches                 | Mobile-only components           |
| `client:only="react"`               | Client only (no SSR)                     | Browser-API-dependent components |
| (none)                              | Never — renders static HTML only         | Default for all components       |

## .astro Component Syntax

```astro
---
// Component script (runs at build time / server)
import Layout from '../layouts/Layout.astro';
import Card from '../components/Card.astro';

interface Props {
  title: string;
  description?: string;
}

const { title, description = 'Default description' } = Astro.props;
const posts = await getCollection('blog');
const isProduction = import.meta.env.PROD;
---

<!-- Template (HTML + expressions) -->
<Layout title={title}>
  <h1>{title}</h1>
  <p set:html={description} />   <!-- render raw HTML (careful: XSS) -->

  {posts.map(post => (
    <Card title={post.data.title} url={`/blog/${post.slug}`} />
  ))}

  {isProduction && <Analytics />}
</Layout>

<style>
  /* Scoped by default — won't leak */
  h1 { color: navy; }
</style>

<style is:global>
  /* Global styles when needed */
  body { margin: 0; }
</style>

<script>
  // Runs in the browser — bundled per-page
  document.querySelector('h1')?.addEventListener('click', () => {
    alert('clicked');
  });
</script>
```

### Key Differences from JSX

- No `className` — use `class`
- No `htmlFor` — use `for`
- No self-closing tags for HTML elements
- Expressions use `{}` but no JSX-style `<>` fragments in templates
- Scoped styles by default
- `<script>` tags are real browser scripts, not component logic

## Content Collections

Type-safe content management for Markdown, MDX, JSON, YAML:

```typescript
// src/content.config.ts
import { defineCollection, z } from "astro:content";

const blog = defineCollection({
  type: "content", // Markdown/MDX files
  schema: z.object({
    title: z.string(),
    date: z.date(),
    draft: z.boolean().default(false),
    tags: z.array(z.string()).default([]),
    image: z.string().optional(),
    author: z.object({
      name: z.string(),
      avatar: z.string(),
    }),
  }),
});

const authors = defineCollection({
  type: "data", // JSON/YAML files
  schema: z.object({
    name: z.string(),
    bio: z.string(),
    twitter: z.string().optional(),
  }),
});

export const collections = { blog, authors };
```

### Querying Collections

```astro
---
import { getCollection, getEntry } from 'astro:content';

// Get all non-draft posts, sorted by date
const posts = await getCollection('blog', ({ data }) => !data.draft);
posts.sort((a, b) => b.data.date.valueOf() - a.data.date.valueOf());

// Get single entry
const post = await getEntry('blog', 'my-first-post');
const { Content, headings } = await post.render();
---

<Content />  <!-- Rendered Markdown -->
```

## Routing

### File-Based Pages

```
src/pages/
├── index.astro           → /
├── about.astro           → /about
├── blog/
│   ├── index.astro       → /blog
│   └── [slug].astro      → /blog/:slug (dynamic)
├── [...path].astro       → catch-all (404 page)
└── api/
    └── users.ts          → /api/users (endpoint)
```

### Dynamic Routes (SSG)

```astro
---
// src/pages/blog/[slug].astro
import { getCollection } from 'astro:content';

export async function getStaticPaths() {
  const posts = await getCollection('blog');
  return posts.map(post => ({
    params: { slug: post.slug },
    props: { post },
  }));
}

const { post } = Astro.props;
const { Content } = await post.render();
---
<Content />
```

## SSG / SSR / Hybrid

```javascript
// astro.config.mjs
export default defineConfig({
  output: "static", // default: full SSG
  output: "server", // full SSR
  output: "hybrid", // static by default, opt-in SSR per page
});

// In hybrid mode, opt a page into SSR:
// src/pages/dashboard.astro
export const prerender = false; // this page is server-rendered
```

## Framework Integrations

Astro renders components from multiple frameworks on the same page:

```javascript
// astro.config.mjs
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import vue from "@astrojs/vue";
import svelte from "@astrojs/svelte";

export default defineConfig({
  integrations: [react(), vue(), svelte()],
});
```

```astro
---
import ReactCounter from '../components/Counter.tsx';
import VueWidget from '../components/Widget.vue';
import SvelteForm from '../components/Form.svelte';
---

<ReactCounter client:load />
<VueWidget client:visible />
<SvelteForm client:idle />
```

## View Transitions

Built-in page transitions (no client-side router needed):

```astro
---
import { ViewTransitions } from 'astro:transitions';
---
<head>
  <ViewTransitions />
</head>

<!-- Named transitions -->
<h1 transition:name="title">{post.title}</h1>
<img transition:name={`hero-${post.slug}`} src={post.image} />

<!-- Transition animations -->
<div transition:animate="slide">Content</div>
<!-- Built-in: fade, slide, none, or custom -->
```

## Image Optimization

```astro
---
import { Image } from 'astro:assets';
import heroImage from '../assets/hero.jpg';
---

<Image
  src={heroImage}
  width={800}
  alt="Hero"
  format="avif"
  quality={80}
/>
<!-- Outputs optimized image with proper width/height (no CLS) -->
```

## Middleware

```typescript
// src/middleware.ts
import { defineMiddleware } from "astro:middleware";

export const onRequest = defineMiddleware(async (context, next) => {
  const { request, locals, redirect } = context;

  // Auth check
  const session = getSession(request.headers.get("cookie"));
  if (!session && context.url.pathname.startsWith("/dashboard")) {
    return redirect("/login");
  }
  locals.user = session?.user;

  const response = await next();
  response.headers.set("X-Custom", "value");
  return response;
});
```

## Endpoints (API Routes)

```typescript
// src/pages/api/users.ts
import type { APIRoute } from "astro";

export const GET: APIRoute = async ({ request }) => {
  const users = await db.user.findMany();
  return new Response(JSON.stringify(users), {
    headers: { "Content-Type": "application/json" },
  });
};

export const POST: APIRoute = async ({ request }) => {
  const body = await request.json();
  const user = await db.user.create({ data: body });
  return new Response(JSON.stringify(user), { status: 201 });
};
```

## Server Adapters

For SSR, deploy to various targets:

| Adapter               | Target                          |
| --------------------- | ------------------------------- |
| `@astrojs/node`       | Node.js (standalone or Express) |
| `@astrojs/vercel`     | Vercel (serverless + edge)      |
| `@astrojs/cloudflare` | Cloudflare Workers/Pages        |
| `@astrojs/netlify`    | Netlify Functions               |
| `@astrojs/deno`       | Deno Deploy                     |

## Internationalization

```javascript
// astro.config.mjs
export default defineConfig({
  i18n: {
    defaultLocale: "en",
    locales: ["en", "fr", "es"],
    routing: {
      prefixDefaultLocale: false, // / = en, /fr = french
    },
  },
});
```

## Astro DB

Built-in libSQL database for content sites:

```typescript
// db/config.ts
import { defineDb, defineTable, column } from "astro:db";

const Comment = defineTable({
  columns: {
    id: column.number({ primaryKey: true }),
    postSlug: column.text(),
    author: column.text(),
    body: column.text(),
    createdAt: column.date({ default: new Date() }),
  },
});

export default defineDb({ tables: { Comment } });
```

## Comparison

| Feature            | Astro                      | Next.js               | Gatsby                |
| ------------------ | -------------------------- | --------------------- | --------------------- |
| Default JS shipped | None                       | React runtime         | React runtime         |
| Partial hydration  | Yes (islands)              | No (all-or-nothing)   | No                    |
| Multi-framework    | Yes                        | React only            | React only            |
| Content focus      | Primary                    | Secondary             | Primary (but heavy)   |
| SSR                | Yes                        | Yes                   | Limited               |
| Build speed        | Fast (Vite)                | Moderate              | Slow (webpack/Gatsby) |
| Best for           | Content sites, docs, blogs | Full apps, dashboards | Legacy content sites  |
