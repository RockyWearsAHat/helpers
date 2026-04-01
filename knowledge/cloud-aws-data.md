# AWS Data & Analytics Services

## Redshift

### Architecture

Redshift is a columnar, MPP (massively parallel processing) data warehouse. Data distributed across compute nodes in slices.

| Node Type  | Description                                                     | Use Case                         |
| ---------- | --------------------------------------------------------------- | -------------------------------- |
| RA3        | Managed storage (RMS), scales compute and storage independently | Default for new clusters         |
| DC2        | Dense compute, local SSD storage                                | Small datasets, low latency      |
| Serverless | Auto-scaling, pay per RPU-hour                                  | Variable/unpredictable workloads |

**RA3 nodes** use Redshift Managed Storage (RMS): hot data on local SSD, warm/cold data on S3, transparent caching. You pay separately for compute (node hours) and storage (per GB in RMS).

### Distribution and Sort Keys

```sql
-- Distribution styles
CREATE TABLE orders (
    order_id BIGINT,
    customer_id BIGINT,
    order_date DATE,
    amount DECIMAL(10,2)
)
DISTKEY(customer_id)   -- Hash-distributed by customer_id
SORTKEY(order_date);   -- Sorted by order_date for range scans

-- Distribution styles:
-- KEY:   Hash on specified column — co-locate join partners
-- EVEN:  Round-robin across slices — when no clear join key
-- ALL:   Full copy on every node — small dimension tables (<5M rows)
-- AUTO:  Redshift chooses (starts ALL, switches to EVEN/KEY as table grows)
```

**Compound sort key**: prefix-based, efficient for queries filtering on leading columns. **Interleaved sort key**: equal weight to all sort columns — better for ad hoc queries filtering on any column, but higher maintenance cost (requires regular `VACUUM REINDEX`).

### Redshift Spectrum

Query data directly in S3 without loading:

```sql
-- Create external schema pointing to Glue Data Catalog
CREATE EXTERNAL SCHEMA spectrum_schema
FROM DATA CATALOG
DATABASE 'analytics_db'
IAM_ROLE 'arn:aws:iam::ACCT:role/RedshiftSpectrumRole'
CREATE EXTERNAL DATABASE IF NOT EXISTS;

-- Create external table over Parquet files in S3
CREATE EXTERNAL TABLE spectrum_schema.web_logs (
    request_timestamp TIMESTAMP,
    url VARCHAR(2048),
    status_code INT,
    response_time FLOAT
)
STORED AS PARQUET
LOCATION 's3://data-lake/web-logs/';

-- Query joins Redshift tables with S3 data
SELECT c.name, COUNT(*) as hits, AVG(l.response_time)
FROM local_schema.customers c
JOIN spectrum_schema.web_logs l ON c.id = l.customer_id
WHERE l.request_timestamp > DATEADD(day, -7, GETDATE())
GROUP BY c.name;
```

Spectrum pushes filtering and aggregation to a shared Spectrum compute layer — scales independently from your cluster. Best with columnar formats (Parquet, ORC) and partitioned data.

### Redshift Serverless

```bash
aws redshift-serverless create-workgroup \
  --workgroup-name analytics \
  --namespace-name analytics-ns \
  --base-capacity 32  # RPU (Redshift Processing Units), min 8

# Scales 8–512 RPU automatically based on query complexity
# Pay only for RPU-seconds consumed during queries
# Idle = $0 (after configurable idle timeout)
```

No cluster management. Usage limits prevent runaway costs:

```sql
CREATE USAGE LIMIT daily_limit
  AMOUNT 1000  -- RPU-hours
  PERIOD DAILY
  BREACH_ACTION LOG;  -- LOG, DEACTIVATE, or ALERT
```

## Athena

### Querying S3 Data

