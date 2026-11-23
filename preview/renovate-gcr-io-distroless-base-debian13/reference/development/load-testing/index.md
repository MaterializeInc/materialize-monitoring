# Load testing




`mz-monitoring-benchmark` puts realistic write load on the monitoring stack without running Materialize at scale.
This page is for you if you're a contributor sizing the gateway, Thanos, or Loki, or measuring what a change costs them.

Each benchmark pod impersonates one Materialize process.
It serves that process's metrics endpoints, expanded to the size you ask for, and writes that process's logs at the rates you ask for.
The goal is the load: series counts, churn, scrape sizes, and log volume and shape.
The graphs the load draws aren't meant to look like a real environment's.

## How it works

The tool has two halves.

`fixture-gen`
: Captures each role's real scrapes from a live install and anonymizes them into fixtures under `packages/mz-monitoring-benchmark/fixtures/`.
  User entities become numbered placeholders, and everything a user named or that identifies the install is removed.

`gen-load`
: Serves the fixtures, with every placeholder expanded into as many generated entities as you request, and writes log lines in the role's own format.

The following table lists the roles a pod can take:

| Role | Endpoints | Log format |
|---|---|---|
| `environmentd` | `/metrics`, and the SQL exporter's `/metrics/mz_compute`, `/metrics/mz_frontier`, `/metrics/mz_storage`, and `/metrics/mz_usage` | tracing JSON on stderr, startup banner on stdout |
| `clusterd` | `/metrics` | tracing JSON on stderr |
| `balancerd` | `/metrics` | tracing text on stderr |
| `orchestratord` | `/metrics` | tracing text on stderr |
| `console` | `/metrics` | nginx access log on stdout, error log on stderr |

### Expansion

A fixture series that names a user entity copies itself once per generated entity of that kind.
Generated collection `j` copies the series of exemplar `j mod E`, where `E` is the number of collections in the fixture.
The generated set therefore keeps the captured mix of sources, tables, views, and indexes.
The entities a series also names are derived from the one it's copied for:
collection `j` lives on cluster `j mod C`, and a series that names a replica fans out across its cluster's replicas.
System objects exist in every environment, so they stay literal and are emitted once.

### Identities and values

Every identity a pod generates derives from its seed, which defaults to the pod's name.
A StatefulSet pod keeps its name, so after a restart it serves the same persist shard IDs, which look like `s` followed by a version 4 UUID.

Nothing is stored per series.
Counters are the captured value plus the captured rate times the integral of the scenario's shape.
They never decrease, and their `rate()` follows the shape.
Gauges are the captured value times the shape and per-second noise.
Gauges that hold a wall-clock time, such as frontiers, stay their captured distance behind the clock.
Every count, rate, and value carries a seeded spread of ±10% by default, which you set with `--jitter`.

## Run a load test

The chart in `test/load/` runs the pods, each behind a `ServiceMonitor`.
You need a test install of the stack, whose gateway scrapes every `ServiceMonitor` in the cluster.

1. Install the chart:

   ```bash
   helm install bench test/load --namespace mzmon-bench --create-namespace
   ```

   The chart isn't published, so install it from a checkout.
   Its image tag defaults to the crate's version, the tag a release of the `benchmark-images` component publishes.
   The chart reads that version from `files/Cargo.toml`, a symlink to the crate's manifest.
   To run a dev build, set `--set image.repository=REPOSITORY,image.tag=TAG`.
   Replace `REPOSITORY` and `TAG` with the image you built, such as one that `make CONTAINER_REGISTRY=REGISTRY benchmark-image` built and you pushed.
   A pull request that changes the crate also publishes a dev image tagged `VERSION-dev.0--pr.gSHA`.

1. Watch the targets come up in the `mzmon-bench` namespace, and the load arrive on the **infra-alloy** and **infra-loki** dashboards.

1. Remove the load when you're done:

   ```bash
   helm uninstall bench --namespace mzmon-bench
   ```

