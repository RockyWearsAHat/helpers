# AWS Container Services

## ECR (Elastic Container Registry)

### Repository Management

```bash
# Create repository with scanning and encryption
aws ecr create-repository \
  --repository-name myapp \
  --image-scanning-configuration scanOnPush=true \
  --encryption-configuration encryptionType=KMS,kmsKey=alias/ecr-key \
  --image-tag-mutability IMMUTABLE

# Cross-account access policy
aws ecr set-repository-policy --repository-name myapp --policy-text '{
  "Version": "2012-10-17",
  "Statement": [{
    "Sid": "CrossAccountPull",
    "Effect": "Allow",
    "Principal": {"AWS": "arn:aws:iam::ACCOUNT_ID:root"},
    "Action": ["ecr:GetDownloadUrlForLayer", "ecr:BatchGetImage"]
  }]
}'
```

### Image Scanning

| Feature  | Basic Scanning | Enhanced Scanning (Inspector) |
| -------- | -------------- | ----------------------------- |
| Engine   | Clair (CVE)    | Amazon Inspector (Snyk)       |
| Triggers | Push only      | Push + continuous             |
| Coverage | OS packages    | OS + language packages        |
| Findings | ECR console    | Inspector + Security Hub      |
| Cost     | Free           | Per image scanned/month       |

Enhanced scanning uses Inspector v2 — detects vulnerabilities in npm, pip, Maven, Go modules, not just OS packages. Continuous rescanning catches new CVEs against existing images.

### Lifecycle Policies

```json
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "Keep last 10 tagged images",
      "selection": {
        "tagStatus": "tagged",
        "tagPrefixList": ["v"],
        "countType": "imageCountMoreThan",
        "countNumber": 10
      },
      "action": { "type": "expire" }
    },
    {
      "rulePriority": 2,
      "description": "Expire untagged after 7 days",
      "selection": {
        "tagStatus": "untagged",
        "countType": "sinceImagePushed",
        "countUnit": "days",
        "countNumber": 7
      },
      "action": { "type": "expire" }
    }
  ]
}
```

Key behaviors: rules evaluated by priority (lowest first), matched images are marked for expiration within 24 hours, tag mutability IMMUTABLE prevents tag overwrites (forces unique tags).

### ECR Pull-Through Cache

Cache upstream registries (Docker Hub, GitHub, Quay, ECR Public) to avoid rate limits:

```bash
aws ecr create-pull-through-cache-rule \
  --ecr-repository-prefix docker-hub \
  --upstream-registry-url registry-1.docker.io \
  --credential-arn arn:aws:secretsmanager:us-east-1:ACCT:secret/dockerhub-creds
```

Images cached on first pull. Subsequent pulls served from ECR. Lifecycle policies apply to cached images.

## ECS (Elastic Container Service)

### Task Definition Anatomy

```json
{
  "family": "webapp",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "containerDefinitions": [
    {
      "name": "app",
      "image": "ACCT.dkr.ecr.REGION.amazonaws.com/myapp:v1.2.3",
      "portMappings": [{ "containerPort": 8080, "protocol": "tcp" }],
      "healthCheck": {
        "command": [
          "CMD-SHELL",
          "curl -f http://localhost:8080/health || exit 1"
        ],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      },
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/webapp",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "app"
        }
      },
      "secrets": [
        {
          "name": "DB_PASSWORD",
          "valueFrom": "arn:aws:secretsmanager:us-east-1:ACCT:secret:db-pass"
        }
      ],
      "environment": [{ "name": "NODE_ENV", "value": "production" }]
    }
  ],
  "executionRoleArn": "arn:aws:iam::ACCT:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::ACCT:role/ecsTaskRole"
}
```

**executionRoleArn** = ECS agent permissions (pull images, push logs, read secrets). **taskRoleArn** = application permissions (access S3, DynamoDB, etc.).

### Deployment Controllers

| Controller              | Behavior                            | Rollback                 | Use Case              |
| ----------------------- | ----------------------------------- | ------------------------ | --------------------- |
| Rolling update          | Replace tasks in batches            | Manual (redeploy old)    | Default, simple apps  |
| Blue/green (CodeDeploy) | Shift traffic between target groups | Automatic via CodeDeploy | Zero-downtime, canary |
| External                | User-managed                        | User-managed             | Custom controllers    |

