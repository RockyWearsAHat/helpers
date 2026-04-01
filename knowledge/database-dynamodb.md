# Amazon DynamoDB

## Data Model

DynamoDB is a fully managed, serverless key-value and document database. Tables have items (rows) with attributes (columns). Schema is only enforced on the primary key — all other attributes are flexible per item.

### Primary Key Design

| Key Type                 | Structure | Use Case                                        |
| ------------------------ | --------- | ----------------------------------------------- |
| Partition key only       | `PK`      | Simple lookups by unique ID                     |
| Partition key + sort key | `PK + SK` | Range queries, hierarchical data, relationships |

```
Table: Users
  PK: userId (partition key only)
  → GetItem by userId

Table: Orders
  PK: customerId (partition key)
  SK: orderId (sort key)
  → Query all orders for a customer, get specific order
```

The partition key determines which physical partition stores the item (hashed). The sort key determines order within that partition (stored sorted, enabling range queries).

### Single-Table Design

The dominant DynamoDB pattern: store multiple entity types in one table using generic key names.

```
PK              | SK                  | Type     | Data...
----------------|---------------------|----------|--------
USER#alice      | PROFILE             | User     | name, email
USER#alice      | ORDER#2025-001      | Order    | total, status
USER#alice      | ORDER#2025-002      | Order    | total, status
ORG#acme        | METADATA            | Org      | name, plan
ORG#acme        | MEMBER#alice        | Member   | role, joined
PRODUCT#widget  | METADATA            | Product  | name, price
PRODUCT#widget  | REVIEW#alice#20250101| Review  | rating, text
```

**Access patterns served by this layout**:

- Get user profile: `PK = USER#alice, SK = PROFILE`
- List user's orders: `PK = USER#alice, SK begins_with ORDER#`
- Get org members: `PK = ORG#acme, SK begins_with MEMBER#`
- Get product reviews: `PK = PRODUCT#widget, SK begins_with REVIEW#`

### Item Size and Limits

| Limit                    | Value                        |
| ------------------------ | ---------------------------- |
| Max item size            | 400 KB                       |
| Max partition key length | 2048 bytes                   |
| Max sort key length      | 1024 bytes                   |
| Max attributes per item  | No hard limit (400 KB total) |
| Max partition throughput | 3000 RCU / 1000 WCU          |

## Secondary Indexes

### Global Secondary Index (GSI)

- Different partition key and/or sort key from the base table
- Has its own provisioned capacity (separate from the table)
- Eventually consistent reads only
- Can be created/deleted anytime
- Max 20 GSIs per table

```
GSI: GSI1
  GSI1PK = Type     | GSI1SK = SK
  → Query all Orders: GSI1PK = "Order"
  → Query all Users:  GSI1PK = "User"

GSI: GSI2 (inverted index)
  GSI2PK = SK        | GSI2SK = PK
  → Find which user placed ORDER#2025-001
  → Find all members of an org
```

**Sparse indexes**: If the GSI key attributes don't exist on an item, that item is not projected into the GSI. Useful for filtering.

```
GSI: ActiveOrders
  PK = status (only items with "status" attribute appear)
  → Query where status = "PENDING"
```

### Local Secondary Index (LSI)

- Same partition key as the base table, different sort key
- Shares capacity with the base table
- Supports strongly consistent reads
- Must be created at table creation time (cannot add later)
- Max 5 LSIs per table
- Imposes a 10 GB partition size limit

```
Table PK: customerId, SK: orderId
LSI SK: orderDate
→ Query customer's orders sorted by date instead of orderId
```

### GSI Overloading

Use generic GSI attribute names to serve multiple access patterns:

```
Base Table: PK, SK, GSI1PK, GSI1SK, GSI2PK, GSI2SK

For Users:    GSI1PK = email,        GSI1SK = created
For Orders:   GSI1PK = status,       GSI1SK = orderDate
For Products: GSI1PK = category,     GSI1SK = price
```

## DynamoDB Streams

Captures a time-ordered sequence of item-level changes (insert, modify, delete). Each stream record appears exactly once and in order per item.

| View Type            | Captured Data                 |
| -------------------- | ----------------------------- |
| `KEYS_ONLY`          | Only the key attributes       |
| `NEW_IMAGE`          | Full item after modification  |
| `OLD_IMAGE`          | Full item before modification |
| `NEW_AND_OLD_IMAGES` | Both before and after         |

**Common stream consumers**:

