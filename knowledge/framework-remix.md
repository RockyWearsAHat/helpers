# Remix

## Overview

Remix is a full-stack React framework focused on web standards and progressive enhancement. Its core insight: the server/client boundary lives at the route level, and forms + HTTP are the interaction primitives — not client-side state management. Every route is both a UI component and a server endpoint.

## Nested Routing

Remix uses file-based routing with nested layouts. The file structure maps to the URL:

```
app/routes/
├── _index.tsx              → /
├── about.tsx               → /about
├── users.tsx               → /users (layout)
├── users._index.tsx        → /users (index content)
├── users.$userId.tsx       → /users/:userId
├── users.$userId.posts.tsx → /users/:userId/posts
├── blog_.tsx               → /blog (escapes parent layout with _)
├── ($lang).about.tsx       → /about or /en/about (optional param)
└── $.tsx                   → splat/catch-all route
```

### Dot Delimiters

- `.` creates URL segments: `users.new.tsx` → `/users/new`
- `_` prefix on parent: layout route without URL segment
- `_` suffix: escape parent layout nesting

### Nested Layouts

Parent routes render an `<Outlet />` where child routes appear:

```tsx
// app/routes/users.tsx (layout for all /users/* routes)
export default function UsersLayout() {
  return (
    <div>
      <Sidebar />
      <main>
        <Outlet /> {/* child route renders here */}
      </main>
    </div>
  );
}
```

## Loaders (Server-Side Data Fetching)

Loaders run on the server before the route renders. They're the primary data-fetching mechanism:

```tsx
import { json, type LoaderFunctionArgs } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";

export async function loader({ params, request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = parseInt(url.searchParams.get("page") || "1");

  const user = await db.user.findUnique({
    where: { id: params.userId },
  });

  if (!user) {
    throw new Response("Not Found", { status: 404 });
  }

  return json({ user, page });
}

export default function UserProfile() {
  const { user, page } = useLoaderData<typeof loader>();
  return <h1>{user.name}</h1>;
}
```

### Loader Rules

- Only runs on the server — safe to use DB, secrets, file system
- Runs in parallel for nested routes (parent + child loaders)
- Re-runs on client-side navigation (fetched via `fetch()`)
- Must return a `Response` (use `json()` helper)
- Throw a Response to trigger error/catch boundaries

## Actions (Mutations)

Actions handle form submissions and non-GET requests:

```tsx
import { redirect, type ActionFunctionArgs } from "@remix-run/node";

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "delete") {
    await db.post.delete({ where: { id: formData.get("id") } });
    return redirect("/posts");
  }

  const title = formData.get("title");
  const body = formData.get("body");

  const errors: Record<string, string> = {};
  if (!title) errors.title = "Title is required";
  if (!body || body.toString().length < 10)
    errors.body = "Body must be 10+ chars";

  if (Object.keys(errors).length) {
    return json({ errors }, { status: 400 });
  }

  const post = await db.post.create({ data: { title, body } });
  return redirect(`/posts/${post.id}`);
}

export default function NewPost() {
  const actionData = useActionData<typeof action>();

  return (
    <Form method="post">
      <input name="title" />
      {actionData?.errors?.title && <span>{actionData.errors.title}</span>}
      <textarea name="body" />
      {actionData?.errors?.body && <span>{actionData.errors.body}</span>}
      <button type="submit">Create</button>
    </Form>
  );
}
```

### Form Component

Remix's `<Form>` is an enhanced HTML `<form>`:

```tsx
import { Form, useNavigation } from "@remix-run/react";

function PostForm() {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <Form method="post" encType="multipart/form-data">
      <input name="title" />
      <button disabled={isSubmitting}>
        {isSubmitting ? "Saving..." : "Save"}
      </button>
    </Form>
  );
}

// Multiple forms on one page — use intent
<Form method="post">
  <input type="hidden" name="intent" value="publish" />
  <button>Publish</button>
</Form>
<Form method="post">
  <input type="hidden" name="intent" value="draft" />
  <button>Save Draft</button>
</Form>
```

## Progressive Enhancement

Forms work without JavaScript. With JS loaded, Remix intercepts submissions and uses `fetch()` + automatic revalidation. This means:

- Zero JS → full page reload, still works
- JS loaded → SPA-like experience, no full reload
- `useFetcher()` for non-navigating mutations

```tsx
function LikeButton({ postId }: { postId: string }) {
  const fetcher = useFetcher();
  const isLiking = fetcher.state !== "idle";

  return (
    <fetcher.Form method="post" action={`/posts/${postId}/like`}>
      <button disabled={isLiking}>{isLiking ? "♥" : "♡"}</button>
    </fetcher.Form>
  );
}
```

## Error Boundaries

```tsx
export function ErrorBoundary() {
  const error = useRouteError();

  if (isRouteErrorResponse(error)) {
    // Thrown Response (404, 403, etc.)
    return (
      <div>
        <h1>
          {error.status} {error.statusText}
        </h1>
        <p>{error.data}</p>
      </div>
    );
  }

  // Unexpected error
  return (
    <div>
      <h1>Something went wrong</h1>
      <p>{error instanceof Error ? error.message : "Unknown error"}</p>
    </div>
  );
}
```

