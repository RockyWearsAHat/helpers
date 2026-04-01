# Next.js

## App Router Architecture

Next.js 13+ introduced the App Router (`app/` directory) alongside the Pages Router (`pages/`). The App Router uses React Server Components by default.

### File Conventions

| File            | Purpose                                 | Renders as                                                       |
| --------------- | --------------------------------------- | ---------------------------------------------------------------- |
| `page.tsx`      | Route UI                                | Required to make route accessible                                |
| `layout.tsx`    | Shared UI wrapper                       | Wraps page and nested layouts (does NOT re-render on navigation) |
| `template.tsx`  | Like layout but re-mounts on navigation | New instance per navigation                                      |
| `loading.tsx`   | Loading UI (wraps page in Suspense)     | Shown during route transitions                                   |
| `error.tsx`     | Error boundary (`"use client"`)         | Catches errors in page and children                              |
| `not-found.tsx` | 404 UI                                  | Triggered by `notFound()`                                        |
| `route.ts`      | API endpoint (Route Handler)            | GET, POST, PUT, DELETE, PATCH                                    |
| `default.tsx`   | Fallback for parallel routes            | Shown when slot has no matching segment                          |

### Route Structure

```
app/
  layout.tsx          # Root layout (required — wraps everything)
  page.tsx            # / (home)
  globals.css
  dashboard/
    layout.tsx        # Nested layout for /dashboard/*
    page.tsx          # /dashboard
    settings/
      page.tsx        # /dashboard/settings
  blog/
    [slug]/
      page.tsx        # /blog/:slug (dynamic segment)
    [...catchAll]/
      page.tsx        # /blog/* (catch-all)
    [[...optional]]/
      page.tsx        # /blog or /blog/* (optional catch-all)
  (marketing)/        # Route group — no URL segment
    about/
      page.tsx        # /about (grouped for layout sharing)
  @modal/             # Parallel route (named slot)
    (.)photo/[id]/
      page.tsx        # Intercepting route
```

## Server Components vs Client Components

**Server Components** (default in App Router):

- Run only on the server
- Can directly use `async/await` for data fetching
- Can access databases, filesystem, environment secrets
- Cannot use hooks (useState, useEffect), browser APIs, or event handlers
- Zero JS sent to client for the component itself

**Client Components** (opt-in with `"use client"`):

- Run on both server (initial HTML) and client (hydration)
- Can use all React hooks and browser APIs
- Their code is included in the JS bundle

```tsx
// Server Component (default)
async function ProductPage({ params }: { params: { id: string } }) {
  const product = await db.products.findUnique({ where: { id: params.id } });
  return <ProductDetails product={product} />;
}

// Client Component
("use client");
function AddToCart({ productId }: { productId: string }) {
  const [qty, setQty] = useState(1);
  return <button onClick={() => addToCart(productId, qty)}>Add</button>;
}
```

**Composition pattern:** Server Components can import Client Components, but Client Components cannot import Server Components. Pass Server Components as `children` to Client Components.

## Server Actions

```tsx
// Inline in Server Component
async function TodoPage() {
  async function addTodo(formData: FormData) {
    "use server";
    const text = formData.get("text") as string;
    await db.todos.create({ data: { text } });
    revalidatePath("/todos");
  }

  return (
    <form action={addTodo}>
      <input name="text" required />
      <button type="submit">Add</button>
    </form>
  );
}

// Separate file (reusable)
// app/actions.ts
("use server");
export async function deleteItem(id: string) {
  await db.items.delete({ where: { id } });
  revalidateTag("items");
}
```

Server Actions are POST endpoints under the hood. They work with progressive enhancement — forms submit without JS.

## Data Fetching

### In Server Components

```tsx
// Direct async — no useEffect, no useState
async function Page() {
  const data = await fetch("https://api.example.com/items", {
    next: { revalidate: 3600 }, // ISR: revalidate every hour
  });
  const items = await data.json();
  return <ItemList items={items} />;
}
```

### fetch Options

```tsx
// Static (cached forever until revalidated)
fetch(url); // default: { cache: 'force-cache' } in Next.js 14
fetch(url, { cache: "force-cache" });

// Dynamic (never cache)
fetch(url, { cache: "no-store" });

// ISR (revalidate after N seconds)
fetch(url, { next: { revalidate: 60 } });

// Tag-based revalidation
fetch(url, { next: { tags: ["products"] } });
// Then: revalidateTag('products') in a Server Action
```

**Note:** Next.js 15 changed the default to `cache: 'no-store'`. Opt into caching explicitly.

## Caching Layers

