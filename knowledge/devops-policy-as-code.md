# Policy as Code — OPA/Rego, Kyverno, Sentinel & Compliance Automation

## Overview

Policy as Code embeds compliance, security, and operational rules as executable policies evaluated against infrastructure, configuration, and deployment decisions. Instead of manual review checklists, policies become automated gates in CI/CD pipelines, admission controllers, and infrastructure provisioning workflows. This decouples policy logic from application code: policies can be versioned, tested, updated, and enforced consistently without code redeploys.

## Core Concepts

### Guardrails vs. Gates

| Concept | Mechanism | Failure Mode | Use Case |
|---------|-----------|--------------|----------|
| **Guardrail** | Audit + guidance | Warn/log violation, allow operation | Security best practice hints |
| **Gate** | Hard policy enforcement | Reject operation outright | Compliance hard requirements |

Runways deploy with guardrails first (collect data, identify violations), then harden to gates once baseline is established.

### Policy Evaluation Points

- **Build time** — Scan infrastructure code (Terraform, CloudFormation) before deployment
- **CI gates** — Enforce policy checks in pipeline; fail PR if policy violated
- **Admission control** — Block/mutate Kubernetes resources at cluster entry
- **Runtime drift detection** — Monitor running infrastructure against policy; alert on divergence
- **Deployment gates** — Require approval or auto-reject deploys that violate policy

## Open Policy Agent (OPA) & Rego

OPA is a general-purpose policy engine. Policies are written in Rego, a declarative query language. OPA is language- and domain-agnostic: runs on K8s clusters, AWS Lambda, Terraform, CI/CD systems, and anywhere a policy decision is needed.

### Rego Fundamentals

Rego is a **logic programming language** (subset of Datalog). A rule declares what is true:

```rego
# Allow deployment if image is from approved registry
allow {
    input.image_registry == "gcr.io/company"
    input.image_digest != ""  # Must be pinned to digest, not tag
}

# Deny CPU requests below minimum
deny["CPU request too low"] {
    input.spec.containers[_].resources.requests.cpu < "100m"
}

# Compute derived facts
approved_registries := [
    "gcr.io/company",
    "ghcr.io/company",
]

is_approved {
    approved_registries[_] == input.image_registry
}
```

### OPA in Kubernetes (Gatekeeper)

Gatekeeper wraps OPA as a ValidatingWebhookConfiguration. On every resource create/update, K8s calls Gatekeeper to evaluate policy:

```yaml
apiVersion: constraints.gatekeeper.sh/v1beta1
kind: K8sRequiredLabels
metadata:
  name: require-owner-label
spec:
  match:
    kinds:
      - apiGroups: ["apps"]
        kinds: ["Deployment"]
  parameters:
    labels: ["app", "owner", "cost-center"]
```

Gatekeeper **constraint templates** define reusable policies; **constraints** are instances that enforce them.

## Kyverno: Kubernetes-Native Policy

Kyverno policies are **Kubernetes resources** (CRDs), evaluated by webhooks at admission time. Policies read/write like YAML; no new language to learn.

### ClusterPolicy Example

```yaml
apiVersion: kyverno.io/v1
kind: ClusterPolicy
metadata:
  name: require-image-digest
spec:
  validationFailureAction: audit  # audit = warn; enforce = deny
  rules:
    - name: check-image-digest
      match:
        resources:
          kinds:
            - Pod
      validate:
        message: "Image must be pinned to digest, not tag"
        pattern:
          spec:
            containers:
              - image: "*/*/image@sha256:*"
```

Kyverno supports:
- **Validation** — Check resource compliance
- **Mutation** — Auto-modify resources (add labels, inject sidecars)
- **Generation** — Create derived resources (e.g., NetworkPolicy from Pod label)

Kyverno policies are simpler and more declarative than OPA's Rego for typical K8s scenarios.

## Sentinel: HashiCorp Policy Language

Sentinel is HashiCorp's policy language, integrated into Terraform Cloud, Vault, and Consul. Policies are written in an imperative language closer to general-purpose programming than Rego.

### Sentinel in Terraform

```hcl
# Enforce tag requirements on EC2
import "tfplan/v2" as tfplan

deny_if_tags_missing = rule {
  all tfplan.resource_changes.aws_instance as _, instances {
    all instances as _, instance {
      instance.change.after.tags contains "Environment" and
      instance.change.after.tags contains "Owner"
    }
  }
}

main = rule {
  deny_if_tags_missing and (
    tfplan.terraform_version matches "1\.\\d+\\.\\d+" or
    tfplan.terraform_version matches "\\d+\\.\\d+\\.\\d+"
  )
}
```

Sentinel integrates with Terraform Cloud **policy sets** — reusable policy bundles scoped to organizations/projects.

## Conftest: Policy Testing Framework

Conftest uses OPA/Rego to test structured configuration files (JSON, YAML, TOML, HCL). Runs locally or in CI, with a focus on testability.

```bash
conftest test deployment.yaml -p policies/
conftest verify -f passing-files.txt -f failing-files.txt
```

Conftest is lighter than Gatekeeper; useful for pre-deployment validation before pushing to K8s.

## Compliance Automation & Drift Detection

### Policy-Driven Compliance

- **STIG/CIS enforcement** — Policy rules embed hardening standards; drift detection flags divergence
- **Audit trail** — Log all policy violations and enforcement actions for compliance reporting
- **Approval workflows** — Policy violations trigger approval gates; on-call engineers review and override if justified

### Drift Detection

Infrastructure can diverge from policy due to manual changes or feature drift. Solutions:

- **Continuous scanning** — Regularly re-evaluate all resources; alert on violations
- **Resource immutability** — Prevent post-deployment modifications; re-deploy instead of remediate manually
- **Automated remediation** — Policy violation triggers corrective action (disable resource, restore config)

## Policy-as-Code Trade-offs

### Strengths
- Compliance embedded in infrastructure; updates propagate automatically
- Consistent enforcement across teams, regions, cloud accounts
- Audit trail of policy decisions for regulatory reporting
- Enables shift-left: catch violations before deployment

### Challenges
- Policy complexity grows with organizational requirements; hard-to-read policies defeat the purpose
- False positives/negatives; human review still needed for edge cases
- Performance overhead (admission webhooks add latency to every resource operation)
- Policy conflicts (Policy A forbids X; Policy B requires X) hard to diagnose

## Best Practices

1. **Start with guardrails.** Audit mode collects baseline violations. Harden to gates only when >90% compliance achieved.
2. **Policy versioning.** Treat policies as code: code review, testing, staged rollout.
3. **Testable policies.** Policies should have unit tests (Conftest, OPA testing frameworks).
4. **Owner assignment.** Each policy should have an owner; violated policies route tickets to owners.
5. **Regular review.** Audit policy violations quarterly; remove policies that consistently override or create noise.
6. **Domain-specific languages appropriately.** Use Kyverno for K8s validation (simpler); OPA for cross-domain decisions (more powerful).

## See Also

- [Architecture — Resilience Patterns](architecture-resilience.md)
- [DevOps — CI/CD Patterns](devops-cicd-patterns.md)
- [Security — Compliance Frameworks](security-compliance-frameworks.md)