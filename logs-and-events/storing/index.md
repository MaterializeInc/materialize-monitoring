# Storing




# Storing Logs

Loki keeps all durable log data in a single [object storage](../architecture/#storage) backend, maintained by the [Loki Compactor](../architecture/#backend).
This page covers where the gateway writes logs, the storage layout, the index, retention, and disaster recovery.
See the [logging architecture](../architecture/) for how storage relates to the ingesters that write it and the queriers that read it.

## Where the gateway writes logs {#destinations}

The gateway writes processed logs to each destination in `pipeline.destinations`.
The map holds one entry by default, `loki`, which writes to the bundled Loki.
Each entry has its own components and its own queue.

An entry's `type` sets how the gateway writes to it:

`loki`
: Pushes logs through `loki.write`, labeled with the entry's name.

`otlp`
: Sends logs, metrics, or both over one OTLP connection, through components labeled `dest_<name>`.
  A signal is on when its block is present.

To write logs somewhere else as well, add an entry.
The following example adds an external Loki that never slows down delivery to the bundled one:

```yaml
pipeline:
  destinations:
    archive:
      type: loki
      url: https://logs.example.com/loki/api/v1/push
      authType: bearer
      queue:
        capacity: 64MiB
        blockOnOverflow: false
```

The following example sends logs to an OTLP backend that takes only logs, on that signal's own endpoint:

```yaml
pipeline:
  destinations:
    opensearch:
      type: otlp
      url: http://data-prepper.logging.svc:21890
      logs:
        url: http://data-prepper.logging.svc:21892/v1/logs
```

An `otlp` entry with a `metrics` block also receives metrics, filtered to its own `metrics.minMetricImportance`, over the same connection and credentials.
Over gRPC, set `protocol: grpc` and write `url` as `host:port`; gRPC serves every signal on one endpoint.

To authenticate to a destination with a client certificate, put the certificate and key in a Secret named `mzmon-alloy-gateway-destination-tls` in the gateway's namespace.
The gateway mounts it at `/etc/mzmon/destination-tls`, so set the destination's `tls.certFile` and `tls.keyFile` to files there.
Until the Secret exists, the mount is empty and a destination that names a file in it fails to load.

To stop writing to the bundled Loki, set `pipeline.destinations.loki.enabled` to `false`.
For every field, see [`pipeline.destinations` in the values reference](../../reference/helm/materialize-monitoring-values/#destinations).

### Reduce what one destination receives {#destination-reduction}

A destination can receive a reduced copy of the logs while every other destination keeps everything.
Set any of the following fields in the destination's `logs` block:

| Field | What it does |
|---|---|
| `selector` | Keeps only entries matching a LogQL stream selector, such as `{namespace=~"environment-.+"}`. Line filters are allowed after it. |
| `minLevel` | Drops entries below a level: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`, or `CRITICAL`. |
| `keepUnknown` | Set to `false` to drop entries with an `UNKNOWN` level, which `minLevel` keeps. A panic arrives without a level the gateway can parse, so it is `UNKNOWN`. Defaults to `true`. |
| `sampleRate` | Keeps a fraction of entries, above 0 and at most 1. |
| `rateLimit` | Drops entries above `rate` lines per second, allowing bursts of `burst` whole lines. `burst` defaults to `rate`, rounded up, and can be lower than it. |

The gateway applies them in that order, in a `loki.process` component of the destination's own, `loki.process "dest_<name>"`.
Each drop is counted in `loki_process_dropped_lines_total`, with `reason` set to `destination_selector`, `destination_level`, `destination_sampling`, or `ratelimit_drop_stage`.

The following example sends only warnings and above from environment namespaces, at most 500 lines per second:

```yaml
pipeline:
  destinations:
    controlPlane:
      type: otlp
      url: https://ingest.example.com
      logs:
        selector: '{namespace=~"environment-.+"}'
        minLevel: WARN
        rateLimit:
          rate: 500
```

To send a destination no logs at all, set `logs.enabled` to `false` rather than a sample rate of 0.

### Keep one destination from holding back another {#destination-isolation}

The gateway hands every log entry to each destination in turn.
A destination that blocks when its queue is full makes the gateway wait for room, and while it's stalled, no other destination receives anything either.

The types differ in their default:

- A `loki` destination blocks.
  Set `queue.blockOnOverflow: false` on every `loki` destination except the one whose copy must be complete.
  It then drops entries when its queue is full, and counts them in `loki_write_dropped_entries_total{reason="queue_is_full"}`.
  `queue.capacity` sets how far it can fall behind before it drops.
- An `otlp` destination drops.
  It counts what it drops in `otelcol_exporter_enqueue_failed_log_records_total`, and `queue.queueSize` sets how many requests it holds.

The render warns when more than one log destination blocks.

> [!NOTE]
>   `pipeline.logging.gateway.destination.loki` is deprecated.
>   Keys you still set there apply to `pipeline.destinations.loki`, and the render warns.
>   Move each key across, so `pipeline.logging.gateway.destination.loki.url` becomes `pipeline.destinations.loki.url`.

## Object storage

Loki is a black box over an S3-style object store.
The supported backends are **S3-compatible** storage (AWS S3, MinIO, Ceph, R2, …), **Google Cloud Storage**, and **Azure Blob Storage**.
For integration testing, Loki instead uses a local **filesystem** store in single-binary mode — no object storage required.

A single bucket holds everything, separated by prefix:

| Prefix | Contents |
|---|---|
| `/loki/chunks` | compressed log chunks and the TSDB index |
| `/loki/ruler` | [Loki Ruler](../architecture/#ruler) rule definitions |

> [!INFO]
>   Prefer granting access through cloud **workload identity** (IRSA on AWS, Workload Identity on GKE, Azure Workload Identity) so no long-lived credentials live in the cluster.
>   Manually configured credentials are supported as a documented escape hatch for environments where workload identity is unavailable.

### Selecting the backend

The chart's defaults are S3-shaped, so an AWS install only has to supply bucket names.
Every other backend has to be named in **three** places, and all three have to agree:

| Value | Read by |
|---|---|
| `loki.loki.storage.object_store.type` | the chunk and index clients, and the [Ruler](../architecture/#ruler) store |
| `loki.loki.schemaConfig.configs[].object_store` | the chunk client for that schema period |
| `loki.loki.compactor.delete_request_store` | the compactor's delete-request store |

None of these fail softly.
The client is chosen by name and then validated against a configuration that was never populated, so the component that reads a stale value crash-loops with `create bucket: no s3 endpoint in config file`.
The schema period is the one that hurts most — it selects the chunk client, so **every ingester** fails at startup.

There is a fourth, `loki.loki.storage.type`, which is the pre-Thanos selector.
Loki ignores it while `use_thanos_objstore` is on and logs that it is doing so, so a stale value is misleading rather than broken — set it anyway, so the rendered configuration does not contain a contradictory store.

The chart validates the set at render time and refuses to install a mismatched config, naming the value to fix.
The exception is a schema period that is *not* the newest: those are allowed to name the previous backend, because that is what an append-only backend migration looks like.

> [!TIP]
>   `charts/materialize-monitoring/profiles/gcp-example.values.yaml` and `azure-example.values.yaml` set all four for their backend and are the shortest path to a correct non-AWS config.
>   The Azure profile carries more than the other two, because Entra Workload ID needs a pod label the Thanos chart cannot render — read its header before copying.
>   The [Terraform module](../../getting-started/terraform/) derives all four from `object_storage.cloud`, so there is nothing to keep in sync there.

## Granting object-storage access (workload identity)

On every managed cloud the recommended way to give Loki access to its bucket is **workload identity** — no static keys in the cluster.
The shape is the same across providers: a Loki pod runs as a Kubernetes **ServiceAccount** annotated to reference a cloud identity → the platform projects a signed token into the pod → that token is exchanged for short-lived cloud credentials → Loki uses them against the object store.
Only the binding mechanism differs. Pick your provider:

<div class="book-tabs" >
<input type="radio" class="toggle" name="tabs-0" id="tabs-0-0" checked="checked" /><label for="tabs-0-0">AWS · EKS (IRSA)</label><div class="book-tabs-content markdown-inner">
<p><strong>IRSA</strong> (IAM Roles for Service Accounts). Chain: ServiceAccount annotated with a role ARN → EKS projects an OIDC token → the SDK calls <strong>STS <code>AssumeRoleWithWebIdentity</code></strong> → temporary credentials → <strong>S3</strong>. Requires the cluster&rsquo;s <strong>OIDC provider</strong> registered in IAM (one-time).</p>
<p><em>Trust policy</em> — scope <code>:sub</code> to the <strong>exact namespace and ServiceAccount Loki runs as</strong>: ServiceAccount <code>loki</code> (a deterministic <code>fullnameOverride</code>) in the release namespace (recommended <code>monitoring</code>). Scope it to Loki&rsquo;s, not another workload&rsquo;s. For several component ServiceAccounts, use <code>StringLike</code> with a <code>*</code> suffix.</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-json" data-lang="json"><span style="display:flex;"><span>{
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">&#34;Version&#34;</span>: <span style="color:#e6db74">&#34;2012-10-17&#34;</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">&#34;Statement&#34;</span>: [{
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">&#34;Effect&#34;</span>: <span style="color:#e6db74">&#34;Allow&#34;</span>,
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">&#34;Principal&#34;</span>: { <span style="color:#f92672">&#34;Federated&#34;</span>: <span style="color:#e6db74">&#34;arn:aws:iam::&lt;account-id&gt;:oidc-provider/oidc.eks.&lt;region&gt;.amazonaws.com/id/&lt;oidc-id&gt;&#34;</span> },
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">&#34;Action&#34;</span>: <span style="color:#e6db74">&#34;sts:AssumeRoleWithWebIdentity&#34;</span>,
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">&#34;Condition&#34;</span>: { <span style="color:#f92672">&#34;StringEquals&#34;</span>: {
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">&#34;oidc.eks.&lt;region&gt;.amazonaws.com/id/&lt;oidc-id&gt;:aud&#34;</span>: <span style="color:#e6db74">&#34;sts.amazonaws.com&#34;</span>,
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">&#34;oidc.eks.&lt;region&gt;.amazonaws.com/id/&lt;oidc-id&gt;:sub&#34;</span>: <span style="color:#e6db74">&#34;system:serviceaccount:monitoring:loki&#34;</span>
</span></span><span style="display:flex;"><span>    }}
</span></span><span style="display:flex;"><span>  }]
</span></span><span style="display:flex;"><span>}</span></span></code></pre></div><blockquote class='book-hint info'>
<p>The default assumes the release is installed into <code>monitoring</code>.
Under <a href="../../operating/production-best-practices/stack-wide/#namespace-layout">split namespaces</a>, the <code>:sub</code> is <code>system:serviceaccount:loki:loki</code> instead.</p></blockquote><p>A trust policy scoped to the wrong namespace/ServiceAccount is what produces <code>STS: AssumeRoleWithWebIdentity … 403 AccessDenied</code>.</p>
<p><em>Permissions policy</em> — least-privilege to the single bucket + <code>/loki/*</code>. <code>DeleteObject</code> is required (compactor retention/compaction and the delete-requests store).</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-json" data-lang="json"><span style="display:flex;"><span>{
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">&#34;Version&#34;</span>: <span style="color:#e6db74">&#34;2012-10-17&#34;</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">&#34;Statement&#34;</span>: [
</span></span><span style="display:flex;"><span>    { <span style="color:#f92672">&#34;Effect&#34;</span>: <span style="color:#e6db74">&#34;Allow&#34;</span>, <span style="color:#f92672">&#34;Action&#34;</span>: [<span style="color:#e6db74">&#34;s3:ListBucket&#34;</span>], <span style="color:#f92672">&#34;Resource&#34;</span>: [<span style="color:#e6db74">&#34;arn:aws:s3:::&lt;bucket&gt;&#34;</span>],
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">&#34;Condition&#34;</span>: { <span style="color:#f92672">&#34;StringLike&#34;</span>: { <span style="color:#f92672">&#34;s3:prefix&#34;</span>: [<span style="color:#e6db74">&#34;loki/*&#34;</span>] } } },
</span></span><span style="display:flex;"><span>    { <span style="color:#f92672">&#34;Effect&#34;</span>: <span style="color:#e6db74">&#34;Allow&#34;</span>, <span style="color:#f92672">&#34;Action&#34;</span>: [<span style="color:#e6db74">&#34;s3:GetObject&#34;</span>, <span style="color:#e6db74">&#34;s3:PutObject&#34;</span>, <span style="color:#e6db74">&#34;s3:DeleteObject&#34;</span>],
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">&#34;Resource&#34;</span>: [<span style="color:#e6db74">&#34;arn:aws:s3:::&lt;bucket&gt;/loki/*&#34;</span>] }
</span></span><span style="display:flex;"><span>  ]
</span></span><span style="display:flex;"><span>}</span></span></code></pre></div><p><em>ServiceAccount</em> — annotate through chart values; the EKS webhook then injects <code>AWS_ROLE_ARN</code> / <code>AWS_WEB_IDENTITY_TOKEN_FILE</code>.</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-yaml" data-lang="yaml"><span style="display:flex;"><span><span style="color:#f92672">loki</span>:
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">serviceAccount</span>:
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">annotations</span>:
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">eks.amazonaws.com/role-arn</span>: <span style="color:#ae81ff">arn:aws:iam::&lt;account-id&gt;:role/&lt;loki-role&gt;</span></span></span></code></pre></div></div>
<input type="radio" class="toggle" name="tabs-0" id="tabs-0-1"  /><label for="tabs-0-1">GCP · GKE (Workload Identity)</label><div class="book-tabs-content markdown-inner">
<p><strong>GKE Workload Identity.</strong> Chain: the Loki ServiceAccount is annotated with a Google service account (GSA) → GKE exchanges the pod&rsquo;s token for that GSA&rsquo;s credentials → <strong>GCS</strong>. Requires Workload Identity enabled on the cluster and node pool. Below, <code>&lt;gsa&gt;</code> is the GSA; <code>[&lt;namespace&gt;/loki]</code> is the Kubernetes ServiceAccount (KSA).</p>
<ol>
<li>
<p>Grant the GSA object access on the bucket:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>gcloud storage buckets add-iam-policy-binding gs://&lt;bucket&gt; <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --member<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;serviceAccount:&lt;gsa&gt;@&lt;project&gt;.iam.gserviceaccount.com&#34;</span> <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --role<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;roles/storage.objectAdmin&#34;</span></span></span></code></pre></div></li>
<li>
<p>Bind the GSA&rsquo;s IAM policy so the Loki KSA may impersonate it — the KSA <strong>must match Loki&rsquo;s namespace/ServiceAccount</strong>:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-bash" data-lang="bash"><span style="display:flex;"><span>gcloud iam service-accounts add-iam-policy-binding &lt;gsa&gt;@&lt;project&gt;.iam.gserviceaccount.com <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --role<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;roles/iam.workloadIdentityUser&#34;</span> <span style="color:#ae81ff">\
</span></span></span><span style="display:flex;"><span>  --member<span style="color:#f92672">=</span><span style="color:#e6db74">&#34;serviceAccount:&lt;project&gt;.svc.id.goog[monitoring/loki]&#34;</span></span></span></code></pre></div><blockquote class='book-hint info'>
<p>The default assumes the release is installed into <code>monitoring</code>.
Under <a href="../../operating/production-best-practices/stack-wide/#namespace-layout">split namespaces</a>, use <code>--member=&quot;serviceAccount:&lt;project&gt;.svc.id.goog[loki/loki]&quot;</code> here.</p></blockquote></li>
<li>
<p>Annotate the ServiceAccount — and set the GCS backend in all four places from <a href="#selecting-the-backend">Selecting the backend</a>, which the annotation alone does not do:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-yaml" data-lang="yaml"><span style="display:flex;"><span><span style="color:#f92672">loki</span>:
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">serviceAccount</span>:
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">annotations</span>:
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">iam.gke.io/gcp-service-account</span>: <span style="color:#ae81ff">&lt;gsa&gt;@&lt;project&gt;.iam.gserviceaccount.com</span></span></span></code></pre></div></li>
</ol>
</div>
<input type="radio" class="toggle" name="tabs-0" id="tabs-0-2"  /><label for="tabs-0-2">Azure · AKS (Workload ID)</label><div class="book-tabs-content markdown-inner">
<p><strong>Microsoft Entra Workload ID.</strong> Chain: ServiceAccount annotated with a managed-identity client ID → AKS projects a token → exchanged with Entra for the identity&rsquo;s credentials → <strong>Azure Blob</strong>. Requires the OIDC issuer + workload identity enabled on the cluster.</p>
<ol>
<li>
<p>Grant the user-assigned managed identity <strong><code>Storage Blob Data Contributor</code></strong> on the storage account (or container scope).</p>
</li>
<li>
<p>Create a <strong>federated identity credential</strong> on that identity — subject <strong>must match Loki&rsquo;s namespace/ServiceAccount</strong>:</p>
<ul>
<li>issuer = the AKS cluster&rsquo;s OIDC issuer URL</li>
<li>subject = <code>system:serviceaccount:monitoring:loki</code></li>
<li>audience = <code>api://AzureADTokenExchange</code></li>
</ul>
<blockquote class='book-hint info'>
<p>The default assumes the release is installed into <code>monitoring</code>.
Under <a href="../../operating/production-best-practices/stack-wide/#namespace-layout">split namespaces</a>, the subject is <code>system:serviceaccount:loki:loki</code> instead.</p></blockquote></li>
<li>
<p>Annotate the ServiceAccount, label the pods so the webhook injects the token, and set the Azure backend in all four places from <a href="#selecting-the-backend">Selecting the backend</a>:</p>
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-yaml" data-lang="yaml"><span style="display:flex;"><span><span style="color:#f92672">loki</span>:
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">serviceAccount</span>:
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">annotations</span>:
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">azure.workload.identity/client-id</span>: <span style="color:#ae81ff">&lt;client-id&gt;</span>
</span></span><span style="display:flex;"><span>  <span style="color:#75715e"># The workload-identity webhook only acts on pods carrying this label;</span>
</span></span><span style="display:flex;"><span>  <span style="color:#75715e"># apply it via the chart&#39;s pod-label values for the Loki components.</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">podLabels</span>:
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">azure.workload.identity/use</span>: <span style="color:#e6db74">&#34;true&#34;</span></span></span></code></pre></div></li>
</ol>
</div>
</div>

> [!INFO]
>   **The token exchange and the object store are both 443 hops** to your cloud's identity and storage endpoints (AWS: STS + S3; GCP: `sts.googleapis.com`/`oauth2.googleapis.com` + GCS; Azure: `login.microsoftonline.com` + Blob). If you enable the Loki NetworkPolicy you must allow that egress (`networkPolicy.externalStorage`), or the credential fetch fails — a *blocked* egress hangs the compactor at startup (a silent connect timeout), while an *allowed* egress with a bad binding returns a fast auth error. See [Operating > Production Best Practices](../../operating/production-best-practices/loki/#object-storage-egress).

> [!NOTE]
>   **Alternatives.** **EKS Pod Identity** is a newer AWS alternative to IRSA (a pod-identity *association*, no `oidc-provider` to manage). **Static keys / connection strings** are the last-resort escape hatch on any cloud — supply them as a Secret consumed by reference, for environments without workload identity.

**Verifying.** Confirm the ServiceAccount carries the provider annotation and the pod has the injected identity env/token, then split the two failure modes: a failure during the **token exchange** (`403`/AccessDenied on `AssumeRoleWithWebIdentity`, or the equivalent GCP/Azure exchange error) is a **binding/trust-scope** problem — usually a namespace/ServiceAccount subject mismatch; an authorization error on the **storage operation itself**, *after* the exchange succeeds, is a **permissions** problem on the bucket/container.

## Chunks and the index

Two kinds of data live in the bucket:

- **Chunks** — the compressed log lines themselves, flushed from ingesters in batches.
- **The index** — the map from [stream labels](../../o11y-glossary/#logs-and-events) to the chunks that contain them, written in the [TSDB](https://grafana.com/docs/loki/latest/operations/storage/tsdb/) index format (schema **v13**).

Because Loki indexes only labels, the index stays small relative to the log volume — the chunks dominate storage.
[Structured metadata](../architecture/#storage) is stored with the chunks, queryable but not part of the label index.

> [!WARNING]
>   The schema is configured in append-only **periods** with a future start date; a period that is already in use can never be changed retroactively.
>   Plan schema changes (for example a future index-format bump) as a new period with a `from` date ahead of now — never by editing a past period.

## Compaction

The [Loki Compactor](../architecture/#backend) merges the many small index files produced by individual ingesters into a single compacted index per tenant per day.
This keeps reads efficient as volume grows.
The compactor is a **singleton** — exactly one instance coordinates against the shared bucket.

## Retention

Retention is enforced by the compactor, not by the object store's own lifecycle rules.

- A global **retention period** sets how long logs are kept before deletion.
- **Tiered (per-stream) retention** lets different log streams keep data for different lengths of time — for example, keeping `ERROR` and audit-relevant streams far longer than high-volume `INFO` chatter. This is a primary cost lever at fleet scale.
- The **deletion API** processes targeted deletes (for example, compliance "right to be forgotten" requests) outside the normal retention schedule.

## Scaling

- **Ingesters** run **ephemerally** (node-local `emptyDir`, no PVC); durability comes from the replication factor of 3, so you run at least three. Scale past three on memory / stream cardinality, not bytes.
- **Object storage** scales on its own — there is no capacity to provision, only cost and retention to manage.
- Scaling the read side (queriers, frontend) is independent and covered in [Querying](../querying/).

> [!WARNING]
>   Ingester rollouts happen one at a time (guarded by a PodDisruptionBudget). `flush-on-shutdown` is best-effort — a truncated flush is covered by the other replicas, so durability does not depend on a graceful stop.
>   See the [ingester](../architecture/#loki-ingester) notes and [Operating > Upgrading](../../operating/upgrading/).

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

- [Logging Architecture](../architecture/) — storage in the context of the full pipeline.
- [Collecting](../collecting/) — how logs arrive before they are stored.
- [Querying](../querying/) — reading stored logs back.
- [Loki storage](https://grafana.com/docs/loki/latest/operations/storage/) and [retention](https://grafana.com/docs/loki/latest/operations/storage/retention/) (official).

