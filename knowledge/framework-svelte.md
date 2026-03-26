# Svelte

## Compiler Approach

Svelte is a **compiler**, not a runtime framework. Components are compiled to imperative JavaScript that surgically updates the DOM — no virtual DOM, no diffing. The output is plain JS that calls `element.textContent = value` directly.

**Result:** smaller bundles (no framework runtime), faster initial load, and updates that scale with the number of changes rather than the number of components.

Compilation happens at build time via a Vite plugin (`@sveltejs/vite-plugin-svelte`). The compiler analyzes the component and generates:

- A `create_fragment` function for initial DOM creation
- Update functions that run only when specific variables change
- Cleanup/destroy functions

## Svelte 5 Runes

Svelte 5 replaces the implicit `$:` reactivity with explicit **runes** — compiler directives prefixed with `$`:

### $state

```svelte
<script>
  let count = $state(0);
  let user = $state({ name: 'Alice', age: 30 });

  // Deep reactivity — mutations are tracked
  function birthday() {
    user.age += 1; // triggers update automatically
  }
</script>

<button onclick={() => count++}>{count}</button>
```

`$state` creates deeply reactive state. Object/array mutations are tracked through Proxies. For non-deep reactivity, use `$state.raw()`.

### $derived

```svelte
<script>
  let count = $state(0);
  let doubled = $derived(count * 2);

  // Complex derivations
  let filtered = $derived.by(() => {
    return items.filter(i => i.active);
  });
</script>
```

Derived values are lazy and cached — they only recompute when their dependencies change.

### $effect

```svelte
<script>
  let count = $state(0);

  $effect(() => {
    // Runs when count changes — dependencies auto-tracked
    document.title = `Count: ${count}`;

    return () => {
      // Cleanup (runs before re-execution and on unmount)
    };
  });

  // Pre-effect (runs before DOM updates)
  $effect.pre(() => {
    // Useful for measuring DOM before changes
  });
</script>
```

`$effect` runs in the browser only, after DOM updates. It tracks dependencies automatically. Avoid setting `$state` inside `$effect` — use `$derived` instead when possible.

### $props and $bindable

```svelte
<script>
  // Typed props with defaults
  let { name, count = 0, onUpdate } = $props();

  // Two-way bindable prop
  let { value = $bindable('') } = $props();

  // Rest props
  let { class: className, ...rest } = $props();
</script>
```

### $inspect (dev only)

```svelte
<script>
  let count = $state(0);
  $inspect(count); // console.log on every change (stripped in prod)
  $inspect(count).with(fn); // custom handler
</script>
```

## Component Structure

```svelte
<script>
  // JavaScript logic with runes
  let items = $state([]);
  let newItem = $state('');

  function add() {
    items.push(newItem); // mutation tracked by $state
    newItem = '';
  }
</script>

<!-- Template (HTML with Svelte syntax) -->
<form onsubmit={e => { e.preventDefault(); add(); }}>
  <input bind:value={newItem} />
  <button>Add</button>
</form>

{#if items.length > 0}
  <ul>
    {#each items as item, i (item)}
      <li>{item}</li>
    {/each}
  </ul>
{:else}
  <p>No items yet</p>
{/if}

<style>
  /* Component-scoped by default */
  li { color: navy; }
  /* :global(selector) escapes scoping */
</style>
```

## Template Syntax

### Control Flow

```svelte
{#if condition}
  ...
{:else if other}
  ...
{:else}
  ...
{/if}

{#each items as item, index (item.id)}
  ...
{:else}
  <p>Empty</p>
{/each}

{#await promise}
  <p>Loading...</p>
{:then data}
  <p>{data}</p>
{:catch error}
  <p>Error: {error.message}</p>
{/await}

{#key expression}
  <!-- Re-creates contents when expression changes -->
  <Component />
{/key}

{#snippet name(params)}
  <p>Reusable template chunk: {params.text}</p>
{/snippet}

{@render name({ text: 'hello' })}
```

### Bindings

```svelte
<input bind:value={text} />
<input type="checkbox" bind:checked={accepted} />
<select bind:value={selected}>...</select>
<textarea bind:value={content} />
<div bind:clientWidth={w} bind:clientHeight={h} />
<audio bind:currentTime={t} bind:paused />
<Component bind:this={ref} />
```

### Event Handling (Svelte 5)

