# AWS Observability

## CloudWatch

### Metrics

**Namespaces**: every AWS service publishes to its own namespace (`AWS/EC2`, `AWS/ECS`, `AWS/Lambda`, etc.). Custom metrics use any namespace.

```bash
# Publish custom metric
aws cloudwatch put-metric-data \
  --namespace "MyApp" \
  --metric-name "OrdersProcessed" \
  --value 42 \
  --unit Count \
  --dimensions Environment=prod,Service=api

# High-resolution metrics (1-second granularity)
aws cloudwatch put-metric-data \
  --namespace "MyApp" \
  --metric-name "Latency" \
  --value 23.5 \
  --unit Milliseconds \
  --storage-resolution 1
```

| Resolution           | Retention            | Cost     |
| -------------------- | -------------------- | -------- |
| 1-second (high-res)  | 3 hours              | Higher   |
| 60-second (standard) | 15 days              | Standard |
| 5-minute             | 63 days              | Standard |
| 1-hour               | 455 days (15 months) | Standard |

**Metric Math**: combine metrics with expressions: `METRICS("m1") / METRICS("m2") * 100` for percentages. `ANOMALY_DETECTION_BAND(m1, 2)` creates ML-based anomaly band.

**EMF (Embedded Metric Format)**: publish metrics from application logs — embed structured JSON in log output, CloudWatch extracts metrics automatically:

```json
{
  "_aws": {
    "Timestamp": 1234567890,
    "CloudWatchMetrics": [
      {
        "Namespace": "MyApp",
        "Dimensions": [["Service", "Environment"]],
        "Metrics": [
          { "Name": "ProcessingTime", "Unit": "Milliseconds" },
          { "Name": "ItemCount", "Unit": "Count" }
        ]
      }
    ]
  },
  "Service": "payment",
  "Environment": "prod",
  "ProcessingTime": 125,
  "ItemCount": 3
}
```

### Alarms

Three states: **OK**, **ALARM**, **INSUFFICIENT_DATA**.

```bash
aws cloudwatch put-metric-alarm \
  --alarm-name "HighCPU" \
  --metric-name CPUUtilization \
  --namespace AWS/EC2 \
  --statistic Average \
  --period 300 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold \
  --evaluation-periods 3 \
  --datapoints-to-alarm 2 \
  --alarm-actions arn:aws:sns:us-east-1:ACCT:ops-alerts \
  --ok-actions arn:aws:sns:us-east-1:ACCT:ops-alerts \
  --treat-missing-data notBreaching
```

**Composite Alarms**: combine multiple alarms with AND/OR logic to reduce noise:

```bash
aws cloudwatch put-composite-alarm \
  --alarm-name "ServiceDown" \
  --alarm-rule 'ALARM("HighCPU") AND ALARM("High5xxRate")' \
  --alarm-actions arn:aws:sns:us-east-1:ACCT:page-oncall
```

`treat-missing-data` options: `breaching` (treat as threshold exceeded), `notBreaching` (treat as within threshold), `ignore` (maintain current state), `missing` (transition to INSUFFICIENT_DATA).

**Anomaly detection alarms**: use ML model instead of static threshold — adapts to daily/weekly patterns automatically.

### CloudWatch Logs Insights

Query language for searching and analyzing log data across log groups:

```sql
-- Top 10 most expensive Lambda invocations
fields @timestamp, @duration, @memorySize, @billedDuration
| filter @type = "REPORT"
| sort @duration desc
| limit 10

-- Error rate over time (5-minute bins)
filter @message like /ERROR/
| stats count(*) as errors by bin(5m)

-- Parse structured logs and aggregate
parse @message "user=* action=* latency=*ms" as user, action, latency
| stats avg(latency) as avg_lat, max(latency) as max_lat, count(*) as cnt by action
| sort avg_lat desc

-- P99 latency from ALB access logs
fields @timestamp, target_processing_time
| stats pct(target_processing_time, 99) as p99,
        pct(target_processing_time, 95) as p95,
        avg(target_processing_time) as avg_lat
  by bin(5m)

-- Find cold starts in Lambda
filter @message like /Init Duration/
| parse @message "Init Duration: * ms" as initDuration
| stats count(*) as coldStarts, avg(initDuration) as avgInit by bin(1h)
```

Key functions: `fields`, `filter`, `stats` (count, sum, avg, min, max, pct), `sort`, `limit`, `parse` (glob and regex), `bin()`.

