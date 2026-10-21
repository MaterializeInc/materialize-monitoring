---
title: "Checklist"
weight: 1
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Production checklist

This checklist is for you if you install or operate the `materialize-monitoring` stack in production.
It covers the collection tier (Alloy), node metrics (node-exporter), and the bundled Loki, Thanos, Grafana, and Alertmanager.

The checklist is ordered by what goes wrong if you skip an item, most severe first:

[Exposure](#exposure)
: Someone outside the trust boundary can read your data or change your configuration.

[Silent failure](#silent-failure)
: Part of the stack stops working without an error, so nothing tells you.

[Data loss](#data-loss)
: An ordinary restart, node loss, or upgrade loses telemetry or state for good.

[Outage](#outage)
: Part of the stack fails to install, upgrade, or stay up. These failures are loud.

[Capacity and cost](#capacity-and-cost)
: The stack runs short of resources, or costs more than it needs to.

[Hardening and extras](#hardening-and-extras)
: Nothing breaks. Each item adds defense in depth, or extra signal where your environment calls for it.

Each item is tagged with its owner under the [shared responsibility model](#shared-responsibility-model), and names the component it touches.
The component links to the page that explains the setting.
Open items are yours to act on.
What the chart already ships is checked, and collapsed at the end of each tier.

For the trust boundaries, the cluster permissions the stack holds, and where credentials live, see [Securing](../../securing/).

## Shared responsibility model

Four parties share responsibility for a production deployment.
Checklist items are tagged with the **primary** owner:

| Tag | Party | Owns |
|---|---|---|
| `[upstream]` | **Upstream service** (Loki, Alloy, Grafana) | component behavior: the ring, WAL, compaction, query engine, defaults |
| `[chart]` | **`materialize-monitoring` chart** | topology and wiring, opinionated defaults, config rendering, validation, shipped dashboards/alerts, the profiles |
| `[consumer]` | **Chart consumer** (Terraform / Pulumi / ArgoCD / FluxCD) | cloud resources (bucket, IAM/IRSA, StorageClass, DNS), secret provisioning, version pinning, selecting the profile |
| `[operator]` | **Human operator** (end-user) | size selection, retention budget, tenant policy, incident response, day-2 procedures, capacity watch |

The following table shows how the parties split each area:

| Area | Upstream | Chart | Consumer | Operator |
|---|---|---|---|---|
| Component logic (ring, WAL, compaction, query) | **owns** | configures | — | — |
| Topology, defaults, validation, dashboards/alerts | — | **owns** | selects profile | — |
| Cloud resources: bucket, IAM/IRSA, StorageClass | — | consumes by name | **owns** | approves |
| Secrets provisioning | — | consumes by name | **provides** | rotates |
| Rolling Alloy after a config change | — | cannot | **owns** | verifies |
| Size, retention budget, tenant policy | — | offers profiles | sets values | **decides** |
| Incident response, upgrades, DR, capacity | — | tooling + alerts | applies changes | **owns** |

### If you install with Terraform {#terraform-consumer}

The [Terraform modules](../../../getting-started/terraform/) are a `[consumer]` implementation, so several items are already done when you use them.
Those are marked **(Terraform: automatic)**; read them to understand what's happening, not as work to do.

The modules satisfy the following:

| | |
|---|---|
| Object storage | One bucket per backend on AWS and GCP, with versioning on; on Azure, one storage account with a container per backend |
| Workload identity | One identity per backend: IRSA on AWS, Workload Identity on GCP, a user-assigned managed identity with a federated credential on Azure. No static keys |
| Version pinning | The module and the chart are one release, so the module ref names the chart version |
| Retention posture | No bucket expiry by default, leaving deletion to each compactor |
| Grafana admin secret | Generated and supplied by name |
| Alloy rollouts | A values hash on both pod templates, so a config change rolls Alloy and an unchanged apply does not |

The following stay yours on any install path:

- A usable **StorageClass**, for the four PVC-backed workloads: Alertmanager, the Loki ruler, and the Thanos Store Gateway and Compactor.
  The modules don't create one.
  For what "usable" means on GCP, see the StorageClass item under [Outage](#outage).
- **Sizing and retention budgets**, node-pool capacity, and the profile choice.
- **In-cluster TLS between components.**
  Certificates and the mTLS profiles ship off by default on both paths; for which hops each phase covers, see
  [Securing > What is not there yet](../../securing/#gaps).
- Everything tagged `[operator]`.

On Azure, the module turns on no blob versioning, so the disaster-recovery item under [Data loss](#data-loss) stays yours there in full.

## Exposure {#exposure}

Skip one of these and someone outside the trust boundary can read your data or change your configuration.

- [ ] `[operator]` [Network policies](../stack-wide/#network-policies): **Confirm your CNI enforces NetworkPolicy.**
  Cilium and Calico do.
  `kindnet`, the default in `kind` and therefore in the E2E tiers, ignores NetworkPolicy entirely, so a green E2E run
  isn't evidence that any of this is enforced.
- [ ] `[operator]` [node-exporter](../node-exporter/#exposure): **Confirm port 9100 isn't reachable from outside the cluster.**
  The pods are host-networked, which puts them outside pod NetworkPolicy on most CNIs, so this is a security-group or
  firewall question, not a Kubernetes one.
- [ ] `[consumer]` [Object storage](../../../logs-and-events/storing/#granting-object-storage-access-workload-identity):
  **(Terraform: automatic)** Grant Loki and Thanos bucket access by **workload identity** (IRSA, GKE
  Workload Identity, or Azure Workload Identity) rather than static keys, and supply any storage secret by reference.
  Static keys are a documented escape hatch only.
  For Thanos, see [Metrics > Storing](../../../metrics/storing/#thanos-object-storage).
- [ ] `[operator]` [Loki](../loki/#tenancy): **Know that isolation within the Loki tenant is label-based.**
  The hard isolation boundary is the install.
  That's fine for trusted internal consumers; per-team or customer-facing access needs Grafana label-based access
  control or tenant-per-environment writes.
- [ ] `[operator]` [Grafana](../../../dashboards/grafana/auth/): **Configure an identity provider** under `grafana.ini`
  before exposing Grafana to anyone but yourself.
  Any `auth.*` section Grafana supports works.
  A render-time check warns when an exposed Grafana has none.
- [ ] `[consumer]` [Grafana](../../../dashboards/grafana/auth/): Provide the OAuth **client secret as a Secret**,
  referenced with `$__file{}` or `$__env{}`.
  Never put it in `grafana.ini` as a literal, because that renders into a ConfigMap.
- [ ] `[operator]` [Grafana](../../../dashboards/grafana/auth/): Map an IdP group claim onto Grafana roles with
  **`role_attribute_path`**, so group membership does the provisioning and nobody is added by hand.
  After it's right, set `role_attribute_strict: true`, so a broken claim fails the login instead of quietly falling back to `Viewer`.
- [ ] `[operator]` [Grafana](../../../dashboards/grafana/auth/): **Know that Grafana's permissions aren't a data boundary.**
  Every data source is queryable by anyone who can reach it, so a Viewer in Grafana still reads every metric in Thanos and every log in the tenant.
- [ ] `[consumer]` [Grafana](../../../dashboards/grafana/architecture/#ingress-and-service-are-not-interchangeable):
  **Choose the protocol layer deliberately.**
  A `LoadBalancer` Service gives you L4 on every cloud and terminates no TLS.
  An Ingress gives you L7, terminates TLS, and needs a controller you might not have.
  L7 is what a public Grafana wants, because a WAF and edge authentication are the two things L4 can't do, but L4 has no
  request-timeout ceiling, which matters for the long Thanos and Loki queries a panel makes.
  The `grafana-ingress` profile is the L7 shape; the Terraform wrappers use the Service.
- [ ] `[consumer]` [Grafana](../grafana/): **Terminate TLS in front of Grafana.**
  It authenticates with a session cookie, so without TLS that cookie and the admin password cross the network in the clear.
  Use either an Ingress `tls` block or termination at the load balancer against a cloud-held certificate (ACM on AWS, a managed certificate on GKE).
  A render-time check warns on an Ingress with no `tls`; it can't see the second case, so confirm rather than dismiss it.
  A bare `LoadBalancer` Service with no controller in front is the one shape with no TLS anywhere.
- [ ] `[operator]` [Grafana](../grafana/): **Supply the certificate or the issuer.**
  The consumer wires it; where the trust comes from, such as a cert-manager `ClusterIssuer`, an ACM ARN, or an uploaded
  certificate, is a policy decision.
- [ ] `[consumer]` [Grafana](../grafana/): Set `security.cookie_secure: true` **after TLS is in place**.
  Before that, the session cookie is never sent and nobody can log in.
- [ ] `[operator]` [Grafana](../grafana/): Rotate the generated admin password, or supply your own with `grafana.admin.existingSecret`.
  Grafana reads it once at startup, so a rotation needs a restart.
- [ ] `[operator]` [Grafana](../grafana/#state): **Never inline the database password into `grafana.ini`**, because it renders into a ConfigMap.
  Use `$__file{}` against a mounted Secret, or `$__env{}` against `envValueFrom`.
  The subchart's `assertNoLeakedSecrets` check fails the render if you forget; leave it on.
- [ ] `[operator]` [Alertmanager](../alertmanager/#exposure): **Don't expose Alertmanager without authentication in front of it.**
  It has none of its own, and its API creates silences.
  Grafana's Alertmanager data source gives you the alert list and the silence editor behind Grafana's authentication.
- [ ] `[consumer]` [Alertmanager](../../../alerting/channels/#cloud): **Publish to Amazon SNS with workload identity**,
  not keys: an IRSA role or EKS Pod Identity association for the `alertmanager` ServiceAccount, allowing `sns:Publish`
  on the topic (plus `kms:GenerateDataKey*` and `kms:Decrypt` for an encrypted topic).
  SNS is the only integration that can use one; SES and Azure Communication Services email authenticate over SMTP with a credential in the Secret.

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Network policies](../stack-wide/#network-policies): On by default for every component, with a
  render-time warning on each one that's switched off.
- [x] `[chart]` [Network policies](../stack-wide/#network-policies): Validators reject the shapes that render a policy
  which can't work: an Alloy `flavor` the subchart doesn't implement (it renders nothing, silently), an Alloy or
  `kube-state-metrics` policy declaring `Egress` with no rules, a Grafana policy with neither direction enabled, and a
  chart-rendered policy that opens no ports at all.
- [x] `[chart]` [Alloy](../stack-wide/#network-policies): NetworkPolicy on both roles, ingress only.
  The agent accepts `12345` from the gateway alone.
  The gateway accepts its ingest ports (`3100`, `4317`, `4318`, `9090`) from any pod in the cluster, because its clients
  are your workloads, while `12345`, which carries clustering gossip and the debugging UI, is restricted to the gateway's own pods.
- [x] `[chart]` [node-exporter](../node-exporter/#exposure): NetworkPolicy with ingress restricted to the Alloy gateway
  on 9100 and egress denied outright.
  `kube-rbac-proxy` is off, because it would double the per-node cost to protect an endpoint that exposes no secrets.
- [x] `[chart]` [Loki](../loki/#tenancy): **One logical tenant per install**, with `auth_enabled: true` and a single
  named `X-Scope-OrgID`, so a future split is config rather than a data migration.
- [x] `[chart]` [Loki](../loki/): Grafana's Loki data source points at the **Loki Gateway**, two replicas with a PDB,
  which routes only Grafana's reads to the query frontend and Grafana's rule list to the ruler.
  Rule edits, pushes, and admin endpoints are refused.
- [x] `[chart]` [Thanos](../../../metrics/storing/): The objstore config is rendered into a Secret
  (`global.objstore.createSecret`) and consumed by every component.
  A render-time check errors when the identity annotation names a different cloud than the objstore backend, and warns
  when a cloud backend has neither an annotation nor inline credentials.
- [x] `[chart]` [Thanos](../stack-wide/#network-policies): NetworkPolicies through `thanos.global.networkPolicies`, one per enabled component.
  Each allows that component's own service ports from anywhere and leaves egress open, so it closes non-service ports rather than restricting callers.
- [x] `[chart]` [Grafana](../grafana/): **Internal by default.**
  The Service is `ClusterIP` and no Ingress is rendered; `grafana.ingress` and `grafana.service` are surfaced for when you need them.
- [x] `[chart]` [Grafana](../grafana/): **Public requires an allowlist, enforced.**
  A `LoadBalancer` Service with no `loadBalancerSourceRanges`, or a `NodePort`, which has no allowlist mechanism at all, is a render-time **error**.
  `connections.grafana.allowPublicAccess: true` downgrades it to a warning; it's an acknowledgement, not a silencer.
- [x] `[chart]` [Grafana](../grafana/): A render-time check **errors** on `auth.anonymous` enabled while Grafana is
  exposed, unless `connections.grafana.allowPublicAccess` says the exposure is deliberate.
- [x] `[chart]` [Grafana](../grafana/#state): A render-time check **warns** on a database connection with `ssl_mode` unset or `disable`.
- [x] `[chart]` [Grafana](../stack-wide/#network-policies): **NetworkPolicy** on by default, with ingress on `3000` from
  any source and every other port on the pod closed.
  `allowExternal` stays on because every way a human reaches Grafana is unselectable from here: an ingress controller in
  another namespace, a load balancer's client ranges, or `kubectl port-forward` arriving from the API server.
  Narrow it with `explicitNamespacesSelector` and `explicitIpBlocks` after you know your path in.
  Egress is unrestricted, because a partial allowlist fails as a dashboard that renders empty with no error.
- [x] `[chart]` [Grafana](../grafana/): **Image Renderer disabled**, and a validator warns when it's turned on.
  It's a headless Chromium that fetches URLs on Grafana's behalf, which makes it a large attack surface and a
  server-side request forgery pivot into the cluster network.
- [x] `[chart]` [Alertmanager](../../../alerting/channels/): A render-time check **errors on an inline credential** in a
  receiver or in `alerting.global`.
  `alerting.assertNoInlineCredentials` turns it off; leave it on.
- [x] `[chart]` [Alertmanager](../stack-wide/#network-policies): **NetworkPolicy** on by default, with `9093` and `9094`
  (TCP **and** UDP) open to the release namespace and egress open to notification providers on `443`, `587`, and `465`.

</details>

## Silent failure {#silent-failure}

Skip one of these and part of the stack stops working without an error, so nothing tells you.

- [ ] `[consumer]` [Alloy](../alloy/#restart-after-a-config-change): **(Terraform: automatic)** **Restart both Alloy
  roles after any pipeline or filter change.**
  A config change doesn't roll the pods, and a reload doesn't pick up the filter half of the config.
  This is the one place the chart can't own its own rollout.
- [ ] `[operator]` [Alloy](../alloy/#restart-after-a-config-change): Change pipeline behavior (stages, label families,
  endpoints) through the YAML sources, never by hand-editing rendered `.alloy` in a running deployment.
  Edits there are untracked and lost on the next render.
- [ ] `[operator]` [Alloy](../alloy/#cardinality): Alert on the pipeline's own shedding (`drop_counter_reason`), so a
  rate limit that's dropping lines is visible rather than silent data loss.
  The shipped `alloy-log-drops` alert covers only entries `loki.write` fails to deliver.
- [ ] `[operator]` [Alloy](../alloy/#agent-placement) and [node-exporter](../node-exporter/#coverage): **Verify coverage after any node-pool change.**
  The agent's pod count and `count(up{job="node-exporter"})` should both equal `count(kube_node_info)`.
  A persistent gap is a pool the DaemonSets aren't reaching.
- [ ] `[operator]` [Alloy](../alloy/#agent-placement) and [node-exporter](../node-exporter/#coverage): If you use
  `NoExecute` taints for routine pool separation, **add a toleration or lose those nodes**.
  Neither DaemonSet tolerates them by default.
  Add the agent's through the Terraform module's `tolerations`, which appends rather than replaces.
- [ ] `[operator]` [Alloy](../alloy/#agent-placement) and [node-exporter](../node-exporter/#coverage): **Record every pool you exclude.**
  Excluding either DaemonSet from a pool, or narrowing its tolerations, is a deliberate blind spot, not a saving.
  Do it only where the per-node budget can't absorb the pod, about 64Mi for node-exporter.
- [ ] `[consumer]` [Namespaces](../stack-wide/#namespace-layout): **Under the split-namespace layout, add
  `namespaceSelector` entries to the network policies.**
  A `podSelector` without a `namespaceSelector` doesn't cross a namespace boundary, so the pod-label rules match nothing
  and the traffic they allow is dropped, with no error.
  Add them through `networkPolicies.<component>.ingress.extra` and `egress.extra`, and the subcharts' own ingress lists,
  such as `node-exporter.networkPolicy.ingress`; or turn the affected policies off.
  The render warns when it sees the combination.
- [ ] `[consumer]` [Cluster name](../../../alerting/architecture/#cluster-label): **Give every cluster its own name**:
  `cluster_name` on Terraform, `clusterName` on Helm.
  Every log line carries it as `cluster`, and the default alert `group_by` includes it.
  The default, `default`, distinguishes nothing once two clusters share a log store, a channel, or an incident tool.
  Two clusters sending one condition to one PagerDuty service then share an incident and resolve each other's.
- [ ] `[operator]` [node-exporter](../node-exporter/#scraping): **If your cluster already runs a node-exporter, turn
  this one off** with `node-exporter.enabled: false`, or `install_node_exporter = false` on Terraform.
  Running both double-counts silently in any `sum()` or `rate()`.
- [ ] `[operator]` [node-exporter](../node-exporter/#collectors): **Alert on collectors failing, not only on the exporter being up.**
  `node_scrape_collector_success == 0` means that collector returned nothing while the scrape as a whole succeeded, so
  `up` stays green and the metrics never appear.
  Some failures are expected and worth allowlisting rather than chasing: `hwmon` is usually empty on cloud VMs, `nvme`
  needs NVMe devices, and `kernel_hung` needs Linux 6.13+ for `/proc/sys/kernel/hung_task_detect_count`.
- [ ] `[operator]` [node-exporter](../node-exporter/#collectors): `schedstat` has two kernel gates and fails quietly on the second.
  `/proc/schedstat` needs `CONFIG_SCHEDSTATS`, and since Linux 4.6 the counters are also off at runtime unless `kernel.sched_schedstats=1`.
  A missing file shows up in `node_scrape_collector_success`; the sysctl being off doesn't, because the file reads as zeros.
- [ ] `[operator]` [Loki](../loki/): **Wire tier-0 alerts to paging**: ingester unhealthy or flush failures, compactor
  not running, discarded samples (`loki_discarded_samples_total`), object-store errors, and disk usage.
  Loki down is its own incident.
- [ ] `[operator]` [Thanos](../thanos/#receive-replication): **Alert on Receive write failures and quorum errors.**
  With a replication factor of 1 or 2, these are the first sign of a lost pod, and the dashboards don't show it.
- [ ] `[operator]` [Thanos](../thanos/): Keep the Store Gateway enabled.
  It's how queries reach historical blocks; without it, reads are limited to what Receive holds locally.
- [ ] `[operator]` [Grafana](../grafana/): **Set `grafana.ini.server.root_url`** to the URL users actually reach.
  Share links, alert notification links, and OAuth redirect URIs are all built from it, and all three break silently when it disagrees with the host.
  A render-time check covers it.
- [ ] `[operator]` [Grafana](../grafana/#plugins): **A hardened Grafana can't install plugins at start**, so
  `grafana.plugins` silently gets you a Grafana without them.
  Bake them into a derived image instead.
- [ ] `[operator]` [Grafana](../grafana/): Alert on Grafana being down.
  It isn't a data-loss incident, but it's the interface every other alert is investigated through.
- [ ] `[operator]` [Alertmanager](../../../alerting/channels/): **Configure at least one receiver.**
  Until one exists, every alert reaches `mzmon-null`, and the render warns.
- [ ] `[operator]` [Alertmanager](../../../alerting/channels/): **Choose `alerting.preset` deliberately.**
  `important` is the default because it errs in the least damaging direction.
  A deployment for which Materialize is critical infrastructure wants `critical-infrastructure`, which routes `critical` to `page`.
  A preset of your own is one more key under `alerting.presets`.
- [ ] `[operator]` [Alertmanager](../../../alerting/channels/): **Test a receiver end to end** after configuring it,
  with `amtool alert add` from inside a pod.
  The render proves the configuration is well-formed, not that a vendor accepts the notification.
- [ ] `[consumer]` [Alertmanager](../../../alerting/channels/): **Provision the `alertmanager-receivers` Secret** in the
  namespace the Alertmanager pods run in, with External Secrets Operator, Vault Agent, SOPS, or a CSI driver.
  The chart consumes it by name and never creates it.
  It's mounted as optional, so a missing Secret shows up as failed notifications, not as a pod that won't start.
  Rotating a credential in it needs no restart: Alertmanager reads a `*_file` credential on each send, and the kubelet
  refreshes the mount within about a minute.
- [ ] `[operator]` [Alertmanager](../alertmanager/#exposure): Leave **`alertmanager.baseURL`** empty unless Alertmanager
  itself is exposed, and then set it to that address.
  **Never point it at Grafana**, which doesn't serve Alertmanager's UI paths, so every link in a notification breaks.
  The render warns when the two share a host.
- [ ] `[operator]` [Alertmanager](../stack-wide/#network-policies): Under `split-namespace`, the profile opens
  Alertmanager to the ruler and Grafana namespaces.
  A receiver listening on a port other than `443`, `587`, or `465` needs that port added to `networkPolicies.alertmanager.egress.external.ports`.
- [ ] `[operator]` [Alertmanager](../alertmanager/): **Add meta-alerts on the notifier.**
  The shipped rule set has none on `alertmanager_cluster_members`, `alertmanager_config_last_reload_successful`, or
  `alertmanager_notifications_failed_total`, so a notifier that can't deliver is visible on a dashboard and pages nobody.
- [ ] `[operator]` [Alertmanager](../alertmanager/): **Route a deadman's switch to something outside the cluster.**
  Stopped evaluation looks like nothing being wrong, and only an external heartbeat monitor can tell the two apart.
  <!-- TODO: point this at the shipped deadman's-switch rule when it lands; see the alerting design doc (20260917-alerting-self-managed). -->

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Scheduling](../stack-wide/#scheduling-priority): Two PriorityClasses, referenced by name from every
  subchart that accepts a `priorityClassName`.
  The per-node collectors and the gateway run at `monitoring-critical`, because losing one is a blind spot nothing covers.
  The backends, Grafana, grafana-operator, and Alertmanager run at `monitoring-scalable`, because a surviving replica or a retry absorbs the loss.
- [x] `[chart]` [Loki](../stack-wide/#scheduling-priority): `priorityClassName` on every Loki pod.
  `loki.global.priorityClassName` covers everything that renders through the chart's `_pod.tpl`; **three components read
  only their own key and are set separately**: the two memcached StatefulSets and the canary.
  Left unset, the canary would run at priority 0, so the first node under pressure would evict the end-to-end write→read
  check during the incident that caused the pressure.
- [x] `[chart]` [Alloy](../alloy/#agent-placement) and [node-exporter](../node-exporter/#coverage): Both DaemonSets
  tolerate every `NoSchedule` taint, so they reach tainted, spot, and system pools without configuration.
- [x] `[chart]` [Alloy](../alloy/): ServiceMonitor or PodMonitor (or GCP `PodMonitoring`) for both roles, scraping
  Alloy's own component metrics: received and sent bytes, dropped lines, and write failures.
- [x] `[chart]` [Alerting](../../../alerting/architecture/): The rule set ships `alloy-log-drops` (entries `loki.write`
  drops), `loki-push-err-high` (Loki push errors, including 429 rate limits), and `logging-collection-down`, all on by default.
- [x] `[chart]` [node-exporter](../node-exporter/#scraping): ServiceMonitor enabled, and `prometheus.io/scrape: "false"`
  on the Service so annotation-based discovery can't scrape it a second time under another `job` label.
- [x] `[chart]` [Loki](../loki/): ServiceMonitor for the Loki components, with the plaintext exporters scraped
  separately so they survive the mTLS profiles; and `loki-canary` enabled for end-to-end write→read verification.
- [x] `[chart]` [Thanos](../thanos/): Query, Receive, Store Gateway, and Compactor enabled by default, with
  ServiceMonitors for every component (`thanos.global.serviceMonitor`).
- [x] `[chart]` [Grafana](../grafana/): ServiceMonitor for Grafana's own metrics.
- [x] `[chart]` [Grafana](../grafana/): Grafana-managed **unified alerting is not HA out of the box**, because each
  replica evaluates every rule and notifies separately.
  The `grafana-postgres` profile enables gossip (`headlessService: true` plus `unified_alerting.ha_peers`).
  Validators warn on several replicas with no gossip, and on `ha_peers` pointing at a headless Service that was never created.
  The Prometheus rules this chart ships are unaffected either way.
- [x] `[chart]` [Grafana](../stack-wide/#network-policies): Gossip needs pod-to-pod **9094 on TCP and UDP**, which the
  Grafana subchart's own NetworkPolicy closes.
  The chart renders the missing rule as `networkPolicies.grafanaGossip` when `unified_alerting.ha_peers` is set **and** `grafana.networkPolicy` is on.
  Turning the supplement off on its own duplicates notifications rather than failing, and a validator warns on that combination.
- [x] `[chart]` [Alertmanager](../alertmanager/#availability): **Two replicas by default**, gossiping over `9094`,
  spread hard across zones and soft across hosts.
  A validator warns on one replica, and on several replicas with no spread or anti-affinity.
- [x] `[chart]` [Alertmanager](../alertmanager/#availability): **Both rulers notify every replica**, through DNS
  discovery of the headless Service rather than the load-balanced one.
- [x] `[chart]` [Alertmanager](../alertmanager/): **ServiceMonitor** for Alertmanager's own metrics, scraping each
  replica once through the ClusterIP Service, and an **Alertmanager data source** in the bundled Grafana for the alert list and silences.
- [x] `[chart]` [Alertmanager](../../../alerting/channels/): A render-time check **errors** when the selected preset
  names a class no receiver serves, on a route naming an undefined receiver or time interval, on a receiver key that
  isn't an Alertmanager integration, and on a `*_file` path under no mounted volume.
- [x] `[chart]` [Alertmanager](../../../alerting/channels/): **`amtool check-config`** runs in CI over representative
  configurations, in the Alertmanager image the chart pins.
- [x] `[chart]` [Alertmanager](../../../alerting/channels/): Routing changes **reload in place**: the config-reloader
  sidecar POSTs `/-/reload` when the rendered Secret changes, and a configuration Alertmanager rejects leaves the previous one running.
  A validator warns when the sidecar is turned off.
- [x] `[chart]` [Alertmanager](../../../alerting/architecture/#cluster-label): **Every alert carries `cluster`**,
  stamped by both rulers from `clusterName`.
  The render fails on an empty name, or one Loki's configuration can't hold unquoted.
- [x] `[chart]` [Alertmanager](../stack-wide/#network-policies): A validator warns when a narrowed policy closes the
  mesh port on either transport, which duplicates every notification rather than failing.

</details>

## Data loss {#data-loss}

Skip one of these and an ordinary restart, node loss, or upgrade loses telemetry or state for good.

- [ ] `[operator]` [Thanos](../thanos/#receive-replication): **Keep the Receive replication factor odd, and restate
  `receive.extraArgs` in full whenever you set it.**
  The chart sets the factor to 3 through that list, and Helm overwrites lists, so an override that adds any flag drops it to Thanos's default of 1.
  At 1, the un-uploaded window exists in one copy on one node.
  A render-time check warns at factor 1, warns harder at 2, and errors when the factor exceeds `replicaCount`.
- [ ] `[operator]` [Thanos](../thanos/#thanos-retention-compaction): Restate `compactor.extraArgs` in full too.
  It carries the vertical compaction and deduplication flags, and dropping the first halts the Compactor at the first overlapping block.
- [ ] `[operator]` [Thanos](../thanos/#thanos-retention-compaction): **Know that deduplication is irreversible** on an existing bucket.
  Downsampled blocks written before it was turned on can't be vertically compacted, and keep their per-replica copies until retention expires them.
- [ ] `[operator]` [Alloy](../alloy/#delivery): Confirm the `loki.write` durability settings (WAL, retry, and backoff)
  survive gateway restarts to your RPO.
- [ ] `[operator]` [Alloy](../alloy/#agent-placement): Persist the agent's file **positions** and journal cursor on a
  hostPath, so a restart resumes where it left off instead of re-tailing (duplicate lines) or skipping (gaps).
- [ ] `[consumer]` [Loki](../../../logs-and-events/storing/): **(Terraform: automatic)** Align the object-store
  lifecycle policy with Loki's retention, or make it longer, so the compactor owns deletion.
  The modules leave bucket expiry off by default, which is the safe end of that alignment.
- [ ] `[operator]` [Loki](../loki/#schema-periods): **Treat schema periods as append-only.**
  A future format change goes in a new period with a `from` date ahead of now, never in an edit to a past period.
- [ ] `[consumer]` `[operator]` [Loki](../../../logs-and-events/storing/): Plan disaster recovery as object versioning,
  cross-region replication, and, for audit, Object Lock (WORM).
  Restoring means repointing at the bucket; there's no native snapshot.
  The Terraform modules enable **versioning** by default on AWS and GCP, but not on Azure.
  Replication and Object Lock remain yours on every cloud.
- [ ] `[operator]` [Grafana](../grafana/#state): **Move Grafana's state off `emptyDir` before anyone depends on it.**
  Use the `grafana-postgres` profile for production, or `grafana-pvc` for a single small instance with no database available.
- [ ] `[operator]` [Grafana](../grafana/#state): **Back the database up.**
  It holds every service-account token, alert rule, and annotation.
  Nothing in this chart replicates it, and a PVC snapshot is the only copy on the `grafana-pvc` path.
- [ ] `[operator]` [Grafana](../grafana/#state): Switching an existing install from SQLite to PostgreSQL **doesn't carry
  state over**, because Grafana has no migration between them.
  Export what matters through the HTTP API first.
- [ ] `[operator]` [Grafana](../grafana/#configuration): Treat the running Grafana as a **cache, not a source**.
  Dashboards edited in the UI are overwritten at the next resync; change them in `packages/dashboards/` instead.

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Alloy](../alloy/#gateway-autoscaling): The gateway runs **at least 2 replicas**, the HPA's floor.
  It holds in-memory buffers, so a single replica is a delivery gap during restarts and node churn.
- [x] `[chart]` [Loki](../loki/#per-size-resources): Microservice (distributed) mode, with `replication_factor: 3`, at
  least 3 ingesters, a singleton compactor, and a memberlist ring (no Consul or etcd).
- [x] `[chart]` [Loki](../loki/#ingester-durability): **Ingesters are ephemeral**, on node-local `emptyDir` with no PVC.
  Durability is the replication factor, so a killed or rescheduled ingester's unflushed data is recovered from its peers.
- [x] `[chart]` [Loki](../loki/#ingester-durability): `flush_on_shutdown: true` as a best effort, with a **modest**
  `terminationGracePeriodSeconds` of about 60s; replication covers a truncated flush.
- [x] `[chart]` [Loki](../loki/#per-size-resources) and [Thanos](../thanos/#thanos-per-size-resources): **No memory
  limit** on Loki's ingesters or on Thanos Receive, so an OOM-kill can't drop buffered logs or the un-uploaded block window.
  Requests are set for scheduling.
- [x] `[chart]` [Loki](../loki/#schema-periods): `schema_config` uses TSDB, schema **v13**, and a 24h index period.
- [x] `[chart]` [Thanos](../thanos/#receive-replication): `receive.mode: standalone` with `replicaCount: 3`, an
  auto-generated ketama hashring, and `--receive.replication-factor=3` in `receive.extraArgs`.
- [x] `[chart]` [Thanos](../thanos/#receive-replication): **Every profile restates `receive.extraArgs` in full**, so the replication factor survives.
  A unit test pins this at all three sizes.
- [x] `[chart]` [Thanos](../thanos/): **VerticalPodAutoscaler disabled on Receive and the Compactor.**
  The subchart defaults it *on* for exactly those two, in `updateMode: Auto`, so where the VPA CRD is present it would
  rewrite the profile's requests and evict pods holding up to 6h of blocks on `emptyDir`.
  A unit test asserts this with the CRD declared present, so it fails on the value rather than passing on the gate.
- [x] `[chart]` [Grafana](../grafana/#state): A render-time check **errors** on more than one replica, including an HPA
  ceiling above one, without `grafana.ini.database` pointed at a shared database.
  It **warns** when an exposed Grafana keeps its state on an `emptyDir`.
  The `grafana-postgres` and `grafana-pvc` profiles ship the assembled shapes.
- [x] `[chart]` [Grafana](../grafana/#configuration): **grafana-operator manages dashboards and data sources as code**,
  re-pushing anything it owns on drift.
- [x] `[chart]` [Alertmanager](../alertmanager/#state): **A 4Gi volume per replica**, for silences and the notification log.
  Gossip covers losing one replica; the volume covers losing both at once.

</details>

## Outage {#outage}

Skip one of these and part of the stack fails to install, upgrade, or stay up.
These failures are loud, but they can read like chart bugs.

- [ ] `[consumer]` [StorageClass](../../../getting-started/terraform/#storageclass-on-gcp-c4-and-n4-node-pools):
  **(Terraform: `storage_class`)** **A usable StorageClass must exist** for the PVC-backed workloads: Alertmanager, the
  Loki ruler (for its remote-write WAL), and the Thanos Store Gateway and Compactor.
  "Usable" isn't the same as "present".
  On GCP's C4 and N4 machine families, which accept only Hyperdisk, *every* StorageClass GKE creates by default is
  Persistent Disk, and none of them attaches.
  Use `volumeBindingMode: WaitForFirstConsumer`, the default for managed-cloud CSI classes; under `Immediate`, a PVC's
  zone is chosen before scheduling, and the hard zone rules leave pods `Pending`.
  On GKE, the legacy `standard` class is `Immediate`.
- [ ] `[operator]` [Zones](../stack-wide/#fewer-than-two-zones): **Tell the chart how many zones you have.**
  The default assumes two or more, and below that it fails closed: Loki's ingesters and Thanos Receive stay `Pending`,
  and on a cluster with no zone labels so does Alertmanager.
  On Terraform, set `min_zones`; on Helm, apply the `no-zone-spread` profile.
- [ ] `[operator]` [Zones](../thanos/#thanos-topology-spread): **Aim for three or more zones**, and raise `minDomains`
  to the count your node pools can launch in.
  The chart's floor is 2, so with three zones a two-zone placement still satisfies it.
  Setting it *above* your real count leaves pods `Pending` forever.
  With two zones, an AZ loss can break Loki and Thanos write quorum (2 of 3) until the ring recovers.
- [ ] `[operator]` [Thanos](../thanos/#thanos-ephemeral-budget): **Check Receive's `ephemeral-storage` request against
  your nodes' *allocatable* ephemeral storage**, not their disk size, before you pick a profile or raise Receive's local retention.
  A request no node shape fits never schedules, and the cluster autoscaler doesn't rescue it.
- [ ] `[consumer]` [Images](../stack-wide/#pull-secret-globals): **Create the pull Secret in every namespace the stack renders into.**
  Helm doesn't create it, and a Secret is namespaced, so a split-namespace install needs one copy per namespace, not one for the release.
- [ ] `[operator]` [Images](../stack-wide/#images-and-registries): **Verify the vendor's tag strings before install.**
  Hardened-image vendors track upstream versions without always spelling them identically: a leading `v` is commonly
  dropped, and a DHI mirror carries only the tags you chose to mirror.
  The profiles inherit the chart's tags rather than restating them, so add an explicit `tag` for any image whose string differs.
- [ ] `[operator]` [Images](../stack-wide/#images-and-registries): **Alloy is a rebuild, not a retag.**
  The chart runs a Materialize build of Alloy carrying the custom components the pipelines are written against, and a
  stock vendor Alloy fails config validation at startup.
  A mirror carries it fine; the `mirror` profile moves it along with everything else, digest intact.
- [ ] `[operator]` [Images](../stack-wide/#image-pull-secret-gaps): Confirm the `pre-delete` cleanup hook can pull its image.
  An image it can't pull hangs `helm uninstall` with the release half-removed, and `helm uninstall --no-hooks` is the way out.
- [ ] `[operator]` [Scheduling](../stack-wide/#scheduling-priority): **PriorityClasses are cluster-scoped**, so two
  releases of this chart in one cluster fight over them.
  Set `priorityClasses.create: false` on all but one, or rename them per release with `priorityClasses.critical.name`
  and `priorityClasses.scalable.name`.
- [ ] `[operator]` [Scheduling](../stack-wide/#scheduling-priority): **If you rename or disable the PriorityClasses,
  update the subcharts' `priorityClassName` values to match.**
  A `priorityClassName` naming a class that doesn't exist doesn't degrade: the API server *rejects* the pod, and the
  only evidence is an admission error on a ReplicaSet nobody is watching.
  A render-time check warns when it can tell.
- [ ] `[operator]` [Scheduling](../stack-wide/#scheduling-priority): If your platform already defines a priority scheme,
  point the subchart values at your own classes instead.
  `metrics-server` stays on the upstream `system-cluster-critical` deliberately: it backs the metrics API that HPAs and
  the Materialize Console read, so it's cluster plumbing rather than monitoring.
- [ ] `[operator]` [Network policies](../stack-wide/#network-policies): **Nothing here allows the kubelet's liveness and
  readiness probes**, which arrive from the node's own IP.
  Cilium and Calico both exempt host-to-pod traffic, so probes keep working.
  A CNI that doesn't would fail every pod in the namespace at once, which is the symptom to recognize rather than a per-component bug.
- [ ] `[consumer]` [Version pinning](../../upgrading/): **(Terraform: automatic)** Pin the chart version, and with it
  every subchart and image, so upgrades are deliberate.
- [ ] `[consumer]` [Loki](../../../logs-and-events/storing/): **(Terraform: automatic)** Provision the object-storage
  bucket (S3-compatible, GCS, or Azure Blob): a single bucket, with prefixes `/loki/chunks` and `/loki/ruler`.
- [ ] `[consumer]` [Loki](../loki/#schema-periods): **(Terraform: automatic)** On any backend other than S3, name it in
  all **three** load-bearing places: `storage.object_store.type`, the newest `schemaConfig` period, and `compactor.delete_request_store`.
  A stale one crash-loops the component that reads it.
- [ ] `[operator]` [Loki](../loki/#object-storage-egress): **Cover the credential endpoint as well as the bucket in
  Loki's egress**, and declare it explicitly rather than relying on another workload's broad 443 rule.
  Missing STS egress crash-loops the compactor with misleading `memberlist` timeouts.
- [ ] `[operator]` [Loki](../loki/#ingester-durability): **Budget the ingester roll and raise the deploy timeout.**
  The serial roll takes about 1 minute per ingester and overruns Helm's default 5-minute `--wait`.
  Set `helm upgrade --timeout`, Flux `spec.timeout`, or Pulumi `customTimeouts`; ArgoCD is async and tolerant.
  For more information, see [Upgrading](../../upgrading/).
- [ ] `[consumer]` [Grafana](../grafana/#state): Provision the **database and an owning user**.
  Grafana runs its own schema migrations at startup, so a read/write-only grant fails the migration.
- [ ] `[consumer]` [Grafana](../grafana/#state): Provide the database password as a **Secret** in the namespace the Grafana *pod* runs in.
  Under `split-namespace`, that's `grafana`, not the release namespace.
  The chart consumes it by name and doesn't mint it.
- [ ] `[operator]` [Grafana](../grafana/): **Create the DNS record.**
  Neither the Ingress nor the Service publishes the hostname, and the chart can't, because it has no view of your zone.
  Unless something like external-dns already reconciles records in the cluster, this is a manual step that's easy to
  forget until the certificate fails to issue.
- [ ] `[operator]` [Grafana](../../../dashboards/grafana/auth/): **Keep the local admin as break-glass.**
  `disable_login_form: true` still leaves `/login?disableAutoLogin` reachable; know that before an IdP outage takes your incident tooling with it.
- [ ] `[operator]` [Grafana](../grafana/#plugins): **Don't update a bundled data source plugin from Grafana's plugin catalog.**
  On the read-only root filesystem it leaves the data source unloaded until the pod restarts.
- [ ] `[operator]` [Grafana](../grafana/#plugins): Where Grafana can't reach grafana.com, set
  `grafana.ini.plugins.preinstall_disabled: true`, which removes the Drilldown apps' download at every start.
- [ ] `[operator]` [Alertmanager](../alertmanager/#state): Treat **`persistence.enabled` and `persistence.size` as immutable.**
  An upgrade that changes either fails until the StatefulSet is deleted with `--cascade=orphan`.
- [ ] `[operator]` [Alertmanager](../alertmanager/#state): Know the recovery for a replica whose zone is lost: delete
  its PVC, then its pod, so it reschedules into a healthy zone and resyncs from its peer.

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Alloy](../alloy/#restart-after-a-config-change): Pipelines are **authored as code**
  (`packages/alloy-pipelines/*.yaml`) and rendered to `.alloy`.
  The rendered output is committed, CI asserts it matches a fresh render, and `alloy validate` runs on every rendered pipeline.
- [x] `[chart]` [Alloy](../alloy/#restart-after-a-config-change): A pre-install and pre-upgrade `alloy validate` hook,
  so a bad config fails the release rather than a running pod.
- [x] `[chart]` [Images](../stack-wide/#images-and-registries): `profiles/registry/` ships the `mirror` overlay, the two
  hardened-vendor overlays, and the `pull-secret` overlay, with unit tests asserting the pull secret reaches each of the
  four subcharts that read no global.
- [x] `[chart]` [Images](../stack-wide/#image-pull-secret-gaps): The `pre-delete` cleanup hook and both pre-install
  validator Jobs carry `imagePullSecrets`, merged from both global spellings and deduplicated by name.
- [x] `[chart]` [Scheduling](../../../getting-started/helm/#scheduling-profile): The `scheduling` profile carries the
  whole fan-out (node selector, tolerations, and both class names), with every site aliased off four anchors.
  A rename is a two-line edit rather than eleven `priorityClassName` keys across nine subcharts that can half-apply.
- [x] `[chart]` [Network policies](../stack-wide/#network-policies): `metrics-server` accepts `10250` from any source, unlike everything else.
  The aggregation layer dials it from the API server, which on a managed control plane isn't selectable, so closing that
  port would break `kubectl top` and every HPA in the cluster.
  The policy still closes every other port on the pod.
- [x] `[chart]` [Zones](../thanos/#thanos-topology-spread): **Hard zone spread, soft host spread** on Loki's ingesters,
  Thanos Receive, and Alertmanager.
  Receive's constraint sets `minDomains: 2` and `nodeTaintsPolicy: Honor`; Loki's drops the chart's default hard per-host anti-affinity.
  The Store Gateway, Query, and Query Frontend get soft spread on both axes, and the Compactor, a singleton, gets none.
- [x] `[chart]` Disruption budgets: **PodDisruptionBudgets with `maxUnavailable: 1`** on Loki's ingesters, every Thanos
  component, Grafana, and Alertmanager.
  `maxUnavailable` scales with the replica count, and on a singleton `minAvailable: 1` would permit no eviction and hang node drains.
  Validators warn on `minAvailable` for a singleton, on several replicas with no budget, and on a Loki budget or rollout
  above 1, and error when the Receive budget exceeds what write quorum tolerates.
- [x] `[chart]` [Loki](../loki/#ingester-durability): **The index-gateway and compactor are ephemeral too**, so they
  reschedule freely across zones; the compactor singleton is never PVC-pinned to one zone.
- [x] `[chart]` [Loki](../loki/#ingester-durability): The StatefulSet rolls ingesters **one at a time** (`maxUnavailable: 1`).
- [x] `[chart]` [Loki](../loki/#object-storage-egress): Egress to **object storage and the credential endpoint** on 443
  through `networkPolicy.externalStorage` (by default `ports: [443]`, `cidrs: ["0.0.0.0/0"]`).
- [x] `[chart]` [node-exporter](../node-exporter/#sizing): `updateStrategy.rollingUpdate.maxUnavailable: 10%`, so an
  image bump doesn't walk a large fleet one node at a time.
- [x] `[chart]` [Thanos](../thanos/#thanos-retention-compaction): **Vertical compaction on**, so the overlapping blocks
  Receive writes across restarts don't halt the Compactor.
- [x] `[chart]` [Thanos](../thanos/): Render-time validators cover objstore placeholders, backend and identity mismatch,
  component topology, replication-factor quorum, and writers or data sources aimed at a Thanos that isn't deployed.
- [x] `[chart]` [Grafana](../grafana/#state): A render-time check **errors** on a `ReadWriteOnce` volume paired with a
  rolling update, and on a volume asked to back several replicas.
- [x] `[chart]` [Grafana](../grafana/): **Probes** on `/api/health` (liveness and readiness) come from the subchart and are left at their defaults.
- [x] `[chart]` [Grafana](../grafana/#plugins): **Bundled data source plugins stay at the image's versions**
  (`grafana.ini.plugins.preinstall_auto_update: false`), and a validator errors when the auto-update is turned back on.
- [x] `[chart]` [Grafana](../grafana/#plugins): A validator errors when `grafana.extraEmptyDirMounts`,
  `grafana.extraVolumeMounts`, or `grafana.extraVolumes` adds a second `/tmp` mount, which the API server would reject.
- [x] `[chart]` [Alertmanager](../alertmanager/#availability): **Readiness on `/-/ready`, liveness on `/-/healthy`**, so
  a rollout replacing one replica at a time loses no silences.
- [x] `[chart]` [Alertmanager](../alertmanager/#exposure): `extraArgs.web.route-prefix: /` keeps every endpoint at the
  root, so a `baseURL` with a path doesn't move the API the rulers post to.
  Resource names are **pinned** by `alertmanager.fullnameOverride: alertmanager`, so the rulers address
  `alertmanager-headless` whatever the release is called.
  The render warns or fails when either drifts.

</details>

## Capacity and cost {#capacity-and-cost}

Skip one of these and the stack runs short of resources, or costs more than it needs to.

- [ ] `[operator]` [Loki](../loki/#sizing-the-logging-backend): **Pick the Loki size** from the throughput table, and
  record the measured sustained rate and 5-minute burst it's based on.
- [ ] `[operator]` [Thanos](../thanos/#choosing-a-thanos-profile): **Pick the Thanos profile** from inventory, and record the inventory it's based on.
  `thanos-small` and `thanos-large` are deltas from the chart defaults, which target medium.
- [ ] `[operator]` [Thanos](../thanos/#thanos-measure-to-validate): Re-measure head series against the envelope after
  install, and after any change in collection count.
- [ ] `[operator]` [Thanos](../thanos/#cost-control-metric-tiers): **To cut metrics cost, tighten `minMetricImportance`**, not the profile.
  Downshift only after head series have fallen.
- [ ] `[operator]` [Loki](../loki/#protective-limits): Set the per-tenant `ingestion_rate_mb`,
  `ingestion_burst_size_mb`, and `max_global_streams_per_user` from the table.
  These apply per environment.
- [ ] `[operator]` [Loki](../../../logs-and-events/storing/) and [Thanos](../thanos/#retention-and-downsampling): **Set
  retention to your storage budget**: Loki's global `retention_period`, and Thanos's per-resolution retention.
- [ ] `[operator]` [Loki](../../../logs-and-events/storing/): Configure **tiered (per-stream) retention**: keep `ERROR`
  and audit streams long, and expire high-volume `INFO` fast.
- [ ] `[operator]` [Thanos](../thanos/#retention-and-downsampling): Keep raw retention above the downsampling thresholds
  (40h for the 5m tier, 10d for the 1h tier).
  Below them the tier is never produced, and long-range queries silently fall back to raw blocks.
- [ ] `[operator]` [Loki](../loki/#ingester-durability): Add ingesters (N > RF) when per-ingester memory or stream count
  climbs, or to spread the regression burst.
  One tenant concentrates cardinality, so watch per-ingester memory against the total fleet stream count.
- [ ] `[operator]` [Loki](../loki/#per-size-resources): Size memcached per the table.
- [ ] `[operator]` [Loki](../loki/): Scale queriers and frontends, not ingesters, when dashboards feel slow.
- [ ] `[operator]` [Loki](../thanos/#thanos-topology-spread): If you bring nodes up **tainted** until DaemonSets are
  healthy, model the taint as a Karpenter `startupTaint` so it doesn't over-provision.
  The ingester spread already sets `nodeTaintsPolicy: Honor`, so tainted nodes stay out of the skew math.
- [ ] `[operator]` [Alloy](../alloy/#gateway-memory): **Size gateway memory vertically**, from the series each pod
  holds, rather than adding memory to the HPA.
- [ ] `[operator]` [Alloy](../alloy/#cardinality): Understand the backpressure semantics before tuning the rate limits:
  `drop = false` queues, `drop = true` sheds.
- [ ] `[operator]` [node-exporter](../node-exporter/#sizing): **Don't add a CPU limit to node-exporter.**
  It turns each scrape's burst into CFS throttling, which surfaces as scrape timeouts on the loaded nodes whose samples matter most.
- [ ] `[operator]` [Thanos](../thanos/#thanos-per-size-resources): Keep `replicaCount` at or above the replication factor.
  At equality every pod holds every series, which is good availability but no horizontal capacity.
- [ ] `[operator]` [Thanos](../thanos/): Enable **Query Frontend** for production read paths (splitting and result
  caching), and repoint `connections.datasources.thanos.url` at it, or the cache is deployed and bypassed.
  A render-time check warns on that mismatch.
  The `thanos-large` profile does both; at S and M, Query is the data source target.
- [ ] `[operator]` [Thanos](../thanos/): Keep Query's `replicaCount` equal to `autoscaling.minReplicas`.
  The subchart templates a static `replicas` alongside the HPA, so every upgrade or GitOps reconcile writes it back;
  matching the floor makes that reset a no-op.
  A validator warns when the two disagree.
- [ ] `[operator]` [Thanos](../thanos/#thanos-per-size-resources): Keep resource requests on anything you autoscale.
  Without a request the HPA has no CPU target to measure against, so it never scales.

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Alloy](../alloy/#cardinality): The gateway promotes only a small, stable label set and routes
  everything else identifying to **structured metadata**.
- [x] `[chart]` [Alloy](../alloy/#cardinality): Per-level rate limits, a per-node pod-log rate cap in the agent
  (`AGENT_POD_LOG_RATE_LIMIT` / `AGENT_POD_LOG_BURST`), stale lines dropped (`older_than`), and oversized lines cut to fit Loki (`stage.truncate`).
- [x] `[chart]` [Alloy](../alloy/#gateway-autoscaling): **Horizontal autoscaling on CPU only** (2–8 replicas, 50%), with
  slow scale-down: only after an hour of low CPU, and one pod per ten minutes.
- [x] `[chart]` [node-exporter](../node-exporter/#collectors): A collector **allowlist** rather than upstream defaults,
  so a node_exporter release can't add collectors behind you.
  Per-pod veths and kubelet per-pod mounts are excluded from the network and filesystem collectors.
- [x] `[chart]` [node-exporter](../node-exporter/#collectors): Swap covered on both axes (`swap`, `meminfo`, `vmstat`,
  and `pressure`), with the `vmstat` filter widened to admit `pgscan_*` and `pgsteal_*`.
- [x] `[chart]` [node-exporter](../node-exporter/#sizing): 64Mi memory request **and** limit, 10m CPU request, and **no CPU limit**.
- [x] `[chart]` [Loki](../loki/#protective-limits): `reject_old_samples: true` and `reject_old_samples_max_age` set.
- [x] `[chart]` [Loki](../../../logs-and-events/storing/): Compactor `retention_enabled: true`, with
  `delete_request_store` and `retention_delete_delay` configured.
- [x] `[chart]` [Loki](../loki/#per-size-resources): Results cache (query-frontend), chunks cache, and index and stats
  cache on the bundled memcached, with query-result caching enabled (`cache_results`, `max_cache_freshness_per_query`).
- [x] `[chart]` [Loki](../loki/): Query-frontend at 2 or more replicas for queue fairness, with query splitting and parallelism configured.
- [x] `[chart]` [Thanos](../thanos/#thanos-per-size-resources): Resource requests on every enabled component, per the profile tables.
- [x] `[chart]` [Thanos](../thanos/#thanos-retention-compaction): Compactor retention at raw 30d, 5m 90d, 1h 365d, and
  **replicas deduplicated in the bucket** (`--deduplication.replica-label=receive_replica`), so the bucket holds one copy of each series.
- [x] `[chart]` [Thanos](../thanos/#thanos-retention-compaction): Receive TSDB **local retention 6h** (overriding the
  subchart's 24h) with WAL compression, so the `emptyDir` budget fits a modest node.
- [x] `[chart]` [Thanos](../thanos/): **Horizontal autoscaling on Query** (2–5 replicas, 80% CPU), and on Query Frontend when it's enabled.
  Store Gateway autoscaling is deliberately **off**: it's a PVC-backed StatefulSet that syncs the bucket index on
  startup, so scale-up serves nothing until it's warm, and scale-down orphans PVCs.
- [x] `[chart]` [Grafana](../grafana/): Resource **requests and limits** (100m / 256Mi requests, 1Gi memory limit), and
  no CPU limit, because query rendering is bursty.
- [x] `[chart]` [Grafana](../grafana/): **HPA** surfaced and off by default, because it's meaningless on SQLite.
  The `grafana-postgres` profile turns it on (2–5 replicas, 60% CPU), and `minReplicas` becomes the effective floor.
  A validator warns when `autoscaling.targetCPU` is set with no CPU request.
- [x] `[chart]` [Grafana](../grafana/): grafana-operator **requests 10m / 64Mi**, with a 512Mi memory limit.
  The subchart sets none, and a BestEffort pod is evicted under node pressure ahead of every pod within its requests, whatever its priority class.
- [x] `[chart]` [Alertmanager](../alertmanager/): Resource **requests and a memory limit** on both containers (10m /
  64Mi, 256Mi limit for Alertmanager), and no CPU limit.
  A quiet install idles near 15Mi; the limit sits well above the request because it's the one setting that can take both
  replicas down together during an alert storm.

</details>

## Hardening and extras {#hardening-and-extras}

Skip one of these and nothing breaks.
Each adds defense in depth, or extra signal where your environment calls for it.

- [ ] `[operator]` [Network policies](../stack-wide/#network-policies): **Narrow the `0.0.0.0/0` egress rules where your infrastructure lets you.**
  `kube-state-metrics` and `grafana-operator` reach the API server on `443` and `6443` to everywhere, because a chart
  can't derive the control plane's address, and Loki reaches object storage the same way.
  A VPC endpoint's CIDR, or Cilium's `toFQDNs`, is the tighter form; for Loki, see [Object-storage egress](../loki/#object-storage-egress).
- [ ] `[operator]` [TLS](../../securing/#gaps): Turn on in-cluster TLS where your environment requires it.
  Certificates and the mTLS profiles ship off by default.
- [ ] `[operator]` [Alertmanager](../../../alerting/architecture/#tls): **TLS on the API port** comes with the mTLS
  profiles: `alerting.server.tls` moves 9093 to TLS, and the profiles move the probes, the reloader, the scrape, both
  rulers, and the Grafana data source with it.
  `VerifyClientCertIfGiven` at phase 2 is as far as it goes, because the kubelet's probes and the reloader can't present a certificate.
- [ ] `[operator]` [node-exporter](../node-exporter/#exposure): TLS client authentication on `/metrics` is available and
  parked (`kubeRBACProxy.tls.tlsClientAuth` plus `tlsSecret`).
  Turn it on if your environment requires authenticated scrapes, accepting a second container on every node.
- [ ] `[operator]` [Images](../stack-wide/#images-and-registries): **Mirror at the node if you can.**
  A containerd `hosts.toml` or an `ImageDigestMirrorSet` redirects pulls without touching any chart value, which keeps one mapping in one place.
  Reach for `mirror.values.yaml` when you can't configure nodes, or when policy wants the mirror named in the manifest.
- [ ] `[operator]` [Grafana](../grafana/#plugins): **Pin plugin versions** (`name@version`) or bake them in, so a plugin
  can't change underneath a pinned Grafana.
  A validator warns on an unpinned entry.
- [ ] `[operator]` [Thanos](../thanos/): If you re-enable VPA deliberately, use `updateMode: "Off"` for recommendations
  only, and know that its numbers disagree with the profile by design.
- [ ] `[operator]` [node-exporter](../node-exporter/#collectors): **On AWS, consider enabling `ethtool`** with
  `--collector.ethtool.device-include=^(eth|ens|enp)`.
  It's where the ENA allowance counters live (`bw_in_allowance_exceeded`, `bw_out_allowance_exceeded`,
  `pps_allowance_exceeded`, `conntrack_allowance_exceeded`): instance-level network throttling that's invisible in every other metric.
  It's off by default because the stat set is driver-specific, so validate it on your AMI before relying on it.
- [ ] `[operator]` [node-exporter](../node-exporter/#collectors): On bare metal, `edac` (ECC errors) and `rapl` are
  worth adding; hypervisors don't expose either to guests, which is why they're off.
- [ ] `[operator]` [node-exporter](../node-exporter/#collectors): Turn `slabinfo` on during an investigation, not as a default.
  It costs about 1.5k series per node and needs a root init container to chmod `/proc/slabinfo`, and `meminfo` already
  reports `Slab`, `SReclaimable`, and `SUnreclaim`.

<details>
<summary>What the chart already does</summary>

- [x] `[chart]` [Images](../stack-wide/#images-and-registries): Every image is **pinned by tag**.
  The images whose cadence shouldn't be tied to a chart release (Grafana, Alloy, and node-exporter) are pinned in this
  chart's own `values.yaml`, so Renovate bumps them on their own cadence.
- [x] `[chart]` [Images](../stack-wide/#images-and-registries): The Alloy image is pinned **by digest as well as tag**,
  so the identity of the running pipeline binary is fixed rather than merely named.
- [x] `[chart]` [Images](../stack-wide/#images-and-registries): **Hardened base images** are a values change: the
  overlays under `profiles/registry/` repoint every image, Grafana included.
- [x] `[chart]` [Alloy](../alloy/): Distroless Alloy image: FIPS boringcrypto, multi-arch, non-root, and published to GHCR.
- [x] `[chart]` [node-exporter](../node-exporter/#exposure): Distroless image (no shell, no package manager), non-root,
  read-only root filesystem, and `automountServiceAccountToken: false`.
  The container reads the host's `/proc`, `/sys`, and `/`, which makes a shell there a materially better foothold than in most containers.
- [x] `[chart]` [Loki](../../securing/#workload-hardening): `runAsNonRoot`, read-only root filesystem, and all
  capabilities dropped on **every** Loki component, memcached included.
- [x] `[chart]` [Grafana](../grafana/#plugins): **Read-only root filesystem**, with an `emptyDir` at `/tmp` for the Unix
  sockets Grafana's backend plugins listen on.
- [x] `[chart]` [Grafana](../grafana/): `analytics.reporting_enabled` and `check_for_updates` are off, and the subchart's `testFramework` hook is off.
- [x] `[chart]` [Alertmanager](../alertmanager/): **Read-only root filesystem**, all capabilities dropped, no privilege
  escalation, `seccompProfile: RuntimeDefault`, and non-root UID 65534, on both the Alertmanager and the config-reloader containers.
  A validator warns when the root filesystem is made writable.
- [x] `[chart]` [Alertmanager](../alertmanager/): **No ServiceAccount token mounted**, because Alertmanager reads nothing from the Kubernetes API.
- [x] `[chart]` [Alertmanager](../alertmanager/#availability): `cluster.label` on every gossip message, so a peer from
  another Alertmanager cluster that inherits a recycled pod IP is rejected rather than merged.

</details>

<!--
Agent note: unbuilt chart work that used to sit on this page as unchecked `[chart]` items, and belongs in Linear rather than here:
kubeconform over the rendered chart in CI (`helm lint` already runs), `loki -verify-config` in CI and as an initContainer,
and the Loki mixin dashboards and alerts.
-->
