# AWS CloudFormation — Templates, Stacks, Functions, and Infrastructure as Code

## Overview

**AWS CloudFormation** is a declarative infrastructure-as-code service. Write a template (YAML or JSON) describing AWS resources; CloudFormation creates, updates, and deletes resources to match the template. Templates are versioned like application code, enabling repeatable, auditable infrastructure deployments.

## Template Anatomy

CloudFormation templates are YAML or JSON documents with sections:

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Description: "Production VPC with RDS database"

Parameters:
  DBInstanceClass:
    Type: String
    Default: db.t3.micro
    Description: RDS instance size

  Environment:
    Type: String
    AllowedValues: [dev, staging, prod]

Metadata:
  AWS::CloudFormation::Interface:
    ParameterGroups:
      - Label:
          default: "Database Configuration"
        Parameters:
          - DBInstanceClass

Mappings:
  EnvironmentConfig:
    dev:
      AllowedCIDR: 10.0.0.0/8
    prod:
      AllowedCIDR: 10.100.0.0/8

Conditions:
  IsProd: !Equals [!Ref Environment, prod]

Resources:
  VPC:
    Type: AWS::EC2::VPC
    Properties:
      CidrBlock: 10.0.0.0/16

  RDSInstance:
    Type: AWS::RDS::DBInstance
    Properties:
      DBInstanceClass: !Ref DBInstanceClass
      Engine: postgres
      MasterUsername: admin
      MultiAZ: !If [IsProd, true, false]

Outputs:
  RDSEndpoint:
    Value: !GetAtt RDSInstance.Endpoint.Address
    Export:
      Name: !Sub "${AWS::StackName}-RDSEndpoint"
```

### Sections Explained

**AWSTemplateFormatVersion**: identifies template format version (currently `2010-09-09`).

**Description**: optional text displayed in CloudFormation console.

**Parameters**: inputs to template. Users supply at stack creation time. Types include `String`, `Number`, `AWS::EC2::Image::Id` (CloudFormation validates is valid AMI ID), `List<AWS::EC2::SecurityGroup::Id>`, etc.

**Mappings**: static lookup tables mapping keys to values. Example: region to AMI ID mapping lets single template work across regions.

**Conditions**: boolean expressions evaluated at stack creation time; gate resource creation or property assignment. Example: `IsProd: !Equals [!Ref Environment, prod]` creates condition checking if environment is prod.

**Resources**: AWS resources to create. Each resource has `Type` (e.g., `AWS::EC2::VPC`) and `Properties` (resource-specific configuration).

**Outputs**: values exported from stack. Example: RDS endpoint. Can be used by other stacks (cross-stack references) or displayed in console.

## Intrinsic Functions

Intrinsic functions manipulate values during template evaluation. Common ones:

### Conditionals & Selection

- `!If [ConditionName, ValueIfTrue, ValueIfFalse]`: runtime conditional
- `!Equals [A, B]`: true if A == B
- `!And`, `!Or`, `!Not`: logical operators

### References

- `!Ref LogicalResourceId`: reference resource or parameter by logical ID
- `!GetAtt Resource.AttributeName`: get resource attribute after creation (e.g., `!GetAtt RDSInstance.Endpoint.Address`)

### String Manipulation

- `!Sub "arn:aws:ec2:${AWS::Region}:${AWS::AccountId}:vpc/${VPCId}"`: substitute variables (${var} replaced with value)
- `!Join [delimiter, [list]]`: join array with delimiter

### Base64 & Refs

- `!Base64 String`: encode string as base64 (useful for UserData)

### Supported

- `Fn::Select`: pick element from list
- `Fn::Split`: split string by delimiter

## Resource Types

CloudFormation covers ~800 AWS resource types. Common categories:

|Category|Examples|
|---|---|
|Compute|EC2 Instance, Lambda, ECS, EKS|
|Networking|VPC, Security Group, Route Table, ALB|
|Database|RDS, DynamoDB, ElastiCache, Neptune|
|Storage|S3, EBS, EFS|
|Integration|SNS Topic, SQS Queue, EventBridge Rule|
|Management|CloudWatch Alarm, IAM Role, Systems Manager Parameter|

Each resource supports `DependsOn` (explicit dependency ordering) and `CreationPolicy` (wait for signals before move to next resource).

## Stack Operations

### Create Stack

User provides template + parameters. CloudFormation validates template, creates resources in dependency order, and reports creation status. If creation fails partway through, CloudFormation **rolls back** (deletes partial resources) by default.

### Update Stack

User provides new template. CloudFormation compares old and new templates; for each changed resource, either **updates in place** (no downtime, e.g., security group rule) or **replaces** (delete old, create new, downtime incurred, e.g., EC2 instance with new AMI). User can specify `UpdateReplacePolicy` controlling rollback behavior on update failure.

### Delete Stack

CloudFormation deletes resources in reverse dependency order. Resources with `DeletionPolicy: Retain` are NOT deleted (useful for databases, S3 buckets with important data).

### Stack Status

Statuses include: `CREATE_IN_PROGRESS`, `CREATE_COMPLETE`, `CREATE_FAILED`, `UPDATE_IN_PROGRESS`, `UPDATE_COMPLETE`, `UPDATE_ROLLBACK_IN_PROGRESS`, `DELETE_IN_PROGRESS`, `DELETE_COMPLETE`.

## Nested Stacks

**Nested stacks** are stacks created by parent stacks. Parent template defines `AWS::CloudFormation::Stack` resource:

```yaml
NestedDatabase:
  Type: AWS::CloudFormation::Stack
  Properties:
    TemplateURL: https://s3.amazonaws.com/mybucket/db-template.yaml
    Parameters:
      DBInstanceClass: db.t3.micro
