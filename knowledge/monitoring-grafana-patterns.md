# Grafana Patterns — Dashboard Design, Templating, Alerting & Dashboard-as-Code

## The Grafana Architecture

Grafana is a multi-datasource visualization and dashboarding platform. It sits between observability backends (Prometheus, Loki, Tempo, Datadog, New Relic) and users, providing a unified query interface, templating, alerting, and sharing capabilities.

**Key principle**: Grafana doesn't store data—it queries datasources and renders results. A dashboard is a collection of panels, each with a datasource query and visualization config.

## Dashboard Design Principles

### Visual Hierarchy

Organize panels to guide attention:

1. **Top-left: Key metrics** — SLOs, system health, active incidents. Users glance here first.
2. **Left side: Business/user-facing signals** — Request volume, error rate, latency percentiles.
3. **Right side: Operational details** — CPU, memory, disk, network, pending tasks.
4. **Bottom: Historical trends** — Long-window graphs showing daily/weekly patterns.

A bad dashboard requires scrolling to find the metric that matters. A good dashboard **tells a story** with 3–5 key panels visible without scrolling.

### Title and Description

Each dashboard should start with:

- **Title**: What system/service does this dashboard represent?
- **Description**: When to use this dashboard, who owns it, alert playbook link.

Grafana supports markdown in descriptions. Link to runbooks, Slack channels, or a wiki explaining what "normal" looks like.

### Panel Types

| Type | Use Case |
|------|----------|
| **Graph** | Time series lines. Latency, throughput, resource usage. |
| **Stat** | Single number. Current SLO percentage, error count, queue depth. |
| **Gauge** | Single number in a gauge (0–100%). Disk utilization, availability %. |
| **Bar Gauge** | Horizontal bar for multiple series. CPU per pod, request rate per endpoint. |
| **Table** | Multi-dimensional data. Top queries, slowest endpoints, error breakdown. |
| **Heatmap** | Time series as heatmap (x=time, y=label value, color=magnitude). Latency heatmaps. |
| **Log Panel** | Structured logs from Loki. Error logs, warnings. |
| **Trace Panel** | Trace waterfall from Tempo. Request spans and timing. |
| **Stat Sparkline** | Stat with mini sparkline showing recent trend. Compact and informative. |

**Avoid**: Pie charts (hard to compare), overly fancy visualizations (distract from data), dashboards that require more than 3 clicks to find what you need.

### Threshold and Color Coding

Use consistent thresholds across dashboards:

- **Green**: Healthy, within SLO
- **Yellow**: Degraded, approaching alert threshold
- **Red**: Critical, SLO violated, requires action

```yaml
# Example: Disk utilization gauge
Thresholds:
  - 70: yellow
  - 85: red
```

Users should **immediately recognize** which panels are unhealthy.

## Templating and Variables

### Why Templating

Templating parameterizes dashboards so a single dashboard layout serves multiple environments, services, or time windows.

Instead of creating separate dashboards for `api-prod`, `api-staging`, `web-prod`, use a single dashboard with a `$service` variable:

```promql
# Panel query
rate(http_requests_total{service="$service"}[5m])
```

When you select `api-prod` from the dropdown, the query becomes:

```promql
rate(http_requests_total{service="api-prod"}[5m])
```

All panels using `$service` update instantly.

### Types of Variables

#### Single-Select

```yaml
Name: service
Type: Query
Datasource: Prometheus
Query: label_values(http_requests_total, service)
Refresh on Dashboard Load: true
Multi-select: false
```

Users see a dropdown with all unique `service` label values from Prometheus.

#### Multi-Select

```yaml
Multi-select: true
Include all option: true
```

Panels using `$service` (multi-select) should handle multiple values:

```promql
rate(http_requests_total{service=~"$service"}[5m])
```

The variable expands to `api|web|database` (regex list).

#### Constant

```yaml
Name: time_offset
Type: Constant
Value: 1h
```

Useful for calculations: `offset_time: "1h"`. Can be changed by dashboard editors only.

#### Interval

```yaml
Name: interval
Type: Interval
Values: 1m, 5m, 10m, 30m, 1h
```

Allows users to choose granularity of data aggregation. Used in recording rule or bucket-based aggregations.

#### Custom All Value

```yaml
Multi-select: true
Custom all value: ".+"  # Regex that matches all
```

When "All" is selected, the variable becomes `.+` (regex for any string), so the query works with or without filtering.

### Advanced: Chained Variables

A downstream variable can depend on an upstream variable:

```yaml
# Variable 1: choose region
Name: region
Query: label_values(up, region)

# Variable 2: choose pod (only in selected region)
Name: pod
Query: label_values(up{region="$region"}, pod)
```

When you change `$region`, Grafana queries the updated pod list.

## Alerting Within Grafana

### Alert Rules

Grafana can evaluate queries and fire alerts (alternative to Prometheus/Alertmanager):

```yaml
Alert: High Error Rate
Datasource: Prometheus
Expression: |
  (
    $error_rate{service="$service"}
    /
    $request_rate{service="$service"}
  ) > 0.05
Evaluate every: 1m
For: 5m  # Must be true for 5m before firing
Labels:
  severity: critical
  team: platform
Annotations:
  description: "Error rate > 5%"
```