### Circuit Breaker (Rolling Update)

```json
{
  "deploymentConfiguration": {
    "deploymentCircuitBreaker": {
      "enable": true,
      "rollback": true
    },
    "maximumPercent": 200,
    "minimumHealthyPercent": 100
  }
}
```

Circuit breaker triggers when deployment fails to reach steady state. Thresholds: if task failure count exceeds `min(10, desiredCount)`, deployment is marked FAILED and auto-rolls back.

### Service Connect

Service-to-service communication without service discovery complexity:

```json
{
  "serviceConnectConfiguration": {
    "enabled": true,
    "namespace": "production",
    "services": [
      {
        "portName": "http",
        "discoveryName": "api",
        "clientAliases": [{ "port": 8080, "dnsName": "api.production" }]
      }
    ]
  }
}
```

Uses Envoy sidecar injected automatically. Provides: service discovery, load balancing, connection draining, retries, outlier detection, and per-service traffic metrics in CloudWatch.

### App Mesh Integration

App Mesh is a full service mesh (Envoy-based) for cross-service traffic policies:

- **Virtual Services** — logical service names that route to virtual nodes/routers
- **Virtual Nodes** — map to ECS services, EC2 instances, or Kubernetes pods
- **Virtual Routers** — traffic splitting, weighted routing, path-based routing
- **Virtual Gateways** — ingress from outside the mesh

Service Connect is the simpler alternative. Use App Mesh only when you need: header-based routing, mutual TLS between services, circuit breaking with custom thresholds, or cross-cluster communication.

### Capacity Providers

```bash
# Fargate capacity provider strategy
aws ecs create-service --service-name webapp \
  --capacity-provider-strategy \
    capacityProvider=FARGATE,weight=1,base=2 \
    capacityProvider=FARGATE_SPOT,weight=3

# EC2 Auto Scaling capacity provider
aws ecs create-capacity-provider \
  --name ec2-provider \
  --auto-scaling-group-provider \
    autoScalingGroupArn=arn:aws:autoscaling:...,\
    managedScaling={status=ENABLED,targetCapacity=80},\
    managedTerminationProtection=ENABLED
```

`base` = minimum tasks on that provider. `weight` = proportional distribution of remaining tasks. Fargate Spot: ~70% cheaper, 2-minute interruption warning.

## EKS (Elastic Kubernetes Service)

### Karpenter (Node Autoscaling)

Karpenter replaces Cluster Autoscaler — provisions right-sized nodes in seconds, not minutes.

```yaml
apiVersion: karpenter.sh/v1
kind: NodePool
metadata:
  name: default
spec:
  template:
    spec:
      requirements:
        - key: kubernetes.io/arch
          operator: In
          values: ["amd64", "arm64"]
        - key: karpenter.sh/capacity-type
          operator: In
          values: ["on-demand", "spot"]
        - key: karpenter.k8s.aws/instance-category
          operator: In
          values: ["c", "m", "r"]
        - key: karpenter.k8s.aws/instance-generation
          operator: Gt
          values: ["5"]
      nodeClassRef:
        group: karpenter.k8s.aws
        kind: EC2NodeClass
        name: default
  limits:
    cpu: "1000"
    memory: 1000Gi
  disruption:
    consolidationPolicy: WhenEmptyOrUnderutilized
    consolidateAfter: 30s
```

Key advantages over Cluster Autoscaler: no node groups required, bin-packing across instance types, spot instance diversification, consolidation (replaces underutilized nodes with smaller ones).

### VPC CNI Plugin

```yaml
# Enable prefix delegation for higher pod density
kubectl set env daemonset aws-node -n kube-system \
ENABLE_PREFIX_DELEGATION=true \
WARM_PREFIX_TARGET=1
```

| Mode                    | Pods per node            | IP usage                |
| ----------------------- | ------------------------ | ----------------------- |
| Default (secondary IPs) | Limited by ENI slots     | 1 IP per pod            |
| Prefix delegation       | ~110+ per node           | /28 prefix per ENI slot |
| Custom networking       | Pods in separate subnets | Separate CIDR for pods  |

Security groups for pods: `ENABLE_POD_ENI=true` assigns individual security groups to pods via branch network interfaces. Requires Nitro instances.

