# MongoDB

## Document Model

MongoDB stores data as BSON (Binary JSON) documents in collections. No enforced schema by default — documents in the same collection can have different shapes.

### Document Size and Structure

- Max document size: **16MB**
- Max nesting depth: 100 levels
- Field names are stored in every document (use concise names for high-volume collections)
- `_id` field is mandatory and unique per collection (auto-generated ObjectId if omitted)

### ObjectId Anatomy

12 bytes: `[4-byte timestamp][5-byte random][3-byte counter]`

```javascript
const id = new ObjectId();
id.getTimestamp(); // Date embedded in the ID — free creation timestamp
```

### Data Modeling Patterns

| Pattern   | When                                  | Example                                                         |
| --------- | ------------------------------------- | --------------------------------------------------------------- |
| Embed     | 1:1 or 1:few, data accessed together  | User → address                                                  |
| Reference | 1:many, many:many, independent access | Order → product_id                                              |
| Subset    | Large doc, not all accessed at once   | Movie with top 10 reviews embedded, rest in separate collection |
| Bucket    | Time-series, high-frequency inserts   | IoT readings grouped by hour                                    |
| Computed  | Expensive aggregations cached         | Store running totals alongside raw data                         |
| Outlier   | Most docs small, rare ones huge       | Flag `has_overflow: true`, store excess in linked collection    |

Rule of thumb: **if you always read it together, embed it; if you sometimes read it independently, reference it**.

## Aggregation Pipeline

Stages execute sequentially — each stage transforms the document stream. Order matters for performance.

### Core Stages

```javascript
db.orders.aggregate([
  { $match: { status: "completed", date: { $gte: ISODate("2024-01-01") } } },
  { $unwind: "$items" },
  {
    $group: {
      _id: "$items.product_id",
      totalRevenue: { $sum: { $multiply: ["$items.price", "$items.qty"] } },
      orderCount: { $sum: 1 },
      avgPrice: { $avg: "$items.price" },
    },
  },
  { $sort: { totalRevenue: -1 } },
  { $limit: 20 },
  {
    $lookup: {
      from: "products",
      localField: "_id",
      foreignField: "_id",
      as: "product",
    },
  },
  { $unwind: "$product" },
  {
    $project: {
      name: "$product.name",
      totalRevenue: 1,
      orderCount: 1,
      avgPrice: { $round: ["$avgPrice", 2] },
    },
  },
]);
```

### Key Stage Reference

| Stage                     | Purpose                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `$match`                  | Filter documents (use early to reduce pipeline work)                                                       |
| `$group`                  | Group by key, apply accumulators (`$sum`, `$avg`, `$min`, `$max`, `$push`, `$addToSet`, `$first`, `$last`) |
| `$project` / `$addFields` | Reshape documents, add computed fields                                                                     |
| `$unwind`                 | Deconstruct array into one doc per element                                                                 |
| `$lookup`                 | Left outer join to another collection                                                                      |
| `$sort`                   | Order documents                                                                                            |
| `$limit` / `$skip`        | Pagination                                                                                                 |
| `$facet`                  | Run multiple pipelines in parallel on same input                                                           |
| `$bucket` / `$bucketAuto` | Group into ranges                                                                                          |
| `$graphLookup`            | Recursive lookup (tree/graph traversal)                                                                    |
| `$merge` / `$out`         | Write results to collection                                                                                |
| `$unionWith`              | Combine documents from multiple collections                                                                |
| `$setWindowFields`        | Window functions (5.0+)                                                                                    |
| `$densify`                | Fill gaps in time-series (5.3+)                                                                            |
| `$fill`                   | Fill null/missing values (5.3+)                                                                            |

### Performance Tips

- Put `$match` and `$sort` at the beginning — they can use indexes
- `$match` + `$sort` + `$limit` sequence is heavily optimized
- `$lookup` with `pipeline` sub-query is more flexible than simple field matching
- Check `explain("executionStats")` for pipeline plans

## Indexing

### Index Types

```javascript
// Single field
db.users.createIndex({ email: 1 }); // ascending
db.users.createIndex({ score: -1 }); // descending

// Compound
db.orders.createIndex({ customer_id: 1, created_at: -1 });

// Multikey (arrays — auto-detected)
db.posts.createIndex({ tags: 1 });

// Text (full-text search)
db.articles.createIndex({ title: "text", body: "text" });

// Hashed (for hash-based sharding)
db.users.createIndex({ user_id: "hashed" });

// 2dsphere (geospatial)
db.places.createIndex({ location: "2dsphere" });

// Wildcard (dynamic/polymorphic schemas)
db.events.createIndex({ "metadata.$**": 1 });

// TTL (auto-expire documents)
db.sessions.createIndex({ expires_at: 1 }, { expireAfterSeconds: 0 });

// Partial (only index documents matching a filter)
db.orders.createIndex(
  { status: 1 },
  { partialFilterExpression: { status: "active" } },
);

// Unique
db.users.createIndex({ email: 1 }, { unique: true });

// Sparse (skip docs without the field)
db.users.createIndex({ phone: 1 }, { sparse: true });

// Hidden (keep index but exclude from planner — test before dropping)
db.users.hideIndex("email_1");
```

### ESR Rule for Compound Indexes

**E**quality → **S**ort → **R**ange — order compound index fields this way for optimal query performance.

```javascript
// Query: { status: "active", age: { $gte: 25 } }, sort: { name: 1 }
// Optimal index:
db.users.createIndex({ status: 1, name: 1, age: 1 }); // E, S, R
```

## Schema Validation

