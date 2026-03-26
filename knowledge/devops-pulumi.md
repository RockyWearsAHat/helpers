# Pulumi — Infrastructure as Real Code

## Core Concept

Pulumi uses general-purpose programming languages (TypeScript, Python, Go, C#, Java, YAML) instead of DSLs for infrastructure. You get loops, conditionals, functions, classes, type checking, IDE support, testing frameworks, and package management — everything a real language provides.

```typescript
// TypeScript — a VPC with subnets in a loop
import * as aws from "@pulumi/aws";

const vpc = new aws.ec2.Vpc("main", { cidrBlock: "10.0.0.0/16" });

const azs = ["us-east-1a", "us-east-1b", "us-east-1c"];
const subnets = azs.map(
  (az, i) =>
    new aws.ec2.Subnet(`subnet-${i}`, {
      vpcId: vpc.id,
      availabilityZone: az,
      cidrBlock: `10.0.${i}.0/24`,
    }),
);
```

```python
# Python equivalent
import pulumi_aws as aws

vpc = aws.ec2.Vpc("main", cidr_block="10.0.0.0/16")

azs = ["us-east-1a", "us-east-1b", "us-east-1c"]
subnets = [
    aws.ec2.Subnet(f"subnet-{i}",
        vpc_id=vpc.id,
        availability_zone=az,
        cidr_block=f"10.0.{i}.0/24",
    )
    for i, az in enumerate(azs)
]
```

## Resource Model

Every cloud resource is a class instantiation. The constructor takes a logical name and a property bag.

```typescript
// new <ResourceType>(logicalName, properties, options?)
const bucket = new aws.s3.Bucket(
  "my-bucket",
  {
    acl: "private",
    versioning: { enabled: true },
    tags: { Environment: "production" },
  },
  {
    protect: true, // prevent accidental deletion
    retainOnDelete: false,
    ignoreChanges: ["tags"],
  },
);
```

### Resource Options

| Option                | Description                                      |
| --------------------- | ------------------------------------------------ |
| `dependsOn`           | Explicit dependencies (usually auto-detected)    |
| `protect`             | Prevent deletion (must unprotect first)          |
| `retainOnDelete`      | Keep cloud resource when removed from code       |
| `ignoreChanges`       | Skip diff on specific properties                 |
| `parent`              | Set parent for component tree                    |
| `provider`            | Use a specific provider instance                 |
| `aliases`             | Previous names (for renaming resources)          |
| `deleteBeforeReplace` | Delete old before creating new                   |
| `replaceOnChanges`    | Force replacement on specific property changes   |
| `transformations`     | Modify resource properties programmatically      |
| `import`              | Import existing cloud resource into Pulumi state |

### Outputs and Inputs

Resource properties that aren't known until deployment are `Output<T>` — wrapped promises that resolve during `pulumi up`:

```typescript
const bucket = new aws.s3.Bucket("data");

// bucket.id is Output<string> — not a raw string
// Use .apply() to transform outputs
const bucketUrl = bucket.id.apply((id) => `https://${id}.s3.amazonaws.com`);

// interpolate helper (cleaner)
const bucketUrl2 = pulumi.interpolate`https://${bucket.id}.s3.amazonaws.com`;

// Pass outputs directly as inputs to other resources
const policy = new aws.s3.BucketPolicy("policy", {
  bucket: bucket.id, // Output<string> accepted as Input<string>
  policy: bucket.arn.apply((arn) =>
    JSON.stringify({
      Version: "2012-10-17",
      Statement: [
        {
          Effect: "Allow",
          Principal: "*",
          Action: "s3:GetObject",
          Resource: `${arn}/*`,
        },
      ],
    }),
  ),
});
```

**Principle:** Avoiding `.apply()` when possible keeps dependency tracking intact. Pulumi tracks dependencies through Outputs. Breaking the chain with `.apply()` loses dependency info unless you return another Output.

## Stacks

A stack is an isolated instance of a Pulumi program — one per environment (dev, staging, prod).

```bash
pulumi stack init dev
pulumi stack init staging
pulumi stack init prod

pulumi stack select prod
pulumi up                    # deploy to prod stack
```

### Stack Configuration

```bash
pulumi config set aws:region us-east-1
pulumi config set dbName myapp
pulumi config set --secret dbPassword hunter2   # encrypted
```

```typescript
const config = new pulumi.Config();
const dbName = config.require("dbName"); // fails if missing
const dbPort = config.getNumber("dbPort") ?? 5432; // optional with default
const dbPassword = config.requireSecret("dbPassword"); // Output<string>, encrypted
```

Stack config stored in `Pulumi.<stack>.yaml`:

```yaml
config:
  aws:region: us-east-1
  myproject:dbName: myapp
  myproject:dbPassword:
    secure: AAABADDhN2E... # encrypted
```

## State Management

### Pulumi Cloud (Default)

State stored in Pulumi Cloud service. Free tier for individuals. Provides UI, RBAC, audit logs, drift detection.

### Self-Managed Backends

```bash
# Local filesystem
pulumi login --local
pulumi login file://~/.pulumi-state

# S3
pulumi login s3://my-pulumi-state

# Azure Blob
pulumi login azblob://my-container

# GCS
pulumi login gs://my-pulumi-state
```

State file (`*.json`) contains the full resource graph — `urn`, `id`, `inputs`, `outputs`, `dependencies`. **Never manually edit state.** Use `pulumi state` commands:

```bash
pulumi state delete 'urn:pulumi:prod::myproject::aws:s3/bucket:Bucket::data'
pulumi state unprotect <urn>
pulumi state rename <old-urn> <new-name>
```

## Providers

Providers are plugins that manage cloud resources. Each provider maps to a cloud platform or service.

| Provider   | Package                | Resources                   |
| ---------- | ---------------------- | --------------------------- |
| AWS        | `@pulumi/aws`          | EC2, S3, Lambda, RDS, etc.  |
| Azure      | `@pulumi/azure-native` | Compute, Storage, AKS, etc. |
| GCP        | `@pulumi/gcp`          | GCE, GCS, GKE, etc.         |
| Kubernetes | `@pulumi/kubernetes`   | All K8s resources           |
| Docker     | `@pulumi/docker`       | Images, containers          |
| Cloudflare | `@pulumi/cloudflare`   | DNS, Workers, R2            |
| GitHub     | `@pulumi/github`       | Repos, teams, actions       |
| Random     | `@pulumi/random`       | Random strings, IDs         |

### Multiple Provider Instances

```typescript
const usEast = new aws.Provider("us-east", { region: "us-east-1" });
const euWest = new aws.Provider("eu-west", { region: "eu-west-1" });

const usBucket = new aws.s3.Bucket("us-data", {}, { provider: usEast });
const euBucket = new aws.s3.Bucket("eu-data", {}, { provider: euWest });
```

## Component Resources

Custom abstractions that group multiple resources into a reusable component — Pulumi's equivalent of a Terraform module.

```typescript
import * as pulumi from "@pulumi/pulumi";
import * as aws from "@pulumi/aws";

interface StaticSiteArgs {
  domain: string;
  indexDocument?: string;
}

class StaticSite extends pulumi.ComponentResource {
  public readonly bucketName: pulumi.Output<string>;
  public readonly url: pulumi.Output<string>;

  constructor(
    name: string,
    args: StaticSiteArgs,
    opts?: pulumi.ComponentResourceOptions,
  ) {
    super("custom:web:StaticSite", name, {}, opts);

    const bucket = new aws.s3.Bucket(
      `${name}-bucket`,
      {
        website: { indexDocument: args.indexDocument ?? "index.html" },
      },
      { parent: this },
    );

    const cdn = new aws.cloudfront.Distribution(
      `${name}-cdn`,
      {
        origins: [
          {
            domainName: bucket.bucketRegionalDomainName,
            originId: bucket.id,
          },
        ],
        enabled: true,
        defaultCacheBehavior: {
          targetOriginId: bucket.id,
          viewerProtocolPolicy: "redirect-to-https",
          allowedMethods: ["GET", "HEAD"],
          cachedMethods: ["GET", "HEAD"],
          forwardedValues: { queryString: false, cookies: { forward: "none" } },
        },
        restrictions: { geoRestriction: { restrictionType: "none" } },
        viewerCertificate: { cloudfrontDefaultCertificate: true },
      },
      { parent: this },
    );

    this.bucketName = bucket.id;
    this.url = cdn.domainName.apply((d) => `https://${d}`);

    this.registerOutputs({ bucketName: this.bucketName, url: this.url });
  }
}

// Usage
const site = new StaticSite("docs", { domain: "docs.example.com" });
export const siteUrl = site.url;
```

## Stack References

Share outputs between stacks (cross-stack dependencies):

```typescript
// In network stack (exports VPC ID)
export const vpcId = vpc.id;
export const subnetIds = subnets.map((s) => s.id);

// In app stack (imports from network stack)
const networkStack = new pulumi.StackReference("myorg/network/prod");
const vpcId = networkStack.getOutput("vpcId");
const subnetIds = networkStack.getOutput("subnetIds");

const cluster = new aws.ecs.Cluster("app", {});
const service = new aws.ecs.Service("app", {
  networkConfiguration: {
    subnets: subnetIds,
  },
});
```

## Secrets

Pulumi encrypts secrets in state. Secrets stay encrypted in transit and at rest.

```bash
pulumi config set --secret apiKey sk-abc123
```

```typescript
// Programmatic secrets
const secret = pulumi.secret("supersecret"); // Output<string> marked as secret

// Secret outputs from resources are auto-detected
const dbPassword = new random.RandomPassword("db-pw", { length: 32 });
// dbPassword.result is automatically secret
```

**Encryption providers:** Pulumi Cloud (default), `passphrase`, `awskms`, `azurekeyvault`, `gcpkms`, `hashivault`.

```bash
pulumi stack init prod --secrets-provider="awskms://alias/pulumi"
```

## Policy as Code (CrossGuard)

Enforce compliance rules on infrastructure changes. Policies run before deployment.

```typescript
// policy-pack/index.ts
import { PolicyPack, validateResourceOfType } from "@pulumi/policy";
import * as aws from "@pulumi/aws";

new PolicyPack("security", {
  policies: [
    {
      name: "s3-no-public-read",
      description: "S3 buckets must not have public read ACLs",
      enforcementLevel: "mandatory", // mandatory | advisory | disabled
      validateResource: validateResourceOfType(
        aws.s3.Bucket,
        (bucket, args, reportViolation) => {
          if (
            bucket.acl === "public-read" ||
            bucket.acl === "public-read-write"
          ) {
            reportViolation("S3 buckets must not be publicly readable");
          }
        },
      ),
    },
    {
      name: "required-tags",
      description: "All resources must have Environment and Team tags",
      validateResource: (args, reportViolation) => {
        if (args.props.tags) {
          const tags = args.props.tags;
          if (!tags.Environment || !tags.Team) {
            reportViolation("Missing required tags: Environment, Team");
          }
        }
      },
    },
  ],
});
```

```bash
pulumi up --policy-pack ./policy-pack
# or publish to Pulumi Cloud for org-wide enforcement
pulumi policy publish ./policy-pack
```

## Testing

### Unit Tests

Mock the Pulumi engine — test resource construction without deploying:

```typescript
// __tests__/infra.test.ts
import * as pulumi from "@pulumi/pulumi";

// Mock all resources
pulumi.runtime.setMocks({
  newResource(args) {
    return { id: `${args.name}-id`, state: args.inputs };
  },
  call(args) {
    return args.inputs;
  },
});

describe("S3 bucket", () => {
  let bucket: typeof import("../index");

  beforeAll(async () => {
    bucket = await import("../index");
  });

  test("bucket has versioning enabled", (done) => {
    bucket.dataBucket.versioning.apply((v) => {
      expect(v?.enabled).toBe(true);
      done();
    });
  });

  test("bucket is private", (done) => {
    bucket.dataBucket.acl.apply((acl) => {
      expect(acl).toBe("private");
      done();
    });
  });
});
```

### Integration Tests

Deploy to an ephemeral stack, validate, destroy:

```typescript
import { LocalWorkspace } from "@pulumi/pulumi/automation";

test("deploys successfully", async () => {
  const stack = await LocalWorkspace.createOrSelectStack({
    stackName: "test",
    projectName: "myproject",
    program: async () => {
      /* inline program */
    },
  });

  await stack.setConfig("aws:region", { value: "us-east-1" });
  const upResult = await stack.up({ onOutput: console.log });

  expect(upResult.outputs.url.value).toContain("https://");

  await stack.destroy();
}, 300_000);
```

### Property Tests

Validate properties of resources post-deployment without assertions in code:

```bash
pulumi up --expect-no-changes   # detect drift
```

## Automation API

Embed Pulumi in your own tools — CI/CD systems, CLIs, web services:

```typescript
import { LocalWorkspace, InlineProgramArgs } from "@pulumi/pulumi/automation";

async function deployEnvironment(envName: string) {
  const args: InlineProgramArgs = {
    stackName: envName,
    projectName: "myplatform",
    program: async () => {
      const bucket = new aws.s3.Bucket("data", { acl: "private" });
      return { bucketId: bucket.id };
    },
  };

  const stack = await LocalWorkspace.createOrSelectStack(args);
  await stack.setConfig("aws:region", { value: "us-east-1" });

  const result = await stack.up({ onOutput: console.log });
  return result.outputs;
}

// Use in an Express API, CLI tool, GitHub Action, etc.
const outputs = await deployEnvironment("pr-123");
```

## Pulumi vs Terraform

| Feature      | Pulumi                                  | Terraform                                       |
| ------------ | --------------------------------------- | ----------------------------------------------- |
| Language     | Real languages (TS, Python, Go, C#)     | HCL (DSL)                                       |
| State        | Pulumi Cloud or self-managed (S3, etc.) | Terraform Cloud or self-managed (S3 + DynamoDB) |
| Loops        | Native language loops                   | `count`, `for_each`                             |
| Conditionals | Native if/else                          | `condition ? true : false`                      |
| Testing      | Standard test frameworks                | `terraform test` (limited)                      |
| IDE support  | Full (types, autocomplete, refactoring) | HCL extensions                                  |
| Abstractions | Classes, functions, packages            | Modules                                         |
| Secrets      | Built-in encryption                     | Requires external (Vault, SOPS)                 |
| Policy       | CrossGuard (same languages)             | Sentinel (HashiCorp), OPA                       |
| Import       | `pulumi import`                         | `terraform import`                              |
| Providers    | Uses Terraform providers via bridge     | Native providers                                |

## Migration from Terraform

### tf2pulumi

Convert HCL to Pulumi program:

```bash
# Install
brew install pulumi/tap/tf2pulumi

# Convert
cd terraform-project/
tf2pulumi --language typescript > index.ts

# Or convert and import state
pulumi import --from terraform ./terraform.tfstate
```

### Manual Import

```bash
# Import existing resource into Pulumi state
pulumi import aws:s3/bucket:Bucket my-bucket my-actual-bucket-name

# Bulk import from JSON
pulumi import -f resources.json
```

```json
// resources.json
{
  "resources": [
    {
      "type": "aws:s3/bucket:Bucket",
      "name": "data",
      "id": "my-actual-bucket-name"
    },
    {
      "type": "aws:ec2/instance:Instance",
      "name": "web",
      "id": "i-0123456789abcdef0"
    }
  ]
}
```

### Coexistence Strategy

For gradual migration, reference Terraform outputs from Pulumi:

```typescript
import * as terraform from "@pulumi/terraform";

const tfState = new terraform.state.RemoteStateReference("network", {
  backendType: "s3",
  bucket: "terraform-state",
  key: "network/terraform.tfstate",
  region: "us-east-1",
});

const vpcId = tfState.getOutput("vpc_id");
```

## CLI Reference

```bash
pulumi new typescript          # scaffold new project
pulumi up                      # preview + deploy
pulumi preview                 # preview only (dry run)
pulumi destroy                 # tear down all resources
pulumi refresh                 # sync state with actual cloud
pulumi stack ls                # list stacks
pulumi stack output            # show stack outputs
pulumi config set key value    # set config
pulumi logs                    # tail cloud function logs
pulumi watch                   # auto-deploy on file changes
pulumi import <type> <name> <id>  # import existing resource
```