### AWS Load Balancer Controller

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: app
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing
    alb.ingress.kubernetes.io/target-type: ip
    alb.ingress.kubernetes.io/certificate-arn: arn:aws:acm:...
    alb.ingress.kubernetes.io/ssl-policy: ELBSecurityPolicy-TLS13-1-2-2021-06
    alb.ingress.kubernetes.io/actions.ssl-redirect: |
      {"type":"redirect","redirectConfig":{"protocol":"HTTPS","port":"443","statusCode":"HTTP_301"}}
spec:
  rules:
    - host: app.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: app
                port:
                  number: 80
```

`target-type: ip` (recommended for Fargate + faster deregistration) vs `target-type: instance`. NLB via `service.beta.kubernetes.io/aws-load-balancer-type: external` annotation.

### External Secrets Operator

```yaml
apiVersion: external-secrets.io/v1beta1
kind: SecretStore
metadata:
  name: aws-secrets
spec:
  provider:
    aws:
      service: SecretsManager
      region: us-east-1
      auth:
        jwt:
          serviceAccountRef:
            name: external-secrets-sa
---
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: db-creds
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secrets
  target:
    name: db-credentials
  data:
    - secretKey: password
      remoteRef:
        key: prod/db/credentials
        property: password
```

Supports Secrets Manager, Parameter Store, and other backends. IRSA (IAM Roles for Service Accounts) provides pod-level AWS auth without static credentials.

### EKS Add-ons

| Add-on             | Purpose                  | Managed? |
| ------------------ | ------------------------ | -------- |
| vpc-cni            | Pod networking           | Yes      |
| kube-proxy         | Service networking       | Yes      |
| CoreDNS            | Cluster DNS              | Yes      |
| EBS CSI Driver     | Persistent volumes (EBS) | Yes      |
| EFS CSI Driver     | Shared file storage      | Yes      |
| ADOT               | Observability collector  | Yes      |
| Pod Identity Agent | IRSA replacement         | Yes      |

EKS Pod Identity (newer than IRSA): simpler setup, no OIDC provider needed, cross-account support, works with add-on pods.

## Fargate

### Fargate vs EC2 Launch Type

| Dimension        | Fargate                                | EC2                  |
| ---------------- | -------------------------------------- | -------------------- |
| Management       | Serverless (no instances)              | You manage instances |
| Pricing          | Per vCPU/GB per second                 | EC2 instance pricing |
| GPU              | Not supported                          | Supported            |
| Privileged mode  | Not supported                          | Supported            |
| DaemonSets (EKS) | Not supported                          | Supported            |
| Max resources    | 16 vCPU / 120 GB                       | Instance limits      |
| Startup time     | 30-60s cold start                      | Instance + pull time |
| Storage          | 20 GB ephemeral (expandable to 200 GB) | Instance storage     |

Fargate Spot: same as Fargate but up to 70% cheaper with 2-minute interruption notice. Not suitable for stateful workloads.

### Fargate Platform Versions

Pinning platform version in production avoids unexpected behavior changes. `LATEST` can change behavior:

- `1.4.0` — ephemeral storage, EFS support, SYS_PTRACE capability
- `1.5.0` (Linux) — current default, reduced cold start

## App Runner

Fully managed container service — no clusters, no scaling config, no load balancers:

```yaml
# apprunner.yaml
version: 1.0
runtime: python312
build:
  commands:
    build:
      - pip install -r requirements.txt
run:
  command: gunicorn app:app --bind 0.0.0.0:8080
  network:
    port: 8080
```

| Feature       | App Runner                                | ECS Fargate                     |
| ------------- | ----------------------------------------- | ------------------------------- |
| Setup         | Minutes                                   | Hours                           |
| Scaling       | Auto (concurrency-based)                  | Manual rules or target tracking |
| Networking    | Public by default, VPC connector optional | Full VPC control                |
| Customization | Limited                                   | Full                            |
| Cost          | Higher per-request at scale               | Lower at steady state           |
| Use case      | Simple web apps, APIs                     | Complex microservices           |

App Runner: auto-deploys from ECR or source repo, auto-scales to zero on no traffic (with pause/resume), HTTPS by default, custom domains, VPC connector for private resources.
