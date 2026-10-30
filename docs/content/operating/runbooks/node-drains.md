---
title: "Node drains"
weight: 30
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Node drains runbook

This page is for you if you operate the Kubernetes cluster that Materialize runs on, and a node drain alert fired.
It covers how to find what is draining a node and why the drain hasn't finished,
how to move Materialize off the node without an outage, and when to hand the drain to whoever manages the node pool.

## How a drain affects Materialize

A drain cordons a node, so that nothing new schedules on it, and then evicts the pods that are on it.
Most workloads run several replicas, so the others keep serving while an evicted pod's replacement starts on another node.

Materialize's `environmentd` and each cluster replica's `clusterd` run as one pod each.
Evicting `environmentd` interrupts its environment until the replacement starts.
Evicting a `clusterd` pod restarts that replica, which then rehydrates.
Both happen even when a replacement node is ready.

The Materialize operator annotates these pods with `karpenter.sh/do-not-disrupt: "true"`
and `cluster-autoscaler.kubernetes.io/safe-to-evict: "false"`, so neither autoscaler removes a node that runs them.
A cordon, `kubectl drain`, and a managed node pool upgrade don't read those annotations.

The graceful way off a node is a Materialize rollout.
A rollout starts the instance's next generation of `environmentd` and `clusterd` pods, which can schedule only on nodes that aren't cordoned.
The operator tears the old generation down after the next one has caught up.
After the rollout, the node holds no Materialize pods, and draining it interrupts nothing.
On GKE, the operator can start these rollouts itself for a node pool upgrade; see the GKE tab in [Check the node pool](#check-the-node-pool).

## The alerts

The alerts treat a node as draining when it's cordoned,
or when it carries the removal taint of the cluster autoscaler (`ToBeDeletedByClusterAutoscaler`) or Karpenter (`karpenter.sh/disrupted`).
The following table lists them:

| Alert | Severity | Fires after | Covers |
|---|---|---|---|
| `materialize-node-draining` | notice | 10 minutes | Each `environmentd` and `clusterd` pod on a draining node |
| `materialize-node-drain-slow` | warning | 2 hours | The same pods |
| `materialize-node-drain-stuck` | critical | 24 hours | The same pods |
| `node-drain-stuck` | warning | 4 days | Draining nodes that run no `environmentd` or `clusterd` pods |

Every alert names the `node`.
The `materialize-node-*` alerts also name the `namespace` and `pod`.
You can change how long an alert waits with `rules.overrides`.
For more information, see [Configuring Alerting](../../../alerting/configuring/).

### materialize-node-draining

A node running `environmentd` or a cluster replica has been cordoned or marked for removal for 10 minutes.
Someone usually started it: a node pool upgrade, a scale-down, or maintenance.
If a rollout is moving the pods, the alert resolves when the rollout finishes.
Otherwise, [find what is draining the node](#find-what-is-draining-the-node),
and [move Materialize off the node](#move-materialize-off-the-node) before the drain evicts it.

### materialize-node-drain-slow

The Materialize pods have stayed on the draining node for 2 hours.
Either no rollout started, or one is stuck.
To find out which, [check the instance's rollout](#check-the-instances-rollout).

### materialize-node-drain-stuck

The Materialize pods have stayed on the draining node for 24 hours.
A managed upgrade drains the node anyway after its wait runs out, which for GKE's blue-green upgrades is three days by default.
[Check the instance's rollout](#check-the-instances-rollout).
If the drain belongs to a node pool you don't manage, [escalate to the node pool's owner](#escalate-to-the-node-pools-owner).

### node-drain-stuck

A node that runs no `environmentd` or `clusterd` pods has been cordoned or marked for removal for 4 days.
Nothing new schedules on it, so the cluster runs short of the node's capacity.
[Find what blocks the drain](#find-what-blocks-the-drain).

GKE's blue-green upgrades keep every old node in a pool cordoned for their whole wait, which is three days by default.
If your node pools wait longer, lengthen this alert's `for` with `rules.overrides` to match.

## Diagnose the drain

Each procedure in this section reads cluster state and changes nothing.
In each snippet, set `KUBECONTEXT` to your cluster's context and `NODE` to the node the alert names.

### Find what is draining the node

1. List the node's taints, and whether it's cordoned:

   ```bash
   KUBECONTEXT=my-cluster
   NODE=gke-example-pool-1234abcd-wxyz

   kubectl --context "$KUBECONTEXT" get node "$NODE" \
     -o jsonpath='unschedulable={.spec.unschedulable}{"\n"}{range .spec.taints[*]}{.key}={.value}:{.effect}{"\n"}{end}'
   ```

   A cordon also shows as the `node.kubernetes.io/unschedulable` taint.
   Match what you see to what started the drain:

   | You see | Started by |
   |---|---|
   | `ToBeDeletedByClusterAutoscaler` | The cluster autoscaler, scaling the node pool down |
   | `karpenter.sh/disrupted` | Karpenter, for consolidation, drift, or expiry |
   | `unschedulable=true` and neither taint | A cordon: `kubectl cordon`, `kubectl drain`, or a managed node pool upgrade |

1. Read the node's events:

   ```bash
   KUBECONTEXT=my-cluster
   NODE=gke-example-pool-1234abcd-wxyz

   kubectl --context "$KUBECONTEXT" get events -A --sort-by=.lastTimestamp \
     --field-selector involvedObject.kind=Node,involvedObject.name="$NODE"
   ```

   The API server keeps events for an hour by default.
   The stack collects them into Loki, so for an older drain, run the following LogQL query in Grafana's **Explore**:

   ```logql
   {job="loki.source.kubernetes_events"} | kind="Node" | name="gke-example-pool-1234abcd-wxyz"
   ```

   A `NodeNotSchedulable` event marks the cordon.

1. If no person cordoned the node, [check the node pool](#check-the-node-pool) for an upgrade or a provisioner that's replacing it.

### List what is still on the node

Run the following command:

```bash
KUBECONTEXT=my-cluster
NODE=gke-example-pool-1234abcd-wxyz

kubectl --context "$KUBECONTEXT" get pods -A -o wide --field-selector spec.nodeName="$NODE"
```

Materialize's pods are named `INSTANCE_ID-environmentd-GENERATION-0` and `INSTANCE_ID-cluster-CLUSTER_ID-replica-REPLICA_ID-gen-GENERATION-0`.
`GENERATION` is the deployment generation the pod belongs to.

### Check the instance's rollout

1. List each Materialize instance's active generation, and whether a rollout is still applying:

   ```bash
   KUBECONTEXT=my-cluster
   FIELDS='NAMESPACE:.metadata.namespace,NAME:.metadata.name,ACTIVE:.status.activeGeneration'
   FIELDS+=',UP_TO_DATE:.status.conditions[?(@.type=="UpToDate")].status'
   FIELDS+=',MESSAGE:.status.conditions[?(@.type=="UpToDate")].message'

   kubectl --context "$KUBECONTEXT" get materialize -A -o custom-columns="$FIELDS"
   ```

1. Find the row for the namespace the alert names, and match its state:

   - **`True`, with `Successfully applied changes for generation N`:** no rollout is in progress.
     To move the pods, [move Materialize off the node](#move-materialize-off-the-node).
   - **`Unknown`, with `Applying changes for generation N`:** a rollout to generation `N` is in progress.
     The pods on the node belong to the generation before it.
     Check generation `N`'s pods in the next step.

1. List the pods of the generation the rollout is applying:

   ```bash
   KUBECONTEXT=my-cluster
   NAMESPACE=materialize-environment
   GENERATION=2

   kubectl --context "$KUBECONTEXT" -n "$NAMESPACE" get pods -o wide \
     | grep -E -- "-(environmentd|gen)-${GENERATION}-[0-9]+ "
   ```

   Match what you see to what the rollout is waiting on:

   - **No pods, or pods in `Pending`:** they have no schedulable node to go to.
     Check that replacement nodes are joining; see [Check the node pool](#check-the-node-pool).
     If they aren't, [escalate to the node pool's owner](#escalate-to-the-node-pools-owner).
   - **Pods that restart or never become ready:** the next generation can't start.
     See [Troubleshooting Materialize](../../troubleshooting-materialize/).
   - **Pods that are running and ready:** the next generation is hydrating,
     or, with the `ManuallyPromote` rollout strategy, waiting for you to promote it.
     The **Generations** tab of the Materialize Upgrade dashboard shows how far it has caught up.

### Find what blocks the drain

A drain evicts a pod only when the pod's PodDisruptionBudget allows a disruption.

1. List the PodDisruptionBudgets that allow none:

   ```bash
   KUBECONTEXT=my-cluster
   FIELDS='NAMESPACE:.metadata.namespace,NAME:.metadata.name,ALLOWED:.status.disruptionsAllowed'
   FIELDS+=',HEALTHY:.status.currentHealthy,DESIRED:.status.desiredHealthy'

   kubectl --context "$KUBECONTEXT" get pdb -A -o custom-columns="$FIELDS" | awk 'NR == 1 || $3 == 0'
   ```

   A budget whose pods are already unhealthy allows no disruption, however many replicas its workload has.

1. List the pods on the node that ask the autoscalers to leave the node alone:

   ```bash
   KUBECONTEXT=my-cluster
   NODE=gke-example-pool-1234abcd-wxyz
   TEMPLATE='{range .items[*]}{.metadata.namespace}/{.metadata.name}'
   TEMPLATE+=' do-not-disrupt={.metadata.annotations.karpenter\.sh/do-not-disrupt}'
   TEMPLATE+=' safe-to-evict={.metadata.annotations.cluster-autoscaler\.kubernetes\.io/safe-to-evict}{"\n"}{end}'

   kubectl --context "$KUBECONTEXT" get pods -A --field-selector spec.nodeName="$NODE" -o jsonpath="$TEMPLATE"
   ```

## Resolve the drain

How you resolve a drain depends on whether the node is leaving the cluster.

### Move Materialize off the node

Before you start, check that the nodes that aren't cordoned have room for the instance's next generation.
The default rollout strategy, `WaitUntilReady`, runs both generations side by side until the next one has caught up.

1. Cordon every node that's leaving, so that the next generation can't land on another of them:

   ```bash
   KUBECONTEXT=my-cluster
   NODE=gke-example-pool-1234abcd-wxyz

   kubectl --context "$KUBECONTEXT" cordon "$NODE"
   ```

   A managed node pool upgrade has usually cordoned them already.

1. Force a rollout of the instance.
   Use the tab for the Materialize CRD version that the instance uses.

   {{< tabs >}}
   {{% tab "Materialize CRD v1" %}}
   Set `forceRollout` to a new UUID:

   ```bash
   KUBECONTEXT=my-cluster
   NAMESPACE=materialize-environment
   INSTANCE=main

   kubectl --context "$KUBECONTEXT" -n "$NAMESPACE" patch materialize "$INSTANCE" --type=merge \
     -p "{\"spec\": {\"forceRollout\": \"$(uuidgen)\"}}"
   ```
   {{% /tab %}}
   {{% tab "Materialize CRD v1alpha1" %}}
   Set `requestRollout` and `forceRollout` to the same new UUID:

   ```bash
   KUBECONTEXT=my-cluster
   NAMESPACE=materialize-environment
   INSTANCE=main
   UUID="$(uuidgen)"

   kubectl --context "$KUBECONTEXT" -n "$NAMESPACE" patch materialize "$INSTANCE" --type=merge \
     -p "{\"spec\": {\"requestRollout\": \"$UUID\", \"forceRollout\": \"$UUID\"}}"
   ```
   {{% /tab %}}
   {{< /tabs >}}

1. Watch the rollout until `UpToDate` is `True` and the old generation's pods are gone:

   ```bash
   KUBECONTEXT=my-cluster
   NAMESPACE=materialize-environment

   kubectl --context "$KUBECONTEXT" -n "$NAMESPACE" get pods -o wide --watch
   ```

   The `materialize-node-*` alerts resolve after the old generation's pods leave the node.

For the full procedure, including replacing a node pool,
see [Resize node pools](https://materialize.com/docs/self-managed-deployments/deployment-guidelines/resize-node-pools/)
in the Materialize documentation.

### Keep the node

If the node isn't leaving, such as after a `kubectl cordon` for maintenance that's over, make it schedulable again:

```bash
KUBECONTEXT=my-cluster
NODE=gke-example-pool-1234abcd-wxyz

kubectl --context "$KUBECONTEXT" uncordon "$NODE"
```

> [!WARNING]
> Don't uncordon a node that a managed node pool upgrade is draining.
> Ask the node pool's owner to pause, reschedule, or roll back the upgrade instead.

Uncordoning doesn't remove an autoscaler's taint.
If the cluster autoscaler is no longer removing the node, remove its taint:

```bash
KUBECONTEXT=my-cluster
NODE=gke-example-pool-1234abcd-wxyz

kubectl --context "$KUBECONTEXT" taint node "$NODE" ToBeDeletedByClusterAutoscaler-
```

## Check the node pool

Each cloud provider drains nodes for its own reasons, on its own schedule.
The following tabs cover the node pools that the
[Materialize Terraform modules](https://github.com/MaterializeInc/materialize-terraform-self-managed) create on each cloud.

{{< tabs >}}
{{% tab "Google Cloud (GKE)" %}}
GKE upgrades node pools automatically, for example to roll out a new node image.
You can delay an upgrade with maintenance windows and exclusions, but you can't turn upgrades off.

The Terraform modules put node pools on autoscaled blue-green upgrades, which run in the following order:

1. GKE creates an empty replacement (green) pool, and cordons every node in the original (blue) pool.
1. GKE waits for the pool's `wait_for_drain_duration`, which is three days by default and seven at most.
   The modules set it with `upgrade_wait_for_drain_duration`.
1. GKE drains the blue nodes in batches, and then deletes them.

During the wait, the Materialize operator's node-upgrade rollout trigger rolls out each instance with pods on the blue nodes,
so the instance moves to the green pool.
The modules' examples turn the trigger on with `enable_node_upgrade_rollout_trigger`.
For setting it up without the modules,
see [GKE node pool upgrades](https://materialize.com/docs/self-managed-deployments/deployment-guidelines/gke-node-pool-upgrades/)
in the Materialize documentation.

While the trigger works, `materialize-node-draining` resolves after each instance's rollout,
and only `node-drain-stuck` could fire for the blue nodes, after its 4 days.
If `materialize-node-drain-slow` fires during an upgrade, the trigger's rollout didn't move the instance.
[Check the instance's rollout](#check-the-instances-rollout), and the operator's logs.

To find a running upgrade and its phase, list the cluster's running node upgrades:

```bash
PROJECT_ID=my-project
LOCATION=us-east1

gcloud container operations list --project "$PROJECT_ID" --location "$LOCATION" \
  --filter="operationType=UPGRADE_NODES AND status=RUNNING" \
  --format="table(name,targetLink.basename(),startTime,detail)"
```

The `detail` column names the node pool and the phase, such as `Phase: WAITING_TO_DRAIN_BLUE_POOL`.
GKE starts draining when the wait runs out, which is the operation's start time plus the pool's `wait_for_drain_duration`.
The node pool's owner can roll back an upgrade that's still running with `gcloud container node-pools rollback`.
{{% /tab %}}
{{% tab "AWS (EKS)" %}}
The Terraform modules run Materialize and the other workloads on nodes from Karpenter, in separate Karpenter node pools.
Karpenter itself, with a few cluster add-ons, runs on an EKS managed node group.

**Karpenter.**
The modules' Karpenter node pools set `expireAfter` to `Never` and leave `terminationGracePeriod` unset.
With those settings, Karpenter never removes a node while Materialize pods run on it, because they carry `karpenter.sh/do-not-disrupt`.
To replace such a node, for example after a change to its instance type,
cordon it and [move Materialize off the node](#move-materialize-off-the-node).
Karpenter then removes the empty node, because the modules consolidate nodes when they're empty.

To see what Karpenter is doing, list its node claims and the disruptions it can't make:

```bash
KUBECONTEXT=my-cluster

kubectl --context "$KUBECONTEXT" get nodeclaims
kubectl --context "$KUBECONTEXT" get events -A --field-selector reason=DisruptionBlocked
```

**Managed node groups.**
An update to a managed node group cordons and drains each old node.
If a PodDisruptionBudget keeps a pod on a node for 15 minutes, the update fails with `PodEvictionFailure`, unless it was started with `--force`.
To find the node group's updates, run the following commands:

```bash
CLUSTER=my-cluster
NODEGROUP=my-nodegroup
UPDATE_ID=00000000-0000-0000-0000-000000000000

aws eks list-updates --name "$CLUSTER" --nodegroup-name "$NODEGROUP"
aws eks describe-update --name "$CLUSTER" --nodegroup-name "$NODEGROUP" --update-id "$UPDATE_ID"
```

For more information,
see [Understand each phase of node updates](https://docs.aws.amazon.com/eks/latest/userguide/managed-node-update-behavior.html)
in the Amazon EKS documentation.
{{% /tab %}}
{{% tab "Azure (AKS)" %}}
The Terraform modules run Materialize and the other workloads on nodes from Node Auto Provisioning, which is Karpenter managed by AKS.
The cluster's `default` node pool is an AKS node pool.

**Node Auto Provisioning.**
The modules' node pools behave as they do with Karpenter on EKS.
Karpenter never removes a node while Materialize pods run on it.
To replace such a node, cordon it and [move Materialize off the node](#move-materialize-off-the-node).
The `kubectl get nodeclaims` and `DisruptionBlocked` commands on the EKS tab work here too.

**AKS node pools.**
An AKS node pool upgrade adds surge nodes, and then cordons and drains each old node.
It waits up to the pool's drain timeout, 30 minutes by default, for a pod that a PodDisruptionBudget holds on the node.
What happens next depends on the pool's undrainable node behavior:

- **`Schedule`:** the upgrade fails.
- **`Cordon`:** AKS leaves the blocked node cordoned, labels it `kubernetes.azure.com/upgrade-status=Quarantined`, and continues.
  A quarantined node stays until someone resolves it.

To check a node pool's upgrade state and settings, and to find quarantined nodes, run the following commands:

```bash
KUBECONTEXT=my-cluster
RESOURCE_GROUP=my-resource-group
CLUSTER=my-cluster
NODE_POOL=default

az aks nodepool show --resource-group "$RESOURCE_GROUP" --cluster-name "$CLUSTER" --name "$NODE_POOL" \
  --query "{state: provisioningState, upgrade: upgradeSettings}"
kubectl --context "$KUBECONTEXT" get nodes -l kubernetes.azure.com/upgrade-status=Quarantined
```

For more information, see [Upgrade options](https://learn.microsoft.com/en-us/azure/aks/upgrade-options) in the AKS documentation.
{{% /tab %}}
{{< /tabs >}}

## Escalate to the node pool's owner

The node pools often belong to a different team from the one that runs Materialize, such as a platform or IT team that manages the Kubernetes cluster.
If you support an install on someone else's behalf, the owner is usually that organization's own IT team.

Escalate in the following cases:

- A managed upgrade, an autoscaler, or a maintenance process that you don't control started the drain,
  and it needs pausing, rescheduling, or rolling back.
- Replacement nodes aren't joining, so the next generation's pods stay `Pending`.
  Quota, a node pool's maximum size, and zone capacity are fixed outside Kubernetes.
- A PodDisruptionBudget or an annotation on a workload that you don't own blocks the drain.
- `materialize-node-drain-stuck` fires on a node pool that you don't manage.
  The owner's upgrade drains the node after its wait runs out, whether or not Materialize has moved.

Don't escalate when a rollout fixes the drain.
Start the rollout yourself, and if the next generation can't start, see [Troubleshooting Materialize](../../troubleshooting-materialize/).

Give the owner the following information:

- The node's name, its taints, and when the drain started, which the alert's start time or the `NodeNotSchedulable` event shows.
- What is still on the node.
- The cloud provider's operation or update ID, if one is running.
- When the provider drains the node, if it says.
