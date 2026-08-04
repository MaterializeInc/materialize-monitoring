---
title: "Uninstalling"
weight: 45
---

# Uninstalling materialize-monitoring

Uninstalling is not symmetric with installing.
The chart's Grafana resources are reconciled by an operator that also owns their deletion, so tearing the release down in an arbitrary order deadlocks.

## The grafana-operator finalizer deadlock

grafana-operator attaches the finalizer `operator.grafana.com/finalizer` to the custom resources it reconciles.
This chart creates three kinds that carry it:

| Kind | Created by |
|---|---|
| `Grafana` | the Grafana instance |
| `GrafanaDatasource` | the Loki and Thanos datasources |
| `GrafanaManifest` | the bundled dashboards |

**Only the operator removes that finalizer.**
Delete the operator first and the finalizer has no remover: every one of those resources sits in `Terminating` forever, `helm uninstall --wait` never returns, and the namespace cannot finish terminating either.

Neither Helm nor Terraform prevents this, and the reason is subtler than delete ordering.
Helm's uninstall order actually issues the custom-resource deletes *before* the operator's Deployment — but it does not **wait** for them to complete.
Those objects enter `Terminating` with their finalizer still set, Helm moves on and deletes the operator, and the finalizer's only remover is gone.
`terraform destroy` inherits the same behaviour through `helm_release`.

> [!WARNING]
>   The same trap is worse one level up. Deleting the CRDs release (`materialize-monitoring-crds`, or `enable_monitoring_crds = true` in the Terraform module) cascades to **every** `Grafana*` and `Prometheus*` custom resource in the cluster, including ones this stack did not create.
>   With finalizers still pending, the CRD itself hangs in `Terminating`, which blocks re-installing the CRDs as well as removing them.

## Ordered teardown

Delete the custom resources **while the operator is still running**, then remove the release:

```bash
kubectl -n monitoring delete grafanadatasources,grafanamanifests,grafanas --all
```

The operator observes the deletions, unregisters each object from the Grafana instance, drops the finalizer, and the objects go away.
Only then:

```bash
helm uninstall mzmon -n monitoring
```

With the Terraform module, the same first step applies before the destroy — the module orders its two releases (the main release is destroyed before the CRDs release), but ordering *within* a release is not something Terraform controls:

```bash
kubectl -n monitoring delete grafanadatasources,grafanamanifests,grafanas --all
terraform destroy
```

> [!TIP]
>   Deleting the `Grafana` instance is what makes the other two finalizable even if their unregistration fails: the operator drops the finalizer once no matching instance exists.
>   That still requires a running operator, so the rule to remember is simply **the operator must outlive its custom resources**.

## Recovering a stuck teardown

If the operator is already gone and resources are wedged in `Terminating`, the finalizer has to be removed by hand.
Nothing else will do it.

```bash
for kind in grafanadatasource grafanamanifest grafana; do
  for name in $(kubectl -n monitoring get "$kind" -o name 2>/dev/null); do
    kubectl -n monitoring patch "$name" --type=merge -p '{"metadata":{"finalizers":null}}'
  done
done
```

This is safe at teardown time and only at teardown time.
The finalizer exists so the operator can unregister the object from Grafana before it disappears; stripping it skips that cleanup, which does not matter when Grafana is being deleted too.

If a **CRD** is stuck in `Terminating`, the pending resources are the cause — clear their finalizers as above and the CRD completes on its own.
Do not strip the finalizer from the CRD itself: that orphans the resources instead of deleting them.

## See more

- [Upgrading](../upgrading/) — the other asymmetric lifecycle operation.
- [Getting Started > Terraform](../../getting-started/terraform/) — teardown through the module.