- Lambda triggers for event-driven architectures
- Replication to Elasticsearch/OpenSearch for full-text search
- Materialized view maintenance (update denormalized copies)
- Cross-region replication (Global Tables use streams internally)
- Audit logging and CDC (change data capture)

```python
# Lambda trigger receives stream records
def handler(event, context):
    for record in event['Records']:
        if record['eventName'] == 'INSERT':
            new_item = record['dynamodb']['NewImage']
            # process new item
        elif record['eventName'] == 'MODIFY':
            old_item = record['dynamodb']['OldImage']
            new_item = record['dynamodb']['NewImage']
            # process change
```

## DAX (DynamoDB Accelerator)

In-memory cache cluster sitting in front of DynamoDB. Microsecond read latency for cached items.

- **Item cache**: Caches individual GetItem/BatchGetItem results by primary key
- **Query cache**: Caches Query/Scan results by exact parameter set
- **Write-through**: Writes go through DAX to DynamoDB, updating the item cache
- **TTL**: Default 5 minutes for items, 1 minute for queries

**When to use**: Read-heavy workloads with repeated access to the same items. Not useful for write-heavy workloads or scans over large datasets.

**When NOT to use**: Strongly consistent reads (DAX is eventually consistent only), write-heavy tables, infrequent reads.

## TTL (Time-To-Live)

```python
# Enable TTL on a table (one TTL attribute per table)
client.update_time_to_live(
    TableName='sessions',
    TimeToLiveSpecification={'Enabled': True, 'AttributeName': 'expiresAt'}
)

# Set TTL on an item (epoch seconds)
import time
table.put_item(Item={
    'PK': 'SESSION#abc123',
    'SK': 'METADATA',
    'expiresAt': int(time.time()) + 3600  # expires in 1 hour
})
```

- TTL attribute must be a Number type containing a Unix epoch timestamp (seconds)
- Items are typically deleted within 48 hours of expiry (not instantaneous)
- Expired items still appear in queries until actually deleted — filter them client-side
- Deletions are free (no WCU consumed) and appear in Streams as system deletes

## Transactions

```python
# TransactWriteItems: up to 100 items, all-or-nothing
client.transact_write_items(
    TransactItems=[
        {'Put': {
            'TableName': 'Orders',
            'Item': {'PK': {'S': 'ORDER#123'}, 'status': {'S': 'PLACED'}},
            'ConditionExpression': 'attribute_not_exists(PK)'
        }},
        {'Update': {
            'TableName': 'Inventory',
            'Key': {'PK': {'S': 'PRODUCT#widget'}},
            'UpdateExpression': 'SET stock = stock - :qty',
            'ConditionExpression': 'stock >= :qty',
            'ExpressionAttributeValues': {':qty': {'N': '1'}}
        }}
    ]
)

# TransactGetItems: up to 100 items, consistent snapshot read
response = client.transact_get_items(
    TransactItems=[
        {'Get': {'TableName': 'Orders', 'Key': {'PK': {'S': 'ORDER#123'}}}},
        {'Get': {'TableName': 'Inventory', 'Key': {'PK': {'S': 'PRODUCT#widget'}}}}
    ]
)
```

Transactions cost 2x the WCU/RCU of non-transactional operations. Max 4 MB total per transaction.

## Conditional Expressions

```python
# Only update if item exists and meets condition
table.update_item(
    Key={'PK': 'USER#alice', 'SK': 'PROFILE'},
    UpdateExpression='SET #s = :new_status',
    ConditionExpression='#s = :expected AND attribute_exists(PK)',
    ExpressionAttributeNames={'#s': 'status'},
    ExpressionAttributeValues={':new_status': 'active', ':expected': 'pending'}
)

# Optimistic locking with version number
table.put_item(
    Item={'PK': 'DOC#1', 'content': 'updated', 'version': 2},
    ConditionExpression='version = :v',
    ExpressionAttributeValues={':v': 1}
)
```

### Expression Reference

| Expression Type          | Purpose                                   | Example                               |
| ------------------------ | ----------------------------------------- | ------------------------------------- |
| `UpdateExpression`       | SET, REMOVE, ADD, DELETE attributes       | `SET #a = :val, REMOVE #b`            |
| `ConditionExpression`    | Conditional writes                        | `attribute_exists(PK) AND #s <> :val` |
| `FilterExpression`       | Post-query filtering (still consumes RCU) | `#type = :t`                          |
| `ProjectionExpression`   | Select specific attributes                | `#name, #email, orders[0]`            |
| `KeyConditionExpression` | Filter on sort key in Query               | `PK = :pk AND SK BETWEEN :a AND :b`   |

