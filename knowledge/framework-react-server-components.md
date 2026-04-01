# React Server Components — Rendering Boundaries, Streaming & Integration

## Overview

React Server Components (RSC) are components that run exclusively on the server, never included in the client JavaScript bundle. They transform React from a client-side library into a full-stack framework where the server and client coordinate rendering. Unlike traditional server-side rendering (which produces static HTML), RSC maintains React's interactivity by rendering a **serializable component tree** that the client merges with interactive Client Components.

## Core Architecture: Server vs Client Boundary

The fundamental shift: React applications split into three execution contexts:

- **Server Components** (default in App Router): Run on the server, access databases and secrets directly, produce serialized component output
- **Client Components**: Run in the browser, use hooks and browser APIs, hydrate with interactive features
- **Shared boundaries**: Components at the edge must handle serialization carefully

**Key invariant**: You cannot pass a Server Component as a prop to a Client Component. The component tree must flow from server → client. Client Components can never render Server Components, because the server has already completed execution before Client JS loads.

### What Server Components Can Do

- Execute `async/await` directly in component bodies (no wrapper needed)
- Access backend resources: databases, filesystem, environment secrets
- Keep large dependencies on the server (expensive libraries stay out of the bundle)
- Cache computations using memoization primitives
- Run expensive operations only once at request time, not for each client

### What Server Components Cannot Do

- Use hooks (`useState`, `useEffect`, etc.)
- Use browser APIs (localStorage, geolocation)
- Add event listeners directly
- Use context for state (only for read-only provider setup)

## Directives: `'use server'` and `'use client'`

Directives mark component execution context and enable interoperability.

### `'use client'`

Marks a file or module boundary as Client Component code. When applied at the top of a file, all exports become client-side. Client Components can have Client Component children and Server Component children.

```jsx
'use client';
import { useState } from 'react';
import ServerSidebar from './sidebar'; // This MUST be a Server Component

export default function Dashboard() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <ServerSidebar /> {/* Server renders, streamed to client */}
      <button onClick={() => setOpen(!open)}>Toggle</button>
    </>
  );
}
```

### `'use server'`

Marks a function as a **Server Action** — a function that runs on the server when invoked from the client. Enables forms and mutations without separate API endpoints.

```jsx
// app/actions.ts
'use server';

export async function updateProfile(formData: FormData) {
  const name = formData.get('name');
  // Safely runs on server; result returns to client
  await db.users.update({ name });
  revalidatePath('/profile');
}

// app/profile.tsx (Client Component)
'use client';
export default function Profile() {
  return (
    <form action={updateProfile}>
      <input name="name" />
      <button type="submit">Save</button>
    </form>
  );
}
```

Server Actions automatically serialize return values and throw errors safely back to the client. The function source is never sent to the browser.

## Streaming & Progressive Rendering

RSC splits rendering into chunks that stream to the client incrementally, improving perceived performance. Instead of "wait for all data, then render": streaming enables "render what's ready, then fill in asynchronous parts."

### Suspense Boundaries in Server Context

```jsx
// app/page.tsx (Server Component)
import { Suspense } from 'react';
import SlowPost from './slow-post';
import PostSkeleton from './skeleton';

export default function Page() {
  return (
    <>
      <h1>Blog</h1>
      <Suspense fallback={<PostSkeleton />}>
        <SlowPost /> {/* Fetches data, suspends tree */}
      </Suspense>
    </>
  );
}
```

The server renders `<PostSkeleton />` first while `<SlowPost>` fetches, then replaces it once data arrives. The skeleton streams to the client immediately (lower Time to First Byte), creating an interactive progressive experience.

### Streaming Chunks

React sends the component tree as chunks:
1. Initial HTML shell (`<PostSkeleton />`) streams first
2. Once `<SlowPost>` resolves, its tree is patched in
3. Client reconstructs the full tree without re-rendering the skeleton

This differs from traditional SSR, where the server blocks until all data is ready.

## Data Fetching Patterns

Server Components enable co-location of data fetching and rendering—no separate data layer or API layer needed.

### Direct Async/Await

```jsx
// app/posts/[id]/page.tsx
export default async function PostPage({ params }) {
  // Direct database access — no API wrapper
  const post = await db.posts.find(params.id);
  
  if (!post) notFound();
  
  return <PostContent post={post} />;
}
```

This replaces patterns like `getServerSideProps` or `useEffect` + API calls. Data fetches run at request time, results hydrate the component.

### Request-Level Caching

The React request cache memoizes fetch results during a single render pass:

```jsx
async function getAuthor(id) {
  const res = await fetch(`/api/author/${id}`, { 
    next: { revalidate: 3600 } // ISR: revalidate hourly
  });
  return res.json();
}

// Even if called 5 times, fetch runs once per request
export default async function AuthorPage() {
  const author1 = await getAuthor(1);
  const author2 = await getAuthor(1); // Cached result
}
```

### Data Fetching Trade-offs

**Advantages**: Type-safe queries, no JSON serialization overhead, single render pass, direct database access.

