# AWS IAM Deep Dive — Policy Evaluation, Permission Boundaries, and Access Patterns

## Overview

**AWS IAM (Identity and Access Management)** controls who can access what resources. Two fundamental questions: "who is making the request?" (identity) and "what are they allowed to do?" (authorization). IAM evaluation is surprisingly intricate: multiple policy types interact (identity-based, resource-based, service control policies, permission boundaries), evaluation order matters, and condition keys narrow permissions.

## Core Concepts

**Principal**: AWS entity (user, role, service) making a request.

**Permission**: ability to perform action on resource. Granted via policies (JSON documents listing actions and resources).

**Trust relationship**: allows a principal to assume a role. IAM role has two policies: trust policy (who can assume) and access policy (what they can do once assumed).

**Account**: AWS account (root container); principals exist in accounts.

## Policy Evaluation Logic

When principal makes request (GET s3://mybucket/myobject), AWS evaluates policies in **specific order**:

```
1. Explicit Deny? → DENY (game over, no appeal)
2. Is there an Allow? → Check identity-based policies + resource-based policies
3. Found Allow? → ALLOW
4. Otherwise → IMPLICIT DENY (default)
```

### Explicit Deny

If any applicable policy includes explicit `Deny`, request DENIED. No Allow can override Deny (Deny is absolute). Example:

```json
{
  "Effect": "Deny",
  "Action": "s3:DeleteBucket",
  "Resource": "*"
}
```

Even if identity-based policy allows deletion, Deny blocks it.

### Allow Evaluation

Allow requires BOTH identity-based policy (on principal) AND resource-based policy (on resource) if both applicable, OR either if only one applies.

**Example 1 (Identity-based only)**:
- Principal: IAM user alice
- Identity-based policy on alice: `["s3:GetObject"]` on `arn:aws:s3:::mybucket/*`
- Resource: S3 object in mybucket (no resource-based policy)
- Result: ALLOW (identiy-based policy grants)

**Example 2 (Resource-based only)**:
- Principal: alice from different AWS account
- Identity-based policy on alice: (none)
- Resource: S3 object with bucket policy: allows `s3:GetObject` from other accounts
- Result: ALLOW (resource-based policy grants cross-account access)

**Example 3 (Both required)**:
- Principal: EC2 instance with IAM role
- Identity-based policy on role: `["ec2:TerminateInstances"]`
- Resource: EC2 instance with resource-based policy: NONE (EC2 instances don't have resource policies)
- Result: ALLOW (identity-based policy sufficient)

### Implicit Deny

If no Allow found, request DENIED. Principle: **deny by default**; must explicitly grant permissions.

## Identity-Based Policies

Identity-based policies are attached to **principals** (users, roles, groups). Policies are portable; same policy can be attached to multiple principals.

### Policy Structure

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "DenyDeleteDatabase",
      "Effect": "Deny",
      "Action": "rds:DeleteDBInstance",
      "Resource": "*"
    },
    {
      "Sid": "AllowS3Read",
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:ListBucket"],
      "Resource": ["arn:aws:s3:::mybucket/*", "arn:aws:s3:::mybucket"]
    }
  ]
}
```

- **Version**: policy language version (always `2012-10-17`)
- **Statement**: array of statements
- **Effect**: `Allow` or `Deny`
- **Action**: AWS API actions (e.g., `s3:GetObject`, `ec2:TerminateInstances`)
- **Resource**: ARN(s) specifying resources (e.g., `arn:aws:s3:::bucket/prefix/*`)
- **Condition**: optional condition keys narrowing permission (see below)
- **Principal**: NOT in identity-based policies (principal is the policy owner); used in resource-based policies

### Action Wildcards

Actions support wildcards:

- `s3:*` → all S3 actions
- `ec2:*SecurityGroup` → all VPC security group actions
- `*` → all actions (danger! grants admin on everything)

### Resource ARN Format

```
arn:partition:service:region:account:resource
```

- `arn:aws:s3:::mybucket/*` → all objects in mybucket (partition=aws, service=s3, no region/account for S3)
- `arn:aws:ec2:us-east-1:ACCOUNT:instance/*` → all EC2 instances in us-east-1
- `*` → all resources (danger!)

## Resource-Based Policies

Resource-based policies are **attached to resources** (S3 bucket, SQS queue, KMS key, etc.). Not all resources support resource-based policies (EC2 instances don't; Lambda functions do).

### Resource Policy Example (S3 Bucket Policy)

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {"AWS": "arn:aws:iam::OTHER-ACCOUNT:root"},
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::mybucket/*"
    }
  ]
}
```

Grants cross-account access: user in OTHER-ACCOUNT can read objects in mybucket.

### Trust Policy (Role Resource Policy)

IAM roles are resources with resource-based policies called **trust policies**. Trust policy specifies **who can assume the role**:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {"Service": "ec2.amazonaws.com"},
      "Action": "sts:AssumeRole"
    }
  ]
}
```

Allows EC2 service to assume role (EC2 instances launch with this role, EC2 service automatically assumes role on instance behalf).

Example: Lambda assuming role for cross-account access:

```json
{
  "Effect": "Allow",
  "Principal": {"Service": "lambda.amazonaws.com"},
  "Action": "sts:AssumeRole"
}
```

## Permission Boundaries

**Permission boundaries** set **maximum** permissions a user can have. Unlike Deny (absolute), boundary is a **ceiling**: user's actual permissions are the **intersection** of identity-based policies and boundary.

### Example

- Permission boundary: `s3:*`, `ec2:Describe*` (can do any S3 action, describe EC2 instances)
- Identity-based policy: `s3:DeleteBucket`, `ec2:TerminateInstances` (delete buckets, terminate instances)
- Actual permissions: NONE (identity policy tries to do actions OUTSIDE boundary; intersection is empty)

If identity policy were `s3:GetObject`, actual permission is `s3:GetObject` (within boundary).

### Use Cases

- **Project isolation**: boundary limits team to project resources (prevents accidental access to prod)
- **Contractor access**: boundary for contract workers limits damage if credentials leaked
- **Permission delegation with safeguards**: admin delegates power to junior dev but boundary caps max damage

### Cost

Boundaries add complexity; admin must maintain two policy layers. Often simpler to use specific identity-based policies.

## Service Control Policies (SCPs)

**Service Control Policies** are like permission boundaries but apply to **AWS accounts** (not individual principals). SCP attached to organization unit (OU) in AWS Organizations limits all principals in accounts under OU.

### Example

```json
{
  "Effect": "Deny",
  "Action": ["rds:DeleteDBInstance", "s3:DeleteBucket"],
  "Resource": "*"
}
```

Applied to OU containing prod accounts: no one (even admins) in prod can delete RDS or S3 without human approval process.

### Difference from Permission Boundaries

|Aspect|Permission Boundary|SCP|
|---|---|---|
|Scope|Individual IAM user/role|Entire AWS account(s)|
|Type|Maximum permission ceiling|Deny (blocks everyone)|
|Use case|Permission delegation safeguard|Organizational compliance|

## Condition Keys

**Condition keys** narrow permissions: grant `s3:GetObject` but ONLY from IP range 10.0.0.0/8.

### Syntax

```json
{
  "Effect": "Allow",
  "Action": "s3:GetObject",
  "Resource": "arn:aws:s3:::mybucket/*",
  "Condition": {
    "IpAddress": {
      "aws:SourceIp": "10.0.0.0/8"
    }
  }
}
```

### Common Condition Keys

|Key|Meaning|Example|
|---|---|---|
|`aws:SourceIp`|Request source IP|`10.0.0.0/8`|
|`aws:username`|Requesting IAM username|`alice`|
|`aws:PrincipalOrgID`|Organization ID (SCP)|`o-12345`|
|`s3:x-amz-server-side-encryption`|S3 encryption required|`AES256`|
|`aws:CurrentTime`|Request time|`2025-12-31T23:59:59Z`|
|`aws:userid`|Caller identity ID (unique)|`AIDACKCEVSQ6C2EXAMPLE`|
|`aws:PrincipalArn`|ARN of principal|`arn:aws:iam::ACCOUNT:role/MyRole`|

### Common Operators

- `StringEquals`: exact match
- `StringLike`: wildcard match (`*`)
- `IpAddress`: CIDR range
- `DateGreaterThan`: time-based

Example: grant LambdaRole permission to write logs, but only during business hours:

```json
{
  "Effect": "Allow",
  "Action": "logs:PutLogEvents",
  "Resource": "*",
  "Condition": {
    "DateGreaterThan": {"aws:CurrentTime": "2025-01-01T09:00:00Z"},
    "DateLessThan": {"aws:CurrentTime": "2025-01-01T17:00:00Z"}
  }
}
```

## Cross-Account Access

Cross-account access occurs when principal in Account A accesses resources in Account B. Requires **both**:

1. **Identity-based policy in Account A**: principal (in A) must have permission to do action
2. **Resource-based policy in Account B**: resource (in B) must trust Account A or specific principal in A

### Pattern

Account A setup:
```json
{
  "Effect": "Allow",
  "Action": "s3:GetObject",
  "Resource": "arn:aws:s3:::account-b-bucket/*"
}
```

Account B setup:
```json
{
  "Effect": "Allow",
  "Principal": {"AWS": "arn:aws:iam::ACCOUNT-A:root"},
  "Action": "s3:GetObject",
  "Resource": "arn:aws:s3:::account-b-bucket/*"
}
```

Account A's root grants permission; Account B's S3 bucket policy trusts Account A. User in Account A can read Account B bucket.

### Cross-Account Role Assumption

More common pattern: user in Account A assumes **role** in Account B. Role in B has trust policy allowing Account A.

Account A:
```json
{
  "Effect": "Allow",
  "Action": "sts:AssumeRole",
  "Resource": "arn:aws:iam::ACCOUNT-B:role/CrossAccountRole"
}
```

Account B (role trust policy):
```json
{
  "Effect": "Allow",
  "Principal": {"AWS": "arn:aws:iam::ACCOUNT-A:root"},
  "Action": "sts:AssumeRole"
}
```

User in Account A runs `aws sts assume-role --role-arn arn:aws:iam::ACCOUNT-B:role/CrossAccountRole` → receives temporary credentials (valid 15 minutes) for role in Account B → accesses Account B resources.

## IAM Roles Everywhere

Modern AWS practice places **IAM roles on all compute** (EC2, Lambda, ECS, etc.) rather than using long-lived credentials. Role assumption is automatic:

- **EC2**: instance metadata service (`http://169.254.169.254/`) automatically vends temporary credentials for instance's role
- **Lambda**: runtime environment automatically sets AWS credentials to function role credentials
- **ECS**: task metadata endpoint provides role credentials

Benefits:

- **No credential management**: no long-lived keys to rotate
- **Automatic refresh**: credentials auto-refresh every ~1 hour
- **Audit trail**: CloudTrail logs role assumption
- **Least privilege**: each compute gets minimal role (not shared with other instances)

Anti-pattern: hardcoding AWS access keys in application code or environment variables.

## Least Privilege Analysis

**Least privilege**: grant minimum permissions needed. In practice, this means:

1. **Start with Deny**: assume nothing is allowed
2. **Add permissions as needed**: as application makes requests, add Allow statements only for required actions/resources
3. **Review periodically**: remove unused permissions

### Tools

**IAM Access Analyzer** identifies unused permissions by analyzing CloudTrail logs (if action not called in past 90 days, marked as unused). Not foolproof (seasonal features, failover code unused in normal operation) but useful.

**IAM Policy Simulator** tests if principal can perform action (simulates policy evaluation).

### Example: Least Privilege Lambda

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ReadConfiguration",
      "Effect": "Allow",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::config-bucket/app-config.json"
    },
    {
      "Sid": "WriteToDatabase",
      "Effect": "Allow",
      "Action": ["dynamodb:PutItem", "dynamodb:Query"],
      "Resource": "arn:aws:dynamodb:us-east-1:ACCOUNT:table/MyTable",
      "Condition": {
        "StringEquals": {"dynamodb:LeadingKeys": ["${aws:username}"]}
      }
    }
  ]
}
```

Lambda can read one S3 object, write to specific DynamoDB table, and condition limits writes to rows matching username.

## Common Pitfalls

1. **Wildcard resources**: `"Resource": "*"` grants all resources; too permissive
2. **Admin policy**: `"Action": "*"` with `"Resource": "*"` grants superuser; avoid
3. **Missing trust policy check**: role created with incorrect trust policy; assume role fails (not a permission issue, a configuration issue)
4. **Condition not met silently**: permission denied if condition not met (e.g., request outside IP range); hard to debug
5. **Resource policy missing**: cross-account access fails if one side (identity or resource) missing permission

## Debugging IAM Denials

CloudTrail logs all API calls and includes decision reason:

```
event.errorCode = "AccessDenied"
event.errorMessage = "User: arn:aws:iam::ACCOUNT:user/alice is not authorized to perform: s3:DeleteBucket on resource: arn:aws:s3:::mybucket because no identity-based policy allows the s3:DeleteBucket action"
```

Read error message carefully—tells you which policy type is missing (identity-based, resource-based, condition key).

**IAM Policy Simulator**: select principal, action, resource → see Allow/Deny decision + reason. Helps verify permissions before applying in production.

See also: [cloud-aws-security](cloud-aws-security.md), [security-cloud-security](security-cloud-security.md)