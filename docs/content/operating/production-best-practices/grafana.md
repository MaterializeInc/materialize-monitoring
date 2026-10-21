---
title: "Grafana"
weight: 7
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Grafana

This page explains what separates the bundled Grafana's defaults from a production Grafana.
For the architecture these settings configure, see [Grafana architecture](../../../dashboards/grafana/architecture/).
What to do about them is in the [production checklist](../checklist/).

Grafana is the one component here that isn't a data store, and that changes what "production" means for it.
Nothing observable is lost when Grafana breaks.
The metrics are in Thanos, the logs are in Loki, and grafana-operator re-pushes the dashboards this chart installs every `resyncPeriod`.
What *is* at risk is everything a human created through the UI, plus the fact that Grafana is where an incident starts.

The chart defaults are the safe shape, not the production shape.
Three things separate them, and they compound: state, reachability, and authentication.

| | Default | Production |
|---|---|---|
| State | SQLite on `emptyDir`, lost on every restart | External PostgreSQL |
| Reachability | `ClusterIP`; `kubectl port-forward` only | Ingress, TLS-terminated, internal by default |
| Authentication | The generated admin password | An identity provider, with group-mapped roles |
| Replicas | 1 | 2+ behind an HPA — only meaningful once state is external |

**Reachability and persistence are one decision, not two.**
Exposing Grafana without a durable backend turns a bundled extra nobody depended on into the primary interface to the stack.
That interface silently discards every dashboard, annotation, and API token its users create.
The chart warns at exactly that combination rather than on every install.

## State {#state}

The following table compares the three backing stores the chart supports:

| Backing store | Set with | Replicas | Suitable for |
|---|---|---|---|
| SQLite on `emptyDir` (**default**) | — | 1 | demos and `kind`; state is lost on every pod restart |
| SQLite on a PersistentVolume | `grafana-pvc` profile | 1 | a single small instance with no database available |
| External PostgreSQL | `grafana-postgres` profile | 2+ | production |

SQLite tolerates exactly one writer, so both SQLite options pin you to a single replica.
More than one is a correctness bug rather than availability, and the chart refuses to render it.
A `ReadWriteOnce` volume also forces `deploymentStrategy.type: Recreate`, because a rolling update deadlocks.
The replacement pod waits for a volume the outgoing pod hasn't released, and the outgoing pod isn't terminated until the replacement is Ready.
External PostgreSQL is the only option that lifts both constraints.
For the full wiring, including why IAM database authentication doesn't remove the need for a password secret, see [State
and persistence](../../../dashboards/grafana/architecture/#state-and-persistence).

## Configuration as code {#configuration}

grafana-operator manages dashboards and data sources as code, as `GrafanaManifest` and `GrafanaDatasource` resources
reconciled in one direction with Kubernetes as the source of truth.
The operator re-pushes anything it owns on drift; anything it doesn't own is yours to keep.
That makes the running Grafana a **cache, not a source**: a dashboard edited in the UI is overwritten at the next
resync, so change it in `packages/dashboards/` instead.

`grafana.ini` is a **verbatim passthrough**.
Any section Grafana understands, such as auth, SMTP, feature toggles, unified alerting, or user provisioning, is
reachable from values and deep-merges over the chart's own keys.

`connections.grafana.operator.spec` is the **break-glass** for `mode: operator`, where grafana-operator owns the server
lifecycle and none of this page applies.
Prefer `mode: bundled`.

## Plugins {#plugins}

`grafana.plugins` downloads from grafana.com at every pod start.
That's a startup dependency on a third-party service, and an unpinned entry lets a plugin change underneath a pinned
Grafana, so pin every entry as `name@version` or bake plugins into a derived image.
A validator warns on an unpinned entry.

**A hardened Grafana can't install plugins at start.**
Hardened images ship no shell and no package manager, so `grafana.plugins` silently gets you a Grafana without those plugins rather than an error.
Bake them into a derived image instead.

**Bundled data source plugins stay at the image's versions** (`grafana.ini.plugins.preinstall_auto_update: false`).
Grafana otherwise updates the Prometheus and Loki plugins it bundles to the newest release on grafana.com at every
start, so a pinned image wouldn't pin them.
On the chart's read-only root filesystem, the update also fails halfway and leaves the data source unloaded, and a
validator errors when it's turned back on.
Updating a bundled data source plugin from Grafana's plugin catalog takes the same path, so it leaves the data source unloaded until the pod restarts.

Grafana's built-in preinstall list also carries the Drilldown apps, which the image doesn't bundle, so they download at
every start regardless of `grafana.plugins`.
Where Grafana can't reach grafana.com, set `grafana.ini.plugins.preinstall_disabled: true` to remove that startup dependency, and the apps with it.

## See also

- [Grafana architecture](../../../dashboards/grafana/architecture/): connection modes, namespaces, and the resource map.
- [State and persistence](../../../dashboards/grafana/architecture/#state-and-persistence): the PostgreSQL wiring in depth.
- [Authentication](../../../dashboards/grafana/auth/): identity provider and role mapping.
- [Grafana configuration reference](https://grafana.com/docs/grafana/latest/setup-grafana/configure-grafana/) (official): every `grafana.ini` key.
