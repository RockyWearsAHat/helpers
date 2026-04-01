# AWS Security Services

## IAM (Identity and Access Management)

### Policy Evaluation Logic

When a principal makes a request, AWS evaluates in this order:

1. **Explicit Deny** → If any policy explicitly denies, request is DENIED (final)
2. **SCPs** → Organization-level boundaries. Must allow (implicit deny if not)
3. **Resource-based policies** → If resource policy grants access AND no explicit deny, ALLOWED (even cross-account)
4. **Identity-based policies** → Must explicitly allow
5. **Permissions boundaries** → If set, must also allow (intersection)
6. **Session policies** → For assumed roles, further restricts

Key rule: Default deny. Explicit deny always wins. Allow requires all applicable policy layers to permit the action.

Cross-account access:

- Same account: Identity-based OR resource-based policy can grant access
- Cross-account: Identity-based AND resource-based policies must BOTH allow (unless resource policy grants to specific principal ARN, which acts as both)

### Policy Types

| Policy Type              | Attached To                | Purpose                               |
| ------------------------ | -------------------------- | ------------------------------------- |
| Identity-based (managed) | User, Group, Role          | Reusable permission sets              |
| Identity-based (inline)  | Single User, Group, Role   | One-off permissions                   |
| Resource-based           | S3, SQS, KMS, Lambda, etc. | Cross-account + specify who           |
| Permissions boundary     | User, Role                 | Maximum permissions ceiling           |
| SCP                      | OU or Account              | Organization-wide guardrails          |
| Session policy           | STS AssumeRole             | Further restrict assumed role session |
| ACL                      | S3, VPC (legacy)           | Legacy cross-account (avoid)          |

### IAM Policy Structure

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "AllowS3ReadOnly",
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:ListBucket"],
      "Resource": ["arn:aws:s3:::my-bucket", "arn:aws:s3:::my-bucket/*"],
      "Condition": {
        "StringEquals": {
          "s3:prefix": ["reports/", "data/"]
        },
        "IpAddress": {
          "aws:SourceIp": "203.0.113.0/24"
        }
      }
    }
  ]
}
```

Common condition keys:

- `aws:SourceIp` — Restrict by IP range
- `aws:RequestedRegion` — Restrict to specific regions
- `aws:PrincipalTag/key` — Attribute-based access control (ABAC)
- `aws:ResourceTag/key` — Match resource tags
- `aws:MultiFactorAuthPresent` — Require MFA
- `aws:CalledVia` — Must be called through specific service
- `aws:PrincipalOrgID` — Must belong to specific organization

### Service Control Policies (SCPs)

Organization-level guardrails that restrict maximum permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "DenyNonApprovedRegions",
      "Effect": "Deny",
      "Action": "*",
      "Resource": "*",
      "Condition": {
        "StringNotEquals": {
          "aws:RequestedRegion": ["us-east-1", "us-west-2", "eu-west-1"]
        },
        "ArnNotLike": {
          "aws:PrincipalARN": "arn:aws:iam::*:role/OrganizationAdmin"
        }
      }
    }
  ]
}
```

SCPs don't grant permissions — they set boundaries. Even if an identity policy allows an action, an SCP deny blocks it. Don't affect management account. Common patterns: Region restriction, preventing root user actions, requiring encryption.

### IAM Access Analyzer

- **External access**: Find resources shared with external entities (S3 buckets, KMS keys, IAM roles, Lambda, SQS, Secrets Manager)
- **Unused access**: Find unused roles, access keys, passwords, permissions
- **Policy validation**: Check policies for errors, security warnings, best practices
- **Policy generation**: Generate least-privilege policy from CloudTrail activity

### IAM Conventions

- Never use root account for daily operations. MFA on root. No access keys for root.
- Use roles over long-lived credentials (access keys). Rotate keys if unavoidable.
- Least privilege: Start with zero permissions, add as needed. Use Access Analyzer to right-size.
- Use permission boundaries for delegated administration
- ABAC (tag-based): Scale permissions without per-resource policies

## KMS (Key Management Service)

### Envelope Encryption

How KMS encrypts data efficiently:

```
1. GenerateDataKey → returns plaintext DEK + encrypted DEK
2. Use plaintext DEK to encrypt data locally
3. Discard plaintext DEK from memory
4. Store encrypted DEK alongside encrypted data
5. To decrypt: send encrypted DEK to KMS → get plaintext DEK → decrypt data locally
```