**Cross-account log querying**: use CloudWatch cross-account observability to query logs from linked accounts.

### CloudWatch Synthetics (Canaries)

Automated scripts that monitor endpoints and APIs on a schedule:

```javascript
// Canary script (Node.js + Puppeteer)
const { synthetics } = require("Synthetics");
const syntheticsConfiguration = synthetics.getConfiguration();

syntheticsConfiguration.setConfig({
  screenshotOnStepStart: true,
  screenshotOnStepSuccess: true,
  screenshotOnStepFailure: true,
});

const pageLoadBlueprint = async function () {
  const page = await synthetics.getPage();
  const response = await page.goto("https://app.example.com/login", {
    waitUntil: "networkidle0",
    timeout: 30000,
  });
  if (response.status() !== 200) {
    throw new Error(`Expected 200, got ${response.status()}`);
  }
  await page.type("#email", "canary@example.com");
  await page.click("#login-btn");
  await page.waitForSelector("#dashboard", { timeout: 10000 });
};
exports.handler = async () => pageLoadBlueprint();
```

Canary types: heartbeat (simple URL check), API (REST API testing), broken link checker, visual monitoring (screenshot diff), GUI workflow (multi-step user flows).

Runs every 1-60 minutes. Stores results + screenshots + HAR files in S3. Integrates with CloudWatch Alarms.

### CloudWatch RUM (Real User Monitoring)

Client-side JavaScript snippet that captures real user performance data:

- **Page load timing**: TTFB, FCP, LCP, FID, CLS (Core Web Vitals)
- **JavaScript errors**: stack traces, error counts by page
- **HTTP errors**: failed API calls from the browser
- **Session tracking**: user journeys, page flows
- **Performance by dimension**: browser, device, country, page

Data feeds into CloudWatch metrics + X-Ray traces. Sampling rate configurable (1-100%). Cookie consent required for session tracking.

## X-Ray

### Tracing Concepts

- **Trace** — end-to-end request path across services
- **Segment** — work done by a single service
- **Subsegment** — granular breakdown (SQL queries, HTTP calls, custom)
- **Annotations** — indexed key-value pairs for filtering (`user_id`, `order_type`)
- **Metadata** — non-indexed data attached to segments (request bodies, debug info)

```python
from aws_xray_sdk.core import xray_recorder, patch_all

patch_all()  # Auto-instrument boto3, requests, sqlite3, etc.

@xray_recorder.capture('process_order')
def process_order(order_id):
    subsegment = xray_recorder.current_subsegment()
    subsegment.put_annotation('order_id', order_id)
    subsegment.put_metadata('order_details', order, 'business')
    # ... process order
```

### Sampling Rules

Control trace volume to manage costs:

```json
{
  "RuleName": "api-orders",
  "Priority": 100,
  "FixedRate": 0.1,
  "ReservoirSize": 5,
  "ServiceName": "order-service",
  "ServiceType": "*",
  "Host": "*",
  "HTTPMethod": "POST",
  "URLPath": "/api/orders/*",
  "ResourceARN": "*"
}
```

**ReservoirSize** = guaranteed traces per second. **FixedRate** = percentage of additional requests sampled. Default rule: reservoir 1/s + 5% of additional.

### X-Ray Service Map

Auto-generated visual topology of your architecture:

- Nodes = services (Lambda, ECS, EC2, DynamoDB, S3, external APIs)
- Edges = connections with latency, error rates, request counts
- Color-coded: green (healthy), yellow (errors), red (faults/throttles)

Filter by: annotations, time range, response time, fault percentage. Click any node to see trace distribution.

### X-Ray Groups

Filter expressions to create focused views:

```
# Traces with errors in prod
fault = true AND annotation.environment = "prod"

# Slow payment service requests
responseTime > 3 AND service("payment-service")

# Specific user's traces
annotation.user_id = "u-12345"
```

Groups can have associated CloudWatch Alarms for proactive monitoring.

## CloudTrail

### Event Types

| Event Type              | Coverage                                                     | Default               | Cost               |
| ----------------------- | ------------------------------------------------------------ | --------------------- | ------------------ |
| Management events       | API calls that manage resources (CreateBucket, RunInstances) | Enabled (1 copy free) | Free (first copy)  |
| Data events             | Operations on/within resources (S3 GetObject, Lambda Invoke) | Disabled              | Per 100K events    |
| Insights events         | Anomalous API activity detection                             | Disabled              | Per event analyzed |
| Network activity events | VPC endpoint activity                                        | Disabled              | Per event          |