```sql
-- Partitioned table for cost-efficient queries
CREATE EXTERNAL TABLE access_logs (
    request_ip STRING,
    request_url STRING,
    status INT,
    bytes BIGINT
)
PARTITIONED BY (year STRING, month STRING, day STRING)
STORED AS PARQUET
LOCATION 's3://logs-bucket/access-logs/'
TBLPROPERTIES ('parquet.compression'='SNAPPY');

-- Add partitions (or use MSCK REPAIR TABLE / Glue Crawler)
ALTER TABLE access_logs ADD
  PARTITION (year='2024', month='01', day='15')
  LOCATION 's3://logs-bucket/access-logs/year=2024/month=01/day=15/';

-- Partition projection (auto-generate partitions, no MSCK needed)
CREATE EXTERNAL TABLE events (
    event_id STRING,
    event_type STRING,
    payload STRING
)
PARTITIONED BY (dt STRING)
ROW FORMAT SERDE 'org.openx.data.jsonserde.JsonSerDe'
LOCATION 's3://events-bucket/events/'
TBLPROPERTIES (
  'projection.enabled' = 'true',
  'projection.dt.type' = 'date',
  'projection.dt.format' = 'yyyy-MM-dd',
  'projection.dt.range' = '2023-01-01,NOW',
  'projection.dt.interval' = '1',
  'projection.dt.interval.unit' = 'DAYS',
  'storage.location.template' = 's3://events-bucket/events/dt=${dt}/'
);
```

### Athena Performance Optimization

| Technique            | Benefit                                                |
| -------------------- | ------------------------------------------------------ |
| Use Parquet/ORC      | Columnar = read only needed columns                    |
| Partition data       | Scan only relevant partitions                          |
| Partition projection | Avoid MSCK REPAIR / Glue Crawler lag                   |
| Compress files       | Snappy (splittable with Parquet), GZIP, ZSTD           |
| Right-size files     | 128 MB–512 MB per file (avoid many small files)        |
| Use CTAS for ETL     | `CREATE TABLE ... AS SELECT` for materializing results |
| Workgroups           | Separate teams, enforce query limits, track costs      |

**Pricing**: $5 per TB scanned. Columnar + compression can reduce costs 90%+. Partitioning avoids scanning irrelevant data entirely.

### Athena for Apache Iceberg

```sql
CREATE TABLE iceberg_orders (
    order_id BIGINT,
    status STRING,
    amount DECIMAL(10,2),
    updated_at TIMESTAMP
)
LOCATION 's3://warehouse/orders/'
TBLPROPERTIES (
  'table_type' = 'ICEBERG',
  'format' = 'parquet',
  'write_compression' = 'zstd'
);

-- ACID transactions: UPDATE, DELETE, MERGE supported
UPDATE iceberg_orders SET status = 'shipped' WHERE order_id = 123;
DELETE FROM iceberg_orders WHERE status = 'cancelled' AND updated_at < current_date - interval '90' day;

-- Time travel
SELECT * FROM iceberg_orders FOR TIMESTAMP AS OF TIMESTAMP '2024-01-15 10:00:00';

-- Schema evolution
ALTER TABLE iceberg_orders ADD COLUMNS (tracking_number STRING);

-- Compaction (merge small files)
OPTIMIZE iceberg_orders REWRITE DATA USING BIN_PACK;

-- Remove old snapshots
VACUUM iceberg_orders;
```

## Glue

### Glue Crawlers

Automatically discover schema from data in S3, JDBC sources, DynamoDB:

```bash
aws glue create-crawler \
  --name s3-logs-crawler \
  --role GlueServiceRole \
  --database-name analytics \
  --targets '{
    "S3Targets": [{
      "Path": "s3://data-lake/raw/",
      "Exclusions": ["**/_temporary/**", "**/.spark-staging/**"]
    }]
  }' \
  --schema-change-policy '{"UpdateBehavior":"UPDATE_IN_DATABASE","DeleteBehavior":"LOG"}' \
  --recrawl-policy '{"RecrawlBehavior":"CRAWL_NEW_FOLDERS_ONLY"}'
```

Crawler detects format (JSON, CSV, Parquet, Avro, ORC), infers schema, registers tables in the Glue Data Catalog. Schedule: on-demand, cron, or event-triggered.

### Glue ETL Jobs

