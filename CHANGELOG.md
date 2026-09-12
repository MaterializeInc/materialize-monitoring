# materialize-monitoring Changelog

<!-- This repo uses different versioning streams for its artifacts.
Artifacts are mapped out in packages/components.yaml.
Unreleased sections are placeholders ("_Changes Pending_") until a
version-update/<component> PR populates and releases them; that PR also bumps
the component's version_paths. See reference/internal/versioning.md and
reference/internal/releasing.md.
-->

## materialize-monitoring Optional CRDs v0.5.0 (Unreleased)

_Changes Pending_

## materialize-monitoring (Helm chart + Terraform module) v0.22.0 (Unreleased)

_Changes Pending_

## Dashboards v0.16.0 (Unreleased)

_Changes Pending_

## Dashboards v0.15.0

* DEP-242 Add infra-nodes dashboard
    * [materialize-monitoring#311](https://github.com/MaterializeInc/materialize-monitoring/pull/311)
    * Adds an infra-nodes dashboard that is installed by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate indexmap to v2.14.1
        * [materialize-monitoring#309](https://github.com/MaterializeInc/materialize-monitoring/pull/309)
        * [`v2.14.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2141-2026-08-28)

## materialize-monitoring (Helm chart + Terraform module) v0.21.0

* DEP-242 Add infra-nodes dashboard
    * [materialize-monitoring#311](https://github.com/MaterializeInc/materialize-monitoring/pull/311)
    * Adds an infra-nodes dashboard that is installed by default
* DEP-209 Add Infrastructure Logs & Events Dashboard
    * [materialize-monitoring#307](https://github.com/MaterializeInc/materialize-monitoring/pull/307)
    * Adds new infra-logs dashboard that is enabled by default

### Dependencies

* Included Dashboards @ v0.15.0..v0.16.0
* Included Pipelines @ v0.12.0..v0.13.0
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate indexmap to v2.14.1
        * [materialize-monitoring#309](https://github.com/MaterializeInc/materialize-monitoring/pull/309)
        * [`v2.14.1`](https://redirect.github.com/indexmap-rs/indexmap/blob/HEAD/RELEASES.md#2141-2026-08-28)
    * Update Rust crate hyper to v1.11.1
        * [materialize-monitoring#302](https://github.com/MaterializeInc/materialize-monitoring/pull/302)
        * [`v1.11.1`](https://redirect.github.com/hyperium/hyper/blob/HEAD/CHANGELOG.md#v1111-2026-08-27)

## Pipelines v0.13.0 (Unreleased)

_Changes Pending_

## Container Images v0.5.0 (Unreleased)

_Changes Pending_

## Dashboards v0.14.0

* DEP-209 Add Infrastructure Logs & Events Dashboard
    * [materialize-monitoring#307](https://github.com/MaterializeInc/materialize-monitoring/pull/307)
    * Adds new infra-logs dashboard that is enabled by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Update Rust crate hyper to v1.11.1
        * [materialize-monitoring#302](https://github.com/MaterializeInc/materialize-monitoring/pull/302)
        * [`v1.11.1`](https://redirect.github.com/hyperium/hyper/blob/HEAD/CHANGELOG.md#v1111-2026-08-27)

## Prometheus Scrapers v0.5.0 (Unreleased)

_Changes Pending_

## mzmon-lib (shared library) v0.12.0 (Unreleased)

_Changes Pending_

## Dashboards v0.13.0

* DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
    * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
    * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
* DEP-209 Create a dashboard for Materialize logs and events
    * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
    * Adds the new mz-mon-env-logs dashboard, installed by default
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)

## mzmon-lib (shared library) v0.11.0

* DEP-222 Remove the old python implementation for dashboards fully
    * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
* Release Dashboards v0.12.0
    * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)

## materialize-monitoring (Helm chart + Terraform module) v0.20.0

* Update Helm release alloy to v1.12.1
    * [materialize-monitoring#279](https://github.com/MaterializeInc/materialize-monitoring/pull/279)
    * [`v1.12.1`](https://redirect.github.com/grafana/helm-charts/releases/tag/alloy-1.12.1)
    * [`v1.12.0`](https://redirect.github.com/grafana/helm-charts/releases/tag/tempo-1.12.0)
* DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
    * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
    * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default

### Dependencies

* Included Dashboards @ v0.13.0..v0.14.0
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* Included Pipelines @ v0.12.0..v0.13.0
    * DEP-241 Remove deprecated alias `k8s_*` labels from logging pipeline; drop pod to metadata only
        * [materialize-monitoring#300](https://github.com/MaterializeInc/materialize-monitoring/pull/300)
        * **Removed:** the `k8s_namespace`, `k8s_app`, and `k8s_container` Loki stream labels, deprecated to consumers in April, 2026. Use `namespace`, `app` and `container` respectively
        * **Removed:** the `k8s_pod` label has been fully removed with expectation to use the `pod` _structured metadata_. Prefer the `namespace` and `app` labels for label filters. Remember that you can always filter by structured metadata like `{app="my-app"} | pod="my-app-pod-12345"`
* Included Prometheus Scrapers @ v0.4.0..v0.5.0
* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)

## materialize-monitoring (Helm chart + Terraform module) v0.19.0

* DEP-232 Support multiple prometheus RemoteWrite destinations
    * [materialize-monitoring#272](https://github.com/MaterializeInc/materialize-monitoring/pull/272)
    * terraform: add new variables for tuning prometheus remote_write destinations (as a map)
    * **breaking** helm: move prometheusRemoteEntries destinations down one to a keyed map `pipeline.metrics.gateway.destination.prometheusRemoteWrite` -> `pipeline.metrics.gateway.destination.prometheusRemoteWrite.thanos`
* Ensure auto-format on version-update PRs does not update Chart.lock
    * [materialize-monitoring#262](https://github.com/MaterializeInc/materialize-monitoring/pull/262)
* Update Helm release metrics-server to ^3.14.0
    * [materialize-monitoring#260](https://github.com/MaterializeInc/materialize-monitoring/pull/260)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.11.0..v0.12.0
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)

## materialize-monitoring (Helm chart + Terraform module) v0.18.1

* Fix snapshots and memcached registry in latest loki
    * [materialize-monitoring#257](https://github.com/MaterializeInc/materialize-monitoring/pull/257)

## Pipelines v0.12.0

* DEP-241 Remove deprecated alias `k8s_*` labels from logging pipeline; drop pod to metadata only
    * [materialize-monitoring#300](https://github.com/MaterializeInc/materialize-monitoring/pull/300)
    * **Removed:** the `k8s_namespace`, `k8s_app`, and `k8s_container` Loki stream labels, deprecated to consumers in April, 2026. Use `namespace`, `app` and `container` respectively
    * **Removed:** the `k8s_pod` label has been fully removed with expectation to use the `pod` _structured metadata_. Prefer the `namespace` and `app` labels for label filters. Remember that you can always filter by structured metadata like `{app="my-app"} | pod="my-app-pod-12345"`
* DEP-210 Upgrade visibility dashboard
    * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
    * Adds a new mz-env-upgrade dashboard, enabled by default
        * Requires materialize v26.41.0
    * Enable sql scrapers by default
* DEP-232 Support multiple prometheus RemoteWrite destinations
    * [materialize-monitoring#272](https://github.com/MaterializeInc/materialize-monitoring/pull/272)
    * terraform: add new variables for tuning prometheus remote_write destinations (as a map)
    * **breaking** helm: move prometheusRemoteEntries destinations down one to a keyed map `pipeline.metrics.gateway.destination.prometheusRemoteWrite` -> `pipeline.metrics.gateway.destination.prometheusRemoteWrite.thanos`
* DEP-127 Policy for deprecations and breaking changes
    * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
        * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
        * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)

## materialize-monitoring (Helm chart + Terraform module) v0.18.0

* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
* DEP-192 Implement networkpolicies across applications
    * [materialize-monitoring#252](https://github.com/MaterializeInc/materialize-monitoring/pull/252)
* Update docker.io/grafana/grafana Docker tag to v13.2.0
    * [materialize-monitoring#253](https://github.com/MaterializeInc/materialize-monitoring/pull/253)
* Update astral-sh/setup-uv action to v10
    * [materialize-monitoring#240](https://github.com/MaterializeInc/materialize-monitoring/pull/240)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.11.0..v0.12.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)

## materialize-monitoring (Helm chart + Terraform module) v0.17.0

* DEP-204 Expose OTLP and Datadog configs to Terraform
    * [materialize-monitoring#247](https://github.com/MaterializeInc/materialize-monitoring/pull/247)

## Container Images v0.4.0

* Update dependency grafana/alloy to v1.19.2
    * [materialize-monitoring#278](https://github.com/MaterializeInc/materialize-monitoring/pull/278)
    * [`v1.19.2`](https://redirect.github.com/grafana/alloy/releases/tag/v1.19.2)
    * [`v1.19.0`](https://redirect.github.com/grafana/alloy/releases/tag/v1.19.0)
* Update debian:13 Docker digest to f324c7f
    * [materialize-monitoring#287](https://github.com/MaterializeInc/materialize-monitoring/pull/287)

## materialize-monitoring (Helm chart + Terraform module) v0.16.2

* DEP-231 Document and provide examples for DHI and Chainguard images
    * [materialize-monitoring#245](https://github.com/MaterializeInc/materialize-monitoring/pull/245)
* DEP-195 Proposal for TLS authentication
    * [materialize-monitoring#244](https://github.com/MaterializeInc/materialize-monitoring/pull/244)
* DEP-203 DEP-185 Run E2E tests against Tier 2 Terraform; support static s3 creds
    * [materialize-monitoring#241](https://github.com/MaterializeInc/materialize-monitoring/pull/241)
* DEP-230 Fix node logs on bottlerocket
    * [materialize-monitoring#239](https://github.com/MaterializeInc/materialize-monitoring/pull/239)
* DEP-185 Add an E2E test suite
    * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
* DEP-230 Fixes for collecting node logs (journald)
    * [materialize-monitoring#234](https://github.com/MaterializeInc/materialize-monitoring/pull/234)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.10.0..v0.11.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0

## materialize-monitoring (Helm chart + Terraform module) v0.16.1

* DEP-227 DEP-197 Fix s3 endpoint; agent tolerations; relax gateway HPA; add pre-delete for GFx resources
    * [materialize-monitoring#231](https://github.com/MaterializeInc/materialize-monitoring/pull/231)

## Pipelines v0.11.0

* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
    * Provide Datadog queries in documentation
        * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)

## Container Images v0.3.0

* DEP-230 Fixes for collecting node logs (journald)
    * [materialize-monitoring#234](https://github.com/MaterializeInc/materialize-monitoring/pull/234)
* Update debian:13 Docker digest to 34cd9e9
    * [materialize-monitoring#212](https://github.com/MaterializeInc/materialize-monitoring/pull/212)

## materialize-monitoring (Helm chart + Terraform module) v0.16.0

* Update loki helm chart to v18.8.0
    * [materialize-monitoring#225](https://github.com/MaterializeInc/materialize-monitoring/pull/225)
* Update ghcr.io/materializeinc/mzmon-alloy Docker tag to v1.18.1
    * [materialize-monitoring#224](https://github.com/MaterializeInc/materialize-monitoring/pull/224)
* Ensure thanos fits onto default self-managed nodes
    * [materialize-monitoring#223](https://github.com/MaterializeInc/materialize-monitoring/pull/223)
* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Update docker.io/grafana/grafana Docker tag to v13.1.3
    * [materialize-monitoring#221](https://github.com/MaterializeInc/materialize-monitoring/pull/221)
* DEP-190 Provide separate sizing profiles for thanos
    * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
* Update docker.io/grafana/grafana Docker tag to v13.1.2
    * [materialize-monitoring#208](https://github.com/MaterializeInc/materialize-monitoring/pull/208)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.10.0..v0.11.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)

## Pipelines v0.10.0

* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)

## materialize-monitoring (Helm chart + Terraform module) v0.15.0

* CLO-180 Support sidechannel logging path (agent doesn't read its own logs)
    * [materialize-monitoring#202](https://github.com/MaterializeInc/materialize-monitoring/pull/202)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
* Upgrade all subcharts to latest version (Loki 15->18, etc)
    * [materialize-monitoring#198](https://github.com/MaterializeInc/materialize-monitoring/pull/198)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.9.0..v0.10.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0

## mzmon-lib (shared library) v0.10.0

* DEP-222 Port Materialize Environment Overview to rust sdk
    * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
    * Dashboards have been rewritten under a different dashboard framework
        * Dashboard queries have adopted queries from our query registry
* DEP-222 Rust implementation of Grafana Dashboard framework
    * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* DEP-222 Add generated grafana models
    * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
* DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
    * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
* DEP-127 Policy for deprecations and breaking changes
    * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
* DEP-237 Support additional release notes in changelogs
    * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
* Update Rust crate rustls-pki-types to v1.15.1
    * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
* DEP-195 Implement TLS across stack
    * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
* Update Rust crate jsonschema to v0.49.9
    * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
    * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
    * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
    * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
    * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
    * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
* DEP-185 Add an E2E test suite
    * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
* Update Rust crate thiserror to v2.0.20
    * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
    * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
* DEP-187 Scrape cadvisor from kubelet instead of via daemonset
    * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
* Update Rust crate clap to v4.6.6
    * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
    * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
* DEP-190 Provide separate sizing profiles for thanos
    * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
* Update Rust crate jsonschema to v0.49.4
    * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
    * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
* Convert raw blocks into structured configs
    * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)

## materialize-monitoring (Helm chart + Terraform module) v0.14.0

* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)

## materialize-monitoring (Helm chart + Terraform module) v0.13.1

* CLO-111 Remaining fixes for grafana LB/persistence
    * [materialize-monitoring#194](https://github.com/MaterializeInc/materialize-monitoring/pull/194)

## materialize-monitoring (Helm chart + Terraform module) v0.13.0

* Update docker.io/grafana/grafana Docker tag to v13.1.1
    * [materialize-monitoring#193](https://github.com/MaterializeInc/materialize-monitoring/pull/193)
* CLO-111 DEP-202 DEP-196 Production defaults for Grafana
    * [materialize-monitoring#192](https://github.com/MaterializeInc/materialize-monitoring/pull/192)
* More documentation cleanups
    * [materialize-monitoring#187](https://github.com/MaterializeInc/materialize-monitoring/pull/187)

## materialize-monitoring (Helm chart + Terraform module) v0.12.0

* DEP-184 Add Azure Profiles and TF Module Support
    * [materialize-monitoring#183](https://github.com/MaterializeInc/materialize-monitoring/pull/183)

## materialize-monitoring (Helm chart + Terraform module) v0.11.1

* DEP-186 Add an E2E suite for kind + helm/tf
    * [materialize-monitoring#181](https://github.com/MaterializeInc/materialize-monitoring/pull/181)
* DEP-123 Support GCM and handle alloy rolls
    * [materialize-monitoring#179](https://github.com/MaterializeInc/materialize-monitoring/pull/179)

## materialize-monitoring (Helm chart + Terraform module) v0.11.0

* DEP-182 Remaining fixes to get GCP TF working
    * [materialize-monitoring#177](https://github.com/MaterializeInc/materialize-monitoring/pull/177)

## materialize-monitoring (Helm chart + Terraform module) v0.10.0

* DEP-182 materialize-monitoring Terraform Module
    * [materialize-monitoring#171](https://github.com/MaterializeInc/materialize-monitoring/pull/171)

## materialize-monitoring (Helm chart + Terraform module) v0.9.0

* DEP-191 Improve validation for thanos and alloy
    * [materialize-monitoring#170](https://github.com/MaterializeInc/materialize-monitoring/pull/170)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)

## materialize-monitoring Optional CRDs v0.4.0

* Ensure auto-format on version-update PRs does not update Chart.lock
    * [materialize-monitoring#262](https://github.com/MaterializeInc/materialize-monitoring/pull/262)
* Upgrade all subcharts to latest version (Loki 15->18, etc)
    * [materialize-monitoring#198](https://github.com/MaterializeInc/materialize-monitoring/pull/198)
* More documentation cleanups
    * [materialize-monitoring#187](https://github.com/MaterializeInc/materialize-monitoring/pull/187)

## materialize-monitoring Helm Chart v0.8.0

* CLO-111 Grafana Documentation with datasources and bundled resources
    * [materialize-monitoring#164](https://github.com/MaterializeInc/materialize-monitoring/pull/164)
* Move grafana-operator CRDs into the materialize-monitoring-crds chart
    * [materialize-monitoring#161](https://github.com/MaterializeInc/materialize-monitoring/pull/161)
* CLO-153 Add profile example for otel pipelines
    * [materialize-monitoring#159](https://github.com/MaterializeInc/materialize-monitoring/pull/159)

## Pipelines v0.9.0

* CLO-180 Support sidechannel logging path (agent doesn't read its own logs)
    * [materialize-monitoring#202](https://github.com/MaterializeInc/materialize-monitoring/pull/202)
* DEP-187 Collect cAdvisor metrics with Alloy
    * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)

### Dependencies

* Included mzmon-lib (shared library) @ v0.9.0..v0.10.0
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)

## Pipelines v0.8.0

* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Implement a Query Registry for reducing total metric set
        * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)

## Prometheus Scrapers v0.4.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.11.0..v0.12.0
    * DEP-240 Ensure kube-state-metrics metrics do not have namespace overwritten
        * [materialize-monitoring#297](https://github.com/MaterializeInc/materialize-monitoring/pull/297)
        * regression: kube-state-metrics were namespaced as `export_namespace=$target` with `namespace=monitoring` on all because the scraper overwrote that label with the namespace KSM was running in
    * DEP-209 Create a dashboard for Materialize logs and events
        * [materialize-monitoring#296](https://github.com/MaterializeInc/materialize-monitoring/pull/296)
        * Adds the new mz-mon-env-logs dashboard, installed by default
    * DEP-210 Upgrade visibility dashboard
        * [materialize-monitoring#294](https://github.com/MaterializeInc/materialize-monitoring/pull/294)
        * Adds a new mz-env-upgrade dashboard, enabled by default
            * Requires materialize v26.41.0
        * Enable sql scrapers by default
    * Release mzmon-lib (shared library) v0.11.0
        * [materialize-monitoring#290](https://github.com/MaterializeInc/materialize-monitoring/pull/290)
    * DEP-222 Remove the old python implementation for dashboards fully
        * [materialize-monitoring#291](https://github.com/MaterializeInc/materialize-monitoring/pull/291)
    * Release Dashboards v0.12.0
        * [materialize-monitoring#46](https://github.com/MaterializeInc/materialize-monitoring/pull/46)
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Port Materialize Environment Overview to rust sdk
        * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
        * Dashboards have been rewritten under a different dashboard framework
            * Dashboard queries have adopted queries from our query registry
    * DEP-222 Rust implementation of Grafana Dashboard framework
        * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
    * Update Rust crate rustls-pki-types to v1.15.1
        * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
    * DEP-195 Implement TLS across stack
        * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
        * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
        * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
        * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
        * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
        * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
    * Provide Datadog queries in documentation
        * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
    * Update Rust crate thiserror to v2.0.20
        * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
        * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
    * DEP-187 Scrape cadvisor from kubelet instead of via daemonset
        * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
    * Update Rust crate clap to v4.6.6
        * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
        * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
        * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
    * Convert raw blocks into structured configs
        * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
    * DEP-187 Collect cAdvisor metrics with Alloy
        * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * DEP-188 Add node-exporter; support priorityClasses
        * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
    * Update Rust crate clap to v4.6.5
        * [materialize-monitoring#190](https://github.com/MaterializeInc/materialize-monitoring/pull/190)
        * [`v4.6.5`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.4...clap_complete-v4.6.5)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
        * [`v0.12.0`](https://redirect.github.com/astral-sh/uv/blob/HEAD/CHANGELOG.md#0120)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
        * [`v0.49.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0492---2026-07-28)
        * [`v0.49.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0491---2026-07-25)
        * [`v0.49.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0490---2026-07-25)
        * [`v0.48.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0485---2026-07-22)
        * [`v0.48.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0482---2026-07-21)
        * [`v0.48.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0481---2026-07-17)
        * [`v0.48.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0480---2026-07-16)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
        * [`v0.3.4`](https://redirect.github.com/rust-lang/glob/blob/HEAD/CHANGELOG.md#034---2026-07-21)
    * Update Rust crate tokio to v1.53.1
        * [materialize-monitoring#147](https://github.com/MaterializeInc/materialize-monitoring/pull/147)
        * [`v1.53.1`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.1): Tokio v1.53.1
    * Update Rust crate clap to v4.6.4
        * [materialize-monitoring#146](https://github.com/MaterializeInc/materialize-monitoring/pull/146)
        * [`v4.6.4`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#464---2026-07-21)
        * [`v4.6.3`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#463---2026-07-20)
    * CLO-152 Support splitting metrics into tiers
        * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
    * Update Rust crate thiserror to v2.0.19
        * [materialize-monitoring#143](https://github.com/MaterializeInc/materialize-monitoring/pull/143)
        * [`v2.0.19`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.19)
    * Update Rust crate tokio to v1.53.0
        * [materialize-monitoring#139](https://github.com/MaterializeInc/materialize-monitoring/pull/139)
        * [`v1.53.0`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.0): Tokio v1.53.0
    * Update Rust crate serde to v1.0.229
        * [materialize-monitoring#142](https://github.com/MaterializeInc/materialize-monitoring/pull/142)
        * [`v1.0.229`](https://redirect.github.com/serde-rs/serde/releases/tag/v1.0.229)
    * Update Rust crate anyhow to v1.0.104
        * [materialize-monitoring#141](https://github.com/MaterializeInc/materialize-monitoring/pull/141)
        * [`v1.0.104`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.104)
    * Update Rust crate serde_json to v1.0.151
        * [materialize-monitoring#144](https://github.com/MaterializeInc/materialize-monitoring/pull/144)
        * [`v1.0.151`](https://redirect.github.com/serde-rs/json/releases/tag/v1.0.151)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
        * [`v1.13.1`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1131-2026-07-15)
        * [`v1.13.0`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1130-2026-07-09)
        * [`v1.12.4`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1124-2025-06-09)
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Implement a Query Registry for reducing total metric set
        * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
        * [`v4.6.2`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#462---2026-07-15)

## materialize-monitoring Helm Chart v0.7.0

* CLO-153 Metric storage documentation; otlp fixes
    * [materialize-monitoring#155](https://github.com/MaterializeInc/materialize-monitoring/pull/155)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* Drop renovate on legacy/; tune helm updates
    * [materialize-monitoring#135](https://github.com/MaterializeInc/materialize-monitoring/pull/135)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 Replace prometheus-style pipeline with otelcol for processing
    * [materialize-monitoring#115](https://github.com/MaterializeInc/materialize-monitoring/pull/115)
* CLO-152 add schema support for otelcol pipeline blocks
    * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Included Pipelines @ v0.8.0..v0.9.0
* Included Prometheus Scrapers @ v0.3.0..v0.4.0
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)

## materialize-monitoring Helm Chart v0.6.0

* Allow configuring otlpExporter, googleCloudExporter, datadogExporter
    * [materialize-monitoring#106](https://github.com/MaterializeInc/materialize-monitoring/pull/106)

## materialize-monitoring Helm Chart v0.5.0

* CLO-112 Harden Long-Term storage in GCP
    * [materialize-monitoring#103](https://github.com/MaterializeInc/materialize-monitoring/pull/103)

## Pipelines v0.7.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 Replace prometheus-style pipeline with otelcol for processing
    * [materialize-monitoring#115](https://github.com/MaterializeInc/materialize-monitoring/pull/115)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

## mzmon-lib (shared library) v0.9.0

* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
* Update dependency uv_build to >=0.12,<0.13
    * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
* Update Rust crate jsonschema to 0.49.0
    * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
* Update Rust crate glob to v0.3.4
    * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* Update Rust crate regex to v1.13.1
    * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
* CLO-152 Support importance axis for extracted metrics
    * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Update Rust crate tokio to v1.52.4
    * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
* Port metric registry to rust
    * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Update Rust crate clap to v4.6.2
    * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 add schema support for otelcol pipeline blocks
    * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

## Pipelines v0.6.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)

## Pipelines v0.5.0

* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Split loki.write out of main processing pipeline
    * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0

## Prometheus Scrapers v0.3.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Enable alloy pipelines in materialize-monitoring
        * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

## Container Images v0.2.0

* Update dependency grafana/alloy to v1.18.1
    * [materialize-monitoring#218](https://github.com/MaterializeInc/materialize-monitoring/pull/218)
* Update dependency grafana/alloy to v1.18.0
    * [materialize-monitoring#145](https://github.com/MaterializeInc/materialize-monitoring/pull/145)
* Update gcr.io/distroless/base-debian13 Docker digest to f4a335c
    * [materialize-monitoring#102](https://github.com/MaterializeInc/materialize-monitoring/pull/102)
* Update gcr.io/distroless/base-debian13 Docker digest to 7c4468d
    * [materialize-monitoring#85](https://github.com/MaterializeInc/materialize-monitoring/pull/85)
* Update dependency grafana/alloy to v1.17.1
    * [materialize-monitoring#77](https://github.com/MaterializeInc/materialize-monitoring/pull/77)

## mzmon-lib (shared library) v0.8.0

* Update Rust crate jsonschema to 0.47.0
    * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Update Rust crate jsonschema to v0.46.10
    * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Update dependency grafana-foundation-sdk to v0.0.18
    * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Update Rust crate reqwest to 0.13
    * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)

## materialize-monitoring Helm Chart v0.4.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Pin ghcr.io/materializeinc/mzmon-alloy Docker tag to c47e937
    * [materialize-monitoring#90](https://github.com/MaterializeInc/materialize-monitoring/pull/90)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Included Pipelines @ v0.6.0..v0.7.0
    * Split loki.write out of main processing pipeline
        * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Included Prometheus Scrapers @ v0.2.0..v0.3.0
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)

## Dashboards v0.12.0

* DEP-222 Port Materialize Environment Overview to rust sdk
    * [materialize-monitoring#285](https://github.com/MaterializeInc/materialize-monitoring/pull/285)
    * Dashboards have been rewritten under a different dashboard framework
        * Dashboard queries have adopted queries from our query registry
* DEP-222 Rust implementation of Grafana Dashboard framework
    * [materialize-monitoring#280](https://github.com/MaterializeInc/materialize-monitoring/pull/280)
* Provide Datadog queries in documentation
    * [materialize-monitoring#249](https://github.com/MaterializeInc/materialize-monitoring/pull/249)
* DEP-188 Add node-exporter; support priorityClasses
    * [materialize-monitoring#196](https://github.com/MaterializeInc/materialize-monitoring/pull/196)
* CLO-152 Support splitting metrics into tiers
    * [materialize-monitoring#151](https://github.com/MaterializeInc/materialize-monitoring/pull/151)
* CLO-152 Support importance axis for extracted metrics
    * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
* Implement a Query Registry for reducing total metric set
    * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
* Update dependency grafana-foundation-sdk to v0.0.18
    * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * [`v0.0.18`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.17...v0.0.18)
    * [`v0.0.17`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.16...v0.0.17)
    * [`v0.0.16`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.15...v0.0.16)
    * [`v0.0.15`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.13...v0.0.15)
    * [`v0.0.13`](https://redirect.github.com/grafana/grafana-foundation-sdk/compare/v0.0.12...v0.0.13)
* Implement Loki with Production Configuration
    * [materialize-monitoring#48](https://github.com/MaterializeInc/materialize-monitoring/pull/48)
* Add annotations to distinguish dashboards; roadmapping
    * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)

### Dependencies

* Included mzmon-lib (shared library) @ v0.10.0..v0.11.0
    * Release mzmon-lib (shared library) v0.10.0
        * [materialize-monitoring#201](https://github.com/MaterializeInc/materialize-monitoring/pull/201)
    * DEP-222 Add generated grafana models
        * [materialize-monitoring#282](https://github.com/MaterializeInc/materialize-monitoring/pull/282)
    * DEP-238 Refresh vendored grafana foundation sdk schemas; keep updated
        * [materialize-monitoring#275](https://github.com/MaterializeInc/materialize-monitoring/pull/275)
    * DEP-127 Policy for deprecations and breaking changes
        * [materialize-monitoring#271](https://github.com/MaterializeInc/materialize-monitoring/pull/271)
    * DEP-237 Support additional release notes in changelogs
        * [materialize-monitoring#268](https://github.com/MaterializeInc/materialize-monitoring/pull/268)
    * Update Rust crate rustls-pki-types to v1.15.1
        * [materialize-monitoring#256](https://github.com/MaterializeInc/materialize-monitoring/pull/256)
    * DEP-195 Implement TLS across stack
        * [materialize-monitoring#254](https://github.com/MaterializeInc/materialize-monitoring/pull/254)
    * Update Rust crate jsonschema to v0.49.9
        * [materialize-monitoring#209](https://github.com/MaterializeInc/materialize-monitoring/pull/209)
        * [`v0.49.9`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0499---2026-08-09)
        * [`v0.49.8`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0498---2026-08-08)
        * [`v0.49.7`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0497---2026-08-07)
        * [`v0.49.6`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0496---2026-08-06)
        * [`v0.49.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0495---2026-08-05)
    * DEP-185 Add an E2E test suite
        * [materialize-monitoring#233](https://github.com/MaterializeInc/materialize-monitoring/pull/233)
    * Update Rust crate thiserror to v2.0.20
        * [materialize-monitoring#229](https://github.com/MaterializeInc/materialize-monitoring/pull/229)
        * [`v2.0.20`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.20)
    * DEP-187 Scrape cadvisor from kubelet instead of via daemonset
        * [materialize-monitoring#222](https://github.com/MaterializeInc/materialize-monitoring/pull/222)
    * Update Rust crate clap to v4.6.6
        * [materialize-monitoring#219](https://github.com/MaterializeInc/materialize-monitoring/pull/219)
        * [`v4.6.6`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#466---2026-08-06)
    * DEP-190 Provide separate sizing profiles for thanos
        * [materialize-monitoring#210](https://github.com/MaterializeInc/materialize-monitoring/pull/210)
    * Update Rust crate jsonschema to v0.49.4
        * [materialize-monitoring#206](https://github.com/MaterializeInc/materialize-monitoring/pull/206)
        * [`v0.49.4`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0494---2026-08-04)
    * Convert raw blocks into structured configs
        * [materialize-monitoring#203](https://github.com/MaterializeInc/materialize-monitoring/pull/203)
    * DEP-187 Collect cAdvisor metrics with Alloy
        * [materialize-monitoring#200](https://github.com/MaterializeInc/materialize-monitoring/pull/200)
    * Release mzmon-lib (shared library) v0.9.0
        * [materialize-monitoring#114](https://github.com/MaterializeInc/materialize-monitoring/pull/114)
    * Update Rust crate clap to v4.6.5
        * [materialize-monitoring#190](https://github.com/MaterializeInc/materialize-monitoring/pull/190)
        * [`v4.6.5`](https://redirect.github.com/clap-rs/clap/compare/clap_complete-v4.6.4...clap_complete-v4.6.5)
    * Update dependency uv_build to >=0.12,<0.13
        * [materialize-monitoring#168](https://github.com/MaterializeInc/materialize-monitoring/pull/168)
        * [`v0.12.0`](https://redirect.github.com/astral-sh/uv/blob/HEAD/CHANGELOG.md#0120)
    * Update Rust crate jsonschema to 0.49.0
        * [materialize-monitoring#134](https://github.com/MaterializeInc/materialize-monitoring/pull/134)
        * [`v0.49.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0492---2026-07-28)
        * [`v0.49.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0491---2026-07-25)
        * [`v0.49.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0490---2026-07-25)
        * [`v0.48.5`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0485---2026-07-22)
        * [`v0.48.2`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0482---2026-07-21)
        * [`v0.48.1`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0481---2026-07-17)
        * [`v0.48.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0480---2026-07-16)
    * Update Rust crate glob to v0.3.4
        * [materialize-monitoring#149](https://github.com/MaterializeInc/materialize-monitoring/pull/149)
        * [`v0.3.4`](https://redirect.github.com/rust-lang/glob/blob/HEAD/CHANGELOG.md#034---2026-07-21)
    * Update Rust crate tokio to v1.53.1
        * [materialize-monitoring#147](https://github.com/MaterializeInc/materialize-monitoring/pull/147)
        * [`v1.53.1`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.1): Tokio v1.53.1
    * Update Rust crate clap to v4.6.4
        * [materialize-monitoring#146](https://github.com/MaterializeInc/materialize-monitoring/pull/146)
        * [`v4.6.4`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#464---2026-07-21)
        * [`v4.6.3`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#463---2026-07-20)
    * Update Rust crate thiserror to v2.0.19
        * [materialize-monitoring#143](https://github.com/MaterializeInc/materialize-monitoring/pull/143)
        * [`v2.0.19`](https://redirect.github.com/dtolnay/thiserror/releases/tag/2.0.19)
    * Update Rust crate tokio to v1.53.0
        * [materialize-monitoring#139](https://github.com/MaterializeInc/materialize-monitoring/pull/139)
        * [`v1.53.0`](https://redirect.github.com/tokio-rs/tokio/releases/tag/tokio-1.53.0): Tokio v1.53.0
    * Update Rust crate serde to v1.0.229
        * [materialize-monitoring#142](https://github.com/MaterializeInc/materialize-monitoring/pull/142)
        * [`v1.0.229`](https://redirect.github.com/serde-rs/serde/releases/tag/v1.0.229)
    * Update Rust crate anyhow to v1.0.104
        * [materialize-monitoring#141](https://github.com/MaterializeInc/materialize-monitoring/pull/141)
        * [`v1.0.104`](https://redirect.github.com/dtolnay/anyhow/releases/tag/1.0.104)
    * Update Rust crate serde_json to v1.0.151
        * [materialize-monitoring#144](https://github.com/MaterializeInc/materialize-monitoring/pull/144)
        * [`v1.0.151`](https://redirect.github.com/serde-rs/json/releases/tag/v1.0.151)
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
        * [`v1.13.1`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1131-2026-07-15)
        * [`v1.13.0`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1130-2026-07-09)
        * [`v1.12.4`](https://redirect.github.com/rust-lang/regex/blob/HEAD/CHANGELOG.md#1124-2025-06-09)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)
        * [`v4.6.2`](https://redirect.github.com/clap-rs/clap/blob/HEAD/CHANGELOG.md#462---2026-07-15)
    * Enable MZ podmonitors by default
        * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
        * [`v0.47.0`](https://redirect.github.com/Stranger6667/jsonschema/blob/HEAD/CHANGELOG.md#0470---2026-07-08)
    * Implement alloy metrics pipelines
        * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Enable alloy pipelines in materialize-monitoring
        * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
        * [`v2.14.2`](https://redirect.github.com/pydantic/pydantic-settings/compare/v2.14.1...v2.14.2)
        * [`v2.14.1`](https://redirect.github.com/pydantic/pydantic-settings/releases/tag/v2.14.1)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)

## Dashboards v0.11.0

* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0

## Prometheus Scrapers v0.2.0

* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

## materialize-monitoring Optional CRDs v0.3.0

* Move grafana-operator CRDs into the materialize-monitoring-crds chart
    * [materialize-monitoring#161](https://github.com/MaterializeInc/materialize-monitoring/pull/161)

## Container Images v0.1.1

* Update debian Docker tag to trixie-20260623
    * [materialize-monitoring#73](https://github.com/MaterializeInc/materialize-monitoring/pull/73)
* Create distroless images for alloy
    * [materialize-monitoring#72](https://github.com/MaterializeInc/materialize-monitoring/pull/72)

## Container Images v0.1.0

* Bootstrapped

## Dashboards v0.10.0

* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Prometheus Scrapers v0.1.1

* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Add PodMonitors for prometheus.operator
    * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## Prometheus Scrapers v0.1.0

* Bootstrapped

## Dashboards v0.9.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)

## mzmon-lib (shared library) v0.7.0

* Update dependency pydantic-settings to v2.14.2 [SECURITY]
    * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
* Update python Docker tag to v3.14
    * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
* Update Rust crate jsonschema to v0.46.9
    * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
* Update Rust crate anyhow to v1.0.103
    * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
* Update Rust crate itertools to 0.15.0
    * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Add annotations to distinguish dashboards; roadmapping
    * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
* Release Dashboards v0.11.0
    * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
* Release Dashboards v0.10.0
    * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
* Update PR description on bump updates
    * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Generate PodMonitoring resources for GCP
    * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
* Upgrade to rust 1.96
    * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release Dashboards v0.9.0
    * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
* Only upload artifacts while in a draft state
    * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
* Release Dashboards v0.8.0
    * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)

## Pipelines v0.4.0

* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)

## materialize-monitoring Helm Chart v0.3.0

* Implement Loki with Production Configuration
    * [materialize-monitoring#48](https://github.com/MaterializeInc/materialize-monitoring/pull/48)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release materialize-monitoring Helm Chart v0.3.0
    * [materialize-monitoring#17](https://github.com/MaterializeInc/materialize-monitoring/pull/17)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Use global_id to not run into errors on right join
        * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
    * Coalesce object names into dashboards
        * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Included Pipelines @ v0.3.0..v0.4.0
* Included Prometheus Scrapers @ v0.1.1..v0.2.0
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Add PodMonitors for prometheus.operator
        * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)
* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## materialize-monitoring Optional CRDs v0.2.0

* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Dashboards v0.8.0

* Use global_id to not run into errors on right join
    * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
* Coalesce object names into dashboards
    * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## Pipelines v0.3.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.5.0..v0.6.0
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## mzmon-lib (shared library) v0.6.0

* Include artifacts when creating github releases
    * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
* Support generating a release when version bump PRs are merged
    * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
* Support auto-formatting based on labels
    * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
* Generated automated versioning PRs
    * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## materialize-monitoring Helm Chart v0.2.0

### Dependencies

* Included Dashboards @ v0.6.0..v0.7.0
    * Fix cloud compatibility with Environment Monitoring dashboards
        * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
    * Update for self-managed workloads
        * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)
* Included Pipelines @ v0.1.0..v0.2.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)
* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## Dashboards v0.7.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Pipelines v0.2.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## mzmon-lib (shared library) v0.5.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Dashboards v0.6.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

## Pipelines v0.1.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)

## mzmon-lib (shared library) v0.4.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## materialize-monitoring Helm Chart v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Provide helm reference documentation for materialize-monitoring
    * [materialize-monitoring#9](https://github.com/MaterializeInc/materialize-monitoring/pull/9)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)

## materialize-monitoring Optional CRDs v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)

## mzmon-lib (shared library) v0.3.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)

## Dashboards v0.5.0

* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)
