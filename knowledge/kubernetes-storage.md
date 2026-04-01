# Kubernetes Storage — PersistentVolumes, StatefulSets, CSI, and Volume Modes

## Overview

Kubernetes storage abstracts physical storage (disk, NFS, cloud block storage) behind a declarative model: applications request storage via **PersistentVolumeClaims (PVCs)**, and cluster administrators provision backing storage via **PersistentVolumes (PVs)** or **StorageClasses**.

The containerized workload (pod) is ephemeral; data must survive pod termination. Kubernetes storage decouples compute (pod lifecycle) from state (volume lifetime).

## Core Abstractions

### PersistentVolume (PV)

A **PersistentVolume** is a cluster-wide resource representing actual storage: a block device, NFS export, cloud volume, or local filesystem. PVs are created by administrators (or auto-provisioned by a StorageClass).

**Lifecycle:** PVs are independent of pods. A PV can outlive many pod instantiations.

**Access modes:**

- **ReadWriteOnce (RWO):** Mounted by a single node for read-write. Most common; typical for databases.
- **ReadOnlyMany (ROMany):** Mounted by multiple nodes, read-only. Not all storage backends support.
- **ReadWriteMany (RWMany):** Mounted by multiple nodes, read-write. Requires shared storage (NFS, CephFS). Complex failure semantics; use cautiously.

**Reclaim policies** control what happens when a PVC is deleted:

- **Retain:** PV is not deleted; manual cleanup required. Conservative; prevents accidental data loss.
- **Delete:** PV and backing storage are deleted when PVC is deleted. Common for cloud volumes.
- **Recycle (deprecated):** Old method; avoid.

### PersistentVolumeClaim (PVC)

A **PersistentVolumeClaim** is a pod-level request for storage: "I need 10 Gi of storage with ReadWriteOnce access." The cluster binds a PVC to a matching PV.

```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: db-data
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: fast-ssd
  resources:
    requests:
      storage: 100Gi
```

**Binding:** After a PVC is created, the scheduler finds a matching PV (or a StorageClass auto-provisions one). Binding is binding; a PVC cannot be rebound to a different PV without deletion and recreation.

### StorageClass

A **StorageClass** is a template for dynamic PV provisioning. Rather than manually creating PVs, a StorageClass defines the provisioner (e.g., AWS EBS, GCP Persistent Disk, CSI driver) and parameters (disk type, IOPS, encryption).

When a PVC references a StorageClass, Kubernetes automatically creates a PV with the specified parameters.

```yaml
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: ebs.csi.aws.com
parameters:
  type: gp3
  iops: "3000"
  throughput: "125"
  encrypted: "true"
allowVolumeExpansion: true
```

