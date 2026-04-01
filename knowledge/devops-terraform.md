# Terraform

## HCL Syntax Fundamentals

```hcl
# Blocks
resource "aws_instance" "web" {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = var.instance_type

  tags = {
    Name = "web-${var.environment}"
  }
}

# Expressions
locals {
  common_tags = merge(var.default_tags, {
    ManagedBy   = "terraform"
    Environment = var.environment
  })
  # Conditional
  instance_type = var.environment == "prod" ? "m5.large" : "t3.micro"
  # Splat
  instance_ids = aws_instance.web[*].id
}
```

### Type System

| Type         | Example                     | Notes                    |
| ------------ | --------------------------- | ------------------------ |
| `string`     | `"hello"`                   | Interpolation with `${}` |
| `number`     | `42`, `3.14`                |                          |
| `bool`       | `true`, `false`             |                          |
| `list(type)` | `["a", "b"]`                | Ordered, indexed         |
| `set(type)`  | `toset(["a", "b"])`         | Unordered, unique        |
| `map(type)`  | `{ key = "val" }`           | String keys              |
| `object({})` | `object({ name = string })` | Structural               |
| `tuple([])`  | `tuple([string, number])`   | Positional types         |

## Providers

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"    # >= 5.0, < 6.0
    }
  }
  required_version = ">= 1.5"
}

provider "aws" {
  region = "us-east-1"
  # Assume role for cross-account
  assume_role {
    role_arn = "arn:aws:iam::123456789012:role/TerraformRole"
  }
  default_tags {
    tags = local.common_tags
  }
}

# Aliased provider for multi-region
provider "aws" {
  alias  = "west"
  region = "us-west-2"
}

resource "aws_s3_bucket" "west_bucket" {
  provider = aws.west
  bucket   = "my-west-bucket"
}
```

**Version constraints**: `= 5.0` (exact), `>= 5.0` (minimum), `~> 5.0` (pessimistic — allows 5.x but not 6.0), `>= 5.0, < 5.5` (range).

## Resources and Data Sources

```hcl
# Resource — creates and manages infrastructure
resource "aws_security_group" "web" {
  name_prefix = "web-"
  vpc_id      = data.aws_vpc.main.id

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

# Data source — reads existing infrastructure
data "aws_vpc" "main" {
  filter {
    name   = "tag:Name"
    values = ["main-vpc"]
  }
}

data "aws_caller_identity" "current" {}
# Usage: data.aws_caller_identity.current.account_id
```

## Variables and Outputs

```hcl
variable "environment" {
  type        = string
  description = "Deployment environment"
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Must be dev, staging, or prod."
  }
}

variable "instance_config" {
  type = object({
    instance_type = string
    volume_size   = number
    enable_monitoring = optional(bool, true)  # default value
  })
}

output "instance_ip" {
  value       = aws_instance.web.public_ip
  description = "Public IP of web instance"
  sensitive   = false
}
```

**Variable precedence** (highest to lowest): `-var` CLI flag → `-var-file` → `*.auto.tfvars` → `terraform.tfvars` → `TF_VAR_*` env vars → default value.

## State Management

### Remote Backends

```hcl
# S3 + DynamoDB (standard AWS pattern)
terraform {
  backend "s3" {
    bucket         = "myorg-terraform-state"
    key            = "prod/networking/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"    # lock table
    kms_key_id     = "alias/terraform"
  }
}

# Terraform Cloud
terraform {
  cloud {
    organization = "myorg"
    workspaces {
      name = "prod-networking"
    }
  }
}
```

### State Manipulation

```bash
# Move resource to new address (refactoring)
terraform state mv aws_instance.old aws_instance.new
# Move into a module
terraform state mv aws_instance.web module.compute.aws_instance.web

# Remove from state (stop managing, don't destroy)
terraform state rm aws_instance.legacy

# Import existing infrastructure
terraform import aws_s3_bucket.existing my-existing-bucket
# 1.5+ import blocks (declarative)
import {
  to = aws_s3_bucket.existing
  id = "my-existing-bucket"
}

# List all resources in state
terraform state list
# Show specific resource
terraform state show aws_instance.web

# Replace a tainted resource
terraform apply -replace="aws_instance.web"
```

## Loops and Conditionals

### for_each vs count

```hcl
# count — use for identical copies or conditional creation
resource "aws_instance" "web" {
  count         = var.create_instances ? var.instance_count : 0
  ami           = var.ami_id
  instance_type = "t3.micro"
  tags = { Name = "web-${count.index}" }
}
# Reference: aws_instance.web[0], aws_instance.web[*].id

# for_each — use for distinct items (stable addressing)
resource "aws_iam_user" "users" {
  for_each = toset(["alice", "bob", "carol"])
  name     = each.value
}
# Reference: aws_iam_user.users["alice"]