```

CloudFormation creates nested stack as separate stack; outputs from nested stacks referenced via `!GetAtt NestedDatabase.Outputs.RDSEndpoint`.

### Use Cases

- **Reusable modules**: database template, VPC template, monitoring template used by multiple parent stacks
- **Team ownership**: different teams manage different nested stacks
- **Complexity management**: parent stack 200 lines, delegated to nested stacks

### Limitations

- **Debugging harder**: errors in nested stack less clear in parent stack
- **Circular dependencies possible**: nested stack A references parent output, parent creates A (CloudFormation detects and fails)

## Stack Sets

**StackSets** deploy stacks across multiple AWS accounts and regions from single template. Example: deploy VPC template to 100 accounts across 3 regions = 300 stacks.

### Use Cases

- **Multi-account compliance**: deploy security monitoring stack to all accounts
- **Global infrastructure**: deploy app stack across us-east-1, eu-west-1, ap-southeast-1 simultaneously
- **Hub-and-spoke**: deploy spoke stack to all member accounts

### Operations

- **Create StackSet**: define template + target accounts/regions
- **Create Stack Instances**: CloudFormation creates stacks in specified accounts/regions
- **Update StackSet**: new stack instances created in added accounts/regions; existing instances updated

## Change Sets

**Change sets** preview changes before applying. Example: update template changing RDS multi-AZ from false to true (requires replacement). Before applying, visualize impact: "Update RDS instance (replacement)"—alerts you to downtime.

### Workflow

1. Create change set: `aws cloudformation create-change-set --template-body ... --change-set-name my-changes`
2. Review changes in console: shows list of added/modified resources
3. Execute change set: `aws cloudformation execute-change-set --stack-name my-stack --change-set-name my-changes`

Change sets avoid surprises: apply once reviewed.

## Drift Detection

**Drift** occurs when stack resources differ from template definition. Example: template defines security group allowing port 80; operator manually allows port 443 in console—stack drifted.

**Drift detection** compares actual resources to template. CloudFormation scans stack resources and reports drifted ones. Options:

- **Ignore drift**: proceed (drift likely intentional)
- **Fix drift**: re-apply stack update, forcing resources back to template state

### Drift Causes

- Manual changes via console
- Third-party tools (Terraform applied separately)
- Automated scaling changes (auto-scaling groups modify size outside template)

## Custom Resources

**Custom resources** allow templates to interact with external systems (on-prem databases, third-party APIs, custom provisioning logic). Custom resource triggers an SNS topic or Lambda function.

### Example

```yaml
MyDatabase:
  Type: AWS::CloudFormation::CustomResource
  Properties:
    ServiceToken: arn:aws:lambda:us-east-1:ACCOUNT:function:ProvisionDB
    DatabaseName: mydb
