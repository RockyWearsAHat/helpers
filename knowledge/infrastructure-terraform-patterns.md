# Terraform Patterns — Module Design, State Management, Workspaces & Migration

## Module Design Principles

Terraform modules are reusable packages of resources sharing a common purpose. Effective modules balance abstraction with flexibility.

### Structural Guidelines

A well-designed module encapsulates **related resources** serving a single concern: networking infrastructure, database stack, CI/CD pipeline, etc. Flat module structures (all resources in one directory) work for simple use cases but scale poorly. Nested modules (modules calling other modules) enable hierarchical composition.

```hcl
# Root module structure
terraform/
├── main.tf            # Resources, local values, variable declaration aggregation
├── variables.tf       # Input variable definitions with validation
├── outputs.tf         # Output value definitions (what consumers consume)
├── terraform.tfvars   # .gitignore this; use for local overrides
└── modules/
    ├── networking/
    │   ├── main.tf
    │   ├── variables.tf
    │   └── outputs.tf
    └── database/
        ├── main.tf
        ├── variables.tf
        └── outputs.tf
```

### Variable Management

Input variables define a module's contract. Expose only parameters consumers need to configure; internalize implementation details.

**Validation patterns**: Use `validation` blocks to reject invalid inputs early:

```hcl
variable "instance_count" {
  type    = number
  default = 1
  validation {
    condition     = var.instance_count > 0 && var.instance_count <= 10
    error_message = "Instance count must be 1-10."
  }
}

variable "environment" {
  type = string
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}
```

**Sensitive inputs**: Mark secrets with `sensitive = true` to prevent logging in output:

```hcl
variable "db_password" {
  type      = string
  sensitive = true
}
```

### Output Design

Outputs expose module state to consumers. Export granular values (IDs, ARNs, endpoints) rather than entire resource objects when possible—changes to underlying resources won't force consumer plan changes.

```hcl
# Good: specific, stable values
output "sg_id" {
  value = aws_security_group.app.id
}

# Avoid: coupling consumers to internal structure
output "security_group" {
  value = aws_security_group.app
}
```

## State Management

Terraform state tracks resource relationships and real-world infrastructure. Mismanagement creates silent divergence, forcing manual recovery.

### Local vs. Remote State

**Local state** (`terraform.tfstate` in working directory) is insecure and unmergeable. Multiple engineers cannot safely apply changes concurrently; state file locks are insufficient.

**Remote state** stores state in a shared backend (AWS S3, Terraform Cloud, Azure Storage, etc.), enabling concurrent access and audit trails. Most production deployments require remote state.

### Remote Backends

Backends define where and how state is stored.

**S3 backend** (AWS):
```hcl
terraform {
  backend "s3" {
    bucket         = "my-terraform-state"
    key            = "prod/terraform.tfstate"
    region         = "us-east-1"
    dynamodb_table = "terraform-locks"
    encrypt        = true
  }
}
```

State locking via DynamoDB prevents concurrent modifications. The DynamoDB table must have a primary key named `LockID`.

**State structure**: Remote state files are still sensitive—they contain plaintext secrets, database passwords, and infrastructure metadata. Encrypt state in transit (TLS) and at rest (S3 encryption, Azure managed encryption).

### State Locking

When multiple operators run `terraform apply` simultaneously, state locking ensures serialization. Without locking, concurrent writes corrupt state.

Most modern backends support locking automatically. DynamoDB, Azure Blob Storage, and Terraform Cloud handle locking transparently. Local backends do not lock reliably across network boundaries.

**Forced unlock**: If a lock becomes stuck (operator crashes mid-apply), use `terraform force-unlock <LOCK_ID>` cautiously. Premature unlock with a running operation in flight corrupts state.

## Workspaces

Workspaces isolate infrastructure within the same configuration. Each workspace has its own state file but shares code.

```bash
terraform workspace list        # List workspaces
terraform workspace new prod    # Create
terraform workspace select dev  # Switch
terraform workspace delete test # Delete (only if no resources)
```

**Use cases**: Separate environments (dev, staging, prod) sharing identical infrastructure, or test multiple configurations against the same backend.

**Limitations**: Workspaces do not enforce resource isolation—both workspaces can reference the same provider and create conflicts. Do not treat workspaces as security boundaries. For multi-environment production, separate state files or separate AWS accounts are more robust.

