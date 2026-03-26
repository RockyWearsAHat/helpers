# Monitoring as Code: Infrastructure Automation for Observability

## Overview

"Monitoring as code" treats dashboards, alert rules, SLOs, and incident response workflows as declarative infrastructure. Tools: Terraform (Datadog/PagerDuty providers), Crossplane (cloud-native control plane), Pulumi (imperative code), Grafana provisioning (dashboard-as-code), alert rules stored in Git with peer review and deploy CI/CD.

---

## Terraform for Monitoring Infrastructure

### Datadog Provider: Monitoring and Alerting as Code

```hcl
# terraform/main.tf
terraform {
  required_providers {
    datadog = {
      source  = "DataDog/datadog"
      version = "~> 3.0"
    }
  }
}

provider "datadog" {
  api_key = var.datadog_api_key
  app_key = var.datadog_app_key
}

# Monitor: alert when CPU > 80% for 5 minutes
resource "datadog_monitor" "cpu_alert" {
  name              = "High CPU on production instances"
  description       = "Alert if CPU > 80% for 5 min on prod hosts"
  type              = "metric alert"
  query             = "avg(last_5m):avg:system.cpu.user{env:prod} > 0.8"
  thresholds = {
    critical = 0.8
    warning  = 0.7
  }
  notification_presets = "show_all"
  notify_no_data   = true
  no_data_timeframe = 10

  notify_list = [
    "@slack-#alerts",
    "@pagerduty-prod"
  ]

  tags = ["env:prod", "service:infrastructure"]
}

# Dashboard: pre-built visualization
resource "datadog_dashboard" "prod_overview" {
  title       = "Production System Overview"
  description = "High-level metrics and health status"
  url_handle  = "prod-overview"

  widget {
    size_x = 12
    size_y = 6
    type   = "timeseries"
    title  = "CPU Usage"

    request {
      query = "avg:system.cpu.user{env:prod}"
      display_type = "line"
      on_right_yaxis = false
    }
    request {
      query = "avg:system.cpu.system{env:prod}"
      display_type = "line"
    }
  }

  widget {
    size_x = 6
    size_y = 4
    type   = "query_value"
    title  = "Error Rate (5m)"

    request {
      query = "avg(last_5m):sum:trace.web.request.errors{env:prod}.as_count() / sum:trace.web.request.total{env:prod}.as_count()"
    }
  }
}

# Service Level Objective (SLO)
resource "datadog_service_level_objective" "api_availability" {
  name       = "API Availability (99.9%)"
  type       = "metric"
  description = "HTTP requests returning 2xx/3xx"

  query {
    numerator   = "sum:trace.web.request.hits{env:prod,status:2xx}.as_count()+sum:trace.web.request.hits{env:prod,status:3xx}.as_count()"
    denominator = "sum:trace.web.request.hits{env:prod}.as_count()"
  }

  threshold_windows {
    time_window = "7d"
    rolling_slo = 99.9
  }

  tags = ["env:prod", "slo:critical"]
}

# Downtime: silence alerts during maintenance
resource "datadog_downtime" "maintenance_window" {
  scope     = ["env:prod"]
  start     = 1700000000  # Unix timestamp
  end       = 1700003600  # 1 hour later
  message   = "Planned database maintenance"
  monitor_tags = ["env:prod"]
}

output "monitor_ids" {
  value = {
    cpu_alert = datadog_monitor.cpu_alert.id
    slo_api   = datadog_service_level_objective.api_availability.id
  }
}
```

**Deploy:**

```bash
terraform init
terraform plan
terraform apply
```

Terraform creates monitors, dashboards, SLOs in Datadog. Changes to `.tf` files → `terraform plan` shows diff → peer review → `terraform apply` updates live monitoring.

### PagerDuty Provider: On-Call Management as Code

