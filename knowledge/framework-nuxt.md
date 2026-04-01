# Nuxt — Vue 3 Meta-Framework

## Overview

**Nuxt** is an opinionated, batteries-included meta-framework built on Vue 3. It provides file-based routing, server-side rendering, code generation, and a unified development experience for building full-stack applications with Vue.

Nuxt 3 (current) is built from scratch on Vue 3's Composition API and introduces **Nitro**, a new universal server engine.

```javascript
// App.vue — Nuxt auto-imports it; no import statements needed
<template>
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>

// pages/about.vue — becomes /about route automatically
<template>
  <div>{{ title }}</div>
</template>

<script setup>
definePageMeta({ layout: 'custom' });
const { data: title } = await useFetch('/api/title');
</script>
```

## Nitro Server Engine

**Nitro** is Nuxt's revolutionary server layer. It's a universal server that runs on Node.js, Deno, Cloudflare Workers, and service workers without code changes.

### Key Features

- **API Routes**: Automatically exposed as `/api/*` endpoints
- **Server Middleware**: Intercept and transform requests
- **Standalone output**: `nuxt build` generates a `.output` directory with a self-contained server (no `node_modules` needed)
- **Direct API calls**: Using `$fetch()` inside server-rendered components calls the server endpoint directly without HTTP round-trip
- **Hybrid rendering**: Supports SSR, SSG, ISR (incremental static regeneration), SWR (stale-while-revalidate)
- **h3 HTTP framework**: Built on h3, a lightweight HTTP library with helpers for cookies, headers, body parsing, redirects

```typescript
// server/api/users.ts — becomes GET /api/users
export default defineEventHandler(async (event) => {
  const query = getQuery(event);
  const users = await db.users.find(query);
  return users; // auto-JSON serialized
});

// server/middleware/auth.ts — runs on all requests
export default defineEventHandler((event) => {
  const token = getCookie(event, 'auth-token');
  if (!token && event.node.req.url?.startsWith('/api/admin')) {
    throw createError({ statusCode: 401, message: 'Unauthorized' });
  }
});
```

### Nitro Benefits

- **No separate backend required**: Server routes, middleware, and business logic coexist with frontend code
- **Type-safe API calls**: Nuxt generates TypeScript types for API routes, accessible in client code via `$fetch()`
- **Serverless ready**: Deploy `.output` to any platform — Vercel, Netlify, AWS Lambda, Deno Deploy, etc.
- **Unified deployment**: Single artifact runs anywhere

## Auto-Imports & Conventions

Nuxt auto-imports:

- **Components**: `components/` contains `.vue` files; use them without `import`
- **Composables**: `composables/` contains reusable logic; auto-imported in `<script setup>`
- **Utils**: `utils/` helpers are auto-imported
- **Vue APIs**: `ref`, `computed`, `watch` don't need imports

This reduces boilerplate but requires discipline—many globals can mask unclear dependencies.

```javascript
// pages/products.vue
<script setup>
// No need to import ref, reactive, or the composable
const filters = ref({});
const { data: products } = await useFecth('/api/products'); // composable auto-imported
</script>
```

## Composables & Server-Side Data

### useFetch / useAsyncData

```javascript
// Composable to fetch data
const { data, pending, error } = await useFetch('/api/products');

// Or more control:
const { data } = await useAsyncData('products', () => 
  $fetch('/api/products', { headers: { Authorization: token } })
);
```

These composables run on server (hydrating initial data into HTML) and rerun on client if called directly. They prevent redundant API calls.

### Typed API Routes

If your server route returns data:

```typescript
// server/api/users/[id].ts
export default defineEventHandler((event) => {
  const id = getRouterParam(event, 'id');
  return { id, name: 'Alice', email: 'alice@test.com' };
});

// In client, types are auto-inferred
const { data: user } = await useFetch(`/api/users/${userId}`);
// user.value has type: { id: string; name: string; email: string }
```

This is rare in the ecosystem—most frameworks lack this DX polish.

