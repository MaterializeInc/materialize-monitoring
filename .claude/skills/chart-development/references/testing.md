# Testing Helm Charts

## Installing Helm Unittest Plugin

Installation can be verified by checking `helm plugin list` for `unittest`
and only needs to be run if you have not already installed the plugin.

Helm unittest is a BDD plugin for writing and testing helm unit tests.

`helm plugin install https://github.com/helm-unittest/helm-unittest`

Note that if installation fails due to "plugin source does not support verification"
and you are running helm v4, you will need this temporary (until
[helm-unittest#777](https://github.com/helm-unittest/helm-unittest/issues/777) is resolved)
workaround:

`helm plugin install --verify=false https://github.com/helm-unittest/helm-unittest`

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