```hcl
resource "pagerduty_service" "api_service" {
  name             = "API Service"
  alert_creation   = "create_alerts_and_incidents"
  alert_grouping   = "intelligent"
  auto_resolve_timeout = 14400  # 4 hours
  escalation_policy = pagerduty_escalation_policy.engineering.id
}

resource "pagerduty_escalation_policy" "engineering" {
  name            = "Engineering Escalation"
  num_loops       = 2

  escalation_rule {
    escalation_delay_in_minutes = 30
    target {
      type = "schedule_reference"
      id   = pagerduty_schedule.on_call.id
    }
  }

  escalation_rule {
    escalation_delay_in_minutes = 60
    target {
      type = "user_reference"
      id   = data.pagerduty_user.engineering_lead.id
    }
  }
}

resource "pagerduty_service_integration" "datadog" {
  name            = "Datadog"
  service         = pagerduty_service.api_service.id
  integration_key = pagerduty_service_integration.datadog.integration_key
  type            = "events_api_v2_integration"
}

# Connect Datadog monitors to PagerDuty
resource "datadog_pagerduty_account" "main" {
  account_name = "prod"
  org_name     = "my-org"
}

resource "datadog_pagerduty_service_object" "api_critical" {
  service_key = pagerduty_service_integration.datadog.integration_key
}
```

**Benefit:** On-call schedule, escalation policies, service mappings in Git; review changes before applying.

### Terraform Modules: Reusable Monitoring Patterns

```hcl
# modules/database_monitoring/main.tf
variable "database_name" { type = string }
variable "warning_threshold" { type = number; default = 0.7 }
variable "critical_threshold" { type = number; default = 0.9 }

resource "datadog_monitor" "db_connections" {
  name        = "High DB connections: ${var.database_name}"
  type        = "metric alert"
  query       = "avg(last_5m):avg:postgresql.max_connections{db:${var.database_name}} > ${var.critical_threshold}"
  thresholds  = {
    warning  = var.warning_threshold * 100
    critical = var.critical_threshold * 100
  }
  notify_list = ["@slack-#database-alerts"]
}

resource "datadog_dashboard" "db_overview" {
  title       = "${var.database_name} Dashboard"
  url_handle  = "db-${var.database_name}"
  # ... widgets ...
}

output "monitor_id" {
  value = datadog_monitor.db_connections.id
}

# main.tf using module
module "prod_db_monitoring" {
  source          = "./modules/database_monitoring"
  database_name   = "production_postgres"
  critical_threshold = 0.85
}
```

---

## Crossplane and Pulumi: Infrastructure as Code for Observability

### Crossplane: Declarative Cloud Resource Management

Crossplane adds cloud resources (AWS, GCP, Azure, Datadog) as Kubernetes Custom Resources. Monitoring infrastructure becomes part of the GitOps workflow.

```yaml
# crossplane/monitoring.yaml
apiVersion: aws.crossplane.io/v1beta1
kind: CloudWatchDashboard
metadata:
  name: prod-dashboard
  namespace: crossplane-system
spec:
  forProvider:
    dashboardName: prod-dashboard
    dashboardBody: |
      {
        "widgets": [
          {
            "type": "metric",
            "properties": {
              "metrics": [
                ["AWS/EC2", "CPUUtilization", {"stat": "Average"}]
              ],
              "period": 300,
              "stat": "Average",
              "region": "us-east-1"
            }
          }
        ]
      }
  providerRef:
    name: aws-provider
  reclaimPolicy: Delete

---

apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: api-rules
  namespace: monitoring
spec:
  groups:
  - name: api
    interval: 30s
    rules:
    - alert: HighErrorRate
      expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
      for: 5m
      annotations:
        summary: "High error rate on {{ $labels.job }}"
      labels:
        severity: critical
    - alert: HighLatency
      expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 1
      for: 10m
      annotations:
        summary: "High P99 latency"
      labels:
        severity: warning
```

**Apply via GitOps:**

