# Node drains




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
| `node-drain-stuck` | warning | 24 hours | Draining nodes that run no `environmentd` or `clusterd` pods |

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

A node that runs no `environmentd` or `clusterd` pods has been cordoned or marked for removal for 24 hours.
Nothing new schedules on it, so the cluster runs short of the node's capacity.
Whatever removes the node next evicts what is still on it, such as a single-instance database or the monitoring stack.
[Find what blocks the drain](#find-what-blocks-the-drain).
If a GKE blue-green upgrade cordoned the node, see the GKE tab in [Check the node pool](#check-the-node-pool).

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

   <div class="book-tabs" >
   <input type="radio" class="toggle" name="tabs-0" id="tabs-0-0" checked="checked" /><label for="tabs-0-0">Materialize CRD v1</label><div class="book-tabs-content markdown-inner">
<p>Set <code>forceRollout</code> to a new UUID:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>KUBECONTEXT<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>NAMESPACE<span style="color:#f92672">=</span>materialize-environment
</span></span><span style="display:flex;"><span>INSTANCE<span style="color:#f92672">=</span>main
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>kubectl --context <span style="color:#e6db74">&#34;</span>$KUBECONTEXT<span style="color:#e6db74">&#34;</span> -n <span style="color:#e6db74">&#34;</span>$NAMESPACE<span style="color:#e6db74">&#34;</span> patch materialize <span style="color:#e6db74">&#34;</span>$INSTANCE<span style="color:#e6db74">&#34;</span> --type<span style="color:#f92672">=</span>merge <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  -p <span style="color:#e6db74">&#34;{\&#34;spec\&#34;: {\&#34;forceRollout\&#34;: \&#34;</span><span style="color:#66d9ef">$(</span>uuidgen<span style="color:#66d9ef">)</span><span style="color:#e6db74">\&#34;}}&#34;</span></span></span></code></pre></div></div>
   <input type="radio" class="toggle" name="tabs-0" id="tabs-0-1"  /><label for="tabs-0-1">Materialize CRD v1alpha1</label><div class="book-tabs-content markdown-inner">
<p>Set <code>requestRollout</code> and <code>forceRollout</code> to the same new UUID:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>KUBECONTEXT<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>NAMESPACE<span style="color:#f92672">=</span>materialize-environment
</span></span><span style="display:flex;"><span>INSTANCE<span style="color:#f92672">=</span>main
</span></span><span style="display:flex;"><span>UUID<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;</span><span style="color:#66d9ef">$(</span>uuidgen<span style="color:#66d9ef">)</span><span style="color:#e6db74">&#34;</span>
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>kubectl --context <span style="color:#e6db74">&#34;</span>$KUBECONTEXT<span style="color:#e6db74">&#34;</span> -n <span style="color:#e6db74">&#34;</span>$NAMESPACE<span style="color:#e6db74">&#34;</span> patch materialize <span style="color:#e6db74">&#34;</span>$INSTANCE<span style="color:#e6db74">&#34;</span> --type<span style="color:#f92672">=</span>merge <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  -p <span style="color:#e6db74">&#34;{\&#34;spec\&#34;: {\&#34;requestRollout\&#34;: \&#34;</span>$UUID<span style="color:#e6db74">\&#34;, \&#34;forceRollout\&#34;: \&#34;</span>$UUID<span style="color:#e6db74">\&#34;}}&#34;</span></span></span></code></pre></div></div>
   </div>

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

<div class="book-tabs" >
<input type="radio" class="toggle" name="tabs-1" id="tabs-1-0" checked="checked" /><label for="tabs-1-0">Google Cloud (GKE)</label><div class="book-tabs-content markdown-inner">
<p>GKE upgrades node pools automatically, for example to roll out a new node image.
You can delay an upgrade with maintenance windows and exclusions, but you can&rsquo;t turn upgrades off.</p>
<p>The Terraform modules put node pools on autoscaled blue-green upgrades, which run in the following order:</p>
<ol>
<li>GKE creates an empty replacement (green) pool, and cordons every node in the original (blue) pool.</li>
<li>GKE waits for the pool&rsquo;s <code>wait_for_drain_duration</code>, which is three days by default and seven at most.
The modules set it with <code>upgrade_wait_for_drain_duration</code>.</li>
<li>GKE drains the blue nodes in batches, and then deletes them.</li>
</ol>
<p>During the wait, the Materialize operator&rsquo;s node-upgrade rollout trigger rolls out each instance with pods on the blue nodes,
so the instance moves to the green pool.
The modules&rsquo; examples turn the trigger on with <code>enable_node_upgrade_rollout_trigger</code>.
For setting it up without the modules,
see <a href="https://materialize.com/docs/self-managed-deployments/deployment-guidelines/gke-node-pool-upgrades/" rel="external" class="external-link">GKE node pool upgrades</a>
in the Materialize documentation.</p>
<p>While the trigger works, <code>materialize-node-draining</code> resolves after each instance&rsquo;s rollout.
If <code>materialize-node-drain-slow</code> fires during an upgrade, the trigger&rsquo;s rollout didn&rsquo;t move the instance.
<a href="#check-the-instances-rollout">Check the instance&rsquo;s rollout</a>, and the operator&rsquo;s logs.</p>
<p>Nothing moves other pods off the blue nodes during the wait.
They stay until GKE drains the node, so <code>node-drain-stuck</code> fires for those nodes a day into the wait.
The wait serves the operator&rsquo;s rollouts, so a node pool that runs no <code>environmentd</code> or <code>clusterd</code> pods gains nothing from it.
To have GKE drain such a pool sooner in its next upgrade, its owner can shorten the pool&rsquo;s wait,
with <code>upgrade_wait_for_drain_duration</code> in the Terraform modules.</p>
<p>To find a running upgrade and its phase, list the cluster&rsquo;s running node upgrades:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>PROJECT_ID<span style="color:#f92672">=</span>my-project
</span></span><span style="display:flex;"><span>LOCATION<span style="color:#f92672">=</span>us-east1
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>gcloud container operations list --project <span style="color:#e6db74">&#34;</span>$PROJECT_ID<span style="color:#e6db74">&#34;</span> --location <span style="color:#e6db74">&#34;</span>$LOCATION<span style="color:#e6db74">&#34;</span> <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --filter<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;operationType=UPGRADE_NODES AND status=RUNNING&#34;</span> <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --format<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;table(name,targetLink.basename(),startTime,detail)&#34;</span></span></span></code></pre></div><p>The <code>detail</code> column names the node pool and the phase, such as <code>Phase: WAITING_TO_DRAIN_BLUE_POOL</code>.
GKE starts draining when the wait runs out, which is the operation&rsquo;s start time plus the pool&rsquo;s <code>wait_for_drain_duration</code>.
The node pool&rsquo;s owner can roll back an upgrade that&rsquo;s still running with <code>gcloud container node-pools rollback</code>.</p>
</div>
<input type="radio" class="toggle" name="tabs-1" id="tabs-1-1"  /><label for="tabs-1-1">AWS (EKS)</label><div class="book-tabs-content markdown-inner">
<p>The Terraform modules run Materialize and the other workloads on nodes from Karpenter, in separate Karpenter node pools.
Karpenter itself, with a few cluster add-ons, runs on an EKS managed node group.</p>
<p><strong>Karpenter.</strong>
The modules&rsquo; Karpenter node pools set <code>expireAfter</code> to <code>Never</code> and leave <code>terminationGracePeriod</code> unset.
With those settings, Karpenter never removes a node while Materialize pods run on it, because they carry <code>karpenter.sh/do-not-disrupt</code>.
To replace such a node, for example after a change to its instance type,
cordon it and <a href="#move-materialize-off-the-node">move Materialize off the node</a>.
Karpenter then removes the empty node, because the modules consolidate nodes when they&rsquo;re empty.</p>
<p>To see what Karpenter is doing, list its node claims and the disruptions it can&rsquo;t make:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>KUBECONTEXT<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>kubectl --context <span style="color:#e6db74">&#34;</span>$KUBECONTEXT<span style="color:#e6db74">&#34;</span> get nodeclaims
</span></span><span style="display:flex;"><span>kubectl --context <span style="color:#e6db74">&#34;</span>$KUBECONTEXT<span style="color:#e6db74">&#34;</span> get events -A --field-selector reason<span style="color:#f92672">=</span>DisruptionBlocked</span></span></code></pre></div><p><strong>Managed node groups.</strong>
An update to a managed node group cordons and drains each old node.
If a PodDisruptionBudget keeps a pod on a node for 15 minutes, the update fails with <code>PodEvictionFailure</code>, unless it was started with <code>--force</code>.
To find the node group&rsquo;s updates, run the following commands:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>CLUSTER<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>NODEGROUP<span style="color:#f92672">=</span>my-nodegroup
</span></span><span style="display:flex;"><span>UPDATE_ID<span style="color:#f92672">=</span>00000000-0000-0000-0000-000000000000
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>aws eks list-updates --name <span style="color:#e6db74">&#34;</span>$CLUSTER<span style="color:#e6db74">&#34;</span> --nodegroup-name <span style="color:#e6db74">&#34;</span>$NODEGROUP<span style="color:#e6db74">&#34;</span>
</span></span><span style="display:flex;"><span>aws eks describe-update --name <span style="color:#e6db74">&#34;</span>$CLUSTER<span style="color:#e6db74">&#34;</span> --nodegroup-name <span style="color:#e6db74">&#34;</span>$NODEGROUP<span style="color:#e6db74">&#34;</span> --update-id <span style="color:#e6db74">&#34;</span>$UPDATE_ID<span style="color:#e6db74">&#34;</span></span></span></code></pre></div><p>For more information,
see <a href="https://docs.aws.amazon.com/eks/latest/userguide/managed-node-update-behavior.html" rel="external" class="external-link">Understand each phase of node updates</a>
in the Amazon EKS documentation.</p>
</div>
<input type="radio" class="toggle" name="tabs-1" id="tabs-1-2"  /><label for="tabs-1-2">Azure (AKS)</label><div class="book-tabs-content markdown-inner">
<p>The Terraform modules run Materialize and the other workloads on nodes from Node Auto Provisioning, which is Karpenter managed by AKS.
The cluster&rsquo;s <code>default</code> node pool is an AKS node pool.</p>
<p><strong>Node Auto Provisioning.</strong>
The modules&rsquo; node pools behave as they do with Karpenter on EKS.
Karpenter never removes a node while Materialize pods run on it.
To replace such a node, cordon it and <a href="#move-materialize-off-the-node">move Materialize off the node</a>.
The <code>kubectl get nodeclaims</code> and <code>DisruptionBlocked</code> commands on the EKS tab work here too.</p>
<p><strong>AKS node pools.</strong>
An AKS node pool upgrade adds surge nodes, and then cordons and drains each old node.
It waits up to the pool&rsquo;s drain timeout, 30 minutes by default, for a pod that a PodDisruptionBudget holds on the node.
What happens next depends on the pool&rsquo;s undrainable node behavior:</p>
<ul>
<li><strong><code>Schedule</code>:</strong> the upgrade fails.</li>
<li><strong><code>Cordon</code>:</strong> AKS leaves the blocked node cordoned, labels it <code>kubernetes.azure.com/upgrade-status=Quarantined</code>, and continues.
A quarantined node stays until someone resolves it.</li>
</ul>
<p>To check a node pool&rsquo;s upgrade state and settings, and to find quarantined nodes, run the following commands:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>KUBECONTEXT<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>RESOURCE_GROUP<span style="color:#f92672">=</span>my-resource-group
</span></span><span style="display:flex;"><span>CLUSTER<span style="color:#f92672">=</span>my-cluster
</span></span><span style="display:flex;"><span>NODE_POOL<span style="color:#f92672">=</span>default
</span></span><span style="display:flex;"><span>
</span></span><span style="display:flex;"><span>az aks nodepool show --resource-group <span style="color:#e6db74">&#34;</span>$RESOURCE_GROUP<span style="color:#e6db74">&#34;</span> --cluster-name <span style="color:#e6db74">&#34;</span>$CLUSTER<span style="color:#e6db74">&#34;</span> --name <span style="color:#e6db74">&#34;</span>$NODE_POOL<span style="color:#e6db74">&#34;</span> <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --query <span style="color:#e6db74">&#34;{state: provisioningState, upgrade: upgradeSettings}&#34;</span>
</span></span><span style="display:flex;"><span>kubectl --context <span style="color:#e6db74">&#34;</span>$KUBECONTEXT<span style="color:#e6db74">&#34;</span> get nodes -l kubernetes.azure.com/upgrade-status<span style="color:#f92672">=</span>Quarantined</span></span></code></pre></div><p>For more information, see <a href="https://learn.microsoft.com/en-us/azure/aks/upgrade-options" rel="external" class="external-link">Upgrade options</a> in the AKS documentation.</p>
</div>
</div>

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