**Provisioner:** Identifies which storage backend to use (CSI driver, or cloud provider's native provisioner).

**Parameters:** Backend-specific options (disk type, replication factor, encryption settings).

**Default StorageClass:** If no StorageClass is specified in a PVC, the default (if configured) is used. Only one StorageClass should be marked default per cluster.

## CSI – Container Storage Interface

**CSI** is the standard plugin interface for storage providers. Third-party storage systems (databases, backup services, NFS appliances) implement CSI to integrate with Kubernetes without modifying Kubernetes code.

### CSI Architecture

A CSI driver runs as a pod cluster and communicates with the kubelet via Unix sockets. The driver receives requests to create volumes, attach them to nodes, mount them, and clean up.

### CSI Plugins vs. Native Provisioners

**Native provisioners** (e.g., kubernetes.io/aws-ebs) are built into earlier Kubernetes releases. Kubernetes 1.20+ favors CSI for new backends.

**CSI drivers** are operator-deployed. Advantages: decoupled release cycle (update without Kubernetes upgrade), richer API (snapshots, expansion, cloning), multi-cloud uniformity.

**Trade-off:** Native provisioners are simpler (fewer pods to deploy); CSI is more flexible but requires dedicated operator management.

## StatefulSet Storage Patterns

A **StatefulSet** is a pod controller for applications requiring stable network identity and persistent storage (databases, message queues). Unlike Deployments (stateless replicas), StatefulSets assign ordinal names (`pod-0`, `pod-1`) and can bind a PVC per pod.

### StatefulSet Example

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: postgres
spec:
  serviceName: postgres-headless
  replicas: 3
  selector:
    matchLabels:
      app: postgres
  template:
    metadata:
      labels:
        app: postgres
    spec:
      containers:
        - name: postgres
          image: postgres:15
          volumeMounts:
            - name: data
              mountPath: /var/lib/postgresql/data
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes:
          - ReadWriteOnce
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 100Gi
```

Each replica gets its own PVC (`data-postgres-0`, `data-postgres-1`, etc.) that survives pod recreation. If a pod restarts, it's rescheduled to the same node and remounts its PVC.

### StatefulSet Limitations

- **Ordinal identity is important:** The pod name determines which PVC is used; reparenting pods is error-prone.
- **No automatic failover across nodes:** If a node fails and a pod's PVC becomes unavailable (due to node-local storage or regional availability zones), the pod cannot be scheduled elsewhere.
- **Scaling down doesn't delete volume:** PVCs remain after pod deletion, preventing accidental data loss but requiring manual cleanup.

## Volume Types and Patterns

### Ephemeral Volumes

Volumes that exist only as long as the pod. Examples:

- **emptyDir:** A pod-local scratch directory; deleted when the pod terminates. Useful for caching, temporary files, log aggregation.
- **configMap/secret:** Mount Kubernetes configuration objects as files.
- **downwardAPI:** Expose pod metadata (labels, annotations) as files.

Trade-off: ephemeral volumes don't survive pod termination; persistent volumes do.

### Projected Volumes

A single volume that projects multiple sources (configMap, secret, serviceAccountToken) as a merged filesystem. Simplifies pod manifests when multiple config sources are needed.

### Local Volumes

A **local volume** references a directory on a specific node. Use cases:

- Developer workstations (minikube with local mounts)
- High-performance local NVMe storage in data center clusters

**Use cautiously:** If the node fails, the pod cannot be rescheduled (no replicas of local storage). Requires explicit pod affinity to the node with The local volume.

## Volume Expansion

Modern StorageClasses support `allowVolumeExpansion: true`, enabling online growth of PVC size.

```bash
kubectl patch pvc db-data -p '{"spec":{"resources":{"requests":{"storage":"200Gi"}}}}'
```

**Backend-dependent:** Expansion works for cloud volumes (AWS EBS, GCP Persistent Disk, Azure Disks) and CSI drivers signaling support. Not all backends support shrinking.

## Snapshots

**VolumeSnapshots** are point-in-time copies of PV data. They enable backup, cloning, and restore workflows.

```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: db-backup-20250325
spec:
  volumeSnapshotClassName: csi-snapshots
  source:
    persistentVolumeClaimName: db-data
```

Snapshots are backed by CSI drivers; not all storage backends support them. Snapshots are useful for disaster recovery and cloning but come with space and performance costs.

## Storage Capacity Tracking

**StorageCapacity** resources track available capacity per StorageClass and topology segment (e.g., availability zone). The scheduler uses this to make better decisions when provisioning new volumes.

Without capacity tracking, the scheduler may create PVCs that fail to bind due to storage exhaustion in a region, causing pending pods.

## Mental Model

Kubernetes storage is a **level of indirection:** applications request storage via PVCs; PVs provide actual storage. StorageClasses automate PV provisioning. CSI is the pluggable interface for any storage backend.

**Ephemeral volumes** (emptyDir, configMap) are pod-local and fast; **persistent volumes** survive pod termination but introduce complexity around node affinity, snapshot management, and expansion.

**StatefulSets** bind PVCs to specific pods; useful for ordered, stateful applications like databases. **Deployments** with shared storage (RWMany PVCs) are problematic for consistency and are rarely the right pattern.

Choose storage based on application needs: always-on databases require StatefulSet + RWO PVC; batch jobs tolerate ephemeral volumes; logs need aggregation to external storage (S3, cloud logging services) rather than PVCs.