```bash
kubectl apply -f crossplane/monitoring.yaml
# Crossplane controller reconciles: creates CloudWatch dashboard, Prometheus rules
```

### Pulumi: Imperative Infrastructure-as-Code

```python
# __main__.py
import pulumi
import pulumi_datadog as datadog
import pulumi_pagerduty as pagerduty

# Config
config = pulumi.Config()
env = config.get("environment") or "prod"
threshold = config.require_int("cpu_threshold")

# Datadog monitor
monitor = datadog.Monitor(f"cpu_{env}",
    name=f"High CPU: {env}",
    type="metric alert",
    query=f"avg(last_5m):avg:system.cpu.user{{env:{env}}} > {threshold / 100}",
    thresholds={
        "critical": threshold / 100,
        "warning": (threshold - 10) / 100,
    },
    notify_list=["@slack-#alerts", "@pagerduty"],
    tags=[f"env:{env}", "managed:pulumi"])

# PagerDuty service
service = pagerduty.Service(f"api_{env}",
    name=f"API {env}",
    escalation_policy=pagerduty.EscalationPolicy(f"api_escalation_{env}",
        name=f"API Escalation {env}",
        escalation_rules=[{
            "escalation_delay_in_minutes": 30,
            "targets": [{
                "type": "schedule_reference",
                "id": on_call_schedule.id,
            }],
        }],
    ).id,
    auto_resolve_timeout=14400)

# SLO
slo = datadog.ServiceLevelObjective(f"api_availability_{env}",
    name=f"API Availability {env}",
    type="metric",
    query={
        "numerator": f"sum:trace.web.request.hits{{env:{env},status:2xx}}.as_count()+sum:trace.web.request.hits{{env:{env},status:3xx}}.as_count()",
        "denominator": f"sum:trace.web.request.hits{{env:{env}}}.as_count()",
    },
    thresholds=[{
        "timeframe": "7d",
        "target": 99.9,
    }],
    tags=[f"env:{env}"])

# Export outputs
pulumi.export("monitor_id", monitor.id)
pulumi.export("service_id", service.id)
pulumi.export("slo_id", slo.id)
```

**Deploy:**

```bash
pulumi config set environment prod
pulumi config set cpu_threshold 80
pulumi up
```

**Advantages over Terraform:**
- Full programming language (loops, conditionals, functions)
- Reusable modules as libraries (npm, pip, etc.)
- Type safety (Python type hints)
- Easier to test (unit test monitoring configs)

---

## Grafana: Provisioning Dashboards and Data Sources

### Dashboard as Code: JSON in Git

```yaml
# grafana/dashboards/api-overview.json
{
  "annotations": {
    "list": [
      {
        "datasource": "Prometheus",
        "name": "Deployments",
        "tagKeys": "deployment"
      }
    ]
  },
  "dashboard": {
    "title": "API Overview",
    "panels": [
      {
        "id": 1,
        "title": "Request Rate",
        "type": "graph",
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0},
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "refId": "A"
          }
        ]
      },
      {
        "id": 2,
        "title": "Error Rate",
        "type": "stat",
        "gridPos": {"h": 4, "w": 6, "x": 12, "y": 0},
        "targets": [
          {
            "expr": "rate(http_requests_total{status=~\"5..\"}[5m])",
            "refId": "A"
          }
        ]
      }
    ]
  }
}
```

### Grafana Provisioning: Infrastructure Declaration

```yaml
# grafana/provisioning/datasources/prometheus.yml
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    orgId: 1
    url: http://prometheus:9090
    isDefault: true
    editable: true

---

# grafana/provisioning/dashboards/api.yml
apiVersion: 1

providers:
  - name: API Dashboards
    orgId: 1
    folder: "Production"
    type: file
    disableDeletion: false
    editable: true
    options:
      path: /etc/grafana/provisioning/dashboards

---

# grafana/provisioning/dashboards/api-overview.json
# (dashboard JSON from above)
```