### Trail Configuration

```bash
aws cloudtrail create-trail \
  --name org-trail \
  --s3-bucket-name audit-logs-ACCT \
  --is-multi-region-trail \
  --is-organization-trail \
  --enable-log-file-validation \
  --kms-key-id alias/cloudtrail-key \
  --cloud-watch-logs-log-group-arn arn:aws:logs:us-east-1:ACCT:log-group:cloudtrail \
  --cloud-watch-logs-role-arn arn:aws:iam::ACCT:role/cloudtrail-cwl

aws cloudtrail put-event-selectors --trail-name org-trail \
  --advanced-event-selectors '[{
    "Name": "S3DataEvents",
    "FieldSelectors": [
      {"Field": "eventCategory", "Equals": ["Data"]},
      {"Field": "resources.type", "Equals": ["AWS::S3::Object"]},
      {"Field": "resources.ARN", "StartsWith": ["arn:aws:s3:::sensitive-bucket/"]}
    ]
  }]'
```

**Log file validation**: SHA-256 digest files every hour — detect tampering of log files. Verify with `aws cloudtrail validate-logs`.

### CloudTrail Insights

Detects unusual patterns: spikes in write API calls, error rate anomalies. Uses ML baseline of normal API call patterns. Generates Insights events when activity deviates >= 2 standard deviations.

Common detections: sudden burst of `DeleteObject` calls, spike in `AuthorizeSecurityGroupIngress`, unusual `RunInstances` volume (potential crypto mining).

### CloudTrail Lake

SQL-based analysis across CloudTrail events (replacement for Athena-on-S3 approach):

```sql
SELECT eventName, sourceIPAddress, COUNT(*) as cnt
FROM cloudtrail_event_data_store
WHERE eventTime > '2024-01-01'
  AND errorCode = 'AccessDenied'
GROUP BY eventName, sourceIPAddress
ORDER BY cnt DESC
```

Event data stores: up to 7 years retention, supports cross-account via Organizations, federated querying.

## ADOT (AWS Distro for OpenTelemetry)

AWS-supported distribution of OpenTelemetry — vendor-neutral instrumentation:

```yaml
# ADOT Collector config
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    send_batch_size: 1000
    timeout: 10s
  resourcedetection:
    detectors: [env, ecs, ec2, eks]

exporters:
  awsxray:
    region: us-east-1
  awsemf:
    region: us-east-1
    namespace: MyApp
  prometheusremotewrite:
    endpoint: https://aps-workspaces.us-east-1.amazonaws.com/workspaces/ws-xxx/api/v1/remote_write
    auth:
      authenticator: sigv4auth

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch, resourcedetection]
      exporters: [awsxray]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [awsemf, prometheusremotewrite]
```

Deploy as: ECS sidecar, EKS DaemonSet, Lambda layer, or standalone EC2 daemon. Replaces X-Ray SDK with OpenTelemetry SDK — portable across providers.

## Amazon Managed Prometheus & Grafana

### Managed Prometheus (AMP)

```bash
# Create workspace
aws amp create-workspace --alias production

# Remote write endpoint (used in ADOT/Prometheus config)
# https://aps-workspaces.REGION.amazonaws.com/workspaces/WORKSPACE_ID/api/v1/remote_write

# Query endpoint (PromQL)
# https://aps-workspaces.REGION.amazonaws.com/workspaces/WORKSPACE_ID/api/v1/query
```

Fully compatible PromQL. No storage management. Auto-scales. 150-day default retention. Ingestion via remote_write (ADOT, self-managed Prometheus, OpenTelemetry Collector). Auth via SigV4.

### Managed Grafana (AMG)

- SSO integration (SAML, AWS SSO/IAM Identity Center)
- Pre-built data source plugins: AMP, CloudWatch, X-Ray, Athena, Redshift, OpenSearch
- Alerting: Grafana-native alerts routed to SNS, PagerDuty, Slack
- Workspace-level IAM roles (service-managed policies)
- Terraform provider support for dashboard-as-code

**Observability Stack Pattern**: Application → ADOT Collector → AMP (metrics) + X-Ray (traces) + CloudWatch Logs → AMG dashboards + alerts.
