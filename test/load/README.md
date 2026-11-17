# Load test manifests

These manifests run one synthetic Materialize environment's worth of `mz-monitoring-benchmark` pods, each behind a `ServiceMonitor`.
Apply them with `kubectl apply -k test/load` after you point `kustomization.yaml` at a pushed image.

For sizing, scenarios, scaling, and regenerating the fixtures, see the
[load testing page](../../docs/content/reference/development/load-testing.md).
