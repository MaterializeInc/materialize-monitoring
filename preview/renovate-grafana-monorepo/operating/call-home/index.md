# Call-home




# Call-home

Call-home sends a reduced copy of your install's metrics and logs to Materialize,
so Materialize can see what's running and what's wrong without a support ticket.
Your own Loki and Thanos keep the full copy, and nothing is sent unless you turn call-home on.
This page is for the operator of a `materialize-monitoring` install who decides whether to turn it on, and how much to send.

<!-- more -->

Call-home is one profile, `profiles/call-home.values.yaml`.
It adds a single destination, `controlPlane`, to [`pipeline.destinations`](../../logs-and-events/storing/#destinations), turns the gateway's [secret filter](#secret-filter) on, and changes nothing else.
Bring Your Own Cloud stacks apply the same profile.
The only difference between the two is how the client certificate is issued, which happens outside this chart.

## What the profile sends {#what-crosses}

The following table lists what the profile sends as written, and the setting under `pipeline.destinations.controlPlane` that controls each part:

| Signal | Sent | Setting |
|---|---|---|
| Metrics | The `essential` tier | `metrics.minMetricImportance` |
| Logs | `WARN` and above | `logs.minLevel` |
| Logs | At most 500 lines per second per gateway replica | `logs.rateLimit` |
| Logs | Not lines whose level the gateway couldn't determine | `logs.keepUnknown` |

The metric tiers are listed in [Controlling what each destination stores](../../metrics/storing/#controlling-what-each-destination-stores).

The gateway levels most lines, including Rust and Go panics, which arrive as `CRITICAL`.
A line it can't level is `UNKNOWN`, and the profile doesn't send those.
To send them as well, set `logs.keepUnknown` to `true`.

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
   Without it, the gateway fails to load its configuration and names the missing file.

1. Add the profile to your install, after the profiles you already use.

   With Helm, add it to the `-f` list of your upgrade command.
   The profiles ship inside the chart package; for more information, see [Get the profiles](../../getting-started/helm/#get-the-profiles).

   ```bash
   helm upgrade RELEASE oci://ghcr.io/materializeinc/helm-charts/materialize-monitoring \
     --version VERSION --namespace NAMESPACE \
     -f YOUR_VALUES.yaml \
     -f materialize-monitoring/profiles/call-home.values.yaml
   ```

   With the Terraform module, append the profile's contents to `additional_values`, for example with `file()`.

1. Confirm the gateway is sending.
   After the gateway restarts, `otelcol_exporter_sent_metric_points_total` and `otelcol_exporter_sent_log_records_total` rise
   for `component_id="otelcol.exporter.otlphttp.dest_controlPlane"`.

The certificate is re-read every five minutes, so a renewed certificate in the Secret is picked up without a restart.

## Credentials are redacted before anything is sent {#secret-filter}

The profile sets `pipeline.logging.secretFilter.mode` to `on`.
The gateway then replaces each credential it finds in a log line with `<redacted:RULE_ID>`, before it copies anything out of the line or sends it anywhere.
Neither your own Loki nor the control plane's copy holds a credential the filter caught.

Keep the filter on while logs are sent.
The render warns if `controlPlane` sends logs with the mode set to anything else; shadow mode doesn't count, because it redacts only the copies it stores.
The filter is an extra layer of protection, not the control: configure your components not to log credentials in the first place.
For what the filter catches and what it costs, see [Secret filtering](../production-best-practices/secret-filtering/).

## Choose how much to send {#levels}

Override the profile's settings in your own values file, after the profile.
The following example sends errors and above, and recommended metrics:

```yaml
pipeline:
  destinations:
    controlPlane:
      logs:
        minLevel: ERROR
      metrics:
        minMetricImportance: recommended
```

To send metrics and no logs, set `pipeline.destinations.controlPlane.logs.enabled` to `false`.

Each setting only reduces what the control plane receives.
Your own Loki and Thanos keep everything.

## Turn off call-home {#disable}

Remove the profile from your install, or set `pipeline.destinations.controlPlane.enabled` to `false`.
The gateway stops sending at its next restart, which the upgrade triggers.

## When the control plane is unreachable {#unreachable}

The control plane's copy is the only one that loses data.
When the gateway can't reach it, the destination's queue fills and then drops entries, and delivery to your own Loki and Thanos continues.
The drops are counted in `otelcol_exporter_enqueue_failed_log_records_total` and `otelcol_exporter_enqueue_failed_metric_points_total`.

## See more

- [Where the gateway writes logs](../../logs-and-events/storing/#destinations): every field a destination takes.
- [Call-Home design](../../reference/development/design-docs/20260917-call-home-self-managed/): the consent model this profile is a first step toward.

