---
title: "Helm"
weight: 40
---

# Installing via Helm

If you are not using the terraform module to install `materialize-monitoring`, you can use the provided Helm charts to install the monitoring stack in your Kubernetes cluster.

> [!INFO]
>  The [Terraform modules](../terraform/) are the recommended path: they create the buckets and workload-identity bindings, derive every backend key from one `object_storage` object, and roll Alloy when its config changes.
>
>  The charts are the full-fidelity surface — everything Terraform does is a layer over them — so reach for Helm when you need a setting the modules do not model, or when Terraform is not how you deploy.
>
>  What you take on by installing directly is the `[consumer]` half of the [shared responsibility model](../../operating/production-best-practices/#shared-responsibility-model): cloud resources, secrets, version pinning, and the Alloy rollout below.

## Dependency: Installing CRDs

`materialize-monitoring` relies on several Custom Resource Definitions (CRDs) to function properly.

A second `materialize-monitoring-crds` Helm chart is provided to install these CRDs separately from the main `materialize-monitoring` chart, which is recommended to manage the lifecycle of these CRDs separately from the main chart.

Install it first, then install `materialize-monitoring` with `--skip-crds`:

```bash
helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds --namespace monitoring
```

The bundled Grafana Operator ships its own copy of the Grafana CRDs and offers no way to opt out of them, so without `--skip-crds` a fresh install of the main chart would create them behind the CRDs chart's back.
See [Dependencies](../dependencies) for the full list of CRDs.

## Dependency: Setting Up Storage

You will likely need to set up storage for your metrics and logs before you can start using `materialize-monitoring`.
The specific steps for setting up storage will depend on your environment and the storage solution you choose.

If you are using both external metric storage and external log storage,
you will not need an object storage bucket.

Two buckets (or containers) are the norm, one per backend — Loki and Thanos want different lifecycle rules, and per-bucket IAM keeps each backend out of the other's data.

### Cloud Managed Kubernetes Service (AWS EKS, Google Cloud GKE, Azure AKS, etc.)

Grant access through **workload identity** rather than static keys. The binding is per-cloud and documented step by step, including the trust-policy subjects that must match the chart's rendered ServiceAccount names:

- [Logs & Events > Storing](../../logs-and-events/storing/#granting-object-storage-access-workload-identity) — IRSA, GKE Workload Identity, and Entra Workload ID, in tabs
- [Metrics > Storing](../../metrics/storing/#authentication) — the same three for Thanos

Then name the backend in your values. **This is the step that most often goes wrong**: the chart's defaults are S3-shaped, so any other backend has to be named in three load-bearing places and none of them fails softly — the client is chosen by name and validated against a config that was never populated, so the component crash-loops. See [Selecting the backend](../../logs-and-events/storing/#selecting-the-backend).

The example profiles do all of it correctly and are the shortest path:

| Profile | Backend |
|---|---|
| `profiles/aws-example.values.yaml` | S3 + IRSA |
| `profiles/gcp-example.values.yaml` | GCS + GKE Workload Identity |
| `profiles/azure-example.values.yaml` | Blob + Entra Workload ID |
| `profiles/aws-amp-example.values.yaml` | S3 for logs, Amazon Managed Prometheus for metrics |

> [!TIP]
>  Read the Azure profile's header before copying it. Entra Workload ID needs a pod label as well as the ServiceAccount annotation, and the two subcharts take that label under different keys — `loki.loki.podLabels` for Loki, `thanos.global.commonLabels` for Thanos, which has no `podLabels` of its own.

### On-Premises Kubernetes Cluster with Access to Cloud Object Storage (S3, GCS, Azure Blob Storage, etc.)

Without a cloud identity provider there is no workload identity to bind, so credentials become **static keys supplied as a Secret** — the documented escape hatch rather than the recommended path. Both backends read them by reference:

- Loki takes them under `loki.loki.storage.object_store.<backend>`; prefer supplying the values through `loki.<component>.extraEnvFrom` (a `secretRef`) over inlining them, since inline values render into a ConfigMap in plaintext.
- Thanos takes them inside `thanos.global.objstore.config`, which becomes a Secret when `createSecret: true`.

Any S3-compatible endpoint works — set `object_storage.endpoint` in Loki's config and `endpoint` in Thanos's. The chart's own tier-2 E2E runs against [rustfs](https://github.com/rustfs/rustfs) this way, so a self-hosted MinIO, Ceph, or rustfs is a supported shape rather than an untested one.

> [!WARNING]
>  The chart's validators grade credential handling and will tell you when a cloud backend has neither an identity annotation nor inline credentials — at which point the component falls back to ambient node credentials, which usually means it silently works in one cluster and fails in another. Read the render output rather than assuming silence.

## Customizing your Helm Installation

The `materialize-monitoring` Helm chart is designed to be highly customizable, so you can easily integrate with your existing observability infrastructure.

Typically, you would want to create a values.yaml file that has your
specific configurations.
You may start fresh or you can copy a preset from
the `charts/materialize-monitoring/profiles/` directory in this repository.

> [!WARNING]
>  Be aware that when merging examples together that you do not have
>  multiple of the same key on the same level since they do not automatically merge.
>  YAML is whitespace sensitive.

You must specify `-f YOUR_VALUES.yaml` in your `helm install`/`helm upgrade` command to apply these customizations.
These are automatically overlaid on top of the default values of the
chart, so you only need to specify the values that are different from the default.

> [!INFO]
>  This documentation may refer to values in dotted notation (e.g., `component.subcomponent.key=value`)
>  which corresponds roughly to this YAML structure:
>
>  ```yaml
>  component:
>    subcomponent:
>      key: value
>  ```

### Choosing which components run, via tags

Which subcharts install is driven by `tags`, and the semantics are **OR**: a component runs if *any* tag covering it is true. There is no need to disable a group before enabling one member.

| Tag | Covers |
|---|---|
| `tags.default` | everything below. `true` by default |
| `tags.bundled-backends` | Loki, Thanos, Alertmanager |
| `tags.managed-grafana` | Grafana and grafana-operator |
| `tags.pipeline` | both Alloy roles (agent + gateway) |
| `tags.cluster-metrics` | kube-state-metrics, metrics-server |

Per-chart overrides — `tags.loki`, `tags.thanos`, `tags.alloy-agent`, `tags.alloy-gateway`, `tags.grafana-standalone`, `tags.grafana-operator`, `tags.alertmanager`, `tags.kube-state-metrics`, `tags.metrics-server` — are OR'd on top.

So a Loki-only install is `tags.default: false` plus `tags.loki: true`, and adding Thanos to a default install needs nothing at all.

### Disabling a Component

Each subchart also has an `enabled` circuit breaker that takes **precedence over every tag**. Setting `loki.enabled: false` turns Loki off even with `tags.default: true`, which is the reliable way to subtract one component from an otherwise default install.

### Sizing

The chart's defaults target a **medium** deployment, so the sizing profiles are deltas in either direction rather than a full configuration:

Each backend has its own pair, because they size off different axes — Loki off log throughput, Thanos off active series and object count.
Pick one from each row you deploy; they compose.

| Profile | Use |
|---|---|
| `profiles/loki-small.values.yaml` | dev, or a constrained node pool |
| `profiles/loki-large.values.yaml` | high-volume logging |
| `profiles/thanos-small.values.yaml` | dev, or a constrained node pool |
| `profiles/thanos-large.values.yaml` | high-cardinality metrics; also enables the Thanos query-frontend and repoints the datasource at it |
| *(none)* | medium — the chart defaults, for either backend |
| `profiles/loki-test.values.yaml` | CI only: SingleBinary Loki on local filesystem, no object storage |
| `profiles/kind.values.yaml` | CI only: shrinks **every** workload to fit one `kind` node. Sizing only — see below |

See Production Best Practices for the envelope each tier assumes: [logging throughput](../../operating/production-best-practices/#sizing-the-logging-backend) for Loki, [active series and collections](../../operating/production-best-practices/#sizing-the-metrics-backend) for Thanos, including how to pick a tier from cluster inventory before you have any metrics to measure.

#### The `kind` profile is sizing only, and composes last {#kind-profile}

The chart defaults request roughly 33Gi of memory and 9 CPU in total, which no standard CI runner will schedule — pods sit Pending and it reads like a chart bug rather than a capacity one.
`profiles/kind.values.yaml` cuts that to under 4Gi and about 1.6 CPU.

It sets **container requests and limits, volume sizes, and cache allocations, and nothing else**: no feature toggles, no `tags`, and deliberately **no replica counts**.
Compose it last so its sizes win:

```bash
helm install mzmon . -f profiles/loki-test.values.yaml -f profiles/kind-tier1.values.yaml -f profiles/kind.values.yaml
```

> [!INFO]
>   **Why no replica counts.** `loki-test` *disables* Loki's distributed components by setting `replicas: 0` on them, so a replica count in a sizing overlay would switch them back on — and Loki then refuses to render at all, with `more than zero replicas configured for both the monolithic and distributed targets`. Separately, Loki's ingesters and Thanos Receive both sit at the replication-factor floor of 3, and the ring behaviour they provide is much of what an E2E run exists to prove. So this profile shrinks each pod and leaves the number of them to whoever owns the topology: three tiny pods, not one big one.
>
>   Two sizing traps it has to work around, both of which fail as an OOM rather than as a small install if you only lower the request: Thanos Store Gateway holds a 2GB chunk pool and a 250MB index cache by default, and memcached derives its pod request from `allocatedMemory`. Both are shrunk explicitly.

### Other shape overlays

| Profile | Use |
|---|---|
| `profiles/existing-grafana.values.yaml` | point at a Grafana you already run instead of installing one |
| `profiles/grafana-postgres.values.yaml` | Grafana state in Postgres rather than SQLite — the production shape |
| `profiles/grafana-pvc.values.yaml` | Grafana state on a PersistentVolume — durable, but still one replica. For a Helm-only install with no database |
| `profiles/grafana-ingress.values.yaml` | make Grafana reachable over an Ingress, internal by default and TLS-terminated. Pair it with one of the two above |
| `profiles/scheduling.values.yaml` | node selector, tolerations, and priority-class names, fanned out to every subchart. Edit the four anchors at the top; see below |
| `profiles/storage-class.values.yaml` | one StorageClass, aimed at the three workloads that claim a volume. Edit the single anchor at the top |
| `profiles/no-zone-spread.values.yaml` | for clusters with **no zone labels, or one zone**. Without it Thanos Receive and Loki's ingesters stay Pending; see [Production Best Practices](../../operating/production-best-practices/#thanos-few-zones) |
| `profiles/split-namespace.values.yaml` | one namespace per subchart. Changes every workload-identity subject; see [Namespace layout](../../operating/production-best-practices/#namespace-layout) |
| `profiles/otel-metrics-fanout.values.yaml` | additional metric destinations (GCM, Datadog) with per-destination importance tiers |
| `profiles/otlp-metrics-honeycomb.values.yaml` | a generic OTLP metrics backend |

### Disabling a Component

If you want further control of the managed components, you can selectively disable components in the `materialize-monitoring` Helm chart by setting the `enabled` field for that component to `false` in your `YOUR_VALUES.yaml` file or via `--set` in your `helm install`/`helm upgrade` command.

#### Scheduling: where each subchart wants it {#scheduling-profile}

The subcharts disagree about where scheduling settings go, which is the whole reason this is a profile rather than three `--set` flags.
`thanos.global` covers everything Thanos runs; `loki.defaults` covers everything Loki runs *except* three components that render from their own templates; Alloy puts it under `controller`; the rest take it at the top level.
A selector written to the wrong one of those renders perfectly and does nothing.

**Edit the four anchors at the top of the file and nothing else** — every fan-out site aliases them.
The two class names match the chart's defaults, so they are a no-op until you change them; aliasing both the created classes and all eleven references off one anchor is what makes renaming them a two-line edit that cannot half-apply.

> [!WARNING]
>   **A nodeSelector and a toleration are not two spellings of the same idea, and per-node collectors need them treated differently.**
>
>   A **nodeSelector narrows** where a pod may run. On a DaemonSet whose job is to observe every node, that is a silent blind spot rather than a placement preference — an `alloy-agent` constrained to a workload pool simply stops collecting logs from everywhere else, and no dashboard shows a hole where the nodes should be. **Tolerations widen** placement, which is exactly what a DaemonSet wants so it can reach tainted, spot, and system pools.
>
>   So `alloy-agent` receives tolerations and never a selector. `node-exporter` receives neither: its chart default already tolerates *every* `NoSchedule` taint, and a toleration list would **replace** that rather than extend it, because Helm merges maps but overwrites lists.

`metrics-server` takes a selector and tolerations but deliberately no priority class — it stays on the upstream `system-cluster-critical`, because it backs the metrics API that HPAs and the Materialize Console read, which makes it cluster plumbing rather than monitoring.

**Terraform users need neither file.** The module's `node_selector`, `tolerations`, and `storage_class` variables carry the same maps and take the values directly. The two copies are kept honest by `make terraform-render`, which plans each example, renders the chart against the values it composed, and asserts that every pod template carries the selector and every `volumeClaimTemplate` carries the class — so a subchart that renames a key fails in this repository rather than in an apply.


## Initial Installation

CRDs first, then the chart with `--skip-crds`. Both releases go in the same namespace; the CRDs are cluster-scoped, so that namespace only holds the release metadata.

```bash
helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds \
  --namespace monitoring --create-namespace \
  --version X.Y.Z --wait
```

```bash
helm install mzmon oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring \
  --namespace monitoring --skip-crds \
  --version X.Y.Z \
  -f my-values.yaml \
  --timeout 15m
```

Pin `--version` on both. The two charts have deliberately separate lifecycles, so they do not share a version — see [Compatibility](../../reference/compatibility/) for which pairs are tested together.

A few flags earn their place:

- **`--timeout 15m`.** Helm's default is 5 minutes, and a first install brings up Loki, Thanos, Grafana, Alertmanager, and both Alloy roles together. If it still times out, **do not raise it again** — a timeout almost always means a pod is broken rather than slow. See [Troubleshooting](../../operating/o11y-troubleshooting/#start-here-a-timeout-is-not-a-duration-problem).
- **`--wait` on the CRDs release only.** The main chart runs pre-install validation Jobs; if you add `--wait` there, add `--wait-for-jobs` too, or their verdict is never observed and a bad config rolls anyway.
- **Not `--atomic`** on a first install. A rollback destroys the evidence of which component failed, and this stack has enough moving parts that the diagnostic is usually worth more than the cleanup.

### Verify it came up

The chart's validators print to `NOTES.txt`, so read the install output rather than assuming silence means success — warnings there are the earliest signal that a backend key or a quorum setting is wrong.

```bash
kubectl --namespace monitoring get pods
kubectl --namespace monitoring get grafana,grafanadatasource,grafanamanifest
```

Grafana is `ClusterIP`, so reach it with a port-forward:

```bash
kubectl --namespace monitoring port-forward svc/grafana 3000:80
```

The admin credentials are in the `grafana` Secret (keys `admin-user` and `admin-password`).

> [!INFO]
>  The chart generates that password and **reuses it across `helm upgrade`**, because the Grafana chart looks the existing Secret up before generating.
>
>  That lookup returns nothing during `helm template` and `--dry-run`, so any render-only pipeline — most [GitOps](../gitops/) setups — regenerates the password on **every sync**. Supply your own Secret and set `grafana.grafana.admin.existingSecret` there. The Terraform module does exactly this, for exactly this reason.

## After changing values

**Restart Alloy after any pipeline or filter change.** This is the one place the chart cannot own its own rollout, and it is the reverse of what a chart normally guarantees: a `helm upgrade` that reports success can leave both Alloy roles serving the *previous* configuration indefinitely.

```bash
kubectl --namespace monitoring rollout restart deployment/alloy-gateway daemonset/alloy-agent
```

The mechanism, and why a reload cannot substitute for a restart, is in [Production Best Practices](../../operating/production-best-practices/#collection-alloy). The short version: the pipeline's `-env` ConfigMaps are consumed with `envFrom`, and environment variables are fixed at container start, so neither Alloy's `/-/reload` nor a config-reloader sidecar can pick up a filter change.

## Going to production

[Production Best Practices](../../operating/production-best-practices/) is the deployment checklist, organized by backend and tagged with who owns each item — the chart, you as the chart consumer, or you as the operator.

On the Helm path every `[consumer]` item is yours, including the buckets and workload-identity bindings that the [Terraform modules](../terraform/) would otherwise create.

## Upgrading and uninstalling

- [Upgrading](../../operating/upgrading/) — ingester rollouts are ordered and readiness-gated, so budget roughly a minute per ingester and raise your client's timeout accordingly.
- [Uninstalling](../../operating/uninstalling/) — **read this before tearing down.** Deleting the release without first removing the Grafana custom resources deadlocks on finalizers that only grafana-operator can remove, and the namespace never finishes terminating.

## Reference

- [Chart values](../../reference/helm/materialize-monitoring-values/) — every value, generated from `values.yaml`
- [CRDs](../../reference/crds/) — what the CRDs chart installs
- [Compatibility](../../reference/compatibility/) — chart, CRDs chart, and Terraform module versions
