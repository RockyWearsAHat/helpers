# GraphQL Patterns — Pagination, Subscriptions, Federation, Caching & Security

## Overview

GraphQL patterns extend beyond schema design to address scalability, real-time updates, distributed queries, and client-side efficiency. This note covers cursor-based pagination (Relay spec), subscription protocols (WebSocket), federation patterns for schema stitching, cache normalization strategies, and security considerations (depth limits, complexity analysis).

## Cursor-Based Pagination (Relay Connections)

The **Relay connection model** is a standardized way to express and page through one-to-many relationships.

### Connection Structure

```graphql
type User {
  id: ID!
  name: String!
  posts(first: Int, after: String, last: Int, before: String): PostConnection!
}

type PostConnection {
  edges: [PostEdge!]!
  pageInfo: PageInfo!
}

type PostEdge {
  cursor: String!
  node: Post!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}
```

### Query Example

```graphql
query {
  user(id: "123") {
    posts(first: 10, after: "abc123") {
      edges {
        cursor
        node { title }
      }
      pageInfo { hasNextPage, endCursor }
    }
  }
}
```

**Cursor:** An opaque, base64-encoded string representing position in the list. Client treats it as a handle and passes it back; server decodes to determine slice. Cursors survive schema changes better than offsets.

**Bidirectional Pagination:** `first` / `after` for forward pagination, `last` / `before` for backward. Combining both can be expensive (requires full list traversal); many implementations restrict this.

**Tradeoff:** Cursor-based is robust for offset pagination (handles insertions/deletions in real-time lists), but can be overkill for small, static datasets.

## Alternative: Offset Pagination

```graphql
type PostConnection {
  items: [Post!]!
  offset: Int!
  limit: Int!
  total: Int!
}
```

Simpler but brittle: if items are inserted/deleted between requests, a user might see duplicates or miss rows. Works for reporting and archives where data doesn't change mid-page.

## Subscriptions & WebSocket Protocol

GraphQL subscriptions enable servers to push updates to clients over a stateful connection (WebSocket, SSE with push framing, etc.).

### Subscription Example

```graphql
subscription OnUserPostCreated($userId: ID!) {
  postCreated(userId: $userId) {
    id
    title
    author { name }
  }
}
```

### Transport: graphql-ws Protocol

1. **Client → Server (connection_init):** Handshake with auth token
2. **Server → Client (connection_ack):** Acknowledges
3. **Client → Server (subscribe):** Registers subscription with ID
4. **Server → Client (next):** Sends updates as they arrive
5. Either side sends **ping/pong** to detect stale connections (heartbeat)
6. **Client → Server (complete):** Subscription done
7. **Server → Client (complete):** Subscription closed

**Authentication:** Passed in `connection_init` payload or URL params; refreshed via `connection_error` if token expires mid-subscription.

### Server Implementation Challenges

- **Fan-out:** One subscription might broadcast to thousands of clients. Requires efficient pub-sub (Redis, in-memory topic tree)
- **Backpressure:** If server pushes updates faster than client processes them, client buffer grows unbounded. Many implementations don't handle this; clients must unsubscribe if overwhelmed
- **Filtering at source:** Server must evaluate subscription conditions (e.g., only notify if author is me) before broadcasting; broadcasting all updates and filtering on client wastes bandwidth
- **Persistence:** If client reconnects, missed updates are typically lost (no replay). Some systems store recent updates or use event logs

### Client-Side Patterns

- **Subscription as augment:** Query initial data, then subscribe for changes
- **Optimistic updates:** Client renders immediately, subscription confirms (or reverts on conflict)
- **Presence tracking:** Lightweight subscriptions tracking who's viewing a resource (Slack-style "typing" indicators)

## Federation & Distributed Schemas

**Schema federation** allows teams to own subgraphs independently. A federated gateway stitches them into a unified schema.

### Core Concepts (Apollo Federation)

```graphql
# Subgraph 1: Users
extend schema {
  @link(url: "https://specs.apollo.dev/federation/v2.0")
}

type User @key(fields: "id") {
  id: ID!
  name: String!
}

# Subgraph 2: Posts
type Post @key(fields: "id") {
  id: ID!
  title: String!
  author: User!
}

extend type User {
  posts: [Post!]! @requires(fields: "id") @external
}
```

**@key:** Marks the primary key for the type in this subgraph. Gateway uses it to uniquely identify entities.

**@requires:** Indicates that a field needs data from another subgraph. Triggers a planned subgraph query before resolving.

**@external:** References a field from another subgraph; this subgraph extends the type but doesn't own the field.

