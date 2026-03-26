# Azure Kubernetes Service (AKS) — Managed Kubernetes, Networking & Scaling

## Overview

**Azure Kubernetes Service (AKS)** is Microsoft's managed Kubernetes offering, eliminating the operational burden of running a Kubernetes control plane. AKS nodes run on Azure Virtual Machines, integrating with Azure's broader platform (networking, identity, monitoring, policy) rather than treating Kubernetes as an isolated abstraction.

## Core Architecture

### Node Pools

AKS clusters contain one or more **node pools**, each with independent configuration: VM size, count, auto-scaling rules, and taints/tolerations. Each node pool:

- Runs on a separate Virtual Machine Scale Set (VMSS)
- Can be independently upgraded or deleted
- Supports distinct workload characteristics (GPU pools for ML, memory-optimized for data processing)
- Enables canary deployments via blue-green node pool patterns

Default node pool cannot be deleted if it would leave zero nodes; consider multiple pools for high availability.

### Networking Models: Azure CNI vs. Kubenet

**Kubenet** (legacy, default on some clusters):
- Simple, lightweight overlay networking
- Pods allocated from cluster-level subnet
- Limited IP address space (1024 nodes/cluster)
- No automatic routing to Azure VNets

**Azure CNI** (recommended):
- Pods receive IPs directly from Azure VNet subnets
- Full Azure networking integration: NSGs, routing tables, private endpoints
- Enables cross-cluster and hybrid connectivity
- Higher IP consumption (each pod holds an Azure NIC)
- Supports Calico for network policies
- Required for advanced networking features

### Overlay Networking (Azure CNI Overlay)

Newer option combining cloud-native networking ease with CNI tight integration:
- Pods use overlay IP space, not VNet IPs
- Reduces IP pressure on production subnets
- Simpler IP planning for large clusters
- Transparent to workloads

## Scaling & Auto-Provisioning

### Horizontal Pod Autoscaling (HPA)

Kubernetes-native: scales pod replicas based on CPU/memory or custom metrics.

### Vertical Pod Autoscaling (VPA)

Recommends or automatically adjusts CPU/memory requests after observing real usage—avoid combined use with HPA on the same metric.

### Cluster Autoscaling

AKS nodes scale within a node pool:
- Removes nodes when underutilized (respects PDBs)
- Adds nodes when pods cannot schedule
- Respects min/max instance count per pool
- Skips nodes with local storage or non-replicated pods

### Virtual Nodes

AKS integrates with **Azure Container Instances (ACI)** via virtual node controller:
- Burst capacity without purchasing VMs
- Pay per-second usage
- Latency higher (container startup overhead)
- Useful for bursty, stateless workloads
- Not recommended for performance-sensitive apps

## Scaling Engines: KEDA Integration

**Kubernetes Event Driven Autoscaling (KEDA)** extends HPA for event sources beyond metrics:

- Scales on message queue depth (Azure Service Bus, RabbitMQ)
- Scales on database row counts
- Scales on HTTP request queues
- Custom scalers via webhooks

KEDA + managed identity enables secure, credential-free scaling from Azure services. For example, scale pods when Service Bus queue grows without embedding connection strings.

## Security & Compliance

### Azure Policy for AKS

Policy enforcement at cluster level:
- Enforce pod security standards (no privileged containers, no host networking)
- Restrict image registries
- Require resource requests/limits
- Enforce labels and taints
- Audit compliance against standards (CIS Kubernetes, PCI-DSS)

Violations can block admission (enforce) or log only (audit).

### Azure AD / Entra ID Integration

- Kubernetes cluster API auth via Azure AD
- RBAC integrates with Azure role assignments
- Service principal identity for cluster
- Workload identity enables pods to authenticate to Azure services without secrets
- Integrates with Azure Policy for fine-grained access control

## Advanced Networking & Observability

### Azure Monitor for Containers

Container Insights:
- Live pod/node metrics and logs
- Performance baselines and anomaly detection
- Multi-cluster dashboards
- Integration with Log Analytics

### Network Policies

Calico or Azure Network Policies restrict traffic:
- Allow/deny ingress and egress rules
- Namespace-level or pod-level granularity
- Useful for zero-trust and compliance boundaries

### Service Mesh Integration

AKS supports Istio installation:
- Traffic management, retries, timeouts
- Mutual TLS between pods
- Distributed tracing via Jaeger
- Observability dashboards (Kiali, Grafana)

## GitOps & Continuous Deployment

### Flux (GitOps Controller)

AKS can deploy Flux as a cluster extension:
- Git repository as source of truth for cluster state
- Automated syncing of manifests
- Version-controlled infrastructure changes
- Combined with Helm for package management

Alternative: `az k8s-configuration flux` CLI for Flux installation and updates.

## Operational Patterns

### Upgrades

- Kubernetes version upgrades follow Azure's release schedule (90 days behind upstream)
- Can update control plane independently from nodes
- Node pools upgrade sequentially per VMSS
- Rolling updates respect pod disruption budgets

### Multi-tenancy

- Namespace-level isolation via RBAC
- Network policies prevent cross-tenant traffic
- Resource quotas limit tenants' consumption
- Azure Policy enforces consistent configurations

### Cost Optimization

- Use spot VMs for interruptible workloads
- Reserved instances reduce compute costs
- Right-sizing node pools prevents waste
- Scheduled scaling for predictable workload patterns
- Virtual nodes for traffic spikes

## Related

See also: [cloud-azure-compute.md](cloud-azure-compute.md), [cloud-azure-networking.md](cloud-azure-networking.md), [cloud-azure-security.md](cloud-azure-security.md), [architecture-service-mesh.md](architecture-service-mesh.md)