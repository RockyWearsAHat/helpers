# Infrastructure Testing — Terratest, Kitchen-Terraform, Policy as Code, Drift Detection, Cost Estimation

## Overview

Infrastructure testing validates that provisioned infrastructure matches intent: correct security groups, database schemas, IAM policies, network ACLs, compliance controls. Unlike application tests (which exercise code), infrastructure tests **provision real resources** (in dev/staging), query them, then tear down. The pipeline: **unit-level** (static analysis, schema validation) → **policy-as-code** (compliance scanning) → **integration** (Terratest) → **compliance** (drift detection).

## Testing Frameworks

### Terratest

Go-based infrastructure testing library from Gruntwork. Provisions real infrastructure, runs assertions, cleans up.

**Project structure**:

```
infrastructure/
├── modules/
│   └── vpc/
│       ├── main.tf
│       ├── variables.tf
│       └── outputs.tf
└── test/
    └── vpc_test.go
```

**Test anatomy**:

```go
package test

import (
    "testing"
    "github.com/gruntwork-io/terratest/modules/terraform"
    "github.com/stretchr/testify/assert"
)

func TestVpcCreation(t *testing.T) {
    opts := &terraform.Options{
        TerraformDir: "../modules/vpc",
        Vars: map[string]interface{}{
            "cidr_block": "10.0.0.0/16",
            "vpc_name":   "test-vpc",
        },
    }
    
    defer terraform.Destroy(t, opts)
    terraform.InitAndApply(t, opts)
    
    vpcId := terraform.Output(t, opts, "vpc_id")
    assert.NotEmpty(t, vpcId)
}
```

**Pattern: provision → query → assert → cleanup**:

1. `InitAndApply`: terraform init + apply, capture outputs
2. Query: Read outputs, make AWS SDK calls to inspect provisioned resources
3. Assert: Validate state against expectations
4. Cleanup: `Destroy` rolls back infrastructure

**Helpers**:
- `terraform.Output()`: Extract TF output
- `terraform.RunTerraformCommand()`: Execute arbitrary terraform commands
- AWS SDK integration: Fetch EC2 instances, RDS databases, security groups, etc.
- `retry.DoWithRetry()`: Wait for eventual consistency (DNS, IAM eventual consistency, etc.)

**Execution**: `go test -v -timeout 30m` (tests take 5–30m depending on provisioning speed).

**Cost**: Each test run provisions real cloud resources (~$0.50–$5 per test). Run in non-production accounts; delete dangling resources carefully.

### Kitchen-Terraform

Ruby-based test framework from Test Kitchen ecosystem. Combines Terraform provisioning with InSpec compliance testing.

**Test lifecycle**: `create` (provision) → `verify` (run InSpec) → `destroy`

**Kitchen YAML config**:

```yaml
---
driver:
  name: terraform
  command_options: -parallelism=5

provisioner:
  name: terraform

verifier:
  name: terraform
  inspec_controls:
    - default

platforms:
  - name: aws
    driver_config:
      aws_region: us-east-1

suites:
  - name: default
    verifier:
      inspec_tests:
        - test/integration/default
```

**InSpec test example** (`test/integration/default/main.tf.rb`):

```ruby
describe aws_security_group(group_id: terraform_output('security_group_id')) do
  its('inbound_rules_count') { should eq 2 }
  its('ip_permissions') { should_not include(
    { from_port: 22, to_port: 22, ip_protocol: 'tcp', ip_ranges: [{ cidr_ip: '0.0.0.0/0' }] }
  ) }
end

describe aws_rds_database('mydb') do
  its('engine') { should eq 'postgres' }
  its('multi_az') { should be true }
end
```

**Execution**: `kitchen converge && kitchen verify && kitchen destroy`

**Distinction from Terratest**: Kitchen is more readable for ops/compliance teams (Ruby-like syntax); Terratest is more powerful for engineers (full Go SDK access, complex orchestration).

## Policy as Code

Enforce infrastructure compliance at **plan time** (before apply) or **deploy time** (admission control).

### OPA/Rego

General-purpose policy engine; policies written in Rego (a query language).

**Core idea**: Evaluate infrastructure configuration (JSON/YAML) against policies; return allow/deny + reason.

**Rego example** (enforce tagging on all AWS resources):

