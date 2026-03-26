# GraphQL: Schema Design, Resolver Patterns & Federation

## Overview

GraphQL lets clients request exactly the data they need through a declarative query language and strongly-typed schema. This note covers schema design principles, resolver execution patterns (including the N+1 problem and DataLoader), federation/stitching for distributed schemas, persisted queries, and security concerns (depth limiting, complexity analysis, caching challenges).

## Schema & Type System

### Core Concepts

A GraphQL schema defines **types**, **fields**, and **operations**:

```graphql
type User {
  id: ID!
  name: String!
  email: String!
  posts: [Post!]!
  createdAt: DateTime!
}

type Post {
  id: ID!
  title: String!
  content: String!
  author: User!
  comments: [Comment!]!
}

type Query {
  user(id: ID!): User
  posts(limit: Int = 10, offset: Int = 0): [Post!]!
}

type Mutation {
  createPost(input: CreatePostInput!): Post!
  updateUser(id: ID!, input: UpdateUserInput!): User
}
```

- **!** (non-null): Field must always be present
- **Scalar types**: Int, String, Boolean, ID, DateTime (custom)
- **Lists**: `[Post!]` (non-empty list of Posts), `[Post]` (nullable list)
- **Input types**: For mutation arguments (separate namespace from output types)

### Schema Design Patterns

#### Avoid God Types

A single User type that contains all possible fields couples unrelated concerns:

```graphql
# Bad: Entity bloat
type User {
  id: ID!
  name: String!
  # ... 20 fields later ...
  internalNotes: String
  billingAddress: Address
  supportTickets: [Ticket!]!
}

# Good: Separate bounded contexts
type User {
  id: ID!
  name: String!
  profile: UserProfile!
  orders: [Order!]!
}

type UserProfile {
  bio: String
  avatar: String
}
```

#### Explicit vs. Implicit Relationships

```graphql
# Implicit: Resolver must fetch author
type Post {
  author: User!
}

# Explicit: Better for large result sets or optional loading
type Post {
  authorId: ID!
  author: User  # nullable, omitted if not requested
}
```

#### Private/Internal Fields

GraphQL has no built-in access control. Exclude sensitive fields from the schema entirely; don't expose them conditionally:

```graphql
# Bad: Trust client to ignore
type User {
  id: ID!
  name: String!
  internalEmployeeId: String  # should not exist
}

# Good: Never expose
type User {
  id: ID!
  name: String!
  # internalEmployeeId is resolved only in business logic
}
```

## Resolvers & The N+1 Problem

### Single Resolver Execution

A resolver is a function that computes a field value:

```javascript
const resolvers = {
  Query: {
    user(parent, { id }, context, info) {
      // parent: null
      // args: { id }
      // context: shared state (DB, auth, etc.)
      // info: introspection metadata
      return fetchUser(id);  // Database query
    }
  },
  User: {
    posts(parent, args, context, info) {
      // parent: the User object
      // Returns posts for this user
      return fetchPostsByUserId(parent.id);
    }
  }
};
```

### The N+1 Problem

Naive resolvers cause exponential queries:

```javascript
// Bad: Each user's posts requires a separate query
const resolvers = {
  Query: {
    users() {
      return db.query("SELECT * FROM users LIMIT 10");  // 1 query
    }
  },
  User: {
    posts(parent) {
      return db.query("SELECT * FROM posts WHERE userId = ?", parent.id);  // N queries
    }
  }
};

// Client query requests all posts for users[0..9]
// Total: 1 + 10 = 11 queries
```

At scale (1000 users, each with 50 posts), this becomes 1001 queries—unacceptable.

### DataLoader Pattern

DataLoader batches similar queries into a single database call:

```javascript
import DataLoader from 'dataloader';

// Create a batch loader function
const postsByUserIdLoader = new DataLoader(async (userIds) => {
  // userIds: [1, 2, 3, 4, 5]
  // Fetches all posts in a single query
  const posts = await db.query(
    "SELECT * FROM posts WHERE userId = ANY(?)",
    [userIds]
  );
  // Return array ordered by userIds
  return userIds.map(id => posts.filter(p => p.userId === id));
});

// In resolvers:
const resolvers = {
  User: {
    posts(parent, args, context) {
      // DataLoader accumulates calls; batches at end of event loop tick
      return context.loaders.postsByUserId.load(parent.id);
    }
  }
};

// In server setup:
const loaders = {
  postsByUserId: postsByUserIdLoader
};
const context = { loaders };
```

With DataLoader, 10 users requesting posts = 1 database query (not 11).

### Resolver Execution & Event Loop Integration

Resolvers execute **serially** for each field in execution order. DataLoader batches **within a single request** (one event loop tick):

1. Client sends query
2. Execute Query.users (1 DB call)
3. For each user, queue User.posts (10 calls queued in DataLoader)
4. Event loop tick ends → DataLoader fires (1 batched DB call)
5. Results returned to resolvers

Key insight: **DataLoader works because GraphQL executes synchronously, allowing time for batch accumulation.**

## Schema Stitching vs. Federation

### Schema Stitching (Legacy)

Combines multiple GraphQL schemas into one by linking them via field resolvers:

```javascript
// Subgraph A (users service)
const userSchema = buildSchema(`
  type User {
    id: ID!
    name: String!
  }
  type Query { user(id: ID!): User }
`);

// Subgraph B (orders service)
const orderSchema = buildSchema(`
  type Order {
    id: ID!
    userId: ID!
    total: Float!
  }
  type Query { order(id: ID!): Order }
