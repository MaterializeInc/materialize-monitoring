# Alertmanager




# Alertmanager

This page explains how the bundled Alertmanager is shaped for production.
For the architecture these settings configure, see [Alert architecture](../../../alerting/architecture/), and for
routing and receivers, see [Alert channels](../../../alerting/channels/).
What to do about them is in the [production checklist](../checklist/).

Alertmanager is the one component in this stack whose failure is silent by construction.
A notifier that's down, partitioned, or misconfigured produces no alert about itself, and the absence of a page reads
the same as the absence of a problem.
So the defaults target surviving a bad day rather than a minimal footprint, and the validators concentrate on the configurations that fail quietly.

| | Default | Production |
|---|---|---|
| Replicas | 2, gossiping, one per zone | 2 |
| State | A 4Gi volume per replica, replicated by gossip | Unchanged |
| Receivers | None; every alert reaches `mzmon-null` | At least one receiver per class the chosen preset names |
| Credentials | A mounted, optional `alertmanager-receivers` Secret | That Secret, provisioned by the secret-management tooling already in use |
| Reachability | `ClusterIP`, and Grafana's Alertmanager data source | Unchanged, or an authenticated ingress with `baseURL` set |

**The one production step the chart can't take is configuring a receiver.**
Everything else ships as a default or is checked at render time.

## Availability {#availability}

Two replicas gossip over `9094` by default.
A single notifier is lost to an ordinary node drain, and it holds the only copy of every silence.

Both rulers notify **every** replica, through DNS discovery of the headless Service rather than the load-balanced one.
Gossip replicates silences and the notification log, not alerts, so a replica a ruler never reached can't notify when its peer is lost.
For more information, see [Why the rulers address every replica](../../../alerting/architecture/#every-replica).

The replicas spread **hard across zones and soft across hosts**, so two replicas are never one notifier to a zone outage.
The zone rule sets no `minDomains`, so single-zone clusters schedule.
It sets no `matchLabelKeys` either, because that would let the upgrade from one replica to two place both in one zone.
On a cluster whose nodes carry no zone label at all, see [Fewer than two zones](../stack-wide/#fewer-than-two-zones).

Readiness uses `/-/ready` and liveness uses `/-/healthy`, rather than the subchart's `/`, which serves the UI.
A replica joins the mesh and pulls its peer's state before it serves, so a rollout that replaces one replica at a time loses no silences.

## State {#state}

Each replica keeps **a 4Gi volume** for silences and the notification log.
Gossip covers losing one replica; the volume covers losing both at once.
4Gi is the smallest disk GCP Hyperdisk and Azure managed disks provision, not what Alertmanager needs.

The volumes are zonal, which has the following consequences:

- **Use a StorageClass with `volumeBindingMode: WaitForFirstConsumer`**, the default for every managed-cloud CSI class.
  Under `Immediate`, both volumes can be provisioned in one zone before scheduling, and the hard zone rule then leaves the second replica `Pending`.
  On GKE, the legacy `standard` class is `Immediate`; `standard-rwo`, `premium-rwo`, and the Hyperdisk classes aren't.
- **A replica whose zone is lost stays `Pending`** until the zone returns, because its volume can't attach elsewhere.
  The other replica keeps notifying.
  To move the stranded replica, delete its PVC and then its pod; it reschedules into a healthy zone and resyncs from its peer.
  Its own copy of the state is discarded, and its peer's is complete.

`persistence.enabled` and `persistence.size` both render into `volumeClaimTemplates`, which Kubernetes refuses to change.
An upgrade that changes either fails until the StatefulSet is deleted with `--cascade=orphan`, so treat both as immutable.

## Exposure and links {#exposure}

Alertmanager has no authentication of its own, and its API creates silences.
Grafana's Alertmanager data source gives you the alert list and the silence editor behind Grafana's authentication, so
most installs never need to expose Alertmanager itself.

`alertmanager.baseURL` is the address Alertmanager builds links into its own UI from, such as `/#/alerts` and `/#/silences/new`.
Leave it empty unless Alertmanager itself is exposed, and then set it to that address.
**Never point it at Grafana**, which doesn't serve those paths; the render warns when the two share a host.
To link notifications to Grafana, build the links in a template; see [Links in notifications](../../../alerting/channels/#links).

`extraArgs.web.route-prefix: /` keeps every endpoint at the root.
A `baseURL` with a path then doesn't move the API both rulers post to, the probes, the reloader, or the data source.
The render fails when a `baseURL` has a path and the pin has been removed.

## See also

- [Alert architecture](../../../alerting/architecture/): the notifier's shape, state, and failure modes.
- [Alert channels](../../../alerting/channels/): receivers, classes, presets, and credentials.
- [Maintenance windows](../../../alerting/maintenance/): silences, mute windows, and inhibition.
- [Alertmanager configuration reference](https://prometheus.io/docs/alerting/latest/configuration/) (official): every
  receiver integration and route option.

