---
title: "Uninstalling"
weight: 45
---

# Uninstalling materialize-monitoring

Uninstalling is not symmetric with installing.
The chart's Grafana resources are reconciled by an operator that also owns their deletion, so tearing the release down in an arbitrary order deadlocks.

**The chart now handles this for you.** A `pre-delete` hook deletes the affected resources while the operator is still running, and blocks until their finalizers clear. Read on for what it is doing and when you still have to intervene.

## The grafana-operator finalizer deadlock

grafana-operator attaches the finalizer `operator.grafana.com/finalizer` to the custom resources it reconciles.
This chart creates two kinds that carry it:

| Kind | Created by |
|---|---|
| `GrafanaDatasource` | the Loki and Thanos datasources |
| `GrafanaManifest` | the bundled dashboards |

The `Grafana` instance CR does **not** carry the finalizer (verified against operator v5.24.0), so Helm removes it unaided.

**Only the operator removes that finalizer.**
Delete the operator first and the finalizer has no remover: every one of those resources sits in `Terminating` forever, `helm uninstall --wait` never returns, and the namespace cannot finish terminating either.

Neither Helm nor Terraform prevents this on its own, and the reason is subtler than delete ordering.
Helm's uninstall order actually issues the custom-resource deletes *before* the operator's Deployment — but it does not **wait** for them to complete.
Those objects enter `Terminating` with their finalizer still set, Helm moves on and deletes the operator, and the finalizer's only remover is gone.
`terraform destroy` inherits the same behaviour through `helm_release`.

## What the pre-delete hook does

`cleanup.grafanaOperator` templates a short-lived Job that runs before Helm deletes anything:

```bash
kubectl delete grafanamanifests.grafana.integreatly.org,grafanadatasources.grafana.integreatly.org \
  --namespace=<release namespace> \
  --selector=app.kubernetes.io/instance=<release name> \
  --ignore-not-found=true --timeout=2m
```

Two details carry the whole design. It runs at `pre-delete`, so the operator is still up and still watching. And `kubectl delete` **waits** by default — the hook returns only once the finalizers have actually been processed, which is precisely what the ordinary uninstall fails to do.

The selector scopes it to one release, so two releases sharing a namespace do not delete each other's resources. (`--all` is not an alternative: kubectl refuses it alongside a selector.)

It runs as a non-root, read-only-rootfs pod using upstream's own distroless `registry.k8s.io/kubectl` image, whose entrypoint is `/bin/kubectl` — there is no shell in the image, and one argv is all this needs.

> [!IMPORTANT]
>   Helm runs pre-delete hooks from the **stored release manifest**, not from the chart on disk. A release installed before this hook existed does not have it, and uninstalling it will still deadlock. Run `helm upgrade` first, or follow the manual teardown below.

Three cases still need you:

- **`helm uninstall --no-hooks`** skips it, by definition.
- **Deleting the CRDs release first** removes the resource types, and the hook fails on an unknown type — no kubectl flag suppresses that. With the CRDs gone the custom resources went with them, so `--no-hooks` is the right response.
- **`cleanup.grafanaOperator.enabled: false`** restores the old behaviour.

> [!WARNING]
>   The same trap is worse one level up. Deleting the CRDs release (`materialize-monitoring-crds`, or `enable_monitoring_crds = true` in the Terraform module) cascades to **every** `Grafana*` and `Prometheus*` custom resource in the cluster, including ones this stack did not create.
>   With finalizers still pending, the CRD itself hangs in `Terminating`, which blocks re-installing the CRDs as well as removing them.

## Ordered teardown, by hand

This is what the hook automates. Do it yourself when the hook is unavailable — a release predating it, `--no-hooks`, or `enabled: false`.

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