The chart's defaults describe one environment.
It has an `environmentd` with 25,000 collections on 10 clusters, one `clusterd` per cluster, and the operator, balancer, and console pods.
To simulate more environments, scale every role's `replicas` by the same factor.
Each pod's seed differs, so every pod adds series of its own.
For every value the chart takes, see its [README](https://github.com/MaterializeInc/materialize-monitoring/blob/main/test/load/README.md).

The pods run in their own namespace, so the gateway's per-namespace log limits meter them apart from real workloads.
Each container takes the name its real process runs under, because the gateway chooses Materialize's log parsing by container name.
That name is the role, except for `orchestratord`, whose container is `materialize-operator`.
The pod labels aren't Materialize's, so the chart's own Materialize monitors and alerts ignore the pods.

## Size the load

The following flags set how many entities a pod generates:

| Flag | Roles | Default |
|---|---|---|
| `--persist-shards` | `environmentd`, `clusterd` | The fixture's shards |
| `--collection-count` | `environmentd`, `clusterd` | The fixture's user collections |
| `--cluster-count` | `environmentd` | The fixture's user clusters |
| `--replicas-per-cluster` | `environmentd` | The fixture's replicas per cluster |

A flag for an entity the role's fixtures never name is an error rather than a silent no-op.

To see what a size costs before you deploy it, run `preview` with the same flags:

```bash
cargo run --release -p mz-monitoring-benchmark -- preview \
  --role environmentd --persist-shards 1024 --collection-count 25000 --cluster-count 10
```

The output gives the series count and size of each endpoint per scrape.
Add `--body PATH` to print an endpoint's body, and `--at SECONDS` to render a later moment of a growing scenario.

## Shape the load over time

The metric and log scenarios share one period, which you set with `--period`, or `period` in the chart's values.
The default is `5m`, and the period has to be at least `1s`.
The period is an argument of its own rather than a key in either scenario, so the two can't run on different clocks.
Every count in either scenario is per period.
A count below 1 fires once every few periods, so at the default period `panic=0.25` is one panic every 20 minutes.

Each shape repeats once per period, aligned to the wall clock, so a metric pulse and a log pulse with the same numbers line up.
Every shape averages to 1 over the period, so a shaped rate keeps its configured mean.
The following shapes are available:

`average`
: Constant.
  `flat` is a synonym.

`bell`
: A Gaussian bump in the middle of the period.

`sine`
: One sine wave per period.

`saw`
: A ramp from nothing at the start of the period to twice the mean at its end.

`pulseD`
: On for the first `D` of each period and off for the rest, such as `pulse180` for three minutes on and two off.
  `D` takes a unit, such as `pulse30s` or `pulse3m`, and is in seconds without one.

`plateauD`
: A ramp up, a hold for `D`, and a ramp down.

### Shape the metrics

`--metric-scenario` takes a comma-separated list of the following keys:

`dist`
: The shape gauges and counter rates follow.
  The default is `average`, which keeps the captured values.

`amp`
: How far the shape moves values, from 0 for none to 1 for the full shape.
  The default is `0.5`.

`collection-grow`, `shard-grow`
: Entities added per period, which grows the series count without bound.

`collection-churn`, `shard-churn`
: Entities replaced per period: the oldest stop being served and new IDs appear.
  Churn is the expensive case for a TSDB head.

### Shape the logs

`--log-scenario` takes a comma-separated list of the following keys:

`trace`, `debug`, `info`, `warn`, `error`
: Mean lines per period at that level.
  Naming any level replaces the default of `info=1500,warn=150,error=15`.

`LEVEL-dist`, `dist`
: The shape of one level's lines, or of every level's when you use `dist`.
  The default is `average`.

`phase`
: `sync`, the default, starts every pod's period on the same wall-clock boundary, so bursts across a fleet land together.
  `seeded` offsets each pod's period.

`non-json`
: Lines per period in no structured format, written to stdout as `environmentd`'s startup banner is.
  Only `environmentd` and `clusterd` accept it, because the other roles' lines are already plain text.

`eicar`
: Lines per period carrying both [secret-filter canaries](../design-docs/20260924-log-secret-filtering/#a-canary-in-the-spirit-of-eicar), at `WARN`.

`panic`
: Multi-line Rust panics per period.
  The console doesn't accept it, because it's nginx.

`panic-exit`
: `true` exits the process after each panic, the way a real panic does, so Kubernetes restarts the pod with the same seed.

Each pod starts every count part of the way to its first line, at a seeded point.
A fleet's rare events, such as panics, therefore don't fire on the same tick.

## Regenerate the fixtures

Regenerate the fixtures when Materialize changes what it exports, and check them against a new Materialize version to see what changed.
Run `fixture-gen` only against a test install you own.
Its anonymizer is built to keep identifiers out of the fixtures, and every fixture still goes through code review before it merges.

The capture needs user objects to take exemplars from.
The maintainers' GCP test install (internal) keeps a small workload for that purpose.
It lives in the `bench_fixture` schema of the install's `materialize-environment` environment.
It holds an `AUCTION` load-generator source, a table, two materialized views, a view, and two indexes.
Don't drop it.

1. Capture and write the fixtures:

   ```bash
   cargo run -p mz-monitoring-benchmark -- fixture-gen \
     --context CONTEXT --namespace NAMESPACE
   ```

   Replace the following:

   - `CONTEXT`: the kube context of the test install.
   - `NAMESPACE`: the namespace of the Materialize environment to capture.

   The command finds one ready pod per role, port-forwards to it, and scrapes each endpoint twice, 15 seconds apart, to measure counter rates.
   The SQL exporter endpoints use the basic auth in the `monitoring/materialize-sql-monitor` Secret, which the chart creates.

1. Read the report.
   It lists the exemplars found per entity kind, any values replaced by a fixed stand-in, and any label the anonymizer has no rule for.
   An unclassified label's values are only scrubbed of recognizable patterns, such as UUIDs, addresses, and host names.
   Classify it in `src/anonymize.rs` before you commit.

   The command also checks every anonymized fixture for addresses, host names, UUIDs, raw user IDs, and cloud regions.
   Its own stand-ins pass, and it writes nothing when anything else turns up.

1. Run the tests, which repeat that check on the committed fixtures and render every role at scale:

   ```bash
   cargo test -p mz-monitoring-benchmark
   ```

To check a Materialize version against the committed fixtures without writing anything, add `--check`.
The command fails when a family in both has a different type or different label names.
It lists families found on one side only without failing.
The SQL exporter emits a family only while its query returns rows, so one capture can't tell a removed family from an idle one.
Add `--strict` to fail on unclassified labels as well.

### What the anonymizer removes

The anonymizer treats each kind of value as follows:

| Value | Becomes |
|---|---|
| Persist shard IDs, and user collection, cluster, and replica IDs (`u` followed by digits) | Numbered placeholders, such as `${shard:3}` |
| Introspection collection IDs on a user replica | Placeholders numbered per replica |
| Names of user objects, schemas, databases, clusters, and replicas | Attributes of the entity's placeholder, such as `${collection:3.name}` |
| Environment IDs and other UUIDs | Placeholders that each pod fills from its seed |
| Application names, zones, and replica sizes outside Materialize's own | Fixed stand-ins, such as `application_0` and `zone-a` |
| IPv4 addresses, host names, email addresses, long hex strings, and environment resource IDs | Fixed stand-ins |
| System object IDs and names, enumerations, and versions | Unchanged |

The log message catalog isn't captured.
A log message is free text that no rule set can promise to clean.
The catalog in `src/logs.rs` is written by hand after lines seen on a live install, and every identifier in it is generated.

## Limits

The following list describes what the load doesn't reproduce:

- **Cross-pod consistency:** a `clusterd` pod's collections aren't the ones an `environmentd` pod lists.
- **Environment labels:** the pods carry no `materialize.cloud/organization-name` label, so dashboards filtered by environment don't show them.
- **Compression:** the endpoints answer uncompressed, as Materialize's do.
- **Log content:** messages come from a fixed catalog, so their variety is lower than a real environment's.