Sort key operators in KeyConditionExpression: `=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`.

## Query vs Scan

|                            | Query                           | Scan                             |
| -------------------------- | ------------------------------- | -------------------------------- |
| **Requires partition key** | Yes                             | No                               |
| **Performance**            | Efficient (reads one partition) | Reads entire table               |
| **Cost**                   | Only RCU for matched partition  | RCU for every item in table      |
| **Pagination**             | Use `LastEvaluatedKey`          | Same, but slower                 |
| **Parallel**               | No                              | Yes (`Segment`, `TotalSegments`) |

```python
# Query: efficient
response = table.query(
    KeyConditionExpression=Key('PK').eq('USER#alice') & Key('SK').begins_with('ORDER#'),
    ScanIndexForward=False,  # descending sort key order
    Limit=10
)

# Scan: expensive but sometimes necessary
response = table.scan(
    FilterExpression=Attr('type').eq('User') & Attr('age').gt(30),
    ProjectionExpression='PK, #n, age',
    ExpressionAttributeNames={'#n': 'name'}
)

# Parallel scan (for large tables)
import concurrent.futures
def scan_segment(segment):
    return table.scan(Segment=segment, TotalSegments=4)

with concurrent.futures.ThreadPoolExecutor() as executor:
    results = list(executor.map(scan_segment, range(4)))
```

## Capacity Modes

| Mode                       | Pricing             | Best For                                  |
| -------------------------- | ------------------- | ----------------------------------------- |
| On-Demand                  | Per-request pricing | Unpredictable traffic, new tables         |
| Provisioned                | Reserved RCU/WCU    | Predictable traffic, cost optimization    |
| Provisioned + Auto Scaling | Dynamic adjustment  | Variable but somewhat predictable traffic |

**RCU**: 1 RCU = 1 strongly consistent read/sec for items up to 4 KB (or 2 eventually consistent reads).
**WCU**: 1 WCU = 1 write/sec for items up to 1 KB.

## Hot Partition Mitigation

When one partition key gets disproportionate traffic, it creates a hot partition.

**Strategies**:

- **Write sharding**: Append random suffix to partition key (`USER#alice#3`), query all shards
- **Burst capacity**: DynamoDB reserves unused capacity for 5-minute bursts (300 seconds of unused throughput)
- **Adaptive capacity**: Automatically redistributes throughput to hot partitions (within table limits)
- **Distribute access**: Design keys to spread load evenly (avoid sequential IDs, use UUIDs or composite keys)

```
# Hot partition: all writes to one partition
PK = "COUNTER"  → single hot partition

# Sharded: distribute writes across partitions
PK = "COUNTER#0" through "COUNTER#9"
→ Aggregate by reading all 10 shards
```

## Adjacency List Pattern

Model many-to-many relationships in single-table design using the sort key to represent edges:

```
PK              | SK              | Data
----------------|-----------------|------------------
INVOICE#1001    | METADATA        | date, total, status
INVOICE#1001    | BILL#acme       | billing entity
INVOICE#1001    | ITEM#widget     | quantity, price
BILL#acme       | BILL#acme       | name, address
BILL#acme       | INVOICE#1001    | (reverse lookup)
BILL#acme       | INVOICE#1002    | (reverse lookup)
```

**With GSI (inverted index)**: `GSI1PK = SK, GSI1SK = PK`

- Find all invoices for a billing entity: Query GSI where `GSI1PK = BILL#acme, GSI1SK begins_with INVOICE#`
- Find all items for an invoice: Query base table where `PK = INVOICE#1001, SK begins_with ITEM#`

## Design Considerations

| Practice                           | Details                                                               |
| ---------------------------------- | --------------------------------------------------------------------- |
| Start with access patterns         | List all queries before designing the table                           |
| Use composite sort keys            | `STATUS#DATE#ID` enables multiple query patterns on one sort key      |
| Keep items small                   | Large items waste RCU/WCU; split large attributes into separate items |
| Use projection expressions         | Only fetch attributes you need                                        |
| Avoid Scan                         | Almost always a design smell; add a GSI instead                       |
| Use BatchGetItem/BatchWriteItem    | Up to 100 items / 25 items respectively; automatic parallelism        |
| Handle throttling                  | Use exponential backoff; SDK handles this by default                  |
| Enable Point-in-Time Recovery      | 35-day continuous backup window                                       |
| Use Global Tables for multi-region | Active-active replication with conflict resolution (last-writer-wins) |