## State Backend Strategies

### Isolated Backends (Multi-Environment)

Each environment (dev, staging, prod) uses a separate backend. Failures or malicious changes in staging cannot propagate to production.

```bash
# dev environment
terraform apply -backend-config="key=dev/terraform.tfstate"

# prod environment
terraform apply -backend-config="key=prod/terraform.tfstate"
```

### Workspace-Based Backends

Single backend, multiple workspaces. Simpler to manage but all environments share risk. Suitable for non-critical infrastructure.

## Provider Configuration

### Explicit Versioning

Pin provider versions to prevent automatic upgrades that introduce breaking changes:

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"  # >= 5.0, < 6.0
    }
  }
  required_version = ">= 1.5"
}
```

### Multi-Region & Multi-Provider

Terraform supports multiple providers via aliases:

```hcl
provider "aws" {
  alias  = "us-west"
  region = "us-west-2"
}

resource "aws_s3_bucket" "west" {
  provider = aws.us-west
  bucket   = "my-west-bucket"
}
```

## Import and Moved Blocks

### Import

`terraform import` binds existing infrastructure to state without recreating it:

```bash
terraform import aws_security_group.existing sg-12345678
```

After import, manually add the resource to `.tf` files and verify the configuration matches the real infrastructure.

**Gotchas**: Import does not fetch full resource state—child resources, dependencies, and optional attributes may be missing. Subsequent applies may modify unintended attributes.

### Moved Blocks

Terraform 1.1+ introduced `moved` blocks to refactor without destroying and recreating resources. Useful for renaming resources or reorganizing module structure:

```hcl
moved {
  from = aws_instance.app
  to   = module.compute.aws_instance.app
}
```

Moved blocks prevent state churn and downtime during refactoring.

## Testing Infrastructure as Code

### Terratest

Terratest is a Go library for testing Terraform configurations. Write tests in Go that deploy infrastructure, validate outputs, and clean up.

```go
// Example (pseudocode)
terraform := terraform.WithDefaultRetryableErrors(t, &terraform.Options{
  TerraformDir: "../examples",
  Vars: map[string]interface{}{
    "instance_type": "t3.micro",
  },
})
defer terraform.Destroy(t)
terraform.InitAndApply(t)
output := terraform.Output(t, "instance_id")
// Assert instance is running
```

### Plan Validation

Test Terraform plans without applying:

```bash
terraform plan -out=tfplan
terraform show tfplan  # Inspect planned changes
```

## Common Migration Patterns

### Monolithic to Modular

Refactor a single `main.tf` containing hundreds of resources into composable modules:

1. Identify logical boundaries (networking, compute, storage).
2. Create modules for each boundary.
3. Move resources into module directories.
4. Update root module to call modules.
5. Use `moved` blocks to maintain state continuity.

### Migrating Between Providers

Switching from one Terraform provider to another (e.g., AWS to GCP) requires updating resource types and arguments. Use `import` strategically or create parallel infrastructure and migrate data.

### Cross-Account Infrastructure

Manage infrastructure across AWS accounts using provider aliases and assume roles:

```hcl
provider "aws" {
  alias = "account-b"
  assume_role {
    role_arn = "arn:aws:iam::ACCOUNT_B:role/TerraformRole"
  }
}

resource "aws_s3_bucket" "cross_account" {
  provider = aws.account-b
  bucket   = "cross-account-bucket"
}
```

## Terragrunt

Terragrunt is a wrapper around Terraform reducing boilerplate for multi-environment deployments.

**DRY principle**: Reference remote Terraform modules and share common variables across environments without duplicating root modules:

```hcl
# terragrunt.hcl
terraform {
  source = "git::https://github.com/myorg/terraform-modules.git//vpc?ref=v1.0"
}

include {
  path = find_in_parent_folders()
}

inputs = {
  region = "us-east-1"
}
```

Terragrunt reduces configuration sprawl but adds a layer of indirection. It's most valuable in large, multi-environment setups.

## Summary

Terraform patterns evolve around state safety (remote backends, locking), modularity (clear abstractions, sensible inputs/outputs), and version control (pin providers, test before applying). Production deployments require remote state, explicit locking, and testing. Terragrunt reduces boilerplate for large-scale multi-environment infrastructure.