# for_each with map
resource "aws_security_group_rule" "ingress" {
  for_each = {
    http  = { port = 80,  cidr = "0.0.0.0/0" }
    https = { port = 443, cidr = "0.0.0.0/0" }
  }
  type              = "ingress"
  security_group_id = aws_security_group.web.id
  from_port         = each.value.port
  to_port           = each.value.port
  protocol          = "tcp"
  cidr_blocks       = [each.value.cidr]
}
```

**Key difference**: `count` uses index (removing item 2 of 5 shifts 3,4,5 → forces recreation). `for_each` uses keys (removing "bob" only affects "bob"). Prefer `for_each` for distinct resources.

### Dynamic Blocks

```hcl
resource "aws_security_group" "dynamic" {
  name = "dynamic-sg"

  dynamic "ingress" {
    for_each = var.ingress_rules
    content {
      from_port   = ingress.value.from_port
      to_port     = ingress.value.to_port
      protocol    = ingress.value.protocol
      cidr_blocks = ingress.value.cidr_blocks
    }
  }
}
```

## Lifecycle Rules

```hcl
resource "aws_instance" "web" {
  # ...
  lifecycle {
    create_before_destroy = true        # new resource before destroying old
    prevent_destroy       = true        # block terraform destroy
    ignore_changes        = [tags, ami] # don't track drift on these
    replace_triggered_by  = [null_resource.trigger.id]

    precondition {
      condition     = data.aws_ami.latest.architecture == "x86_64"
      error_message = "AMI must be x86_64"
    }
    postcondition {
      condition     = self.public_ip != ""
      error_message = "Instance must have a public IP"
    }
  }
}
```

## Modules

```hcl
# Module usage
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "main-vpc"
  cidr = "10.0.0.0/16"
  azs  = ["us-east-1a", "us-east-1b"]
}

# Local module
module "app" {
  source      = "./modules/app"
  environment = var.environment
  vpc_id      = module.vpc.vpc_id
}
```

**Module sources**: local path (`./modules/x`), Terraform Registry (`hashicorp/consul/aws`), GitHub (`github.com/org/repo//modules/x`), S3 (`s3::https://bucket.s3.amazonaws.com/modules/x.zip`), GCS.

### Module Composition Pattern

```
environments/
├── dev/
│   └── main.tf      # calls modules with dev vars
├── staging/
│   └── main.tf
├── prod/
│   └── main.tf
modules/
├── networking/       # VPC, subnets, NAT
├── compute/          # EC2, ASG, ALB
├── database/         # RDS, ElastiCache
└── monitoring/       # CloudWatch, alerts
```

## Terragrunt

```hcl
# terragrunt.hcl — DRY wrapper around Terraform
terraform {
  source = "tfr:///terraform-aws-modules/vpc/aws?version=5.0.0"
}

include "root" {
  path = find_in_parent_folders("root.hcl")
}

inputs = {
  name = "main-vpc"
  cidr = "10.0.0.0/16"
}

dependency "account" {
  config_path = "../account"
}

inputs = {
  account_id = dependency.account.outputs.account_id
}
```

Key features: keeps config DRY across environments, manages dependency ordering between modules, auto-generates backend config, runs hooks before/after terraform commands.

## Tooling

| Tool                     | Purpose                                                                                       |
| ------------------------ | --------------------------------------------------------------------------------------------- |
| `tflint`                 | Linter — catches errors terraform validate misses (invalid instance types, deprecated syntax) |
| `tfsec` / `trivy config` | Security scanner — finds misconfigurations (public S3, open SGs)                              |
| `infracost`              | Cost estimation from plan — integrates into PRs                                               |
| `terraform-docs`         | Auto-generate module documentation from variables/outputs                                     |
| `checkov`                | Policy-as-code scanning (CIS benchmarks, custom policies)                                     |
| `tfenv`                  | Terraform version manager (like nvm for node)                                                 |

### Drift Detection

```bash
# Detect drift without applying
terraform plan -detailed-exitcode
# Exit code 0 = no changes, 1 = error, 2 = changes detected

# Refresh state from real infrastructure
terraform apply -refresh-only

# In CI — automated drift detection
terraform plan -detailed-exitcode -out=plan.tfplan
if [ $? -eq 2 ]; then
  echo "Drift detected — notify team"
fi
```

## Common Patterns

### Conditional resource creation

```hcl
resource "aws_cloudwatch_metric_alarm" "cpu" {
  count = var.enable_monitoring ? 1 : 0
  # ...
}
```

### Data transformation with for

```hcl
locals {
  # Map transformation
  subnet_map = { for s in aws_subnet.private : s.availability_zone => s.id }
  # Filtering
  prod_instances = [for i in var.instances : i if i.environment == "prod"]
}
```

### Moved blocks (refactoring without destroy)

```hcl
moved {
  from = aws_instance.web
  to   = module.compute.aws_instance.web
}
```
