# AWS EKS Patterns — Managed Nodes, Fargate, Autoscaling, and Identity

## Managed Node Groups

Amazon EKS managed node groups automate the provisioning and lifecycle management of EC2 instances for Kubernetes clusters. EKS creates and manages an Auto Scaling Group (ASG) on your behalf; every resource runs in your account. Nodes are automatically tagged for Kubernetes Cluster Autoscaler discovery and can be updated or drained without manual intervention.

### Capacity Types

**On-Demand** — Default; pay per second with no commitments. ASG uses prioritized allocation strategy, trying instance types in the order specified. Suitable for stateful or fault-intolerant workloads where interruptions are unacceptable.

**Spot** — Spare EC2 capacity at steep discounts; can be interrupted with 2-minute notice. EKS automatically enables Spot Capacity Rebalancing, which gracefully drains nodes at risk of interruption and launches replacement nodes before the interruption occurs. Suitable for stateless, fault-tolerant workloads (batch jobs, ML training, queue processing, stateless APIs). Tradeoff: Lower cost, but must tolerate interruptions and graceful shutdown complexity.

### Multi-AZ and Spot Best Practices

Managed node groups span multiple Availability Zones by default. For stateful applications backed by EBS volumes, configure multiple node groups, each scoped to a single AZ, and enable the `--balance-similar-node-groups` feature to prevent imbalanced scaling.

When using Spot, configure flexible instance types with similar vCPU/memory ratios (e.g., c5.xlarge, c5d.xlarge, c5a.xlarge, c5n.xlarge). This increases available Spot capacity pools, reducing interruption risk.

### Launch Templates and Customization

By default, EKS generates a launch template automatically. You can use a custom launch template for kubelet arguments, custom AMI, or additional configuration. Do not manually modify the auto-generated template.

### Node Auto Repair

EKS continuously monitors node health and automatically repairs or replaces unhealthy nodes. This reduces manual intervention and improves overall cluster availability. Works for both self-managed and managed nodes.

## Fargate (Serverless Pods)

AWS Fargate is a serverless compute option for running Kubernetes Pods without provisioning or managing EC2 instances. Each Pod receives isolated compute capacity (separate kernel, CPU, memory, and network interface). Pods are scheduled by Fargate controllers (part of the EKS managed control plane) based on Fargate profiles.

### Fargate Constraints

- **Daemonsets unsupported** — Daemonsets cannot run on Fargate (no node to bind to). Reconfigure as sidecar containers in Pods.
- **No privileged containers** — Fargate does not support privileged mode or HostPort/HostNetwork.
- **Private subnets only** — Pods must run on private subnets with NAT gateway access.
- **No DynamoDB/EBS mounting** — Cannot mount EBS volumes or DynamoDB file systems. Static provisioning of other storage types is available.
- **Default resource limits** — Default soft limits: 1024 (nofile), 1024 (nproc). Hard limits: 65535.
- **No EC2 IMDS access** — The EC2 instance metadata service is unavailable; use IAM Roles for Service Accounts (IRSA) for credentials.
- **No GPU support** — Fargate does not support GPU.
- **Periodic OS patching** — EKS automatically patches Fargate Pods; they may be deleted if eviction fails.

### Fargate Use Cases

Fargate is ideal for latency-sensitive APIs, web applications, and microservices where node overhead is unacceptable. Not ideal for data-heavy batch jobs, CPU-intensive workloads, or applications that require DaemonSets or low-level host access.

### Vertical Pod Autoscaler (VPA) and Horizontal Pod Autoscaler (HPA)

Use VPA to determine initial correct CPU/memory for Fargate Pods. Use HPA to scale Pod count. Set VPA mode to Auto or Recreate to ensure correct redeployment when CPU/memory combinations change.

## Autoscaling

### Cluster Autoscaler vs. Karpenter

**Cluster Autoscaler** — The legacy autoscaling tool. Observes pending Pods and scales node groups up/down based on Pod resource requests. Slower to react; scales in bulk (adds/removes entire nodes). Works with managed node groups and self-managed nodes.

