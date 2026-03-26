# AWS Infrastructure as Code

## CloudFormation

### Template Anatomy

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Description: Production VPC with public and private subnets
Metadata:
  AWS::CloudFormation::Interface:
    ParameterGroups:
      - Label: { default: "Network Configuration" }
        Parameters: [VpcCidr, Environment]

Parameters:
  VpcCidr:
    Type: String
    Default: "10.0.0.0/16"
    AllowedPattern: '(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})/(\d{1,2})'
  Environment:
    Type: String
    AllowedValues: [dev, staging, prod]

Conditions:
  IsProd: !Equals [!Ref Environment, prod]

Mappings:
  RegionAMI:
    us-east-1:
      HVM64: ami-0abcdef1234567890
    eu-west-1:
      HVM64: ami-0fedcba9876543210

Rules:
  ProdRequiresMultiAZ:
    RuleCondition: !Equals [!Ref Environment, prod]
    Assertions:
      - Assert: !Not [!Equals [!Ref "AWS::Region", "us-east-1"]]
        AssertDescription: "Prod cannot deploy to us-east-1"

Resources:
  VPC:
    Type: AWS::EC2::VPC
    DeletionPolicy: Retain
    UpdateReplacePolicy: Retain
    Properties:
      CidrBlock: !Ref VpcCidr
      EnableDnsHostnames: true
      Tags:
        - Key: Name
          Value: !Sub "${Environment}-vpc"

Outputs:
  VpcId:
    Value: !Ref VPC
    Export:
      Name: !Sub "${Environment}-VpcId"
```

Sections order doesn't matter, but convention is: Format → Description → Metadata → Parameters → Rules → Mappings → Conditions → Resources (required) → Outputs.

### Intrinsic Functions

| Function       | Purpose                        | Example                                             |
| -------------- | ------------------------------ | --------------------------------------------------- |
| `!Ref`         | Parameter value or resource ID | `!Ref MyBucket`                                     |
| `!Sub`         | String interpolation           | `!Sub '${AWS::StackName}-bucket'`                   |
| `!GetAtt`      | Resource attribute             | `!GetAtt MyBucket.Arn`                              |
| `!Join`        | Join strings with delimiter    | `!Join ['-', [!Ref Env, api]]`                      |
| `!Select`      | Pick from list by index        | `!Select [0, !GetAZs '']`                           |
| `!Split`       | Split string into list         | `!Split [',', !Ref SubnetList]`                     |
| `!If`          | Conditional value              | `!If [IsProd, 3, 1]`                                |
| `!FindInMap`   | Lookup from Mappings           | `!FindInMap [RegionAMI, !Ref 'AWS::Region', HVM64]` |
| `!ImportValue` | Cross-stack reference          | `!ImportValue prod-VpcId`                           |
| `!GetAZs`      | List AZs in region             | `!GetAZs ''`                                        |
| `!Cidr`        | Generate CIDR blocks           | `!Cidr [!Ref VpcCidr, 6, 8]`                        |
| `!Transform`   | Invoke macro                   | `!Transform {Name: 'AWS::Include', ...}`            |

**Pseudo-parameters**: `AWS::AccountId`, `AWS::Region`, `AWS::StackName`, `AWS::StackId`, `AWS::URLSuffix`, `AWS::NoValue`.

### Nested Stacks

```yaml
Resources:
  NetworkStack:
    Type: AWS::CloudFormation::Stack
    Properties:
      TemplateURL: https://s3.amazonaws.com/templates/network.yaml
      Parameters:
        VpcCidr: !Ref VpcCidr

  AppStack:
    Type: AWS::CloudFormation::Stack
    DependsOn: NetworkStack
    Properties:
      TemplateURL: https://s3.amazonaws.com/templates/app.yaml
      Parameters:
        VpcId: !GetAtt NetworkStack.Outputs.VpcId
        SubnetIds: !GetAtt NetworkStack.Outputs.PrivateSubnetIds