### Helm Values for Grafana

```yaml
# grafana/values.yaml
grafana:
  replicas: 1
  datasources:
    datasources.yaml:
      apiVersion: 1
      datasources:
      - name: Prometheus
        type: prometheus
        url: http://prometheus:9090
        isDefault: true
  dashboardProviders:
    dashboardproviders.yaml:
      apiVersion: 1
      providers:
      - name: Production
        folder: prod
        type: file
        disableDeletion: false
        editable: true
        options:
          path: /var/lib/grafana/dashboards/prod
  dashboards:
    prod:
      api-overview:
        url: https://grafana.com/api/dashboards/1234/revisions/latest/download
      database-metrics:
        url: https://grafana.com/api/dashboards/5678/revisions/latest/download
```

Dashboards and data sources are GitOps-managed; changes to files auto-apply to running Grafana.

---

## Alert Rules as Code: Prometheus + GitOps

### PrometheusRule Custom Resource

```yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: api-alerts
  namespace: monitoring
spec:
  groups:
  - name: api_critical
    interval: 30s
    rules:
    - alert: APIHighErrorRate
      expr: |
        rate(http_request_total{handler="/api",status=~"5.."}[5m]) /
        rate(http_request_total{handler="/api"}[5m]) > 0.05
      for: 5m
      labels:
        severity: critical
        team: platform
      annotations:
        summary: "API error rate {{ $value | humanizePercentage }} > 5%"
        description: "Service {{ $labels.service }} experiencing high errors"
        runbook: "https://runbooks.example.com/api-high-error"

    - alert: APIDatabaseConnectionPool
      expr: |
        pg_stat_activity_count{user="api_user"} /
        pg_settings_max_connections > 0.8
      for: 10m
      labels:
        severity: warning
        team: platform
      annotations:
        summary: "DB connection pool 80% full"

    - record: api:request_rate:1m
      expr: rate(http_requests_total[1m])

    - record: api:error_rate:5m
      expr: |
        rate(http_requests_total{status=~"5.."}[5m]) /
        rate(http_requests_total[5m])
```

**Stored in Git:**

```
monitoring/
├── prometheus/
│   ├── rules/
│   │   ├── api-alerts.yaml
│   │   ├── database-alerts.yaml
│   │   └── infrastructure-alerts.yaml
│   └── values.yaml
```

**Deployed via Helm:**

```bash
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm install prometheus prometheus-community/kube-prometheus-stack \
  -f monitoring/prometheus/values.yaml
```

PrometheusOperator watches PrometheusRule CRDs, auto-reloads alert definitions, no Prometheus restart needed.

---

## SLO Definitions in Code

### Terraform SLO Definitions

```hcl
locals {
  slos = {
    api_availability = {
      description = "API responds to requests successfully"
      target      = 99.9
      window      = "7d"
      numerator   = "sum:trace.web.request.hits{status:2xx,3xx}"
      denominator = "sum:trace.web.request.hits"
    }
    api_latency = {
      description = "API responds in < 1 second"
      target      = 95.0
      window      = "30d"
      numerator   = "sum:trace.web.request.hits{duration:<1000}"
      denominator = "sum:trace.web.request.hits"
    }
    database_availability = {
      description = "Database connection successful"
      target      = 99.95
      window      = "7d"
      numerator   = "sum:pg_up"
      denominator = "sum:pg_queries_total"
    }
  }
}

resource "datadog_service_level_objective" "slos" {
  for_each = local.slos

  name        = each.key
  description = each.value.description
  type        = "metric"
  
  query {
    numerator   = each.value.numerator
    denominator = each.value.denominator
  }

  threshold_windows {
    time_window = each.value.window
    rolling_slo = each.value.target
  }

  tags = ["service:api", "managed:terraform"]
}

output "slo_error_budgets" {
  value = {
    for key, slo in datadog_service_level_objective.slos :
    key => "${slo.name}: ${100 - slo.threshold_windows[0].rolling_slo}% error budget"
  }
}
```

