# Call-home




# Call-home

Call-home sends a reduced copy of your install's telemetry to Materialize,
so Materialize can see what's running and what's wrong without a support ticket.
Your own Loki and Thanos keep the full copy, and nothing is sent unless you choose a call-home level.
This page is for the operator of a `materialize-monitoring` install who decides whether to turn call-home on, and how much to send.

<!-- more -->

One values key, `callHome.level`, chooses how much crosses.
It defaults to `off`, which sends nothing.
Each higher level sends everything the level below it sends, and more.
The level builds a single destination, `controlPlane`, in [`pipeline.destinations`](../../logs-and-events/storing/#destinations),
and changes nothing else.
Bring Your Own Cloud stacks use the same levels.
The only difference between the two is how the client certificate is issued, which happens outside this chart.

Only your values raise the level.
Nothing in the chart reads a level from Materialize.

## Call-home levels {#levels}

The following table lists each level, what it sends, and the profile that sets it:

| Level | What crosses | Profile |
|---|---|---|
| `off` | Nothing. The default. | None |
| `heartbeat` | Nothing. | `profiles/call-home-heartbeat.values.yaml` |
| `alerts` | Nothing. | `profiles/call-home-alerts.values.yaml` |
| `metrics` | Metrics at the `essential` tier. | `profiles/call-home-metrics.values.yaml` |
| `diagnostics` | Metrics at the `essential` tier, and logs at `WARN` and above. | `profiles/call-home.values.yaml` |

<!-- TODO(DEP-281, DEP-282): `heartbeat` gains the heartbeat record and `alerts` gains alert state; update both rows and the profiles' headers. -->

Each profile sets `callHome.level` and nothing else.

At `diagnostics`, the gateway sends at most 500 log lines per second per gateway replica, in bursts of at most 5000.
It levels most lines, including Rust and Go panics, which arrive as `CRITICAL`.
A line it can't level is `UNKNOWN`, and no level sends those.

The metric tiers are listed in [Controlling what each destination stores](../../metrics/storing/#controlling-what-each-destination-stores).

## Turn on call-home {#enable}

Before you begin, get a client certificate for your install from Materialize.
Its URI SAN, `spiffe://c.materialize.cloud/stack/CLIENT_STACK`, is your install's identity.
The control plane reads your identity from the certificate, so the gateway sends no identity of its own.

1. Create a `kubernetes.io/tls` Secret named `mzmon-alloy-gateway-destination-tls` in the gateway's namespace:

   ```bash
   kubectl create secret tls mzmon-alloy-gateway-destination-tls \
     --namespace NAMESPACE --cert CERT_FILE --key KEY_FILE
   ```

   Replace the following:

   - `NAMESPACE`: the namespace the Alloy gateway runs in.
   - `CERT_FILE` and `KEY_FILE`: the certificate and private key Materialize issued.

   The gateway mounts the Secret at `/etc/mzmon/destination-tls`.
   At `metrics` and above, the gateway fails to load its configuration without it, and names the missing file.

1. Add the profile for your level to your install, after the profiles you already use.

   With Helm, add it to the `-f` list of your upgrade command.
   The profiles ship inside the chart package; for more information, see [Get the profiles](../../getting-started/helm/#get-the-profiles).

   ```bash
   helm upgrade RELEASE oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring \
     --version VERSION --namespace NAMESPACE \
     -f YOUR_VALUES.yaml \
     -f materialize-monitoring/profiles/call-home-metrics.values.yaml
   ```

   With the Terraform module, append the profile's contents to `additional_values`, for example with `file()`.

1. Confirm the gateway is sending.
   After the gateway restarts, the install notes show the level, and `otelcol_exporter_sent_metric_points_total` rises
   for `component_id="otelcol.exporter.otlphttp.dest_controlPlane"`.
   At `diagnostics`, so does `otelcol_exporter_sent_log_records_total`.

The certificate is re-read every five minutes, so a renewed certificate in the Secret is picked up without a restart.

## Credentials are redacted before anything is sent {#secret-filter}

At `diagnostics`, the gateway's secret filter is on, whatever `pipeline.logging.secretFilter.mode` says.
The gateway then replaces each credential it finds in a log line with `<redacted:RULE_ID>`,
before it copies anything out of the line or sends it anywhere.
Neither your own Loki nor the control plane's copy holds a credential the filter caught.
The level overrides shadow mode too, because shadow mode redacts only the copies it stores.

Below `diagnostics`, no logs cross, and the mode is yours to set.

The filter is an extra layer of protection, not the control: configure your components not to log credentials in the first place.
For what the filter catches and what it costs, see [Secret filtering](../production-best-practices/secret-filtering/).

## Send less than a level {#narrow}

To send less than your level, write an entry under `pipeline.destinations.controlPlane` in your own values file, after the profile.
The level builds the destination, and your entry is applied over it.
The following example sends errors and above from Materialize's environments, at no more than 100 lines per second:

```yaml
pipeline:
  destinations:
    controlPlane:
      logs:
        minLevel: ERROR
        selector: '{namespace=~"environment-.+"}'
        rateLimit:
          rate: 100
          burst: 1000
```

To send metrics and no logs, set `pipeline.destinations.controlPlane.logs.enabled` to `false`, or use the `metrics` level.

The entry can also change the wire, such as `url`, `tls`, and `queue`.
It can't send more than the level.
The render fails on any of the following:

- Metrics below `metrics`, or a `metrics.minMetricImportance` other than `essential`.
- Logs below `diagnostics`, a `logs.minLevel` below `WARN`, or `logs.keepUnknown: true`.
- A `logs.rateLimit.rate` above 500, or a `logs.rateLimit.burst` above 5000.
- A `type` other than `otlp`.

Your own Loki and Thanos keep everything at every level.

## Turn off call-home {#disable}

Remove the profile from your install, or set `callHome.level` to `"off"`.
Quote it: YAML reads a bare `off` as `false`, which the chart also accepts as `off`.
The gateway stops sending at its next restart, which the upgrade triggers.

At `off`, the render fails if `pipeline.destinations.controlPlane` is still in your values, unless it's `enabled: false`.
That way an entry written by hand, without a level, doesn't stop sending silently.

## When the control plane is unreachable {#unreachable}

The control plane's copy is the only one that loses data.
When the gateway can't reach it, the destination's queue fills and then drops entries, and delivery to your own Loki and Thanos continues.
The drops are counted in `otelcol_exporter_enqueue_failed_log_records_total` and `otelcol_exporter_enqueue_failed_metric_points_total`.

## See more

- [Where the gateway writes logs](../../logs-and-events/storing/#destinations): every field a destination takes.
- [Call-Home design](../../reference/development/design-docs/20260917-call-home-self-managed/): the consent model the levels implement.

