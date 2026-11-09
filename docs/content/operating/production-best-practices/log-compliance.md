---
title: "Log compliance"
weight: 10
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Log compliance

This page is for you if a security or compliance requirement says logs must be kept, unaltered, for a set period.
It covers how to keep that record with the stack, and what the record does and doesn't hold.
For the setup itself, see [Archive logs to write-once storage](../../../logs-and-events/storing/#worm-archive).
What to do about it is in the [production checklist](../checklist/).

{{< rfc-2119 >}}

## Keep the record outside Loki {#outside-loki}

The bundled Loki is the operational copy of your logs, and it isn't a tamper-proof record.
Its compactor deletes logs when their retention ends, and its deletion API removes them on request.
The identity Loki writes with needs permission to delete objects for both, so it can delete any chunk.

For a record that must survive unchanged, you SHOULD archive a second copy through an `s3` destination,
into an S3 bucket with Object Lock in compliance mode.
In compliance mode, no one can delete or overwrite an object version, or shorten its retention, until the retention ends.
`profiles/s3-worm.values.yaml` adds the destination.

The archive MUST use a bucket of its own, not Loki's.
A lock on Loki's bucket keeps every object Loki deletes until its retention ends, so the bucket's storage grows without bound.

## Separate who can change the archive {#separation-of-duties}

Compliance mode stops anyone from deleting an object version.
It doesn't stop an administrator of the bucket's account from changing the bucket policy, or from stopping new writes.

- You SHOULD keep the archive bucket in an AWS account that the people who operate the monitoring stack don't administer.
  The gateway writes there through a role in that account, which it assumes with `roleArn`.
- The gateway's identity MUST have `s3:PutObject` on the bucket and prefix, and nothing else.
  An identity that can delete can hide an object behind a delete marker, which Object Lock allows because the locked version stays underneath.

## Choose the retention mode and period {#retention}

The chart doesn't set a retention on the objects it writes.
The bucket's default retention is what protects each one, so set it on the bucket, in compliance mode, for the period your requirement names.

Governance mode lets an identity with `s3:BypassGovernanceRetention` remove a lock.
You MAY use it while you validate the archive, but you SHOULD NOT treat a governance-mode bucket as a compliance record.

You SHOULD try the whole path in a scratch bucket with a one-day retention first.
A compliance-mode retention can't be shortened after it's set, so a mistake in the period costs storage until the period ends.

## Decide what goes in before it goes in {#before-archiving}

A line in the archive can't be redacted or removed until its retention ends.
That holds for a credential a component logged by mistake, and for personal data.

- You SHOULD [turn on the secret filter](../secret-filtering/#turn-it-on) before the archive receives anything.
  The filter runs before the gateway hands a line to any destination, so it redacts the archive's copy as well.
- You SHOULD narrow the archive to the streams your requirement names, with `logs.selector`.
  Without a selector, the archive receives every line the gateway forwards.
- If a regulation lets people ask for their personal data to be erased, confirm with your compliance team that the archived streams can't contain any.
  Loki's deletion API can honor an erasure request, and the archive can't.

## Know what the archive doesn't hold {#gaps}

The archive is a copy of what the gateway forwards.
It differs from a complete record of what your workloads logged in the following ways:

- **Processing.** The archive holds each line after the gateway processes it, the same line the bundled Loki stores.
  Levels are normalized, secrets the filter finds are redacted, and lines longer than the gateway's limit are cut.
- **Drops.** The archive drops a record when its queue is full, or when S3 still refuses the batch after about five minutes of retries.
  Each drop fires `log-archive-drops`, and the record is missing for good.
- **A killed gateway.** The batch in progress and the queue live in the gateway's memory.
  A gateway that's killed, for example by the OOM killer or a node failure, loses them, and no counter records the loss.
- **Lines that never reach the gateway.** A line lost upstream is missing from every destination.
  A node that fails before its agent ships a line loses it this way.

You SHOULD route `log-archive-drops` to the team that owns the requirement, because each firing marks a window with records missing:

```yaml
alerting:
  routes:
    extra:
      - matchers: ['alertname="log-archive-drops"']
        receiver: COMPLIANCE_RECEIVER
        continue: true
```

Replace `COMPLIANCE_RECEIVER` with a receiver defined under `alerting.receivers`.
For more information, see [Extra routes](../../../alerting/channels/#extra-routes).

## Keep the queue inside the gateway's memory {#queue}

While S3 is slow or unreachable, batches wait in the gateway's memory, up to `queue.queueSize` of them.
Alloy's default is 1000 batches.
On a busy install, that many can outgrow the gateway's memory limit, and the OOM killer then restarts the gateway, which interrupts every destination.

The profile sets `queueSize` to 20, so at most 20 batches of `batch.size` records wait.
Past that, the archive drops new batches and counts them.
You SHOULD raise `queueSize` only together with `alloy-gateway.alloy.resources`; for more information, see [Gateway memory](../alloy/#gateway-memory).

A longer queue doesn't lengthen the outage the archive survives.
Each batch is retried for about five minutes after its first attempt, and then dropped.

## Check the archive {#check}

After you install, confirm that objects land in the bucket and carry the lock:

```bash
aws s3api list-objects-v2 --bucket ARCHIVE_BUCKET --prefix PREFIX/ --max-items 1 --query 'Contents[].Key'
aws s3api get-object-retention --bucket ARCHIVE_BUCKET --key OBJECT_KEY
```

Replace the following:

- `ARCHIVE_BUCKET`: the archive bucket's name.
- `PREFIX`: the destination's `prefix`, which is `mzmon/logs` in the profile.
- `OBJECT_KEY`: a key that the first command returns.

The first object appears within `batch.timeout` of the first archived line, 1 minute by default.
Its retention shows `"Mode": "COMPLIANCE"`, and a `RetainUntilDate` one retention period after the object was written.

## See also

- [Archive logs to write-once storage](../../../logs-and-events/storing/#worm-archive)
- [Secret filtering](../secret-filtering/)
- [Collection (Alloy)](../alloy/)
