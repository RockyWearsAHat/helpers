# Kubernetes Troubleshooting — Pod Debugging, Events, and Operational Diagnosis

## Overview

Kubernetes troubleshooting is systematic observation. When a pod misbehaves—fails to start, crashes, or hangs—the cluster leaves breadcrumbs in events, pod status, logs, and metrics. Effective debugging follows a diagnostic flow: gather information, interpret signals, form hypotheses, test.

This note covers the mental model and tools. Implementation varies by distribution (EKS, GKE, AKS, self-hosted).

## Pod Lifecycle States and Status Phases

A pod progresses through phases reflecting its state:

- **Pending:** Resources requested but not yet scheduled (insufficient nodes, scheduling constraints blocking placement). Check: node capacity, resource requests, node selectors, taints/tolerations.

- **Running:** Pod is scheduled and at least one contained is running. Doesn't guarantee the application is healthy.

- **Succeeded:** All containers in the pod exited with code 0 (common for batch jobs).

- **Failed:** At least one container exited with non-zero code or was terminated. Pod is dead; does not restart (unless the deployment creates a replacement).

- **Unknown:** Communication with the kubelet was lost; true state unknown.

Pods also have **conditions** (Ready, Initialized, etc.) indicating readiness to serve traffic.

## Common Failure Patterns and Diagnosis

### Pattern: CrashLoopBackOff

**Symptom:** Pod restarts repeatedly, entering `CrashLoopBackOff` status. The restart delay increases exponentially (1s, 2s, 4s, 8s, ..., capped at 5min).

**Diagnosis:**

1. **Check pod status and events:**
   ```bash
   kubectl describe pod <pod> -n <ns>
   ```
   Look for status, conditions, and events section. Events often show restart reason.

2. **Examine logs:**
   ```bash
   kubectl logs <pod> -n <ns>
   kubectl logs <pod> -n <ns> --previous  # Logs from previous crashed instance
   ```
   Application logs (stderr, stdout) reveal Why the process exited (exception, config error, permission denied, etc.).

3. **Check container exit code:**
   ```bash
   kubectl get pod <pod> -o jsonpath='{.status.containerStatuses[0].lastState.terminated.exitCode}'
   ```
   Exit code often hints at the problem (1 = generic error, 137 = OOMKilled, 139 = segfault, 143 = terminated by signal).

4. **Common causes:**
   - **Configuration error:** Pod references a ConfigMap or Secret that doesn't exist. Container exits immediately.
   - **Image not found:** Wrong image name or tag; pull fails.
   - **Permission denied:** Container runs as UID 1000 but volume is owned by root.
   - **Memory exhaustion:** Process uses more RAM than requested. Pod is OOMKilled.
   - **Deadlock or hang:** Process starts but waits indefinitely for a dependency (database, API) that isn't available.

### Pattern: OOMKilled (Out of Memory)

**Symptom:** Container exits with code 137 or reason `OOMKilled`. Pod enters CrashLoopBackOff.

**Root cause:** Container's memory usage exceeded its `limits`. Kubernetes terminates the container to protect the node.

**Diagnosis:**

1. Check memory limits and requests:
   ```bash
   kubectl get pod <pod> -o jsonpath='{.spec.containers[0].resources}'
   ```

2. Monitor actual memory usage (if metrics-server is installed):
   ```bash
   kubectl top pod <pod> -n <ns>
   ```

3. **Distinguish two scenarios:**
   - **Memory limit too low:** Legitimate application needs more. Increase `limits` and `requests`.
   - **Memory leak:** Application uses more over time. Check application logs for leaks; fix application code.

4. **Note:** `requests` is used for scheduling; `limits` triggers termination. Set both appropriately.

### Pattern: ImagePullBackOff

**Symptom:** Pod remains in Pending or ImagePullBackOff. Events show image pull failure.

**Causes and fixes:**

1. **Wrong image name/tag:**
   ```bash
   kubectl describe pod <pod> | grep Image  # Actual image being pulled
   ```

2. **Registry unreachable:** Network policy, DNS, or registry down.