```

Nested vs cross-stack references: nested stacks share lifecycle (create/update/delete together), cross-stack refs (`Export`/`ImportValue`) are independent but create hard dependencies that prevent deletion.

**500 resource limit per stack** — use nested stacks to exceed this. Each nested stack gets its own 500-resource limit.

### Stack Sets

Deploy stacks across multiple accounts and regions:

```bash
aws cloudformation create-stack-set \
  --stack-set-name security-baseline \
  --template-body file://security.yaml \
  --permission-model SERVICE_MANAGED \
  --auto-deployment Enabled=true,RetainStacksOnAccountRemoval=false \
  --call-as DELEGATED_ADMIN

aws cloudformation create-stack-instances \
  --stack-set-name security-baseline \
  --deployment-targets OrganizationalUnitIds=ou-abc123 \
  --regions us-east-1 eu-west-1 ap-southeast-1 \
  --operation-preferences MaxConcurrentPercentage=25,FailureTolerancePercentage=10
```

**Permission models**: `SELF_MANAGED` (explicit IAM roles in each account) vs `SERVICE_MANAGED` (Organizations auto-creates roles). Service-managed supports auto-deployment to new accounts.

### Change Sets

Preview changes before applying:

```bash
# Create change set
aws cloudformation create-change-set \
  --stack-name mystack \
  --change-set-name update-v2 \
  --template-body file://updated.yaml

# Review changes
aws cloudformation describe-change-set \
  --stack-name mystack \
  --change-set-name update-v2
# Shows: Add, Modify (replacement true/false/conditional), Remove

# Execute after review
aws cloudformation execute-change-set \
  --stack-name mystack \
  --change-set-name update-v2
```

**Replacement** column is critical: `True` means resource will be destroyed and recreated (data loss risk for RDS, etc). `Conditional` means it depends on runtime conditions.

### Drift Detection

```bash
aws cloudformation detect-stack-drift --stack-name mystack
aws cloudformation describe-stack-drift-detection-status --stack-drift-detection-id ID
aws cloudformation describe-stack-resource-drifts --stack-name mystack \
  --stack-resource-drift-status-filters MODIFIED DELETED
```

Drift status per resource: `IN_SYNC`, `MODIFIED`, `DELETED`, `NOT_CHECKED`. Shows property-level diff (expected vs actual). Not all resources support drift detection.

**Import existing resources**: `aws cloudformation create-change-set --change-set-type IMPORT --resources-to-import` — bring manually-created resources under CloudFormation management.

### Resource Policies

```yaml
Resources:
  Database:
    Type: AWS::RDS::DBInstance
    DeletionPolicy: Snapshot # Create snapshot before delete
    UpdateReplacePolicy: Snapshot # Snapshot before replacement
    Properties: ...

  LogBucket:
    Type: AWS::S3::Bucket
    DeletionPolicy: Retain # Never delete, even on stack delete
```

DeletionPolicy values: `Delete` (default), `Retain`, `Snapshot` (RDS, EBS, ElastiCache, Neptune, Redshift).

## CDK (Cloud Development Kit)

### Construct Levels

| Level | Name          | What It Is                           | Example                                 |
| ----- | ------------- | ------------------------------------ | --------------------------------------- |
| L1    | Cfn resources | 1:1 CloudFormation mapping           | `CfnBucket`                             |
| L2    | Curated       | Opinionated defaults, helper methods | `Bucket`                                |
| L3    | Patterns      | Multi-resource architectures         | `ApplicationLoadBalancedFargateService` |

```typescript
import * as cdk from "aws-cdk-lib";
import {
  Bucket,
  BlockPublicAccess,
  BucketEncryption,
} from "aws-cdk-lib/aws-s3";
import { ApplicationLoadBalancedFargateService } from "aws-cdk-lib/aws-ecs-patterns";

// L1 — raw CloudFormation, no defaults
new cdk.aws_s3.CfnBucket(this, "RawBucket", {
  bucketName: "my-bucket",
  versioningConfiguration: { status: "Enabled" },
});