```python
# Glue PySpark job
import sys
from awsglue.transforms import *
from awsglue.utils import getResolvedOptions
from awsglue.context import GlueContext
from pyspark.context import SparkContext

args = getResolvedOptions(sys.argv, ['JOB_NAME', 'source_database', 'target_bucket'])
sc = SparkContext()
glueContext = GlueContext(sc)
spark = glueContext.spark_session

# Read from Data Catalog
raw = glueContext.create_dynamic_frame.from_catalog(
    database=args['source_database'],
    table_name='raw_events'
)

# Transform
mapped = ApplyMapping.apply(frame=raw, mappings=[
    ('event_id', 'string', 'event_id', 'string'),
    ('timestamp', 'string', 'event_time', 'timestamp'),
    ('payload.user_id', 'string', 'user_id', 'string'),
    ('payload.amount', 'double', 'amount', 'decimal(10,2)')
])

# Resolve choice types (e.g., column that's sometimes int, sometimes string)
resolved = ResolveChoice.apply(frame=mapped, choice='cast:string')

# Write to S3 as partitioned Parquet
glueContext.write_dynamic_frame.from_options(
    frame=resolved,
    connection_type='s3',
    connection_options={
        'path': f"s3://{args['target_bucket']}/processed/",
        'partitionKeys': ['year', 'month']
    },
    format='parquet',
    format_options={'compression': 'snappy'}
)
```

**Job bookmarks**: track processed data to avoid reprocessing. **Worker types**: G.1X (standard), G.2X (memory-intensive), G.025X (cost-efficient). **Flex execution**: non-urgent jobs run on spare capacity at lower cost.

### Glue Data Catalog

Central metadata store — compatible with Hive metastore. Used by Athena, Redshift Spectrum, EMR, Glue ETL.

- **Databases**: logical namespaces for tables
- **Tables**: schema + location (S3 path, JDBC connection)
- **Partitions**: physical data divisions (registered per table)
- **Connections**: JDBC endpoints, network configs for sources

**Data Catalog as Hive metastore**: EMR and Spark can use Glue Data Catalog instead of a standalone Hive metastore — shared schema across all analytics services.

## Lake Formation

Centralized data lake governance layer on top of Glue Data Catalog:

### Permissions Model

```bash
# Grant column-level access
aws lakeformation grant-permissions \
  --principal '{"DataLakePrincipal":{"DataLakePrincipalIdentifier":"arn:aws:iam::ACCT:role/AnalystRole"}}' \
  --resource '{"TableWithColumns":{
    "DatabaseName":"analytics",
    "Name":"customers",
    "ColumnNames":["name","email","purchase_count"]
  }}' \
  --permissions '["SELECT"]'

# Row-level security with data filters
aws lakeformation create-data-cells-filter \
  --table-data '{
    "DatabaseName":"analytics",
    "TableName":"orders",
    "Name":"us-orders-only",
    "RowFilter":{"FilterExpression":"region = '\''us'\''"},
    "ColumnNames":["order_id","amount","status"]
  }'
```

Lake Formation replaces S3 bucket policies + IAM policies for data access. Central "grant" model: one place to control who can access what data at database/table/column/row granularity. Works across Athena, Redshift Spectrum, EMR, Glue.

## EMR (Elastic MapReduce)

### Cluster Modes

| Mode           | Description                         | Use Case                         |
| -------------- | ----------------------------------- | -------------------------------- |
| EMR on EC2     | Traditional Hadoop cluster          | Full control, long-running       |
| EMR on EKS     | Run Spark on existing EKS           | Shared Kubernetes infrastructure |
| EMR Serverless | Auto-scaling, no cluster management | Spark/Hive jobs without ops      |

### EMR Serverless

