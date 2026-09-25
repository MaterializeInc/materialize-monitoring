# Upgrading materialize-monitoring




# Upgrading materialize-monitoring

## StatefulSet fields that cannot be patched

Most of a StatefulSet is mutable, but `volumeClaimTemplates` and `selector` are not.
A release that changes either — a component gaining, losing, or resizing its volume — cannot be applied by `helm upgrade`, which fails partway with:

```text
cannot patch "thanos-compactor" with kind StatefulSet: StatefulSet.apps
"thanos-compactor" is invalid: spec: Forbidden: updates to statefulset spec for
fields other than 'replicas', 'ordinals', 'template', 'updateStrategy',
'persistentVolumeClaimRetentionPolicy' and 'minReadySeconds' are forbidden
```

Nothing in the chart can smooth this over — it is a Kubernetes API constraint, not a Helm one — so the StatefulSet has to be deleted and recreated by the upgrade.

**The two Thanos storage changes in chart v0.16.0 require this**, and only for installs upgrading across it:

| Component | Change | What the upgrade needs |
|---|---|---|
| Compactor | gains a PVC (its scratch outgrew node ephemeral storage) | delete the StatefulSet; the upgrade recreates it with the claim |
| Receive | drops its PVC for `emptyDir` (durability is RF 3, and a volume cannot cross zones) | delete the StatefulSet, then delete the orphaned PVCs |

```bash
# Compactor: local state is scratch, so recreating the pod costs nothing but a
# restarted compaction.
kubectl -n monitoring delete statefulset thanos-compactor
helm upgrade mzmon ... # recreates it with the volumeClaimTemplate

# Receive: the same, plus cleaning up volumes nothing will reclaim. Do this one
# pod at a time if you want to preserve write quorum through the change.
kubectl -n monitoring delete statefulset thanos-receive
helm upgrade mzmon ...
kubectl -n monitoring delete pvc -l app.kubernetes.io/component=receive
```

> [!WARNING]
>  **Deleting a StatefulSet does not delete its PVCs.** The default `persistentVolumeClaimRetentionPolicy` is `Retain`, so volumes created from `volumeClaimTemplates` outlive it. That cuts both ways, and both directions matter here:
>
>  - Recreating a StatefulSet **rebinds the existing PVCs by name** (`data-<sts>-0`), which is what makes the Compactor migration safe to repeat.
>  - Moving *away* from a volume leaves PVCs with nothing to reclaim them, still provisioned and still billed. Receive's must be deleted explicitly — which is safe, because that data was never the durable copy.
>
>  **`--cascade=orphan` is the wrong tool for a volume change.** It keeps the pods running while the StatefulSet is replaced, which is useful when a restart is what you are avoiding — but an orphaned pod keeps the volumes it started with, so the new claim only takes effect once the pod is replaced anyway.

Losing Receive's local data during this is not a data-loss event: at most 2h of it had not yet reached object storage, every write exists on three replicas, and the Store Gateway serves everything older.
The Compactor's scratch is not authoritative at all — the bucket is, and compaction is idempotent.

