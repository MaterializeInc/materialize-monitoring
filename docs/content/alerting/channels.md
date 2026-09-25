---
title: "Alert Channels"
weight: 20
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Alert Channels

This page describes how to tell the bundled Alertmanager where alerts go: which receivers exist, which alerts reach each
of them, and where their credentials come from.
Everything here is configured under the `alerting` values key, which the chart renders into Alertmanager's configuration.
[Alert Architecture](../architecture/#configuration) describes how that rendering works.

<!-- more -->
{{< rfc-2119 >}}

A default install configures no receiver, so every alert reaches `mzmon-null`, a receiver that notifies nobody.
The render warns about that until a receiver exists.

## Receivers, classes, and presets

Three ideas carry the whole configuration.

| Idea | Owned by | Set with |
|---|---|---|
| **Severity** — how bad a condition is | The rule | A rule's `severity` label: `critical`, `warning`, or `notice` |
| **Preset** — what each severity means for this deployment | The operator | `alerting.preset`, naming an entry of `alerting.presets` |
| **Class** — a kind of delivery, such as a page or a low-priority notice | The operator | Each receiver's `class`, and the cells of each preset |

A preset maps each severity to a class.
The three shipped presets express how much the deployment depends on Materialize, and `important` is the default.

| `severity` | `critical-infrastructure` | `important` | `evaluation` |
|---|---|---|---|
| `critical` | `page` | `high` | `normal` |
| `warning` | `high` | `normal` | `normal` |
| `notice` | `normal` | `low` | `suppressed` |

Each receiver declares the classes it serves.
An alert reaches every receiver serving the class its severity maps to.
`suppressed` notifies nobody, and the alert still fires and still shows in Alertmanager and Grafana.

The indirection is what lets one set of rules serve a deployment with a pager and one with a single chat channel.
The rules do not change; the preset and the receivers do.

**Every class the selected preset names MUST be served by some receiver**, or be `suppressed`.
The render fails otherwise, since an unroutable severity is otherwise discovered during the incident it should have reported.
Class names are free-form, and `alerting.presets` is a map, so a single cell can be changed without restating the rest.

A preset of the deployment's own is one more key, with whatever severities and classes it needs:

```yaml
alerting:
  preset: oncall-lite
  presets:
    oncall-lite:
      critical: page
      warning: ticket
      notice: suppressed
```

An alert whose `severity` label is missing, or not in the preset, is routed as `alerting.unknownSeverity` (default `warning`).

## Configuring a receiver

```yaml
alerting:
  receivers:
    <name>:
      class: <class> | [<class>, ...]   # optional
      config:                           # an Alertmanager receiver, without `name`
        <integration>_configs:
          - ...
      route:                            # optional route options for this receiver
        repeat_interval: 1h
```

| Key | Meaning |
|---|---|
| `class` | The classes this receiver serves. A receiver with no class is reachable only from [extra routes](#extra-routes). |
| `config` | An Alertmanager receiver body, verbatim: `slack_configs`, `pagerduty_configs`, `webhook_configs`, `email_configs`, `msteamsv2_configs`, `opsgenie_configs`, and every other integration Alertmanager supports. The key is the receiver's name. |
| `route` | Route options — `group_wait`, `group_interval`, `repeat_interval`, `group_by`, `mute_time_intervals`, `active_time_intervals` — applied wherever the preset routes to this receiver. |

The chart does not model receiver types.
Each integration's fields are documented once, in Alertmanager's [receiver integration
reference](https://prometheus.io/docs/alerting/latest/configuration/#receiver-integration-settings), and a `config`
block is written exactly as that reference describes.
An integration Alertmanager gains in a later release works without a chart change.

## Credentials

**A receiver's credentials MUST come from a mounted Secret, referenced through the field's `_file` variant.**
Values files are committed, diffed, pasted into support threads, and rendered into Terraform plans, so a credential
written into one is a credential published.

| Integration | Inline field | Use instead |
|---|---|---|
| Slack | `api_url` | `api_url_file` |
| PagerDuty | `routing_key`, `service_key` | `routing_key_file`, `service_key_file` |
| Opsgenie | `api_key` | `api_key_file` |
| Microsoft Teams | `webhook_url` | `webhook_url_file` |
| Discord | `webhook_url` | `webhook_url_file` |
| Telegram | `bot_token` | `bot_token_file` |
| Email | `auth_password` | `auth_password_file` |
| Webhook, and any `http_config` | `authorization.credentials`, `basic_auth.password` | `credentials_file`, `password_file` |
| Webhook whose URL carries a token | `url` | `url_file` |

The render fails on an inline credential in a receiver or in `alerting.global`.
`alerting.assertNoInlineCredentials: false` turns that off, and SHOULD NOT be set.

### The default Secret

The chart mounts one Secret by default, named `alertmanager-receivers`, at `/etc/alertmanager/secrets/alertmanager-receivers/`.
Each key in it becomes a file of the same name.

```bash
kubectl --namespace monitoring create secret generic alertmanager-receivers \
  --from-file=slack-url=./slack-url.txt \
  --from-file=pagerduty-key=./pagerduty-key.txt
```

The Secret MUST be in the namespace the Alertmanager pods run in, which under `split-namespace` is `alertmanager`, not the release namespace.
It is mounted as optional, so the pods start before it exists.
Alertmanager reads a `*_file` credential each time it sends, so creating or rotating the Secret takes effect without a
restart once the kubelet refreshes the mount, typically within a minute.

External Secrets Operator, Vault Agent, SOPS, or a cloud secret store's CSI driver MAY own the Secret instead of `kubectl`.
The chart consumes it by name and never creates it.

### Further Secrets

Additional Secrets are mounted through `alertmanager.extraSecretMounts`, conventionally under `/etc/alertmanager/secrets/<secret-name>/`.

```yaml
alertmanager:
  extraSecretMounts:
    # Helm replaces lists, so the default entry is restated.
    - name: alertmanager-receivers
      secretName: alertmanager-receivers
      mountPath: /etc/alertmanager/secrets/alertmanager-receivers
      readOnly: true
      optional: true
    - name: pagerduty
      secretName: pagerduty-routing
      mountPath: /etc/alertmanager/secrets/pagerduty
      readOnly: true
```

Helm replaces a list rather than merging it, so a values file setting `extraSecretMounts` MUST restate the default entry
if receivers still reference it.
The render fails when a `*_file` path falls under no mounted volume, which is how a dropped entry shows up.
Paths under `/etc/ssl/`, the image's own trust store, are exempt.

## Examples

Each example is a complete `alerting` block that renders on its own.
The default preset, `important`, names the `high`, `normal` and `low` classes, so a receiver meant to catch everything on it serves all three.

### One chat channel

The smallest real configuration: one Slack channel receiving everything, on an evaluation deployment where `notice` is suppressed.

```yaml
alerting:
  preset: evaluation
  receivers:
    chat:
      class: normal
      config:
        slack_configs:
          - channel: "#materialize"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
            send_resolved: true
```

On `important` or `critical-infrastructure`, the same receiver needs every class those entries name, for example `class: [high, normal, low]`.

### A pager and a channel

Critical alerts page; everything else goes to a channel and a ticket queue.

```yaml
alerting:
  preset: critical-infrastructure
  receivers:
    oncall:
      class: page
      route:
        group_wait: 10s
        repeat_interval: 1h
      config:
        pagerduty_configs:
          - routing_key_file: /etc/alertmanager/secrets/alertmanager-receivers/pagerduty-key
            send_resolved: true
    platform:
      class: [high, normal]
      config:
        slack_configs:
          - channel: "#platform-alerts"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
            send_resolved: true
    tickets:
      class: normal
      config:
        webhook_configs:
          - url: https://tickets.example.internal/hooks/alertmanager
            http_config:
              authorization:
                credentials_file: /etc/alertmanager/secrets/alertmanager-receivers/tickets-token
```

`notice` maps to `normal` here, and two receivers serve `normal`, so a `notice` alert reaches both `platform` and `tickets`.
The `route` options on `oncall` apply only to its route, so pages are grouped and repeated faster than anything else.

### An incident-management product

Most incident-management products accept Alertmanager's native webhook payload, so a `webhook_configs` entry with a
bearer token is the whole integration.

```yaml
alerting:
  receivers:
    incidents:
      class: [high, normal, low]
      config:
        webhook_configs:
          - url: https://incidents.example.com/v2/alert-sources/alertmanager
            send_resolved: true
            http_config:
              authorization:
                credentials_file: /etc/alertmanager/secrets/alertmanager-receivers/incidents-token
```

On `critical-infrastructure`, the same receiver would add `page`.
A product with a dedicated integration in Alertmanager, such as PagerDuty or Opsgenie, SHOULD use it rather than a
webhook, because the integration maps Alertmanager's grouping and resolution onto the product's own incident lifecycle.

### Email

```yaml
alerting:
  global:
    smtp_smarthost: smtp.example.internal:587
    smtp_from: alertmanager@example.internal
    smtp_auth_username: alertmanager
    smtp_auth_password_file: /etc/alertmanager/secrets/alertmanager-receivers/smtp-password
  receivers:
    ops-email:
      class: [high, normal, low]
      config:
        email_configs:
          - to: ops@example.internal
```

## Extra routes {#extra-routes}

`alerting.routes.extra` takes routes in Alertmanager's own format and places them **ahead of** the preset's severity routes.
An alert is tested against them first, so a specific match wins and the preset remains the fallback.

```yaml
alerting:
  receivers:
    chat:
      class: [high, normal, low]
      config:
        slack_configs:
          - channel: "#materialize"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
    data-team:
      config:
        slack_configs:
          - channel: "#data-platform"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/data-team-slack-url
  routes:
    extra:
      - matchers: ['component="storage"']
        receiver: data-team
        continue: true
```

| `continue` | An alert the route matches |
|---|---|
| `false` (default) | Goes to this route's receiver only |
| `true` | Goes to this route's receiver, then on through the preset as well |

Each `receiver` an extra route names, at any depth, MUST be defined under `alerting.receivers`; the render fails otherwise.
A top-level extra route that names no receiver and does not continue sends the alerts it matches to `mzmon-null`, and the render warns about it.

Storage alerts reach `data-team`, and, because the route continues, also whichever receivers the preset names for their severity.

This is where deployment-specific routing lives, including routing that names a particular customer or environment.
It belongs in that deployment's own values, reviewed by the people who run it.

## Grouping and timing

`alerting.routes.root` sets grouping and timing for the top-level route, and every route below inherits it.

| Key | Default | Meaning |
|---|---|---|
| `group_by` | `[alertname, cluster, namespace]` | One notification per condition per Materialize environment. `cluster` is constant within one Alertmanager; see below |
| `group_wait` | `30s` | How long a new group waits for more alerts before its first notification |
| `group_interval` | `5m` | How long before a group's next notification when alerts are added to it |
| `repeat_interval` | `4h` | How long before an unchanged, still-firing group is notified again |

`cluster` is in the default `group_by` for the incident tools downstream rather than for grouping.
An Alertmanager group key is built from the route and the group labels, and PagerDuty's `dedup_key` and Opsgenie's `alias` are hashes of it.
Without `cluster`, two clusters sending the same condition to one PagerDuty service open a single incident, and either cluster's resolution closes it.
Every alert carries `cluster`; see [Alert Architecture](../architecture/#cluster-label).

A receiver's `route` overrides these wherever the preset routes to it.
The root route's `receiver`, `routes` and `matchers` belong to the chart, and setting them fails the render.

## Notification templates

`alerting.templates` adds Go-template files that every receiver can reference.
Each key is a file name ending in `.tmpl`.

```yaml
alerting:
  templates:
    materialize.tmpl: |
      {{ define "materialize.title" }}[{{ .Status | toUpper }}] {{ .CommonLabels.alertname }}{{ end }}
  receivers:
    chat:
      class: [high, normal, low]
      config:
        slack_configs:
          - channel: "#materialize"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
            title: '{{ template "materialize.title" . }}'
```

## Links in notifications {#links}

Alertmanager's default notification templates link to two pages of its own UI: the alert list (`/#/alerts`) and a
pre-filled silence (`/#/silences/new`).
It builds both from `alertmanager.baseURL`.

| Deployment | `alertmanager.baseURL` |
|---|---|
| Alertmanager is not exposed (default) | Empty. The links name the pod's own address, which is unreachable from outside the cluster and otherwise harmless |
| Alertmanager is exposed, behind authentication | The address operators reach it at. A sub-path is fine, provided the ingress strips it |
| Operators work from Grafana | Still empty. Build Grafana links in a template instead, as below |

**`alertmanager.baseURL` MUST NOT point at Grafana.**
Grafana serves neither of Alertmanager's paths, so every link lands on Grafana's home page, and the render warns when the two share a host.

The chart pins `alertmanager.extraArgs.web.route-prefix` to `/`.
Without that pin, Alertmanager takes its route prefix from the path of `baseURL`, which would move the API both rulers
post to, its probes, the reloader and the Grafana datasource.
The render fails on a `baseURL` with a path once the pin is removed.

### Linking to Grafana instead

Grafana's alerting pages name an Alertmanager by its datasource **name** — `Alertmanager` by default, set by
`connections.datasources.alertmanager.name` — and take each silence matcher as a `matcher=<label>=<value>` parameter.
A template builds both links, and a receiver references them.

```yaml
alerting:
  templates:
    grafana-links.tmpl: |
      {{ define "grafana.alerts.url" -}}
      https://grafana.example.com/alerting/groups?alertmanager=Alertmanager
      {{- end }}
      {{ define "grafana.silence.url" -}}
      https://grafana.example.com/alerting/silence/new?alertmanager=Alertmanager
      {{- range .CommonLabels.SortedPairs }}&matcher={{ .Name }}%3D{{ .Value | urlquery }}{{ end }}
      {{- end }}
  receivers:
    chat:
      class: [high, normal, low]
      config:
        slack_configs:
          - channel: "#materialize"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
            title_link: '{{ template "grafana.alerts.url" . }}'
            actions:
              - type: button
                text: Silence
                url: '{{ template "grafana.silence.url" . }}'
```

The host is the one in `grafana.ini.server.root_url`.
Silences created from that link live in Alertmanager like any other; see [Maintenance Windows](../maintenance/).

## Checking a configuration

The render checks the structure it can see, and fails the install rather than the reload.

| The render fails on | Because Alertmanager would |
|---|---|
| A preset class no receiver serves | Route that severity to nobody |
| A route naming an undefined receiver or time interval | Refuse the configuration |
| A receiver key that does not end in `_configs` | Refuse the configuration |
| An inline credential | Accept it, and the credential would be published with the values |
| A `*_file` path under no mounted volume | Fail every notification through that receiver, at send time |
| A template whose name does not end in `.tmpl` | Never load it |

Anything inside a receiver body passes through unchecked, and Alertmanager validates it on reload.
A rejected configuration leaves the previous one running, and `alertmanager_config_last_reload_successful` drops to 0.

`amtool`, shipped in the Alertmanager image, answers where a given alert would be routed and confirms a receiver delivers.

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- \
  amtool config routes test --config.file=/etc/alertmanager/config/alertmanager.yml severity=critical
```

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- \
  amtool alert add alertname=ReceiverTest severity=warning \
    --annotation=summary="Delivery test for the warning class" \
    --alertmanager.url=http://127.0.0.1:9093
```

The test alert is delivered after `group_wait`, resolves on its own after `resolve_timeout` (5m by default), and is a
real notification to whoever the route reaches.

## Another Alertmanager

Nothing on this page applies when the bundled Alertmanager is disabled.
A deployment that notifies an Alertmanager it already runs configures that Alertmanager's routing itself, and points
both rulers at it as described in [Configuring](../configuring/#what-an-operator-configures-today).