```

On stack create/update/delete, CloudFormation calls Lambda. Lambda provisions database and returns response. If Lambda returns success, CloudFormation proceeds; if failure, CloudFormation fails stack.

### Use Cases

- Provision external database (not in CloudFormation)
- Call third-party API (configure external SaaS)
- Custom validation (ensure parameters meet org requirements)

## Comparison: CloudFormation vs. CDK vs. Terraform

### CloudFormation

- **Declarative**: describe desired state
- **Vendor**: AWS-native; covers ~800 AWS resource types
- **Language**: YAML/JSON (DSL, not Turing-complete)
- **State**: CloudFormation maintains stack state; no separate state file (default)
- **Learning curve**: moderate; JSON/YAML + intrinsic functions

### CDK (Cloud Development Kit)

- **Approach**: imperative; write code generating CloudFormation templates
- **Language**: TypeScript (or Python, Java)
- **Turing-complete**: full programming language (conditionals, loops, functions)
- **Abstraction**: constructs (reusable component libraries) simplify common patterns
- **Output**: CDK compiles to CloudFormation JSON; deploy via CloudFormation
- **State**: same as CloudFormation (maintains stack state)
- **Learning curve**: higher (requires programming knowledge)

### Terraform

- **Vendor**: cloud-agnostic; supports AWS, Azure, GCP, many others
- **Language**: HCL (Terraform's own DSL)
- **State**: separate `.tfstate` file; independent from AWS (user responsible for backing up state)
- **Modularity**: Terraform modules (reusable code) vs. CDK constructs
- **Drift detection**: `terraform plan` shows changes but drift detection is not primary concern
- **Lock-in risk**: lower (Terraform applies to multiple clouds)

### Trade-offs

|Aspect|CloudFormation|CDK|Terraform|
|---|---|---|---|
|AWS-specific features|Extensive|Good (via L1 constructs)|Delayed (LTS lag)|
|Learning curve|Medium|Medium-High|Medium|
|Multi-cloud|No|No|Yes|
|Reusability|StackSets, nested stacks|NPM modules (CDK constructs)|Terraform modules|
|State management|AWS-managed|AWS-managed|User-managed (S3, TTM)|

## SAM (Serverless Application Model)

**SAM** is CloudFormation extension for serverless applications. Provides **shorthand syntax** for Lambda, API Gateway, DynamoDB.

### Example

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Transform: AWS::Serverless-2016-10-31

Global:
  Function:
    Runtime: python3.9
    Environment:
      Variables:
        TABLE_NAME: !Ref MyTable

Resources:
  MyFunction:
    Type: AWS::Serverless::Function
    Properties:
      Handler: index.handler
      CodeUri: src/
      Events:
        GetRequest:
          Type: Api
          Properties:
            Path: /hello
            Method: GET
            RestApiId: !Ref MyAPI

  MyAPI:
    Type: AWS::Serverless::Api
    Properties:
      StageName: Prod

  MyTable:
    Type: AWS::Serverless::SimpleTable
    Properties:
      SSESpecification:
        SSEEnabled: true
```

SAM `Transform` preprocessor expands SAM resources to CloudFormation resources (Lambda → Role + Function, SAM API → API Gateway + Stages, etc.).

### Benefits

- **Less boilerplate**: no role creation, IAM policy generation
- **Local testing**: `sam local start-api` runs Lambda locally
- **Packaging**: `sam package` bundles code and uploads to S3

## Best Practices

1. **Version templates in Git**: track infrastructure changes like code
2. **Use parameters, don't hardcode**: enable reusability
3. **Prefer mappings over parameters for fixed values**: cleaner
4. **Set deletion policies**: `DeletionPolicy: Retain` on stateful resources (databases, storage)
5. **Use change sets before critical updates**: preview impact
6. **Tag resources**: add `Tags` property to resources for cost tracking
7. **Monitor drift**: periodic drift detection alerts to manual changes
8. **Modularize with nested stacks or StackSets**: avoid monolithic templates

See also: [cloud-aws-iac](cloud-aws-iac.md), [cloud-aws-serverless](cloud-aws-serverless.md)