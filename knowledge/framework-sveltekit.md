# SvelteKit

## Overview

SvelteKit is a web application framework built on Svelte (the compiler) that provides file-based routing, server-side rendering, static generation, and modern fullstack capabilities. For React developers, SvelteKit is similar to Next.js. For Vue developers, it's similar to Nuxt.

The core insight: SvelteKit combines Svelte's compiler efficiency with fullstack architecture patterns. Build sizes are small, initial loads are fast, and interactive features scale with component complexity, not app size.

## File-Based Routing

Routes are defined by the filesystem under `src/routes/`. Each directory name maps to a URL path. Special prefixed files (`+` prefix) define route segments:

```
src/routes/
├── +page.svelte          # / (root page)
├── +layout.svelte        # Root layout (wraps all pages)
├── +error.svelte         # Error boundary
├── about/
│   └── +page.svelte      # /about
├── blog/
│   ├── +layout.svelte    # Layout for /blog/*
│   └── [slug]/
│       ├── +page.svelte  # /blog/hello-world
│       └── +page.server.ts
└── api/
    └── random/
        └── +server.ts    # POST /api/random
```

**Dynamic segments** use bracket notation: `[slug]` creates a parameter. `[...rest]` captures remaining path segments (catch-all routes).

## Page Components

`+page.svelte` defines a route's UI. By default, pages render on the server (SSR) and then hydrate in the browser for navigation. Pages receive data from load functions via the `data` prop:

```svelte
<script lang="ts">
  import type { PageProps } from './$types';
  
  let { data } = $props();
</script>

<h1>{data.title}</h1>
<div>{@html data.content}</div>
```

### Page Options

Pages can export config options:

```typescript
// +page.ts (shared between server and client)
export const prerender = true; // Prerender at build time
export const ssr = false;      // Skip SSR (SPA for this route)
export const csr = true;       // Enable client-side rendering

export const load = ({ params }) => {
  return { title: 'Blog Post', slug: params.slug };
};
```

— `prerender: true` — Static generation: page is rendered at build time
— `ssr: false` — SPA mode: page only renders in the browser
— `csr: true` — Enable browser-side navigation (default)

## Load Functions

Load functions run before a page renders to fetch data from APIs, databases, or compute derived values. They run on the server during SSR and in the browser during client-side navigation.

**`+page.js` (shared)** — Runs on both server and client:

```typescript
export const load = ({ params, fetch }) => {
  return {
    postData: fetch(`/api/posts/${params.slug}`).then(r => r.json()),
  };
};
```

**`+page.server.js` (server-only)** — Runs only on the server. Can access databases, private env vars, and HTTP-only cookies:

```typescript
import { error } from '@sveltejs/kit';

export const load = async ({ params, locals }) => {
  const post = await db.posts.findOne({ slug: params.slug });
  if (!post) error(404, 'Not found');
  return { post, user: locals.user };
};
```

Load functions are **hierarchical** — data from parent layouts is passed to child pages. If a child's load function depends on parent data, the parent reruns first.

### Streaming and Promises

Load functions can return unresolved promises, which stream to the browser and resolve in the background:

```typescript
export const load = async ({ params }) => {
  return {
    quickData: await fetchQuick(),
    slowData: fetchSlow(), // Unresolved promise — streams separately
  };
};
```

The page renders with `quickData` available and `slowData` as a pending promise that resolves in the HTML.

## Form Actions

Actions handle form submissions on the server. A `+page.server.js` file can export actions that receive form data and return a result:

```typescript
// +page.server.ts
import { fail, redirect } from '@sveltejs/kit';

export const actions = {
  default: async ({ request, locals }) => {
    const formData = await request.formData();
    const email = formData.get('email');
    
    try {
      await db.users.create({ email });
      redirect(303, '/success');
    } catch (e) {
      return fail(400, { email, error: e.message });
    }
  },

  delete: async ({ params }) => {
    await db.posts.delete(params.slug);
    return { deleted: true };
  },
};
```

Forms work **with or without JavaScript** (progressive enhancement). When JS is disabled, forms POST to the server. When JS is present, form submissions are intercepted and handled via `use:enhance`:

```svelte
<script lang="ts">
  import { enhance } from '$app/forms';
</script>

<form method="POST" use:enhance>
  <input name="email" />
  <button>Sign Up</button>
</form>
```

Multiple actions can be defined in the `actions` object. The form's `action` attribute determines which one runs.

## Layouts

Layouts define shared UI that wraps pages. Nested layouts inherit from parent layouts:

```svelte
<!-- src/routes/+layout.svelte -->
<script>
  let { children } = $props();
</script>

<nav>
  <a href="/">Home</a>
  <a href="/about">About</a>
</nav>

{@render children()}
```

Layouts can have `+layout.js` or `+layout.server.js` files that export load functions. Data from a layout's load function is available to all child pages:

```typescript
// src/routes/settings/+layout.server.ts
export const load = async () => {
  return {
    sections: [
      { slug: 'profile', title: 'Profile' },
      { slug: 'notifications', title: 'Notifications' },
    ],
  };
};
```

Nested layouts only apply to routes below their directory — `/settings/+layout.svelte` only wraps routes under `/settings/`.