Error boundaries are per-route. An error in a child route only replaces that child — parent layouts stay rendered.

## Streaming & Defer

For slow data that doesn't block the page:

```tsx
import { defer } from "@remix-run/node";
import { Await, useLoaderData } from "@remix-run/react";
import { Suspense } from "react";

export async function loader() {
  const criticalData = await db.user.findFirst(); // awaited — blocks render
  const slowData = db.analytics.getReport(); // NOT awaited — streams later

  return defer({
    user: criticalData,
    analytics: slowData,
  });
}

export default function Dashboard() {
  const { user, analytics } = useLoaderData<typeof loader>();

  return (
    <div>
      <h1>{user.name}</h1>
      <Suspense fallback={<Spinner />}>
        <Await resolve={analytics} errorElement={<p>Failed to load</p>}>
          {(data) => <AnalyticsChart data={data} />}
        </Await>
      </Suspense>
    </div>
  );
}
```

## Resource Routes

Routes with no default export — they're API endpoints:

```tsx
// app/routes/api.users.ts (no default export = resource route)
export async function loader({ request }: LoaderFunctionArgs) {
  const users = await db.user.findMany();
  return json(users);
}

// app/routes/reports.$id[.pdf].ts → /reports/123.pdf
export async function loader({ params }: LoaderFunctionArgs) {
  const pdf = await generatePDF(params.id);
  return new Response(pdf, {
    headers: {
      "Content-Type": "application/pdf",
      "Content-Disposition": `attachment; filename="report-${params.id}.pdf"`,
    },
  });
}
```

## Cookie & Session Management

```tsx
import { createCookieSessionStorage, redirect } from "@remix-run/node";

const { getSession, commitSession, destroySession } =
  createCookieSessionStorage({
    cookie: {
      name: "__session",
      httpOnly: true,
      maxAge: 60 * 60 * 24 * 7, // 1 week
      path: "/",
      sameSite: "lax",
      secrets: [process.env.SESSION_SECRET!],
      secure: process.env.NODE_ENV === "production",
    },
  });

export async function requireUser(request: Request) {
  const session = await getSession(request.headers.get("Cookie"));
  const userId = session.get("userId");
  if (!userId) throw redirect("/login");
  return userId;
}

export async function login(request: Request, userId: string) {
  const session = await getSession(request.headers.get("Cookie"));
  session.set("userId", userId);
  return redirect("/dashboard", {
    headers: { "Set-Cookie": await commitSession(session) },
  });
}
```

## Meta & Links

```tsx
export function meta({ data }: MetaArgs) {
  return [
    { title: `${data.user.name} | MyApp` },
    { name: "description", content: data.user.bio },
    { property: "og:title", content: data.user.name },
  ];
}

export function links() {
  return [
    { rel: "stylesheet", href: styles },
    {
      rel: "preload",
      href: "/fonts/inter.woff2",
      as: "font",
      crossOrigin: "anonymous",
    },
  ];
}

export function headers() {
  return { "Cache-Control": "public, max-age=300, s-maxage=3600" };
}
```

## Optimistic UI

```tsx
function TodoItem({ todo }: { todo: Todo }) {
  const fetcher = useFetcher();
  const optimisticComplete = fetcher.formData
    ? fetcher.formData.get("complete") === "true"
    : todo.complete;

  return (
    <fetcher.Form method="post">
      <input
        type="hidden"
        name="complete"
        value={(!optimisticComplete).toString()}
      />
      <button
        style={{ textDecoration: optimisticComplete ? "line-through" : "none" }}
      >
        {todo.title}
      </button>
    </fetcher.Form>
  );
}
```

## Deployment Targets

Remix compiles to different server runtimes via adapters:

| Target             | Adapter                  | Notes                |
| ------------------ | ------------------------ | -------------------- |
| Node.js            | `@remix-run/node`        | Express, standalone  |
| Vercel             | `@vercel/remix`          | Serverless functions |
| Cloudflare Workers | `@remix-run/cloudflare`  | Edge runtime         |
| Deno               | `@remix-run/deno`        | Deno Deploy          |
| Netlify            | `@netlify/remix-adapter` | Serverless           |
| Architect (AWS)    | `@remix-run/architect`   | Lambda               |

## Vite Integration

Remix uses Vite as its compiler (since v2):

```typescript
// vite.config.ts
import { vitePlugin as remix } from "@remix-run/dev";

export default defineConfig({
  plugins: [
    remix({
      ssr: true,
      future: {
        v3_fetcherPersist: true,
        v3_relativeSplatPath: true,
      },
    }),
  ],
});
```

## Remix vs Next.js

| Aspect           | Remix                          | Next.js                               |
| ---------------- | ------------------------------ | ------------------------------------- |
| Data loading     | Loaders (per-route)            | Server Components, getServerSideProps |
| Mutations        | Actions + Form                 | Server Actions, API routes            |
| Streaming        | defer + Await                  | Suspense + streaming SSR              |
| No-JS support    | Full (progressive enhancement) | Limited                               |
| Routing          | Nested file-based              | File-based + App Router               |
| State management | URL + server state             | Client state libraries                |
| Caching          | HTTP cache headers             | ISR, static generation                |
| Philosophy       | Web standards first            | React ecosystem first                 |