**Disadvantages**: Cannot page data (full content fetches for each request), no client-side refetch without re-navigation, waterfalls if dependencies aren't paralleled (fetch A → fetch B).

**Parallel fetching** avoids waterfalls:

```jsx
export default async function Dashboard() {
  // Start both in parallel, wait for both
  const [user, posts] = await Promise.all([
    fetchUser(),
    fetchPosts()
  ]);
  
  return <Dashboard user={user} posts={posts} />;
}
```

## Serialization Constraints

RSC output must be **serializable** — it travels as JSON over the network. Certain values cannot cross the server → client boundary:

**Cannot serialize**:
- Functions (including closures, event handlers)
- Classes
- Symbols
- Circular references

**Can serialize**:
- Primitives (strings, numbers, booleans, null)
- Plain objects and arrays
- Dates, Maps, Sets (via special encoding)
- Component trees (JSX)

This is why event handlers must live in Client Components:

```jsx
// ❌ WRONG — handler is a function, can't serialize
async function ServerPage() {
  async function handleClick() { /* ... */ }
  return <button onClick={handleClick}>Save</button>;
}

// ✅ CORRECT — move to Client Component
'use client';
export default function ClientButton() {
  async function handleClick() { /* ... */ }
  return <button onClick={handleClick}>Save</button>;
}
```

Props flowing from Server to Client are serialized. Complex objects (DB models, class instances) must be converted to plain data.

## Composition Patterns

### Leaf Pattern: Server + Client Boundaries

Minimize Client Component scope—keep them at tree leaves:

```jsx
// ✅ Good: Small Client Component
'use client';
function InteractiveButton({ onClick }) {
  return <button onClick={onClick}>Save</button>;
}

// Server Component uses it
export default async function Page() {
  return <InteractiveButton onClick={handleSave} />;
}
```

Not: Large root Client Component wrapping many Server Components. Each Client Component adds overhead and waterfall.

### Composition with Slots (Next.js Parallel Routes)

Parallel routes create independent renderable areas that render separately:

```jsx
// app/dashboard/@header/page.tsx (Server Component)
export default async function Header() {
  const user = await getUser();
  return <header>{user.name}</header>;
}

// app/dashboard/@sidebar/page.tsx (Server Component)
export default async function Sidebar() {
  const nav = await getNav();
  return <nav>{nav}</nav>;
}

// app/dashboard/layout.tsx
export default function DashboardLayout({ header, sidebar, children }) {
  return (
    <div>
      {header}
      {sidebar}
      <main>{children}</main>
    </div>
  );
}
```

Each slot streams independently—sidebar doesn't block header.

### Islands: Pre-rendered Server + Interactive Client

Pre-render the page as Server Components with embedded interactive islands (Client Components):

```jsx
export default async function BlogPost({ params }) {
  const post = await getPost(params.id);
  
  return (
    <article>
      <h1>{post.title}</h1>
      <MDXContent content={post.content} />
      
      {/* Island: interactive comment widget */}
      <CommentWidget postId={params.id} />
    </article>
  );
}

'use client';
function CommentWidget({ postId }) {
  const [comments, setComments] = useState([]);
  // ... interactive comment logic
}
```

## Next.js App Router Integration

The App Router makes RSC the default. File conventions handle rendering:

| Convention     | Renders as                                          | Default | 
| -------------- | --------------------------------------------------- | ------- |
| `page.tsx`     | Server Component (interactive parts use `'use client'`) | Server  |
| `layout.tsx`   | Server Component                                     | Server  |
| `error.tsx`    | Client Component (must handle error state)          | Client  |
| `loading.tsx`  | Suspense boundary's fallback UI                     | Either  |

The default Server Component model flips the script: opt-in interactivity via `'use client'` instead of opt-in SSR.

### Incremental Static Regeneration (ISR)

RSCs support fine-grained revalidation:

```jsx
export const revalidate = 3600; // Revalidate every hour
// OR
export const dynamicParams = false; // Pre-render all, 404 for unknown

const res = await fetch(url, { next: { revalidate: 3600 } });
revalidatePath('/blog'); // Invalidate a path on demand
revalidateTag('posts');  // Invalidate by tag
```

## Trade-offs & Design Decisions

**When RSC shines**: Public content, data-heavy pages, SEO-critical routes, tight database coupling, large dependencies.

**When RSC is awkward**: Real-time dashboards (polling feels wrong), forms with complex validation UI, extensive client interactivity (becomes many islands).

**Network waterfall risk**: If Server Components fetch sequentially instead of in parallel, every fetch blocks downstream rendering.

**Mental model cost**: Understanding server/client boundary requires discipline. Accidental `'use client'` at the root negates benefits.

**Debugging complexity**: Server-side errors don't appear in browser DevTools; logging and error reporting require explicit setup.

## Related

See also: [framework-nextjs.md](framework-nextjs.md) (App Router), [web-rendering-patterns.md](web-rendering-patterns.md) (SSR/SSG comparison), [web-performance.md](web-performance.md) (streaming, Core Web Vitals).