```bash
aws emr-serverless create-application \
  --name analytics-spark \
  --type SPARK \
  --release-label emr-7.0.0 \
  --initial-capacity '{
    "DRIVER": {"workerCount": 1, "workerConfiguration": {"cpu": "2vCPU", "memory": "4GB"}},
    "EXECUTOR": {"workerCount": 4, "workerConfiguration": {"cpu": "4vCPU", "memory": "8GB"}}
  }' \
  --maximum-capacity '{"cpu": "100vCPU", "memory": "200GB", "disk": "1000GB"}'

aws emr-serverless start-job-run \
  --application-id APP_ID \
  --execution-role-arn arn:aws:iam::ACCT:role/EMRServerlessRole \
  --job-driver '{
    "sparkSubmit": {
      "entryPoint": "s3://scripts/transform.py",
      "sparkSubmitParameters": "--conf spark.sql.catalog.glue=org.apache.iceberg.spark.SparkCatalog"
    }
  }'
```

Pre-initialized capacity avoids cold starts. Auto-scales executors based on workload. Pay per vCPU/GB-hour used.

### Framework Quick Reference

| Framework    | Best For                        | Notes                              |
| ------------ | ------------------------------- | ---------------------------------- |
| Spark        | General ETL, SQL, ML, streaming | Most common choice                 |
| Hive         | SQL-on-Hadoop, batch processing | Familiar SQL interface             |
| Presto/Trino | Interactive SQL queries         | Fast federated queries             |
| HBase        | Random read/write at scale      | Column-family NoSQL                |
| Flink        | Real-time stream processing     | Lower latency than Spark Streaming |

## OpenSearch

### Architecture

Managed Elasticsearch/OpenSearch fork. Cluster = domain.

```bash
aws opensearch create-domain \
  --domain-name logs \
  --engine-version OpenSearch_2.11 \
  --cluster-config '{"InstanceType":"r6g.large.search","InstanceCount":3,"DedicatedMasterEnabled":true,"DedicatedMasterType":"m6g.large.search","DedicatedMasterCount":3,"ZoneAwarenessEnabled":true,"ZoneAwarenessConfig":{"AvailabilityZoneCount":3}}' \
  --ebs-options '{"EBSEnabled":true,"VolumeType":"gp3","VolumeSize":100,"Iops":3000,"Throughput":125}' \
  --encryption-at-rest-options Enabled=true \
  --node-to-node-encryption-options Enabled=true \
  --domain-endpoint-options EnforceHTTPS=true,TLSSecurityPolicy=Policy-Min-TLS-1-2-PFS-2023-10 \
  --advanced-security-options Enabled=true,InternalUserDatabaseEnabled=false,MasterUserOptions={MasterUserARN=arn:aws:iam::ACCT:role/OpenSearchAdmin}
```

### Index Patterns

```json
PUT /logs-2024.01.15
{
  "settings": {
    "number_of_shards": 3,
    "number_of_replicas": 1,
    "index.codec": "zstd_no_dict",
    "index.refresh_interval": "30s"
  },
  "mappings": {
    "properties": {
      "@timestamp": { "type": "date" },
      "message": { "type": "text", "analyzer": "standard" },
      "level": { "type": "keyword" },
      "service": { "type": "keyword" },
      "request_id": { "type": "keyword" },
      "duration_ms": { "type": "float" }
    }
  }
}
```

**UltraWarm**: warm storage tier for infrequently accessed data. **Cold storage**: cheapest tier, detach indices and reattach on demand. **ISM (Index State Management)**: automate lifecycle — hot → warm → cold → delete.

## QuickSight

Serverless BI and dashboarding:

- **SPICE**: in-memory calculation engine (Super-fast, Parallel, In-memory Calculation Engine)
- **Data sources**: Athena, Redshift, RDS, S3, OpenSearch, Salesforce, Jira, etc.
- **ML Insights**: anomaly detection, forecasting, auto-narratives (AI-generated summaries)
- **Embedding**: embed dashboards in web apps via anonymous or authenticated embedding
- **Row-level security**: restrict data visibility per user/group
- **Pricing**: per-session pricing (readers) or per-user (authors) — scales down to $0 for inactive readers

**Q (natural language queries)**: users type questions ("What were total sales last quarter?"), QuickSight generates visualizations. Requires topic configuration defining business terms and metrics.