```javascript
db.createCollection("users", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["name", "email", "role"],
      properties: {
        name: { bsonType: "string", maxLength: 200 },
        email: { bsonType: "string", pattern: "^.+@.+\\..+$" },
        role: { enum: ["user", "admin", "moderator"] },
        age: { bsonType: "int", minimum: 0, maximum: 150 },
        address: {
          bsonType: "object",
          properties: {
            city: { bsonType: "string" },
            zip: { bsonType: "string" },
          },
        },
      },
    },
  },
  validationLevel: "strict", // "moderate" skips existing invalid docs
  validationAction: "error", // "warn" logs but allows
});
```

## Multi-Document Transactions

Available since 4.0 (replica sets) and 4.2 (sharded clusters).

```javascript
const session = client.startSession();
try {
  session.startTransaction({
    readConcern: { level: "snapshot" },
    writeConcern: { w: "majority" },
    readPreference: "primary",
  });

  await orders.insertOne({ customer_id: "abc", total: 99.99 }, { session });
  await inventory.updateOne(
    { sku: "widget-1", qty: { $gte: 1 } },
    { $inc: { qty: -1 } },
    { session },
  );

  await session.commitTransaction();
} catch (error) {
  await session.abortTransaction();
  throw error;
} finally {
  session.endSession();
}
```

Transaction limits: 60-second default timeout, 16MB total oplog entry size. Design for short transactions — don't hold them open.

## Change Streams

Real-time notifications on data changes. Requires replica set or sharded cluster.

```javascript
const pipeline = [
  {
    $match: {
      "fullDocument.status": "urgent",
      operationType: { $in: ["insert", "update"] },
    },
  },
];

const changeStream = db.collection("tickets").watch(pipeline, {
  fullDocument: "updateLookup", // include full doc on updates
});

changeStream.on("change", (event) => {
  console.log(event.operationType, event.fullDocument);
  // event.clusterTime, event.documentKey, event.updateDescription
});

// Resume after failure using resume token
const resumeToken = event._id;
const resumed = collection.watch([], { resumeAfter: resumeToken });
```

## Replica Sets

Minimum 3 members (1 primary + 2 secondaries, or 1 primary + 1 secondary + 1 arbiter).

### Read Preferences

| Mode                 | Reads From                     | Use Case                   |
| -------------------- | ------------------------------ | -------------------------- |
| `primary`            | Primary only (default)         | Consistency-critical reads |
| `primaryPreferred`   | Primary, fallback to secondary | Default for most apps      |
| `secondary`          | Secondaries only               | Analytics, reporting       |
| `secondaryPreferred` | Secondary, fallback to primary | Reduce primary load        |
| `nearest`            | Lowest latency member          | Geo-distributed reads      |

### Write Concern

| Level           | Guarantee                              |
| --------------- | -------------------------------------- |
| `w: 0`          | Fire and forget                        |
| `w: 1`          | Acknowledged by primary                |
| `w: "majority"` | Acknowledged by majority (recommended) |
| `j: true`       | Written to journal                     |

## Sharding

### Architecture

- **mongos**: query router (stateless, run multiple)
- **config servers**: metadata and chunk mapping (3-member replica set)
- **shards**: data partitions (each is a replica set)

### Shard Key Selection

| Criteria        | Good Key                 | Bad Key                               |
| --------------- | ------------------------ | ------------------------------------- |
| Cardinality     | High (email, user_id)    | Low (status, boolean)                 |
| Frequency       | Even distribution        | Monotonic (timestamp, auto-increment) |
| Query isolation | Included in most queries | Rarely queried                        |

```javascript
// Hashed shard key — even distribution but no range queries
sh.shardCollection("mydb.events", { user_id: "hashed" });

// Ranged shard key — supports range queries
sh.shardCollection("mydb.logs", { tenant_id: 1, created_at: 1 });
```

**Zone sharding**: pin data ranges to specific shards (e.g., EU data stays on EU shards for compliance).

## Mongoose ODM Patterns

### Schema Definition

```javascript
const userSchema = new Schema(
  {
    name: { type: String, required: true, trim: true, index: true },
    email: { type: String, required: true, unique: true, lowercase: true },
    role: { type: String, enum: ["user", "admin"], default: "user" },
    profile: {
      avatar: String,
      bio: { type: String, maxlength: 500 },
    },
    tags: [{ type: String, index: true }],
    company: { type: Schema.Types.ObjectId, ref: "Company" },
  },
  {
    timestamps: true, // createdAt, updatedAt
    toJSON: {
      virtuals: true,
      transform: (_, ret) => {
        delete ret.__v;
        return ret;
      },
    },
  },
);

// Virtual (not stored in DB)
userSchema.virtual("posts", {
  ref: "Post",
  localField: "_id",
  foreignField: "author",
});

// Instance method
userSchema.methods.isAdmin = function () {
  return this.role === "admin";
};

// Static method
userSchema.statics.findByEmail = function (email) {
  return this.findOne({ email });
};

// Middleware
userSchema.pre("save", async function (next) {
  if (this.isModified("password")) {
    this.password = await bcrypt.hash(this.password, 12);
  }
  next();
});

// Discriminators (single collection inheritance)
const employeeSchema = new Schema({ department: String, salary: Number });
const Employee = User.discriminator("Employee", employeeSchema);
```

### Query Patterns

```javascript
// Lean queries — skip Mongoose hydration for read-only ops (2-5x faster)
const users = await User.find({ role: "admin" }).lean();

// Population (joins)
const post = await Post.findById(id)
  .populate("author", "name email")
  .populate("comments");

// Pagination
const page = 2,
  limit = 20;
const results = await User.find(filter)
  .sort({ createdAt: -1 })
  .skip((page - 1) * limit)
  .limit(limit);

// Bulk operations
const ops = items.map((item) => ({
  updateOne: {
    filter: { sku: item.sku },
    update: { $set: item },
    upsert: true,
  },
}));
await Product.bulkWrite(ops, { ordered: false });
```