If a Compactor later gets wedged in an unavailable zone by the volume it just gained, that is recoverable by hand; see [The Thanos Compactor is stuck in a zone](../o11y-troubleshooting/#the-thanos-compactor-is-stuck-in-a-zone) for the ordering, which matters because two Compactors running at once is the one thing that corrupts data.

## Alertmanager: one replica to two, renamed, and `alerting`

The release that made Alertmanager highly available ([DEP-226](https://linear.app/materializeinc/issue/DEP-226)) also pinned its resource
names to `alertmanager`, which were previously derived from the release name as `<release>-alertmanager`.
`helm upgrade` applies it without error, and it carries one step to take by hand if the existing silences matter.

| Change | What to expect |
|---|---|
| Resources renamed to `alertmanager` | Helm deletes the old StatefulSet and creates a new one. A StatefulSet's volumes are named after it, so the new replicas start on new, empty volumes and the old ones are left orphaned. **Silences do not carry across** unless exported and imported, below. The notification log does not either, so each alert already firing notifies once more. |
| Replicas go from one to two | Both new volumes are 4Gi, in different zones under the hard zone spread. A cluster whose nodes carry no zone label needs `no-zone-spread` or `min_zones = 0` first, or a replica stays `Pending`. |
| Both rulers retarget `alertmanager-headless` | The Loki ruler's address lives in Loki's shared configuration, so every Loki pod rolls once. |
| Both rulers stamp `cluster` on alerts | Read from a new `ruler-env` ConfigMap in each ruler's namespace; both rulers roll once. Set `pipeline.env.CLUSTER_NAME` (Terraform: `cluster_name`) first if it is still `default`. |
| Routing moves to `alerting` | The subchart's `alertmanager.config` is no longer read. A values file still carrying receivers or routes there fails the render, with the keys to move them to. See [Alert Channels](../../alerting/channels/). |

To keep silences across the upgrade, export them from the old replica first, then import them into the new one.
The old name is `<release>-alertmanager`, or `<release>` alone when the release name already contains `alertmanager`.

```bash
kubectl --namespace monitoring exec mzmon-alertmanager-0 -c alertmanager -- \
  amtool silence query --output=json --alertmanager.url=http://127.0.0.1:9093 > silences.json
```

```bash
kubectl --namespace monitoring exec -i alertmanager-0 -c alertmanager -- \
  amtool silence import --alertmanager.url=http://127.0.0.1:9093 < silences.json
```

The imported silences get new IDs and gossip to the other replica.
Once they are in place, the orphaned volume can go.
It is `storage-<old name>-0`, plus `-1` on an install that ran two replicas under the old name.

```bash
kubectl --namespace monitoring delete pvc storage-mzmon-alertmanager-0
```

## Ingester rollouts: duration and deploy timeouts

Any change to the ingester pod spec — image, resources, or scaling the replica count — rolls the ingester StatefulSet.
That roll is **ordered and readiness-gated**: pods cycle **one at a time**, and the next is not touched until the previous re-joins the ring and reports Ready.
This is deliberate — with `replication_factor: 3`, one-at-a-time keeps at most one of three replicas down, preserving write quorum throughout the roll.

**Budget roughly one minute per ingester**, plus headroom for any new nodes.
Each pod pays: graceful ring-leave + best-effort flush (up to `terminationGracePeriodSeconds`, ~60s) → start → memberlist join → ring `ACTIVE` → readiness stabilization.
A 6-ingester fleet therefore takes ~5 minutes to roll, and it is **not** bounded by how fast new nodes appear.

That overruns tools that wait on the rollout with a short default:

| Tool | Default wait | Fix |
|---|---|---|
| `helm upgrade --wait` | 5m (`--timeout`) | `--timeout 15m` |
| Flux `HelmRelease` | 5m | raise `spec.timeout` |
| Pulumi `helm.v4.Chart` | resource await | extend `customTimeouts` |
| ArgoCD | async (no wait) | tolerant — shows `Progressing` until healthy |

A `--wait` timeout **on a rollout you deliberately triggered** means "still rolling," not "failed" — the rollout completes correctly and the client just stopped watching.

That is the narrow exception, not the rule. On any other timeout, assume something is broken and inspect events before touching the timeout: see [Troubleshooting](../o11y-troubleshooting/#start-here-a-timeout-is-not-a-duration-problem).

> [!WARNING]
>   Do not speed the roll by allowing more than one ingester down at a time (`updateStrategy.rollingUpdate.maxUnavailable > 1`, via the alpha `MaxUnavailableStatefulSet` gate).
>   With RF 3 that can drop you to a single healthy replica and break write quorum mid-roll.
>   `zoneAwareReplication` (roll a whole zone at once) is the only quorum-safe burst, at the cost of cross-AZ replication traffic — see the [logging architecture](../../logs-and-events/architecture/#loki-ingester).