3. **Authentication required:** Private registry needs ImagePullSecrets:
   ```yaml
   imagePullSecrets:
     - name: my-docker-secret
   ```

4. **Verify registry access:**
   ```bash
   kubectl run debug --image=<image> -it --rm -- sh  # Try pulling from inside cluster
   ```

### Pattern: Pod Stuck in Pending

**Symptom:** Pod is scheduled but not running. Status is Running in some sense but containers aren't startable.

**Diagnosis:**

1. **Node capacity exhausted:**
   ```bash
   kubectl top nodes
   kubectl describe node <node>  # Allocatable vs. Requested
   ```

2. **Resource requests too high:**
   ```bash
   kubectl describe pod <pod> | grep -A5 Requests
   ```

3. **Node affinity or taint mismatch:**
   ```bash
   kubectl get nodes --show-labels
   kubectl describe pod <pod> | grep -i affinity  
   kubectl describe nodes  # Look for Taints
   ```

4. **PVC not bound:**
   ```bash
   kubectl get pvc -n <ns>  # Look for Pending or Bound status
   kubectl describe pvc <pvc-name> -n <ns>
   ```

5. **Admission controller rejection:**
   ```bash
   kubectl describe pod <pod> -n <ns>  # Events section
   ```

### Pattern: CrashLoopBackOff on Restart

**The infinite restart loop:** Pod restarts, exits, is recreated, restarts again.

**Likely cause:** The exit condition isn't transient. The pod is misconfigured or depends on something unavailable.

**Debug approach:**

1. Check if the pod is crashing on entry or later:
   ```bash
   kubectl logs <pod> -n <ns> --timestamps=true
   ```
   Look for timing—if first logs appear immediately before crash, likely a startup problem.

2. If startup is OK but later crash suspected:
   ```bash
   kubectl exec <pod> -n <ns> -- ps aux  # Is process running?
   kubectl exec <pod> -n <ns> -- cat /proc/uptime  # How long has it been running?
   ```

3. Set `restartPolicy: Never` on a debug pod copy; runs once without restart, allowing you to inspect state.

## Debugging Tools and Techniques

### kubectl describe

```bash
kubectl describe pod <pod> -n <namespace>
```

Shows pod spec, status, conditions, events. Events are timestamped and often contain the reason for state changes. Essential first step.

### kubectl logs

```bash
kubectl logs <pod> -n <ns>
kubectl logs <pod> -n <ns> -c <container>  # If pod has multiple containers
kubectl logs <pod> -n <ns> --previous      # Logs from crashed container
kubectl logs <pod> -n <ns> -f              # Tail (follow) logs
```

Logs are stdout/stderr from the container. If a container exits before logging setup, logs may be empty. Check application logs (files inside container) via `kubectl exec`.

### kubectl exec

```bash
kubectl exec <pod> -n <ns> -- /bin/sh   # Interactive shell
kubectl exec <pod> -n <ns> -- cat /path/to/config  # Run command
```

Inspect pod's filesystem, environment, running processes. Useful for checking if required files exist, network connectivity, environment variables. Requires at least one running container.

### kubectl port-forward

```bash
kubectl port-forward <pod> 8080:8080 -n <ns>
```

Forwards local port 8080 to port 8080 inside the pod. Useful for accessing services that aren't exposed externally (databases, debugging web interfaces).

### Ephemeral Containers (debug pods)

Modern Kubernetes (1.23+) supports ephemeral containers: debug containers added to a running pod without restarting it.

```bash
kubectl debug <pod> -n <ns> -it --image=alpine
```

Runs a shell inside the pod's namespace, allowing inspection without disrupting the application. If `kubectl debug` isn't available, use `kubectl run` to create a temporary pod in the same namespace:

```bash
kubectl run debug --image=busybox:latest -it --rm -- sh
```

### kubectl get / watch

```bash
kubectl get pods -n <ns> -o wide     # Node assignment, IP addresses
kubectl get events -n <ns> --sort-by='.lastTimestamp'  # Cluster events
kubectl get events -A --all-namespaces | grep Warning  # Warnings cluster-wide
```