// L2 — sensible defaults + helper methods
const bucket = new Bucket(this, "DataBucket", {
  encryption: BucketEncryption.S3_MANAGED,
  blockPublicAccess: BlockPublicAccess.BLOCK_ALL,
  versioned: true,
  removalPolicy: cdk.RemovalPolicy.RETAIN,
  autoDeleteObjects: false,
});
bucket.grantRead(myLambda); // Auto-generates IAM policy

// L3 — entire architecture pattern
new ApplicationLoadBalancedFargateService(this, "WebApp", {
  taskImageOptions: { image: ecs.ContainerImage.fromRegistry("nginx") },
  desiredCount: 2,
  publicLoadBalancer: true,
});
```

### CDK Assets

Local files automatically uploaded to S3/ECR during deployment:

```typescript
// Lambda from local directory — bundled and uploaded to S3
new lambda.Function(this, "Handler", {
  runtime: lambda.Runtime.NODEJS_20_X,
  handler: "index.handler",
  code: lambda.Code.fromAsset("lambda/", {
    bundling: {
      image: lambda.Runtime.NODEJS_20_X.bundlingImage,
      command: ["bash", "-c", "npm ci && cp -r . /asset-output/"],
    },
  }),
});

// Docker image — built and pushed to ECR
new ecs.ContainerImage.fromAsset("./docker/", {
  buildArgs: { NODE_ENV: "production" },
  platform: ecr_assets.Platform.LINUX_AMD64,
});
```

Assets use a bootstrap S3 bucket + ECR repo (created by `cdk bootstrap`). File hash determines if re-upload is needed.

### CDK Aspects

Visit every construct in the tree to enforce policies:

```typescript
import { IAspect, Annotations } from "aws-cdk-lib";
import { CfnBucket } from "aws-cdk-lib/aws-s3";

class BucketEncryptionChecker implements IAspect {
  visit(node: IConstruct): void {
    if (node instanceof CfnBucket) {
      if (!node.bucketEncryption) {
        Annotations.of(node).addError(
          "All S3 buckets must have encryption enabled",
        );
      }
    }
  }
}

class TagEnforcer implements IAspect {
  visit(node: IConstruct): void {
    if (cdk.TagManager.isTaggable(node)) {
      cdk.Tags.of(node).add("ManagedBy", "CDK");
      cdk.Tags.of(node).add("Environment", "prod");
    }
  }
}

// Apply to entire app
cdk.Aspects.of(app).add(new BucketEncryptionChecker());
cdk.Aspects.of(app).add(new TagEnforcer());
```

### CDK Testing

```typescript
import { Template, Match, Capture } from "aws-cdk-lib/assertions";

const app = new cdk.App();
const stack = new MyStack(app, "TestStack");
const template = Template.fromStack(stack);

// Fine-grained assertions
template.hasResourceProperties("AWS::S3::Bucket", {
  BucketEncryption: {
    ServerSideEncryptionConfiguration: [
      {
        ServerSideEncryptionByDefault: {
          SSEAlgorithm: "aws:kms",
        },
      },
    ],
  },
});

// Resource count
template.resourceCountIs("AWS::Lambda::Function", 3);

// Capture values for further assertions
const roleCapture = new Capture();
template.hasResourceProperties("AWS::IAM::Role", {
  AssumeRolePolicyDocument: roleCapture,
});
expect(roleCapture.asObject().Statement[0].Principal).toEqual({
  Service: "lambda.amazonaws.com",
});

// Snapshot testing — detect unintended changes
expect(template.toJSON()).toMatchSnapshot();

// No overly permissive IAM
template.hasResourceProperties(
  "AWS::IAM::Policy",
  Match.not(
    Match.objectLike({
      PolicyDocument: {
        Statement: Match.arrayWith([
          Match.objectLike({ Action: "*", Resource: "*" }),
        ]),
      },
    }),
  ),
);
```

### CDK Pipelines

Self-mutating CI/CD pipeline:

```typescript
import {
  CodePipeline,
  CodePipelineSource,
  ShellStep,
} from "aws-cdk-lib/pipelines";