```rego
package terraform

import data.terraform.resources as tf_resources

deny[msg] {
    resource := tf_resources[_]
    resource.type == "aws_instance"
    not resource.values.tags.Environment
    msg := sprintf("EC2 instance %q missing 'Environment' tag", [resource.address])
}

deny[msg] {
    resource := tf_resources[_]
    resource.type == "aws_rds_cluster"
    resource.values.storage_encrypted != true
    msg := sprintf("RDS cluster %q not encrypted at rest", [resource.address])
}
```

**Integration**: 
- Terraform: `terraform show -json | opa eval -d policy.rego -`
- Kubernetes: OPA Gatekeeper admission webhook validates pod specs

**Strengths**: Domain-agnostic (works for Terraform, Kubernetes, AWS, GCP, etc.); powerful query language.

**Limitation**: Rego syntax has a learning curve; debugging is opaque.

### Sentinel (HashiCorp)

Policy language for Terraform Cloud. Simpler than Rego; Terraform-native.

**Policy example** (enforce instance type whitelist):

```hcl
import "tfstate/v2" as tfstate
import "tfstate/funcs" as funcs

allowed_instance_types = ["t3.micro", "t3.small", "m5.large"]

instances = filter tfstate.resources.aws_instance as _, instances {
  instances.values all { instance in instances.values[_]
    instance.instance_type in allowed_instance_types
  }
}

main = rule {
  length(instances.values) == 0
}
```

**Integration**: Terraform Cloud/Enterprise evaluates policies on every plan; blocks non-compliant applies.

**Advantage**: Tightly integrated with Terraform; no external evaluation needed.

### Kyverno (Kubernetes)

Kubernetes-native policy engine. Policies written as YAML; no special language needed.

**Policy example** (require image registry):

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-image-registry
spec:
  validationFailureAction: audit
  rules:
  - name: validate-image-registry
    match:
      resources:
        kinds:
        - Pod
    validate:
      message: "Image must be from approved registry"
      pattern:
        spec:
          containers:
          - image: "gcr.io/* | docker.io/*"
```

**Integration**: Admission webhook in Kubernetes; validates pods before scheduling.

**Advantage**: No new language; YAML is standard for K8s teams.

## Plan Review & Cost Estimation

### Terraform Plan Analysis

Static analysis of `terraform plan` output before apply.

```bash
# Generate plan in JSON
terraform plan -out=tfplan
terraform show -json tfplan > plan.json

# Analyze with policy tools
opa eval -d policy.rego -i plan.json

# Cost estimate
infracost breakdown --path plan.json
```

### Infracost

Cloud cost estimation tool. Parses Terraform, HCL, CloudFormation; estimates monthly cost of proposed infrastructure.

**Workflow**:

```bash
# In CI/CD, on every PR
infracost breakdown --path . --format json > cost.json
infracost comment github --pr-number $PR --path cost.json
```

**Output** (posted as PR comment):

```
 ✓ Breakdown by resource
 ┌────────────────────────────────────────────────┐
 │ aws_instance.web (t3.large, us-east-1)        │
 │ On-Demand hourly: $0.104 → Monthly: $75.92    │
 │                                                │
 │ aws_rds_cluster_instance (db.r5.large, 2x)   │
 │ On-Demand hourly: $0.54 → Monthly: $394.92   │
 │                                                │
 │ Total monthly: $470.84                        │
 └────────────────────────────────────────────────┘
```

**Prevent cost surprises**: Set budgets, alert on thresholds, require approval for large changes.

## Drift Detection

**Drift**: Infrastructure state diverges from Terraform configuration. E.g., manual console modification, out-of-band script, lambda function, third-party tool.

### Terraform Drift Detection

```bash
terraform refresh
terraform plan -detailed-exitcode
# Exit 0: no changes
# Exit 1: error
# Exit 2: differences detected (drift)
```

**In CI/CD** (scheduled, e.g., daily):

```yaml
# GitHub Actions
name: Drift Detection
on:
  schedule:
    - cron: "0 2 * * *"  # Daily 2 AM UTC

jobs:
  drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: terraform init
      - run: terraform plan -detailed-exitcode
      - if: failure() && steps.plan.outcome == 'failure'
        run: echo "::notice::Drift detected! Manual investigation required"