SLOs in code → easy to review, version control, promote across environments.

---

## Drift Detection and Compliance

### Terraform Drift Detection

```bash
# Detect changes made outside Terraform
terraform refresh
terraform plan

# Output shows: resources created/modified manually not tracked by state
```

**Automated drift detection in CI:**

```yaml
# .github/workflows/monitoring-drift-check.yml
name: Monitoring Drift Check
on:
  schedule:
  - cron: "0 2 * * *"  # Daily at 2am UTC
  workflow_dispatch:

jobs:
  drift-check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: hashicorp/setup-terraform@v2
    - run: terraform init
    - run: terraform plan -detailed-exitcode
    - name: Report drift
      if: failure()
      uses: actions/github-script@v7
      with:
        script: |
          github.rest.issues.createComment({
            issue_number: 1,  # Configuration tracking issue
            body: "Monitoring drift detected. Run `terraform apply` to reconcile."
          })
```

### Policy as Code: Validate Monitoring Configuration

```rego
# opa/monitoring-rules.rego
package monitoring

import future.keywords.contains
import future.keywords.in

# Rule: All critical services must have SLO defined
deny[msg] {
    service := input.services[_]
    service.criticality == "critical"
    slos := [slo | input.slos[_].service == service.name]
    count(slos) == 0
    msg := sprintf("Critical service %s missing SLO", [service.name])
}

# Rule: All alerts must have runbook
deny[msg] {
    alert := input.alerts[_]
    alert.runbook == ""
    msg := sprintf("Alert %s missing runbook annotation", [alert.name])
}

# Rule: SLO target cannot be < 95% (enforce realistic targets)
deny[msg] {
    slo := input.slos[_]
    slo.target < 95
    msg := sprintf("SLO %s target %d% too low (minimum 95%)", [slo.name, slo.target])
}
```

**Test monitoring configs before deploy:**

```bash
opa eval -d opa/monitoring-rules.rego -i monitoring-config.json 'deny[msg]'
# Outputs violations; gate deployment until fixed
```

---

## Peer Review and Deployment Workflow

```yaml
# .github/workflows/monitoring-review.yml
name: Review Monitoring Changes
on:
  pull_request:
    paths:
    - 'terraform/**'
    - 'grafana/**'
    - 'kubernetes/monitoring/**'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: hashicorp/setup-terraform@v2
    - run: terraform init
    - run: terraform validate
    - run: terraform plan -out=tfplan
    - uses: actions/upload-artifact@v4
      with:
        name: tfplan
        path: tfplan

  policy-check:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: open-policy-agent/setup-opa@v1
    - run: opa eval -d opa/monitoring-rules.rego -i monitoring-config.json 'deny[msg]' ||true

  deploy:
    needs: [validate, policy-check]
    if: github.event.pull_request.merged == true && github.base_ref == 'main'
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: hashicorp/setup-terraform@v2
    - run: terraform init && terraform apply -auto-approve tfplan
```

**Workflow:**
1. Developer: commit monitoring changes (alert rule, dashboard, SLO)
2. PR created: CI runs validation, policy checks, generates `terraform plan`
3. Review: team reviews plan, approves
4. Merge: on merge, `terraform apply` auto-runs
5. Reconciliation: monitoring infra matches Git state

---

## Production Patterns

- **Environment promotion:** Dev → Staging → Prod via reusable modules, separate RBAC
- **Alert tuning via PR:** Change threshold, merge, auto-apply, monitor impact, rollback if needed
- **Runbook links in code:** Annotation points to operational runbook; on alert fire, runbook automatically linked
- **Alert testing:** Terraform module smoke tests validate alert expressions (no syntax errors)
- **Disaster recovery:** Monitoring configs in Git → recreate in 10 minutes (state rebuilds from TF)