### Reference Resolution

When a query requests a User and its posts, the gateway:

1. Resolves User from the Users subgraph
2. Detects that posts requires a subgraph call
3. Queries Posts subgraph with User IDs: `{ _entities(representations: [{__typename: "User", id: 1}]) }`
4. Merges results

**Tradeoff:** Powerful but adds latency (nested subgraph calls). Batch operations (`_entities` mutation) and DataLoader patterns mitigate this.

## N+1 Problem & DataLoader

A naive resolver for a list of users with posts:

```javascript
async function getUsers() {
  return users.map(user => ({
    ...user,
    posts: db.posts.find({ userId: user.id })  // N queries!
  }))
}
```

Fetching 100 users triggers 100 post queries (N+1).

### DataLoader Solution

```javascript
const postLoader = new DataLoader(async (userIds) => {
  const posts = await db.posts.find({ userId: { $in: userIds } })
  return userIds.map(id => posts.filter(p => p.userId === id))
})

const User = {
  posts(user) {
    return postLoader.load(user.id)  // batched
  }
}
```

DataLoader queues requests across a single GraphQL execution, batches them into one database query, caches results, and distributes them back.

**When to use:** Resolvers hit external data sources (database, API) on behalf of multiple parent entities. Common in fields that create a 1-to-N relationship.

## Caching Strategies

### Normalized Cache (Client)

Apollo Client, Urql, and others normalize query results into a flat object store keyed by `__typename` and `id`:

```javascript
// Query result normalized
{
  'User:1': { id: 1, name: 'Alice', posts: [Post:10, Post:11] },
  'Post:10': { id: 10, title: 'Hello', author: User:1 },
  'Post:11': { id: 11, title: 'World', author: User:1 },
}

// Update one field; cache invalidates dependent queries automatically
```

**Tradeoff:** Reduces network traffic on subsequent queries, but requires careful cache invalidation on mutations. A mutation might affect multiple cache entries; incorrect invalidation leads to stale UI.

### Persisted Queries

Client-server agreement: instead of sending full query strings, client sends a query hash. Server caches the AST.

```javascript
// Query: { id: 'abc123', variables: { userId: 1 } }
// Server looks up hash 'abc123', finds the query, executes with variables
```

**Benefit:** Reduces bandwidth, prevents clients from sending arbitrary queries. **Drawback:** Requires manual hash registration or tooling to generate hashes.

### Server-Side Response Caching

HTTP caching headers, CDN caching, or application-level response caching:

```graphql
# Resolver directive
@cacheControl(maxAge: 60)
type Query {
  user(id: ID!) @cacheControl(maxAge: 3600): User
}
```

**Tradeoff:** Works for read-heavy fields, but cache invalidation on mutations is non-trivial. Time-based expiry is simple but imprecise.

## Security Patterns

### Query Depth Limiting

Prevent deeply nested queries (DDoS):

```graphql
# Limit depth to 5 levels
query {
  user {
    posts {          # depth 2
      comments {     # depth 3
        author {
          posts {    # depth 5 (ok)
```

A query with depth > 50 might be a denial-of-service attack.

### Query Complexity Analysis

Assign weights to fields; total query complexity must stay under threshold:

```javascript
// Simple sum-based
complexity = 1 (user) * 10 (posts list) * 2 (comments per post)
// Prevents `user { posts { comments { comments { comments } } } }`
```

### Rate Limiting & Throttling

- Per-user: limit requests per minute
- Per-field: prevent abuse of expensive resolvers
- Cost-based: charge users fractional units per query based on complexity

### Authentication & Authorization

- **Field-level:** Mark schema fields with required roles/scopes; gateway enforces before resolution
- **Data-scoped:** Resolvers accept auth context and filter results (e.g., user can only see their own posts)
- **Subscription auth:** Verify token at subscription init and on periodic refresh

## Streaming & Deferred Queries

GraphQL can defer resolution of expensive fields to reduce initial response time:

```graphql
query {
  user(id: 1) { name }
  ... @defer {
    user(id: 1) { largeBiography }  # sent separately later
  }
}
```

Server sends partial response immediately, continues expensive resolution, sends updates. **Requires:** Client support (most frameworks support this now) and streaming transport.

## Conclusion

GraphQL patterns live at the intersection of client sophistication (normalization, subscriptions, deferred queries) and server pragmatism (batching, caching, complexity analysis). There's no single "correct" approach; teams adopt patterns based on workload characteristics, team size, and performance requirements. Over-engineering caching and federation can add complexity faster than it solves problems.