```

**Remediation strategies**:

1. **Auto-remediate**: `terraform apply -refresh-only`, commit drift fix to Git (requires RBAC trust)
2. **Alert + manual**: Slack notification, on-call engineer reviews drift before applying
3. **Block**: Drift discovered → CI blocks deploys until drift is resolved (prevents compounding state divergence)

**Root causes to prevent drift**:
- Enforce GitOps: no manual changes allowed
- Run drift detection regularly (daily for critical infrastructure)
- Immutable infrastructure: replace servers, don't modify

## Compliance Scanning

Automated checks that infrastructure meets security/regulatory standards (HIPAA, SOC2, CIS benchmarks).

### Checkov (Bridgecrew)

Static analysis tool. Scans Terraform/CloudFormation/Kubernetes for compliance violations.

```bash
checkov -d . --framework terraform --check CKV_AWS_1

# Sample output
Check: "Ensure all data stored in the backup is securely encrypted at rest" (CKV_AWS_1)
  FAILED for resource: aws_db_instance.default
  File: main.tf:10-20
  storage_encrypted = false  # ← violation
```

**Runs in CI/CD**: Fail build if violations found.

**Coverage**: 500+ pre-built checks for AWS, Azure, GCP, Kubernetes.

### Snyk (Infrastructure as Code Scanning)

Similar to Checkov. Focuses on security misconfigurations; integrates with GitHub/GitLab.

**Advantage**: Git-native; scans on PR; suggests fixes inline.

### AWS Config + Custom Lambda Rules

AWS-native compliance. ConfigAggregator evaluates resources against rules; Lambda webhooks for custom checks.

```hcl
rule "s3_bucket_encryption" {
  description = "S3 buckets must have default encryption enabled"
  
  source {
    owner             = "CUSTOM_LAMBDA"
    source_identifier = arn:aws:lambda:...encrypt_checker
  }
  
  scope {
    compliance_resource_types = ["AWS::S3::Bucket"]
  }
}
```

## Infrastructure Unit Tests

Validate module interfaces (variable validation, outputs, variables schema).

```hcl
# modules/vpc/main.tf validation block
variable "cidr_block" {
  type        = string
  description = "VPC CIDR block"
  
  validation {
    condition = can(cidrhost(var.cidr_block, 0))
    error_message = "CIDR block must be valid"
  }
}

output "vpc_id" {
  value = aws_vpc.main.id
  # Implicit contract: consumers depend on this output name & type
}
```

**Test** (Terraform `testing` blocks, added in TF 1.6+):

```hcl
run "cidr_validation" {
  command = plan
  
  variables {
    cidr_block = "10.0.0.0/16"
  }
  
  assert {
    condition = aws_vpc.main.cidr_block == "10.0.0.0/16"
    error_message = "CIDR block mismatch"
  }
}

run "invalid_cidr" {
  command = plan
  
  variables {
    cidr_block = "invalid"
  }
  
  expect_failures = [var.cidr_block]  # Expect validation error
}
```

## Integration Patterns

**Local Development**:
1. Write Terraform code
2. `terraform plan` + review
3. `terraform apply` in dev account

**CI/CD Pipeline** (PR → Staging → Prod):
1. PR opened: Terraform `plan`, cost estimate, policy scan (Checkov/OPA), drift detection
2. PR commented with: estimated changes, cost delta, policy violations (if any)
3. Approval & merge to main
4. Main branch: Blue-green or rolling deploy with staged rollout

**Compliance Monitoring** (Continuous):
1. Daily: Drift detection across all environments
2. Weekly: Compliance scan (Checkov) produces audit report
3. On-demand: Cost breakdown with Infracost

## Best Practices

- **Test early, fail fast**: Run Terratest in staging before production
- **Cost visibility**: Every PR should show cost impact (Infracost)
- **Automate compliance**: Policy-as-code catches 80% of issues before they reach staging
- **Separate policy from provisioning**: Use OPA/Sentinel instead of embedded Terraform logic
- **Regular drift detection**: Schedule hourly for critical infrastructure, daily for the rest
- **Immutable infrastructure**: Terraform `replace` instead of `modify`; rebuilds are safer than in-place upgrades

## See Also

- [devops-terraform](devops-terraform.md) — Terraform language & state management
- [devops-cicd-patterns](devops-cicd-patterns.md) — CI/CD integration, trunk-based development
- [devops-kubernetes](devops-kubernetes.md) — Kubernetes as infrastructure; policy via Kyverno
- [security-secrets-management](security-secrets-management.md) — secrets in infra tests