`);

// Gateway: Stitch together
const stitchedSchema = mergeSchemas({
  schemas: [userSchema, orderSchema],
  resolvers: {
    Order: {
      user(parent, args, context) {
        return context.userService.getUser(parent.userId);
      }
    }
  }
});
```

Issues:
- Tight coupling between gateway and subgraphs
- Schema changes require gateway redeployment
- No standardized cross-subgraph data loading
- Deprecated in favor of Apollo Federation

### Apollo Federation (Current Standard)

Each subgraph independently defines its schema and publishes an SDL (Schema Definition Language). The gateway composes schemas at query time:

```javascript
// Users subgraph
const userSchema = buildSubgraphSchema([
  typeDefs: `
    type User @key(fields: "id") {
      id: ID!
      name: String!
    }
    type Query {
      user(id: ID!): User
    }
  `,
  resolvers: {
    User: {
      __resolveReference(user) {
        return fetchUserById(user.id);
      }
    }
  }
]);

// Orders subgraph
const orderSchema = buildSubgraphSchema([
  typeDefs: `
    type Order @key(fields: "id") {
      id: ID!
      user: User
      total: Float!
    }
    type User @key(fields: "id") {
      id: ID!  # stub, filled by users subgraph
    }
    type Query {
      order(id: ID!): Order
    }
  `
]);

// Gateway (Apollo Federation Gateway)
const gateway = new ApolloGateway({
  supergraphSdl: `...` // composed schema from federation registry
});
```

Advantages:
- Subgraphs are independently deployable
- Each team manages their schema
- Gateway composes schemas at startup (no gateway redeployment)
- Standardized entity resolution via `@key` and `__resolveReference`

## Persisted Queries

Clients send a query hash instead of the full query string, reducing bandwidth and enabling server-side caching/whitelisting:

```javascript
// Client sends hash
POST /graphql
{
  "extensions": {
    "persistedQuery": {
      "version": 1,
      "sha256Hash": "abc123def456..."
    }
  }
}

// Server looks up hash → resolves to full query, executes
// If hash not found → 400 error or full query required
```

Benefits:
- Bandwidth savings (hashes are tiny)
- **Whitelist enforcement**: Only pre-registered queries execute (security)
- **Analytics**: Server can track query popularity by hash
- **DDoS mitigation**: Limit by query hash, not by client IP

Tools: Apollo Persisted Queries plugin, GraphQL Armor.

## Security: Depth Limiting & Complexity Analysis

### Query Depth Attacks

Deeply nested queries cause exponential resource consumption:

```graphql
# Malicious query: nested 100 levels deep
query {
  user {
    posts {
      author {
        posts {
          author {
            # ... repeat 100 times ...
          }
        }
      }
    }
  }
}
```

### Depth Limiting

Reject queries exceeding max depth (typically 5–10):

```javascript
const depthLimit = require('graphql-depth-limit');
server.use(depthLimit(10));
```

### Query Complexity Analysis

Each field has a cost; queries exceeding a threshold are rejected:

```javascript
// Field costs
type Query {
  users: [User!]! @cost(complexity: 10)  // 10 per user
  user(id: ID!): User @cost(complexity: 1)  // 1 per user
}

type User {
  posts: [Post!]! @cost(complexity: 5)  // 5 per post
}

// Client query:
# users (10) + posts for 10 users (5 * 10) = 60
query {
  users {
    posts {
      id
    }
  }
}
```

Complexity analysis prevents both deep nesting and wide queries (e.g., requesting 1M items).

## Caching Challenges

HTTP caching doesn't work directly for GraphQL (responses are POST to a single endpoint):

### Strategies

1. **Query hash caching**: Cache entire response by query hash + variables
2. **Fragment caching**: Cache field-level data (via entity caches or Redis)
3. **Automatic persistent caching**: Apollo Server caches query results automatically if configured
4. **Client-side caching**: Apollo Client or Urql cache query results locally

The challenge: **Different queries can request overlapping fields**. Responding to Query A with a cached Query B's response means returning data the client didn't request.

Solution: Use normalized caches (Apollo Client normalizes responses into a flat entity store).

## Fragments & Query Reuse

Fragments reduce repetition and enable reusable field selections:

```graphql
fragment UserFields on User {
  id
  name
  email
  createdAt
}

query GetPosts {
  posts {
    id
    title
    author {
      ...UserFields
    }
  }
}

mutation CreatePost($content: String!) {
  createPost(content: $content) {
    ...UserFields
  }
}
```

## Subscriptions

GraphQL Subscriptions enable real-time data via WebSocket:

```graphql
subscription OnPostCreated {
  postCreated {
    id
    title
    author { name }
  }
}
```

Implementation requires WebSocket server and pub/sub system (Redis, Kafka, or in-process). Subscriptions inherit depth/complexity attack surface—apply same limits.

## Introspection & Documentation

GraphQL schemas are self-documenting via introspection queries:

```graphql
query {
  __schema {
    types {
      name
      fields {
        name
        type { name kind }
        description
      }
    }
  }
}
```

Tools like GraphQL Voyager, Apollo Studio, and Insomnia visualize schemas. Introspection can expose sensitive structure → disable in production if needed.

## Cross-References

See also: [api-design.md](api-design.md), [web-api-patterns.md](web-api-patterns.md), [api-rest-maturity.md](api-rest-maturity.md)