Grafana stores alert state locally (not replicated), so it's suitable for **guidance-level** alerts (watch this metric), not **critical production paging** (use Prometheus + Alertmanager for that).

### Contact Points

Contact points define **where** alerts are routed:

| Type | Config |
|------|--------|
| **Webhook** | HTTP POST to custom handler |
| **PagerDuty** | Incident key, service key |
| **Slack** | Channel, message templating |
| **Email** | SMTP config |
| **Opsgenie** | API key |
| **Alertmanager** | Forward to external Alertmanager |

```yaml
# Contact Point: PagerDuty
Type: PagerDuty
Service Key: <integration-key>
Severity: critical
Custom Details:
  dashboard: https://grafana.example.com/d/abc123
```

### Notification Routing

Define which alerts go where:

```yaml
Root route:
  receiver: default
  group_by: [service, severity]
  group_wait: 30s

Routes:
- match:
    team: platform
  receiver: platform-team
  
- match:
    severity: critical
  receiver: critical-page
```

## Data Source Plugins

Grafana supports dozens of datasources. Common onboarding pattern:

1. **Add datasource** (settings → data sources)
2. **Test connection** (verify Grafana can reach it)
3. **Save** (available to all dashboards)
4. **Use in panels** (select datasource, write query)

### Loki (Logs)

Loki is a log aggregation system. Grafana queries Loki with label selectors:

```logql
{service="api", level="error"} |= "panic"  # Labels + filters + regex
| json                                      # Extract JSON fields
| namespace_latency_ms > 1000               # Filter on extracted field
```

**Loki integration**, Grafana panels:  
- **Logs panel**: Stream logs, search, color by level  
- **Logs volume**: Overlay log line count on graph  
- **Table**: Extract fields and render as table

### Tempo (Traces)

Tempo stores traces (like Jaeger but simpler). Grafana queries traces by TraceID or service+span name:

```
Service: api
Operation: HTTP GET /orders
Min duration: 100ms
Max duration: 1s
```

**Trace panel** renders the waterfall: parent span at top, child spans indented below, accurate timing.

**Link traces to metrics**: A panel can include a link to Tempo, passing `$service` and `$duration`:

```
https://grafana.example.com/d/tempo?service=$service&duration=$duration
```

### Custom Plugins

Grafana community has 300+ plugins. Development pattern:

1. Write plugin (JavaScript/TypeScript, React for UI)
2. Package as npm + zip
3. Install with `grafana-cli plugins install plugin-id`
4. Restart Grafana, enable in settings
5. Use in dashboards

## Dashboard-as-Code

### Grafonnet (Jsonnet)

Grafonnet is a library for generating Grafana dashboards from code (Jsonnet, a JSON extension language):

```jsonnet
local grafana = import 'grafonnet/grafana.libsonnet';
local panel = grafana.panel;

dashboard.new(
  title='API Service',
  desc='Service dashboard for api-server',
  tags=['api', 'prod'],
  refresh='30s'
)
.addPanels(
  [
    panel.graph.new(
      title='Requests/sec',
      datasource='Prometheus'
    ).addTarget(
      target=promQuery('rate(http_requests_total[5m])')
    ),
    
    panel.graph.new(
      title='Latency p95',
      datasource='Prometheus'
    ).addTarget(
      target=promQuery('histogram_quantile(0.95, rate(http_duration_bucket[5m]))')
    ),
  ]
)
```

**Advantages**: Templatization, version control, DRY (don't repeat yourself), reusable panel libraries.

**Disadvantages**: Requires Jsonnet knowledge, harder to visual-edit, CI/CD required to deploy.

### Terraform Provider

Grafana has an official Terraform provider:

```hcl
resource "grafana_dashboard" "api" {
  title       = "API Service"
  folder      = grafana_folder.prod.id
  config_json = jsonencode({
    dashboard = {
      title  = "API Service"
      panels = [
        {
          title       = "Requests/sec"
          datasource  = "Prometheus"
          targets     = [...]
          type        = "timeseries"
        }
      ]
    }
  })
}

resource "grafana_contact_point" "slack" {
  name = "platform-team"
  slack {
    channel = "#incidents"
  }
}
```

**Use case**: Multi-region deployments, repeatable infrastructure, GitOps workflows.

## Best Practices

**Scoping**: One dashboard per service or subsystem, not one per metric.

**Naming**: Use a prefix convention: `[SERVICE] Metric Group`. Example: `[API] Performance`, `[DATABASE] Replication Status`.

**Variables in titles**: Dashboard title can use variables: `$service Overview` changes with selection.

**Links between dashboards**: Reference related dashboards ("More details in Database dashboard").

**SLO dashboards**: Dedicated dashboard showing SLI/SLO status, not operational details.

**Read-only for users**: Share dashboards in "View" mode, not "Edit" mode, to prevent accidental changes.

## Related Topics

See also: [monitoring-prometheus-deep.md](monitoring-prometheus-deep.md), [devops-prometheus-grafana.md](devops-prometheus-grafana.md), [observability-alerting.md](observability-alerting.md), [logging-observability.md](logging-observability.md).