## Error Handling

If a load function throws an error, SvelteKit renders the nearest `+error.svelte` component:

```svelte
<!-- src/routes/+error.svelte -->
<script lang="ts">
  import { page } from '$app/state';
</script>

<h1>{$page.status}: {$page.error?.message}</h1>
<a href="/">Go home</a>
```

Error boundaries walk up the tree — a child route's `+error.svelte` is used if present, otherwise the parent's, then the root's, then SvelteKit's default error page.

## Server Endpoints

`+server.ts` files define API endpoints that handle HTTP requests directly:

```typescript
// src/routes/api/posts/+server.ts
import { json, error } from '@sveltejs/kit';

export const GET = async ({ url }) => {
  const limit = url.searchParams.get('limit') ?? '10';
  const posts = await db.posts.find().limit(Number(limit));
  return json(posts);
};

export const POST = async ({ request }) => {
  const body = await request.json();
  const post = await db.posts.create(body);
  return json(post, { status: 201 });
};

export const DELETE = async ({ params }) => {
  await db.posts.delete(params.id);
  return new Response(null, { status: 204 });
};
```

Server endpoints bypass the page system entirely — no `+page.svelte` is needed. They're useful for webhooks, file uploads, and API routes.

## Hooks

Hooks are functions that intercept the request/response cycle. They live in `src/hooks.server.ts` (server) or `src/hooks.client.ts` (browser).

### Server Hook: `handle`

`handle` runs on every request before load functions. Use it for auth, logging, modifying requests:

```typescript
// src/hooks.server.ts
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  // Before
  if (event.url.pathname.startsWith('/admin')) {
    if (!event.locals.user?.isAdmin) {
      return new Response('Forbidden', { status: 403 });
    }
  }

  // Call the SvelteKit handler
  const response = await resolve(event);

  // After
  response.headers.set('X-Custom-Header', 'value');
  return response;
};
```

`event.locals` stores request-scoped data (e.g., authenticated user) that's accessible throughout the request chain.

### Server Hook: `handleError`

`handleError` catches errors thrown during load functions and server code:

```typescript
export const handleError: HandleServerError = ({ error, event }) => {
  console.error('Unexpected error:', error);
  return {
    message: 'Internal server error',
  };
};
```

### Client Hook: `handleError`

The browser equivalent catches errors during client-side navigation:

```typescript
// src/hooks.client.ts
export const handleError: HandleClientError = ({ error, event }) => {
  return {
    message: 'Something went wrong',
  };
};
```

## Adapters

Adapters determine where and how your app runs. SvelteKit builds to a universal format (handler function), and adapters deploy it to different platforms:

| Adapter | Platform | Characteristics |
|---------|----------|---|
| `@sveltejs/adapter-auto` | Auto-detect | Picks the best adapter based on environment |
| `@sveltejs/adapter-node` | Node.js | Standard server, good for Docker |
| `@sveltejs/adapter-vercel` | Vercel | Edge functions, streaming, analytics |
| `@sveltejs/adapter-cloudflare` | Cloudflare Workers | Edge computing, minimal cold start |
| `@sveltejs/adapter-netlify` | Netlify | Functions, redirects, forms |
| `@sveltejs/adapter-static` | Static | Pure SSG — outputs `build/` folder for S3/CDN |

Set the adapter in `svelte.config.js`:

```javascript
import adapter from '@sveltejs/adapter-vercel';

export default {
  kit: {
    adapter: adapter(),
  },
};
```

## Rendering Modes

Choose how routes render:

— **SSR (default)** — Server renders HTML, browser hydrates. Best for SEO, fast first paint.
— **SSG** — Server renders at build time, outputs static HTML. Fastest, no server needed.
— **SPA** — Browser renders entirely. Best for interactive apps, but no SEO.

Per-route control:

```typescript
// SSG
export const prerender = true;

// SPA
export const ssr = false;

// SSR with no client-side rendering (unlikely)
export const csr = false;
```

## SEO

### Meta Tags and Links

Use `+page.server.ts` or hook to set meta tags:

```typescript
export const load = () => {
  return {
    title: 'My Post',
    description: 'A great read',
  };
};
```

Then in the root `+layout.svelte`:

```svelte
<script>
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{$page.data.title}</title>
  <meta name="description" content={$page.data.description} />
</svelte:head>
```

### Sitemap and Robots

Export prerender routes to generate a sitemap at build time, or serve dynamic sitemaps via `+server.ts`.

## Environmental Variables

— `$env/static/public` — Available on server and client (must start with `PUBLIC_`)
— `$env/static/private` — Server only (never leaked to browser)
— `$env/dynamic/private` — Runtime server secrets (e.g., from env vars)

```typescript
import { PUBLIC_API_URL } from '$env/static/public';
import { PRIVATE_API_KEY } from '$env/static/private';
```

## Key Takeaways

— Filesystem routing keeps code organized by feature
— Load functions separate data fetching from rendering
— Form actions enable progressive enhancement without extra wiring
— Adapters make deployment flexible
— Hooks intercept the request cycle for cross-cutting concerns
— Streaming and promises enable partial page loads and better UX