const pipeline = new CodePipeline(this, "Pipeline", {
  synth: new ShellStep("Synth", {
    input: CodePipelineSource.gitHub("org/repo", "main"),
    commands: ["npm ci", "npm run build", "npx cdk synth"],
    primaryOutputDirectory: "cdk.out",
  }),
  crossAccountKeys: true, // Encrypt artifacts for cross-account
  dockerEnabledForSynth: true,
});

// Deploy to staging first, then production with manual approval
pipeline.addStage(
  new MyAppStage(this, "Staging", {
    env: { account: "111111111111", region: "us-east-1" },
  }),
);

pipeline.addStage(
  new MyAppStage(this, "Production", {
    env: { account: "222222222222", region: "us-east-1" },
  }),
  {
    pre: [new pipelines.ManualApprovalStep("PromoteToProd")],
    post: [
      new ShellStep("IntegrationTest", {
        commands: ["curl -f https://app.example.com/health"],
      }),
    ],
  },
);
```

Pipeline is self-mutating: changes to pipeline definition are deployed automatically before application stages. Bootstrap target accounts with `cdk bootstrap --trust PIPELINE_ACCOUNT`.

## SAM (Serverless Application Model)

### SAM Template

```yaml
AWSTemplateFormatVersion: "2010-09-09"
Transform: AWS::Serverless-2016-10-31
Description: Order processing API

Globals:
  Function:
    Runtime: python3.12
    MemorySize: 256
    Timeout: 30
    Tracing: Active
    Environment:
      Variables:
        TABLE_NAME: !Ref OrdersTable

Resources:
  OrderApi:
    Type: AWS::Serverless::Api
    Properties:
      StageName: prod
      Auth:
        DefaultAuthorizer: CognitoAuth
        Authorizers:
          CognitoAuth:
            UserPoolArn: !GetAtt UserPool.Arn

  CreateOrderFunction:
    Type: AWS::Serverless::Function
    Properties:
      Handler: handlers/create_order.handler
      CodeUri: src/
      Events:
        CreateOrder:
          Type: Api
          Properties:
            RestApiId: !Ref OrderApi
            Path: /orders
            Method: post
      Policies:
        - DynamoDBCrudPolicy:
            TableName: !Ref OrdersTable

  OrdersTable:
    Type: AWS::Serverless::SimpleTable
    Properties:
      PrimaryKey:
        Name: orderId
        Type: String
      ProvisionedThroughput:
        ReadCapacityUnits: 5
        WriteCapacityUnits: 5
```

### SAM CLI

```bash
sam init --runtime python3.12 --app-template hello-world
sam build                              # Build artifacts
sam local start-api                    # Local API Gateway
sam local invoke CreateOrderFunction   # Single invocation
  --event events/create_order.json
sam local start-lambda                 # Local Lambda endpoint
sam deploy --guided                    # Interactive deployment
sam sync --watch                       # Hot reload (dev only)
sam logs -n CreateOrderFunction --tail # Tail CloudWatch logs
sam pipeline init --bootstrap          # CI/CD pipeline scaffolding
```

### SAM Policy Templates

Pre-built IAM policy snippets — avoid writing raw IAM:

| Template                       | Grants                                            |
| ------------------------------ | ------------------------------------------------- |
| `DynamoDBCrudPolicy`           | Full CRUD on a table                              |
| `S3ReadPolicy`                 | GetObject, ListBucket on a bucket                 |
| `SQSPollerPolicy`              | ReceiveMessage, DeleteMessage, GetQueueAttributes |
| `SNSPublishMessagePolicy`      | Publish to a topic                                |
| `StepFunctionsExecutionPolicy` | StartExecution on a state machine                 |
| `KMSDecryptPolicy`             | Decrypt with a KMS key                            |
| `SSMParameterReadPolicy`       | GetParameter, GetParametersByPath                 |

SAM transforms to CloudFormation during `sam build` — every SAM resource becomes one or more CloudFormation resources. SAM Accelerate (`sam sync`) is the dev workflow, `sam deploy` is the production workflow.