| Cache               | Where  | What                                     | Duration                           | Invalidation                                  |
| ------------------- | ------ | ---------------------------------------- | ---------------------------------- | --------------------------------------------- |
| Request Memoization | Server | Duplicate `fetch` calls in single render | Per-request                        | Automatic                                     |
| Data Cache          | Server | `fetch` results                          | Persistent                         | `revalidatePath`, `revalidateTag`, time-based |
| Full Route Cache    | Server | Rendered HTML + RSC payload              | Persistent                         | Revalidation of data cache                    |
| Router Cache        | Client | RSC payload of visited routes            | Session (30s dynamic, 5min static) | `router.refresh()`, revalidation              |

### Opting Out of Caching

```tsx
// Per-fetch
fetch(url, { cache: "no-store" });

// Per-route segment
export const dynamic = "force-dynamic";
export const revalidate = 0;

// Dynamic functions auto-opt-out: cookies(), headers(), searchParams
```

## Route Handlers

```tsx
// app/api/items/route.ts
import { NextRequest, NextResponse } from "next/server";

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const query = searchParams.get("q");
  const items = await db.items.findMany({
    where: { name: { contains: query } },
  });
  return NextResponse.json(items);
}

export async function POST(request: NextRequest) {
  const body = await request.json();
  const item = await db.items.create({ data: body });
  return NextResponse.json(item, { status: 201 });
}
```

Route Handlers in the `app/` directory replace API Routes from `pages/api/`. They support GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS.

## Middleware

```tsx
// middleware.ts (project root)
import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

export function middleware(request: NextRequest) {
  // Auth check
  const token = request.cookies.get("token");
  if (!token && request.nextUrl.pathname.startsWith("/dashboard")) {
    return NextResponse.redirect(new URL("/login", request.url));
  }

  // Add headers
  const response = NextResponse.next();
  response.headers.set("x-custom-header", "value");
  return response;
}

export const config = {
  matcher: ["/dashboard/:path*", "/api/:path*"],
};
```

Middleware runs on the Edge Runtime — limited Node.js APIs. Runs before every matched request.

## Parallel and Intercepting Routes

### Parallel Routes (slots)

```
app/
  layout.tsx        # receives @analytics and @team as props
  @analytics/
    page.tsx        # rendered simultaneously with main page
    default.tsx     # fallback
  @team/
    page.tsx
```

```tsx
// layout.tsx
export default function Layout({ children, analytics, team }) {
  return (
    <div>
      {children}
      <aside>{analytics}</aside>
      <aside>{team}</aside>
    </div>
  );
}
```

### Intercepting Routes

```
app/
  feed/
    page.tsx              # /feed (list of photos)
    (.)photo/[id]/
      page.tsx            # intercepts /photo/:id when navigating from /feed
  photo/[id]/
    page.tsx              # /photo/:id (direct access — full page)
```

Convention: `(.)` same level, `(..)` one level up, `(..)(..)` two levels, `(...)` root.

## Streaming and Suspense

```tsx
// loading.tsx auto-wraps page in Suspense
// Or manually:
export default async function Page() {
  return (
    <div>
      <h1>Dashboard</h1>
      <Suspense fallback={<ChartSkeleton />}>
        <SlowChart /> {/* streams in when ready */}
      </Suspense>
      <Suspense fallback={<TableSkeleton />}>
        <SlowTable />
      </Suspense>
    </div>
  );
}
```

## Image and Font Optimization

```tsx
import Image from "next/image";

<Image
  src="/hero.jpg"
  alt="Hero image"
  width={1200}
  height={630}
  priority // preload (for LCP images)
  placeholder="blur" // blur-up while loading
  sizes="(max-width: 768px) 100vw, 50vw"
/>;
```

Images are automatically lazy-loaded, served in WebP/AVIF, and responsive-resized.

```tsx
import { Inter } from 'next/font/google';
const inter = Inter({ subsets: ['latin'], display: 'swap' });

// In layout
<body className={inter.className}>
```

Fonts are self-hosted at build time — no external requests.

## Metadata API

```tsx
// Static
export const metadata: Metadata = {
  title: "My App",
  description: "Best app ever",
  openGraph: { title: "My App", images: ["/og.png"] },
};

// Dynamic
export async function generateMetadata({ params }): Promise<Metadata> {
  const product = await getProduct(params.id);
  return { title: product.name, description: product.summary };
}
```

## Static Generation

```tsx
// Generate static params for dynamic routes at build time
export async function generateStaticParams() {
  const posts = await getAllPosts();
  return posts.map((post) => ({ slug: post.slug }));
}

// Route segment config
export const dynamic = "force-static"; // ensure static
export const dynamicParams = false; // 404 for unknown params
```
