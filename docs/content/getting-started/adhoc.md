---
title: "Ad-hoc install"
weight: 45
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Install an ad-hoc stack for troubleshooting

This page is for you if you need a Grafana with metrics in it, on a cluster you don't run day to day, as soon as possible.
Typical cases are a day-1 install, an on-premises cluster you haven't seen before, or a node pool that Materialize already fills.

The install uses `profiles/adhoc.values.yaml`, one file that covers the whole stack:

- Every component runs one replica, with a replication factor of 1.
- Nothing claims a PersistentVolume, so the cluster doesn't need a StorageClass.
- Loki runs as one process on its own filesystem, so it needs no bucket.
- Topology spread constraints are removed, so pods schedule on nodes with no zone labels.
- NetworkPolicies are off.

Losing a pod loses that pod's data, so don't run this profile in production.
Grafana keeps users, API keys, and anything else changed in its UI only until it restarts.
The dashboards, data sources, and folders the charts install come back on their own.

For a lasting install, see [Helm](../helm/).

## Before you begin

You need the following:

- `helm` and `kubectl`, with access to the cluster.
- An object store bucket for Thanos.
  Thanos has no local-disk mode that works across its components, so even an ad-hoc install needs one.
  S3, an S3-compatible store such as Ceph RGW or MinIO, GCS, and Azure Blob Storage all work.
- An address you can reach Grafana on, such as a node IP.

## Install the stack

1. Pull the chart, which includes every profile:

   ```bash
   helm pull oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring --version VERSION --untar
   ```

   Replace `VERSION` with the chart version you install.

1. Create the Secret that Thanos reads its bucket from.
   For S3 or an S3-compatible store:

   ```bash
   kubectl create namespace monitoring
   kubectl create secret generic thanos-objstore-config --namespace monitoring \
     --from-file=objstore.yml=/dev/stdin <<'EOF'
   type: S3
   config:
     bucket: BUCKET
     endpoint: HOST:PORT
     access_key: ACCESS_KEY
     secret_key: SECRET_KEY
     insecure: true
   EOF
   ```

   Replace the following:

   - `BUCKET`: the bucket for Thanos blocks.
   - `HOST:PORT`: the store's endpoint, with no scheme.
   - `ACCESS_KEY` and `SECRET_KEY`: the store's credentials.

   Set `insecure: false` when the store serves HTTPS.
   GCS and Azure take the same Secret with their own `type` and `config`.

1. Install the CRDs:

   ```bash
   helm install mzmon-crds oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-crds \
     --namespace monitoring --version VERSION --wait
   ```

1. Install the chart with the profile:

   ```bash
   helm install mzmon oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring \
     --namespace monitoring --skip-crds --version VERSION \
     -f materialize-monitoring/profiles/adhoc.values.yaml \
     --timeout 15m
   ```

   The install output warns about every production setting the profile turns off.
   Read past those for anything else.

1. Install the dashboards, which are a chart of their own:

   ```bash
   helm install mzmon-dashboards oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring-dashboards \
     --namespace monitoring --version VERSION
   ```

   Each chart has its own version, so replace `VERSION` in each command.

1. Check that the pods are running and that the dashboards reached Grafana:

   ```bash
   kubectl get pods --namespace monitoring
   kubectl get grafana,grafanamanifest --namespace monitoring
   ```

## Reach Grafana

A port-forward works, but it drops connections when a dashboard opens many requests at once.
On a cluster whose nodes you can reach, a NodePort Service is steadier.
`kubectl expose` copies the selector and ports from the existing Service:

```bash
kubectl expose service grafana --namespace monitoring --name grafana-nodeport --type NodePort
kubectl get service grafana-nodeport --namespace monitoring -o jsonpath='{.spec.ports[0].nodePort}'
```

Open `http://NODE_IP:NODE_PORT`.
Replace `NODE_IP` with the address of any node, and `NODE_PORT` with the port the second command prints.

The admin password is in the `grafana` Secret:

```bash
kubectl get secret grafana --namespace monitoring -o jsonpath='{.data.admin-password}' | base64 -d
```

Delete the `grafana-nodeport` Service when you're done; Helm doesn't manage it.

## Adjust for the cluster

The profile marks each setting you're likely to change with `SWITCH`.
Edit your copy of `adhoc.values.yaml`, or pass the key with `--set`:

NetworkPolicies
: Each key that renders one is listed, so you can turn a single component's policy back on.
  If you turn them on, see [grafana-operator times out reaching Grafana](../../operating/o11y-troubleshooting/#grafana-operator-timeout) for clusters that run NodeLocal DNSCache.

DNS Service name
: Loki's gateway resolves through a Service named `kube-dns`.
  Set `loki.global.dnsService` when the cluster names it something else.
  For more information, see [`host not found in resolver`](../../operating/o11y-troubleshooting/#host-not-found-in-resolver).

metrics-server
: Off, as at the chart default, because most clusters already serve `metrics.k8s.io`.
  Turn it on when `kubectl get apiservice v1beta1.metrics.k8s.io` finds nothing.

Loki
: Turn it off when the cluster can't fit one more pod.
  Also turn off the Alloy gateway's Loki destination, which the profile lists beside it.

To keep logs in object storage, add `-f materialize-monitoring/profiles/s3-compatible.values.yaml` after `adhoc`.
Its header has the commands for the two Secrets it reads.

## Clean up

Uninstall the releases in the reverse order you installed them:

```bash
helm uninstall mzmon-dashboards --namespace monitoring
helm uninstall mzmon --namespace monitoring
helm uninstall mzmon-crds --namespace monitoring
```

The following outlive the releases:

- The CRDs, which the CRDs release leaves in place.
  Other installs in the cluster might use them.
- The Secrets you created, and any NodePort Service.
- The Thanos blocks in the bucket.

For when and how to remove the CRDs, see [Uninstalling](../../operating/uninstalling/).
