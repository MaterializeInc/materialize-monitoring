# Testing Helm Charts

## Installing Helm Unittest Plugin

Helm unittest is a BDD plugin for writing and testing helm unit tests.

`make helm-unittest-install`

`helm plugin list` reports the installed version, which should match
`HELM_UNITTEST_VERSION` in the Makefile.

The target exists because the version matters.
The plugin does not shell out to the `helm` binary.
It renders through a Helm library compiled into it, so the plugin version alone
decides what a test sees.
v1.1.0 moved that library to Helm 3.20, which stopped dropping null values during
coalescing, and every Loki config checksum in the committed snapshots moved with it.
A plugin installed at some other version will regenerate snapshots that CI rejects.

The target also absorbs two installation quirks.
`helm plugin install` refuses while any version of the plugin is present, so the
target uninstalls first.
Helm 4 rejects the unsigned upstream source with "plugin source does not support
verification" unless `--verify=false` is passed, and that flag does not exist on
Helm 3.
The target detects which major version is on PATH and passes the flag only where
it is accepted.
This workaround stands until
[helm-unittest#777](https://github.com/helm-unittest/helm-unittest/issues/777) is
resolved.

## Helm Unittest Documentation

The general upstream documentation for helm unittest is available in
[helm-unittest/DOCUMENT.md](https://github.com/helm-unittest/helm-unittest/blob/main/DOCUMENT.md).

### Helm Unittest Schema

The JSON schema for helm unittest suites can be found at
[helm-unittest/schema/helm-testsuite.json](https://raw.githubusercontent.com/helm-unittest/helm-unittest/refs/heads/main/schema/helm-testsuite.json).

Prefer that YAML test suites include this schema as a comment at the top for
improved editor support:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/helm-unittest/helm-unittest/main/schema/helm-testsuite.json
# More comments about this test suite
# go on the following lines
suite: my-test-suite
```

## Deterministic Test Outputs

It is important to ensure that inputs that would otherwise change between
test runs are set to static stub values to not require changes on every update.

Generally, you should set a top-level `release` and `chart` object in your test suite
like such:

```yaml
suite: my-test-suite
chart:
  version: "1.2.3"
  appVersion: "1.2.3"
release:
  name: my-release
  namespace: my-namespace
```

### `chart:` propagates to subcharts — pick `version` and `appVersion` separately

`ModifyChartMetadata` applies the `chart:` stub to the chart under test **and to
every dependency**, so in a snapshot suite over subchart templates the stub is
what freezes the subchart's own metadata. The two fields are worth deciding
independently, because they surface in different places:

| Field | Where it surfaces | Bump churn is |
|---|---|---|
| `version` | `helm.sh/chart: <name>-<version>` on every rendered object | noise — no reviewable content |
| `appVersion` | `app.kubernetes.io/version`, and the default image tags | signal — the thing you want to read |

So for a subchart snapshot suite, stub `version` and **leave `appVersion`
real**. A chart-version bump then needs no snapshot update at all, while an
appVersion bump still trips every affected snapshot and puts the image diff in
front of a reviewer. The `loki_snapshots*_test.yaml` suites do exactly this;
before the `version` stub, a loki chart bump churned 123 pure-noise
`helm.sh/chart:` lines across the three `.snap` files.

`matchSnapshot` cannot substitute for this. It takes only `path`,
`matchRegex.pattern`, and `notMatchRegex.pattern` — there is no ignore or mask —
and scoping with `path: spec` does not help, because the chart label sits in both
`metadata.labels` and `spec.template.metadata.labels`.

## BDD Unit Testing

Behavior Driven Development (BDD) unit tests are used to test the logic of
template functions and outputs as a more black box.
Since BDD tests should have very focused inputs and outputs, they are generally
expected to only be updated when the corresponding template logic changes.
