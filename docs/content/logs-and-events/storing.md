---
title: "Storing"
weight: 20
---

# Storing Logs

Loki keeps all durable log data in a single [object storage](../#storage) backend, maintained by the [Loki Compactor](../#backend).
This page covers the storage layout, the index, retention, and disaster recovery.
See the [logging architecture](../) for how storage relates to the ingesters that write it and the queriers that read it.

## Object storage

Loki is a black box over an S3-style object store.
The supported backends are **S3-compatible** storage (AWS S3, MinIO, Ceph, R2, …), **Google Cloud Storage**, and **Azure Blob Storage**.
For integration testing, Loki instead uses a local **filesystem** store in single-binary mode — no object storage required.

A single bucket holds everything, separated by prefix:

| Prefix | Contents |
|---|---|
| `/loki/chunks` | compressed log chunks and the TSDB index |
| `/loki/ruler` | [Loki Ruler](../#ruler) rule definitions |

> [!INFO]
>   Prefer granting access through cloud **workload identity** (IRSA on AWS, Workload Identity on GKE, Azure Workload Identity) so no long-lived credentials live in the cluster.
>   Manually configured credentials are supported as a documented escape hatch for environments where workload identity is unavailable.

## Chunks and the index

Two kinds of data live in the bucket:

- **Chunks** — the compressed log lines themselves, flushed from ingesters in batches.
- **The index** — the map from [stream labels](../../o11y-glossary/#logs-and-events) to the chunks that contain them, written in the [TSDB](https://grafana.com/docs/loki/latest/operations/storage/tsdb/) index format (schema **v13**).

Because Loki indexes only labels, the index stays small relative to the log volume — the chunks dominate storage.
[Structured metadata](../#storage) is stored with the chunks, queryable but not part of the label index.

> [!WARNING]
>   The schema is configured in append-only **periods** with a future start date; a period that is already in use can never be changed retroactively.
>   Plan schema changes (for example a future index-format bump) as a new period with a `from` date ahead of now — never by editing a past period.

## Compaction

The [Loki Compactor](../#backend) merges the many small index files produced by individual ingesters into a single compacted index per tenant per day.
This keeps reads efficient as volume grows.
The compactor is a **singleton** — exactly one instance coordinates against the shared bucket.

## Retention

Retention is enforced by the compactor, not by the object store's own lifecycle rules.

- A global **retention period** sets how long logs are kept before deletion.
- **Tiered (per-stream) retention** lets different log streams keep data for different lengths of time — for example, keeping `ERROR` and audit-relevant streams far longer than high-volume `INFO` chatter. This is a primary cost lever at fleet scale.
- The **deletion API** processes targeted deletes (for example, compliance "right to be forgotten" requests) outside the normal retention schedule.

## Scaling

- **Ingesters** are stateful: each needs a persistent volume for its [write-ahead log](https://grafana.com/docs/loki/latest/operations/storage/wal/), and the default replication factor of 3 means you run at least three.
- **Object storage** scales on its own — there is no capacity to provision, only cost and retention to manage.
- Scaling the read side (queriers, frontend) is independent and covered in [Querying](../querying/).

> [!WARNING]
>   Ingester rollouts must flush or hand off cleanly so un-flushed, in-memory data is not lost.
>   See the [ingester](../#loki-ingester) notes and [Operating > Upgrading](../../operating/upgrading/).

## Disaster recovery

Loki has **no native snapshot** mechanism — recovery is a property of the object store, not a Loki feature:

- **Durability and versioning.** Chunks are immutable once written; enabling object versioning protects against accidental overwrite or deletion.
- **Cross-region replication.** Replicating the bucket to a second region gives you a recovery point if the primary region is lost.
- **Tamper evidence.** Object Lock / WORM (compliance mode) makes stored logs immutable for a fixed window — important when logs must serve as evidence in a security or audit event.
- **Restore.** Recovery is "repoint Loki at the bucket." The WAL is ephemeral and the index is rebuilt from object storage, so there is no separate database to restore.

> [!INFO]
>   During a security or audit event, the log store's guarantees come from these object-store features.
>   Freeze the relevant bucket (or a replicated copy) to preserve logs before retention or deletion can act on them.

## See more

- [Logging Architecture](../) — storage in the context of the full pipeline.
- [Collecting](../collecting/) — how logs arrive before they are stored.
- [Querying](../querying/) — reading stored logs back.
- [Loki storage](https://grafana.com/docs/loki/latest/operations/storage/) and [retention](https://grafana.com/docs/loki/latest/operations/storage/retention/) (official).
