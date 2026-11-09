---
title: "Secret filtering"
weight: 4
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Secret filtering

The gateway can scan every log line for credentials and replace each one it finds with a placeholder before the line is stored.
This page is for you if you operate the stack and decide whether to turn that filter on, and what to do when it catches something.
What to do about it is in the [production checklist](../checklist/).

{{< rfc-2119 >}}

## Keep credentials out of logs at the source {#at-the-source}

**The filter is an extra layer of protection, not the control.**
In a healthy install it SHOULD NOT find a credential.
A redaction usually means a component wrote a credential to its log.
The rules also replace some values that only look like credentials, which [the limits](#what-it-does) list.

The filter protects only what's downstream of the gateway.
The raw line stays where the component wrote it:

- in the container's log file on the node, until log rotation removes it;
- in `kubectl logs`, for anyone allowed to read the component's pod logs;
- in any other agent or tool that reads those files.

So you MUST NOT rely on the filter in place of configuring each component not to log credentials.
Treat a credential the filter redacted as exposed.

When the filter redacts something, take the following steps:

1. Find the line in Loki and the component that wrote it, as [Watch what it catches](#watch) describes.
2. Decide whether the replaced value was a credential.
   The placeholder names the rule, the rest of the line usually shows what the field held, and `kubectl logs` shows the raw line until log rotation removes it.
   If it wasn't a credential, nothing was exposed.
3. Fix the source, so the component stops logging the credential.
4. Rotate the credential.

## What the filter does {#what-it-does}

The filter runs in the gateway after a multi-line record such as a panic is reassembled, and before the gateway parses the line.
So the structured metadata the gateway derives from a line, such as `msg` and `error`, comes from text that's already redacted.

Each credential becomes `<redacted:RULE_ID>`, where `RULE_ID` names the rule that matched.
The placeholder carries no part of the credential and no hash of it.
It contains no quote, backslash, space, or `=`, so a redacted JSON or logfmt line still parses.
Alloy replaces a credential everywhere it occurs in the line, so the same text in another field is replaced too.

The rules are [Alloy's bundled Gitleaks rules](https://grafana.com/docs/alloy/latest/reference/components/loki/loki.secretfilter/), which cover provider-issued credentials such as cloud access keys, tokens, and JSON Web Tokens.
The chart adds rules for credentials that Materialize deployments carry and the bundled rules miss:

- passwords in connection URLs, such as the metadata database's `metadata_backend_url`;
- password fields, including human-chosen passwords and values inside escaped JSON or Rust `Debug` output;
- `Authorization` and `Proxy-Authorization` header values;
- signatures and tokens in URL query strings, such as a presigned URL's `X-Amz-Signature`;
- private keys whose PEM block was cut off before its `END` line;
- base64-encoded `postgres://` URLs, as in a logged Kubernetes Secret;
- Materialize app passwords (`mzp_…`).

Identifier fields whose names say they aren't credentials, such as `token_id`, are exempt by name.
The bundled defaults change with each Alloy release; the project runs a corpus of positive and negative lines through the pinned Alloy version before each one ships.

The filter scans the line and nothing else, so it has the following limits:

- **Labels and structured metadata aren't scanned.**
  A Loki or OTLP push client that sends a credential as structured metadata or as a label stores it as sent.
- **Some credentials don't look like credentials.**
  A short or low-randomness value after a key such as `api_key` or `token` can pass, and so can a password of fewer than four characters.
  A numeric password after a JSON key passes too, because a placeholder in its place, or in place of the same digits elsewhere in the line, would break the line's JSON.
  The filter decodes nothing, so an encoded credential passes unless a rule matches its encoded form, as one does for base64 `postgres://` URLs.
- **Some identifiers look like credentials.**
  A pagination cursor such as `next_page_token`, or the word after `password:` in an error message, is replaced.

## Try it in shadow mode first {#shadow}

Shadow mode shows what the rules match on your own logs without changing what's stored.
The gateway scans a copy of every line and stores only the lines the filter would have redacted, already redacted, with the label `secretfilter="shadow"`.
Use it to find the values the rules would replace by mistake before you turn the filter on.

To turn shadow mode on, set the mode to `shadow`:

```yaml
pipeline:
  logging:
    secretFilter:
      mode: shadow
```

If you install with the [Terraform module](../../../getting-started/terraform/), set `secret_filter = "shadow"` instead.
The mode changes the gateway's config, so the gateway has to restart to pick it up; see [Restart after a config change](../alloy/#restart-after-a-config-change).
Shadow mode costs the same CPU as turning the filter on, because it scans every line.
Off, the chart doesn't wire it, so it costs nothing.

To read what it found, query the label:

```logql
{secretfilter="shadow"}
```

Each line has `<redacted:RULE_ID>` where a value was replaced, and the rest of the line shows which field held it.
`loki_secretfilter_secrets_redacted_by_category_total{component_id="loki.secretfilter.shadow"}` counts the matches by rule and by container.

The stored lines are a sample, and the counter is the full count:

- The gateway stores up to 100 shadow lines per second from each namespace, with bursts of up to 1,000.
  Shadow lines have this limit to themselves, so they don't use up the rate limits that apply to your other lines.
- A copy that runs past `GATEWAY_SECRETFILTER_TIMEOUT` isn't stored, because it might be only partly redacted.
  `loki_secretfilter_lines_timed_out_total{component_id="loki.secretfilter.shadow"}` counts it.

Shadow lines go to every log destination, like any other line.
Each one is redacted, and while the filter is off its original goes to the same destinations unchanged, so a shadow line exposes nothing new.
After you turn the filter on, turn shadow mode off: the stored lines then carry the placeholders themselves.

## Turn it on {#turn-it-on}

The filter ships off.
Each false positive permanently replaces a value in your own logs, so the defaults wait on measurement against real logs.
<!-- TODO(DEP-313): rewrite this section once shadow mode has run and the filter is on by default. -->

To turn it on, set the mode to `on`:

```yaml
pipeline:
  logging:
    secretFilter:
      mode: "on"
```

If you install with the Terraform module, set `secret_filter = "on"` instead.
The gateway has to restart to pick the mode up; see [Restart after a config change](../alloy/#restart-after-a-config-change).

The filter runs as two instances, and `on` sets both of their scan rates to `1`.
`GATEWAY_SECRETFILTER_RATE` is the fraction of lines the main instance scans, and `GATEWAY_SECRETFILTER_BULK_RATE` is the fraction of `DEBUG` and `TRACE` lines, the levels most able to outrun the filter.
At `0`, an instance forwards every line unscanned.
A rate you set in `pipeline.env` takes precedence over the mode.

The following settings change what the filter costs:

`pipeline.env.GATEWAY_SECRETFILTER_BULK_RATE`
: Below `1`, the bulk instance scans that fraction of `DEBUG` and `TRACE` lines and stores the rest unscanned.
  Lower it only when the gateway can't keep up with its input at `1`.

`pipeline.env.GATEWAY_SECRETFILTER_TIMEOUT`
: The most time the filter spends on one line, such as `50ms`.
  The default, `0s`, sets no limit.
  A line that runs past it is stored with whatever the filter redacted in time, and with the label `secretfilter="timed-out"`.

Both instances load their rules when the gateway starts, which costs about 35 MiB of memory per gateway pod whether or not they scan.

## Watch what it catches {#watch}

The gateway exports the following metrics:

`loki_secretfilter_secrets_redacted_by_category_total`
: Redactions, by `rule` and by `origin`, the container that logged the line.
  An increase usually means a component logged a credential; check the line before you act on it.

`loki_secretfilter_entries_bypassed_total`
: Lines an instance forwarded unscanned because of its rate.

`loki_secretfilter_lines_timed_out_total`
: Lines that ran past `GATEWAY_SECRETFILTER_TIMEOUT`.

`loki_secretfilter_processing_duration_seconds`
: Time spent per line. Its high quantiles are where to start when you set the timeout.

The bundled Thanos keeps these metrics.
A remote-write destination that filters by `minMetricImportance` drops them, because the query registry doesn't grade them.
No alert ships for redactions, so to be told about one, alert on any increase in `loki_secretfilter_secrets_redacted_by_category_total`.
<!-- TODO(DEP-316): link the shipped alerts and the dashboard row here. -->

To find the redacted lines in Loki, query for the placeholder:

```logql
{level=~".+"} |= "<redacted:"
```

For how the filter is placed in the pipeline and tested, see the [logging pipeline reference](../../../reference/development/pipelines/logging/#secret-filter).