## Rendering Modes

Nuxt supports multiple rendering strategies:

| Mode | Behavior                                                    | Use Case                            |
| ---- | ----------------------------------------------------------- | ----------------------------------- |
| SSR  | Render on server, ship HTML + JS bundle, hydrate on browser | Full-stack apps, SEO, dynamic data  |
| SSG  | Pre-render all routes at build time, ship static files      | Blogs, docs, marketing sites        |
| ISR  | Pre-render on build; revalidate routes on-demand            | Mostly-static sites with rare updates |
| SWR  | Render on server, cache, serve stale while regenerating    | High-traffic, acceptable staleness  |
| Hybrid | Mix modes per route via `routeRules`                        | Marketing + app sections            |

```javascript
// nuxt.config.ts
export default defineNuxtConfig({
  routeRules: {
    '/': { prerender: true },           // SSG
    '/blog/**': { cache: { maxAge: 60 } }, // ISR with 60s revalidation
    '/api/**': { cache: false },            // No caching
  },
});
```

## Modules — Extending Nuxt

**Modules** are functions that hook into Nuxt's lifecycle, adding features:

```typescript
// modules/analytics.ts
export default defineNuxtModule({
  setup(options, nuxt) {
    // Inject a composable
    addPlugin(resolve('./runtime/plugin.ts'));
    
    // Hook into build
    nuxt.hook('build:before', () => { /* ... */ });
  },
});

// nuxt.config.ts
export default defineNuxtConfig({
  modules: ['./modules/analytics'],
});
```

Thousands of community modules exist: @nuxt/content, @nuxt/image, @nuxt/auth, @nuxtjs/tailwindcss, etc.

## Nuxt Content

**@nuxt/content** turns markdown files into queryable data:

```markdown
---
title: Introducing Nuxt 3
description: A modern Vue framework
date: 2024-01-15
---

# {{ title }}

Content here...
```

```typescript
// Can query and render
const posts = await queryContent('posts').sort({ date: -1 }).find();
const post = await queryContent('/posts/my-post').findOne();
```

This enables blogs, docs, and CMS-like structures without a separate backend.

## Comparison to Next.js & Remix

| Aspect          | Nuxt 3                           | Next.js                              | Remix                        |
| --------------- | -------------------------------- | ------------------------------------ | ---------------------------- |
| **Base**        | Vue 3                            | React                                | React                        |
| **Server**      | Nitro (universal)                | Custom Node server + Edge Functions  | Node.js or Edge Runtime      |
| **Routing**     | File-based conventions           | File-based conventions               | File-based conventions       |
| **Data Fetching** | useFetch/useAsyncData (built-in) | getServerSideProps/getStaticProps   | Loaders (server functions)   |
| **API Routes**  | Nitro server/ directory          | pages/api/ directory                 | action/loader in routes      |
| **Ecosystem**   | Vue plugins                      | React ecosystem (larger)             | Focused on progressive UX    |

**Nitro's advantage**: Single output runs anywhere (Vercel, Netlify, Deno Deploy, Lambda). Nothing language-specific baked in. Vue **disadvantage**: React ecosystem is larger; fewer Nuxt-specific libraries exist.

## Tradeoffs

- **Opinionated**: Nuxt assumes your folder structure, naming, and patterns. Flexibility is lower than Next.js.
- **Convention-heavy**: Auto-imports and magic folder structures can obscure where code comes from.
- **Ecosystem**: Vue ecosystem smaller than React's. Some integration gaps.
- **Iteration speed vs. control**: Nuxt prioritizes DX (shortcuts, auto-imports) over explicitly visible behavior.

## Research Direction

Nuxt 3 is stable and production-ready. Active development focuses on performance (streaming, partial hydration), Edge compute integration, and ORM/data layer improvements. Nitro's universal server approach is gaining traction outside of Nuxt.

**See also**: framework-nextjs, framework-vue, framework-remix, server-side-rendering, paradigm-meta-framework