Watch a pod's progression:
```bash
kubectl get pod <pod> -n <ns> -w  # Watch until status changes (Ctrl-C to exit)
```

## Network Connectivity Debugging

### pod-to-service connectivity fails

**Test from inside pod:**

```bash
kubectl exec <pod> -n <ns> -- nc -zv <service-name> 80
```

If this fails:
1. **Service exists?** `kubectl get svc -n <ns>`
2. **Endpoints exist?** `kubectl get endpoints <service-name> -n <ns>` — should list pod IPs.
3. **Network policies blocking?** `kubectl get networkpolicy -n <ns>`
4. **DNS resolving?** `kubectl exec <pod> -n <ns> -- nslookup <service-name>`

### Cross-namespace service access

```bash
kubectl exec <pod> -n ns1 -- nc -zv myservice.ns2.svc.cluster.local 80
```

Service FQDN: `<service>.<namespace>.svc.cluster.local`.

## Distributed Debugging (Multi-Pod Issues)

### Tracing request flow

If traffic isn't reaching the right pod:

1. **Verify Service endpoints:**
   ```bash
   kubectl get endpoints <service> -n <ns> -o yaml | grep -A5 addresses
   ```

2. **Check kube-proxy rules:**
   ```bash
   kubectl debug node/<node> -it --image=ubuntu
   # Inside node:
   iptables-save | grep <service-ip>  # If using iptables mode
   ```

3. **Trace service load balancing:**
   Send multiple requests from a pod; check if hits are distributed:
   ```bash
   kubectl exec <pod> -n <ns> -- for i in {1..10}; do curl <service>; done
   ```

### Log aggregation across pods

For stateless services, logs are scattered across pod instances:

```bash
kubectl logs -lapp=myapp --all-containers=true -n <ns>
```

For structured debugging, external logging (ELK, Loki, cloud logging services) is more practical.

## Node-Level Debugging

### Node pressure and resource constraints

```bash
kubectl describe node <node-name>
```

Look for `MemoryPressure`, `DiskPressure`, `PIDPressure`, `Ready` conditions. If conditions are False, pods won't be scheduled.

### Node disconnected / kubelet issues

```bash
kubectl describe node <node-name> | grep -A20 "Conditions:"
```

If Ready condition is False or Unknown, the kubelet may be unresponsive. Check kubelet logs on the node (depends on distribution; generally in `/var/log/kubelet.log` or visible via `journalctl -u kubelet`).

## Event Analysis Pattern

Kubernetes events are a timeline of state changes. They're not logs; they're high-level notifications.

```bash
kubectl get events -n <ns> --sort-by='.lastTimestamp'
```

Read events in reverse chronological order. Events show when pods were scheduled, when containers crashed, why they restarted. Events have a TTL (default 1 hour); older events are garbage-collected.

## Systematic Troubleshooting Flow

1. **Describe the pod** (`kubectl describe`). Read status, conditions, events.
2. **Check logs** (`kubectl logs`, `--previous`). Look for exceptions, errors, startup failures.
3. **Inspect the pod** (`kubectl exec`). Check config files, environment, filesystem.
4. **Check dependencies** (ConfigMap, Secret, PVC, Service, network policies). Do they exist and are they correct?
5. **Check node** (`kubectl describe node`). Is the node healthy? Sufficient capacity?
6. **Check cluster events** (`kubectl get events`). Is the cluster experiencing resource pressure, evictions?
7. **Escalate to metrics** (if available). Memory, CPU usage via `kubectl top`.
8. **Test connectivity** from pod to service, DNS, external URLs.

Most issues are resolved by step 4. Complex cases require steps 5-8.

## Mental Model

Pod troubleshooting is **information gathering, then hypothesis testing**. The Kubernetes API exposes pod state (status, conditions), events (state changes), logs (application output), and events (cluster actions). Combine these signals to diagnose.

CrashLoopBackOff usually means the pod is misconfigured or its dependency isn't available. Pending means the scheduler couldn't find a node. OOMKilled means memory limits are too low or the app leaks memory.

Effective troubleshooting avoids guessing and systematically eliminates possibilities. Start with `describe`, then logs, then dependency checks, then node-level issues.