**Karpenter** — Modern autoscaling tool optimized for EKS. Launches nodes on-demand in seconds; bins Pods efficiently to minimize waste. Supports flexible node pools, cost optimization, and faster scale-down. Karpenter does not use ASGs; it interacts with EC2 API directly. Preferred for modern workloads.

Both tools observe pending Pods and schedule them onto nodes. Karpenter scales faster and with better bin-packing. Cluster Autoscaler is more mature and widely deployed.

## Identity and Access Control (IRSA)

IAM Roles for Service Accounts (IRSA) grant fine-grained IAM permissions to Kubernetes Pods without embedding AWS credentials in container images or environment variables. IRSA uses OpenID Connect (OIDC) identity providers to federate Kubernetes service account identities to IAM roles.

Pod retrieves an OIDC token from the Kubernetes API (via projected service account volume). The Pod exchanges the token for AWS STS credentials (via STS AssumeRoleWithWebIdentity). The IAM role is assumed only for the specific Kubernetes workload.

IRSA is the standard pattern for giving workloads (e.g., cluster autoscaling, EBS CSI driver, external-dns) AWS API access. No shared credentials, no credential rotation burden, minimal trust boundary.

### Pod Identity (AWS-native Alternative)

AWS recently introduced Pod Identity as an alternative to IRSA, using AWS-native credential provisioning. Pod Identity is simpler to set up and manage (no OIDC provider management) but is younger and may not support all use cases. Both approaches serve the same purpose; Pod Identity reduces operational overhead for basic use cases.

## EKS Add-ons

Add-ons are managed components that provide operational capabilities (observability agents, Kubernetes drivers, networking plugins). EKS automatically installs VPC CNI, kube-proxy, and CoreDNS for every cluster.

### Add-on Types

**AWS Add-ons** — Built and fully supported by AWS (e.g., EBS CSI driver, EFS CSI driver). Patched by AWS; security updates are timely.

**AWS Marketplace Add-ons** — Scanned by AWS; built and supported by third-party vendors (e.g., Splunk monitoring). Updates and patches are the vendor's responsibility.

**Community Add-ons** — Scanned for version compatibility by AWS; supported by the open source community (e.g., Metrics Server). Patched by the community; AWS does not guarantee rapid security fixes.

### Customization

Add-ons can be customized via server-side apply (Kubernetes feature). Fields not managed by EKS can be modified in the cluster without being overridden on update.

### Namespace Scoping

Add-ons can be deployed to custom namespaces for better organization and isolation. Once installed to a namespace, removal and re-creation is required to change the namespace.

## EKS Auto Mode (Simplified Cluster Management)

EKS Auto Mode combines fully managed control plane with AWS-owned compute infrastructure. Add-ons like VPC CNI, kube-proxy, CoreDNS, EBS CSI, and Pod Identity Agent run on AWS-owned nodes, not your cluster. Add-ons automatically deploy with anti-affinity rules, avoiding unsupported compute types.

EKS Auto Mode simplifies cluster operations but sacrifices granular control. Useful for organizations without Kubernetes expertise or teams that prioritize operational simplicity over customization.

## Cost Optimization

Multi-AZ Spot managed node groups with Karpenter and consolidation policies reduce compute costs. Reserved instances amortize costs for baseline capacity. Fargate costs more per vCPU than EC2 but eliminates node management overhead. Careful workload placement (cost-optimized teams vs. performance-critical teams) drives overall cluster efficiency.

## Monitoring and Observability

Container Insights (AWS managed) provides, cluster and Pod metrics, logs, and performance dashboards. Integration with X-Ray supports distributed tracing. CloudWatch dashboards and alarms monitor resource utilization, error rates, and cost metrics.

Advanced observability requires installing open source tools (Prometheus, Grafana, Loki) on the cluster itself or integrating with third-party SaaS platforms. Community add-ons simplify open source tool deployment.

## Service Mesh

AWS App Mesh provides service-to-service networking, traffic management, and observability without modifying application code. Envoy sidecars intercept traffic. Useful for enforcing mutual TLS, request routing, and retries at the infrastructure layer. Adds operational complexity; not needed for most workloads.

## See Also

Related: `devops-kubernetes`, `cloud-aws-containers`, `devops-service-mesh`