```svelte
<!-- Svelte 5: standard attributes -->
<button onclick={handler}>Click</button>
<button onclick={(e) => handler(e, id)}>Click</button>

<!-- Event modifiers via wrapper functions -->
<button onclick={preventDefault(handler)}>Submit</button>
```

## Transitions and Animations

```svelte
<script>
  import { fade, fly, slide, scale, crossfade } from 'svelte/transition';
  import { flip } from 'svelte/animate';
</script>

{#if visible}
  <div transition:fade={{ duration: 300 }}>Fades in and out</div>
  <div in:fly={{ y: 200 }} out:fade>Flies in, fades out</div>
{/if}

{#each items as item (item.id)}
  <div animate:flip={{ duration: 200 }}>
    {item.name}
  </div>
{/each}
```

Custom transitions return `{ duration, css, tick }` — the `css` function generates a CSS animation string, avoiding main-thread work.

## Actions

Actions are functions that run when an element is mounted — useful for attaching third-party libraries or custom behavior:

```svelte
<script>
  function tooltip(node, text) {
    // Called when element is created
    const tip = createTooltip(node, text);
    return {
      update(newText) { tip.setText(newText); },
      destroy() { tip.remove(); }
    };
  }
</script>

<button use:tooltip={'Click me'}>Hover</button>
```

## Stores (still available)

```js
import { writable, derived, readable, get } from "svelte/store";

const count = writable(0);
count.subscribe((value) => console.log(value));
count.set(5);
count.update((n) => n + 1);

const doubled = derived(count, ($count) => $count * 2);
```

In Svelte 5, `$state` replaces most store use cases. Stores remain useful for shared state across non-component modules.

## Context API

```svelte
<script>
  import { setContext, getContext } from 'svelte';

  // Parent sets context (not reactive by default)
  setContext('theme', { mode: 'dark' });

  // Make it reactive by wrapping in $state
  setContext('counter', { count: $state(0) });

  // Child reads context
  const theme = getContext('theme');
</script>
```

Context is available to the component and all its descendants. It's scoped to the component tree (not global like stores).

## SvelteKit

### Project Structure

```
src/
  routes/
    +page.svelte          # / page
    +page.server.ts       # server load function
    +layout.svelte        # root layout
    +error.svelte         # error page
    about/
      +page.svelte        # /about page
    blog/
      [slug]/
        +page.svelte      # /blog/:slug
        +page.ts           # universal load function
    api/
      users/
        +server.ts         # /api/users endpoint
  lib/                     # $lib alias
  app.html                 # HTML template
```

### Load Functions

```typescript
// +page.server.ts — runs only on server
export async function load({ params, fetch, locals }) {
  const post = await db.getPost(params.slug);
  if (!post) throw error(404, "Not found");
  return { post };
}

// +page.ts — runs on server AND client (universal)
export async function load({ fetch, params }) {
  const res = await fetch(`/api/posts/${params.slug}`);
  return { post: await res.json() };
}
```

### Form Actions

```typescript
// +page.server.ts
export const actions = {
  create: async ({ request, locals }) => {
    const data = await request.formData();
    const title = data.get("title");
    if (!title) return fail(400, { title, missing: true });
    await db.createPost({ title, author: locals.user.id });
    throw redirect(303, "/posts");
  },
  delete: async ({ request }) => {
    /* ... */
  },
};
```

```svelte
<form method="POST" action="?/create" use:enhance>
  <input name="title" value={form?.title ?? ''} />
  {#if form?.missing}<p>Title is required</p>{/if}
  <button>Create</button>
</form>
```

### Hooks

```typescript
// src/hooks.server.ts
export async function handle({ event, resolve }) {
  const session = await getSession(event.cookies.get("sid"));
  event.locals.user = session?.user;
  return resolve(event);
}

export function handleError({ error, event }) {
  console.error(error);
  return { message: "Something went wrong" };
}
```

### Adapters

```js
// svelte.config.js
import adapter from "@sveltejs/adapter-auto"; // auto-detect platform
// Or specific: adapter-node, adapter-static, adapter-vercel, adapter-cloudflare
```

### Rendering Modes

```typescript
// +page.ts or +layout.ts
export const ssr = true; // server-side render (default)
export const csr = true; // client-side hydration (default)
export const prerender = false; // static generation at build time
export const trailingSlash = "never";
```

Set `prerender = true` for static pages. Set `ssr = false` for client-only SPAs. Set `csr = false` for zero-JS server-rendered pages.