Why: KMS has a 4 KB encryption limit. Envelope encryption lets you encrypt unlimited data — only the small DEK touches KMS.

### Key Types

| Type                        | Management                             | Cost               | Use Case                                               |
| --------------------------- | -------------------------------------- | ------------------ | ------------------------------------------------------ |
| AWS managed                 | AWS creates/rotates (yearly)           | Free (API charges) | Default for AWS services (aws/s3, aws/ebs)             |
| Customer managed (CMK)      | You create, control policies, rotation | $1/month + API     | When you need key policy control, cross-account, audit |
| Customer managed (imported) | You provide key material               | $1/month + API     | Regulatory requirements, key escrow                    |
| CloudHSM                    | Dedicated HSM hardware                 | ~$1.50/hour        | FIPS 140-2 Level 3, full key control                   |

Key policies (resource-based) + IAM policies together control access. Key policy MUST grant access — IAM alone cannot authorize KMS operations.

### Key Rotation

- **Automatic**: Enable for CMKs — new key material yearly, old material kept for decryption. No re-encryption needed. KMS uses appropriate material based on which version encrypted the data.
- **Manual**: Create new key, re-encrypt data, update aliases. Required for imported key material.
- AWS managed keys: Rotated automatically every year (you can't change this).

### Multi-Region Keys

Replicate keys across regions for cross-region encrypt/decrypt without cross-region API calls. Same key ID in all regions. Use for: DynamoDB global tables encryption, S3 cross-region replication with client-side encryption.

## Secrets Manager

Managed secret storage with automatic rotation:

```python
import boto3
import json

client = boto3.client('secretsmanager')
response = client.get_secret_value(SecretId='prod/db/credentials')
secret = json.loads(response['SecretString'])
# secret = {"username": "admin", "password": "..."}
```

### Rotation

- Lambda-based rotation function
- Built-in rotation for RDS, Redshift, DocumentDB
- Single-user rotation: Updates password on secret + database
- Alternating-user rotation: Two users alternate — previous stays valid during rotation
- Rotation schedule: Days (1-365) or cron expression
- Rotation window: Limit rotation to specific hours

### Secrets Manager vs Parameter Store

| Feature       | Secrets Manager                    | SSM Parameter Store                               |
| ------------- | ---------------------------------- | ------------------------------------------------- |
| Rotation      | Built-in automatic                 | Manual (Lambda needed)                            |
| Cost          | $0.40/secret/month + $0.05/10K API | Free (standard), $0.05/parameter/month (advanced) |
| Max size      | 64 KB                              | 4 KB (standard), 8 KB (advanced)                  |
| Cross-account | Native                             | Via RAM sharing                                   |
| Encryption    | Always (KMS)                       | Optional (KMS)                                    |
| Versioning    | Built-in (staging labels)          | Built-in                                          |

Use Secrets Manager for: database credentials, API keys needing rotation. Use Parameter Store for: configuration values, feature flags, non-secret parameters.

## ACM (AWS Certificate Manager)

Free public SSL/TLS certificates:

- Automated renewal (certificates renew 60 days before expiry)
- Validation: DNS (recommended, automated) or Email
- Integration: ALB, CloudFront, API Gateway, NLB
- Cannot export public certificates (use only with integrated AWS services)
- Private CA: Issue private certificates for internal services ($400/month for CA)
- Regional: Certificates are regional. CloudFront requires us-east-1, ALB requires same region.

## WAF (Web Application Firewall)

### Rule Groups and Rules

WAF attaches to ALB, CloudFront, API Gateway, AppSync, Cognito:

```
Web ACL
├── AWS Managed Rule Groups
│   ├── AWSManagedRulesCommonRuleSet (OWASP Top 10)
│   ├── AWSManagedRulesSQLiRuleSet (SQL injection)
│   ├── AWSManagedRulesKnownBadInputsRuleSet
│   └── AWSManagedRulesBotControlRuleSet
├── Custom Rules
│   ├── Rate-based (DDoS mitigation)
│   ├── IP set (allow/block lists)
│   ├── Geo match (country-based)
│   └── Regex pattern match
└── Default action (Allow or Block)
```

Rule capacity: Web ACL has 5000 WCU (Web ACL Capacity Units). Each rule consumes varies WCUs (regex costs more than IP match).

Rate-based rules: Block IPs exceeding threshold (100-2,000,000,000 requests per 5-min window). Scope down: Only count requests matching additional conditions (e.g., specific path + rate limit).

### WAF Logging

Send logs to: S3 (cheapest), CloudWatch Logs, Kinesis Data Firehose. Logs include: full request details, matched rules, action taken.

## Shield

DDoS protection:

**Shield Standard** (free, automatic):

- Layer 3/4 protection against common attacks (SYN floods, UDP reflection)
- Applied to all AWS resources automatically

**Shield Advanced** ($3,000/month per organization):

- Layer 7 (application) protection
- Real-time DDoS visibility and reporting
- 24/7 access to DDoS Response Team (DRT)
- Cost protection: Credits for scaling costs during DDoS
- Automatic application layer mitigations (auto-WAF rules)
- Protects: CloudFront, Route 53, ALB, NLB, Elastic IP, Global Accelerator

## Cognito

### User Pools vs Identity Pools

| Feature  | User Pools                                                                              | Identity Pools                                                 |
| -------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Purpose  | Authentication (who are you)                                                            | Authorization (what can you access)                            |
| Output   | JWT tokens (ID, Access, Refresh)                                                        | Temporary AWS credentials (STS)                                |
| Features | Sign-up/sign-in, MFA, password policy, email/SMS verification, social login, SAML, OIDC | Federate identities from User Pools, social, SAML to IAM roles |
| Use case | App authentication, API authorization                                                   | Direct AWS resource access (S3, DynamoDB) from client          |

Common pattern: User Pool authenticates → Identity Pool exchanges JWT for IAM credentials → client accesses AWS resources directly.

### User Pool Features

- **Hosted UI**: Pre-built sign-in/sign-up pages (customizable CSS)
- **Lambda triggers**: PreSignUp, PostConfirmation, PreTokenGeneration, Custom message, etc.
- **Advanced security**: Adaptive authentication (risk-based MFA), compromised credential detection
- **Groups**: Assign users to groups mapped to IAM roles
- **Custom attributes**: Add custom fields to user profile
- **App clients**: Different auth settings per client (PKCE, client secret, scopes)

### Token Types

| Token         | Content                               | Use                                                     |
| ------------- | ------------------------------------- | ------------------------------------------------------- |
| ID Token      | User attributes (email, name, groups) | Frontend display, passing user info to backend          |
| Access Token  | Scopes, groups, client_id             | API authorization (send to API Gateway/backend)         |
| Refresh Token | Opaque                                | Exchange for new ID/Access tokens (1-3650 day lifetime) |

## GuardDuty

Intelligent threat detection:

- Analyzes: CloudTrail events, VPC Flow Logs, DNS logs, EKS audit logs, S3 data events, RDS login events, Lambda network activity, EBS volume data (malware)
- ML-based anomaly detection + known threat intelligence
- Finding types: Reconnaissance, instance compromise, account compromise, data exfiltration, cryptocurrency mining
- Finding severity: Low (1-3), Medium (4-6), High (7-8), Critical (9-10)
- Automated remediation: EventBridge → Lambda/Step Functions

Enable in all regions. Delegated administrator in Organizations for centralized management.

## Security Hub

Centralized security posture management:

- Aggregates findings from: GuardDuty, Inspector, Macie, Firewall Manager, IAM Access Analyzer, third-party tools
- Compliance standards: AWS Foundational Security Best Practices, CIS Benchmark, PCI DSS, NIST 800-53
- Security score per standard
- Automated remediation: EventBridge rules → Lambda/Systems Manager
- Cross-region aggregation to single region
- Organization-wide with delegated admin

Finding format: AWS Security Finding Format (ASFF) — standardized JSON schema across all sources.

## Security Architecture Patterns

### Defense in Depth

```
Internet → CloudFront (Shield + WAF)
  → ALB (Security Group: 443 from CloudFront only)
    → App (Security Group: 8080 from ALB SG)
      → Database (Security Group: 5432 from App SG)
        → Encryption at rest (KMS CMK)
```

### Least Privilege Progression

1. Start with zero permissions
2. Use Access Analyzer policy generation from CloudTrail
3. Add permissions boundaries for delegated admins
4. SCPs for organization-wide guardrails
5. Regular access reviews with IAM Access Analyzer unused access findings
6. Tag-based ABAC for scalable per-resource access
