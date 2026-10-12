# Alert Channels




# Alert Channels

This page describes how to tell the bundled Alertmanager where alerts go: which receivers exist, which alerts reach each
of them, and where their credentials come from.
Everything here is configured under the `alerting` values key, which the chart renders into Alertmanager's configuration.
The Terraform module takes the same configuration as its `alerting` input, and creates the receiver Secret from `alerting_receiver_secrets`; see [Configuring Alerting through Terraform](../terraform/).
[Alert Architecture](../architecture/#configuration) describes how that rendering works.

<!-- more -->

<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


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
| Amazon SNS | `sigv4.secret_key` | The pod's own AWS identity; see [Cloud provider services](#cloud) |

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

The Terraform module creates it from `alerting_receiver_secrets`, keyed the same way, and fails the plan when a receiver reads a key that map does not set; see [Credentials](../terraform/#credentials).
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

## Cloud provider services {#cloud}

Amazon SNS is the one Alertmanager integration that authenticates with the pod's own cloud identity.
Every other integration authenticates with a credential read from a Secret, including email sent through Amazon SES or Azure Communication Services.
GKE Workload Identity and Azure Workload Identity therefore give Alertmanager nothing to use: no integration calls a Google or Azure API.

| Service | Integration | Authenticates with | Workload identity |
|---|---|---|---|
| Amazon SNS | `sns_configs` | The pod's AWS identity | IRSA or EKS Pod Identity |
| Amazon SES | `email_configs`, over SMTP | SES SMTP credentials, derived from an IAM user's key | Not possible: SMTP takes a username and password |
| Azure Communication Services | `email_configs`, over SMTP | An SMTP username and an Entra application's client secret | Not possible, for the same reason |
| Microsoft Teams | `msteamsv2_configs` | A Workflows webhook URL, which is itself the credential | Not applicable |
| Google Cloud | None | No Alertmanager integration calls a Google API | Nothing to authorize |

The default egress policy covers all of them: SNS, STS and the SMTP endpoints are reached on `443`, `587` or `465`.

### Amazon SNS

The pod needs permission to publish to the topic, and nothing else.

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": "sns:Publish",
      "Resource": "arn:aws:sns:us-east-1:123456789012:materialize-alerts"
    }
  ]
}
```

| Also needed when | Permission |
|---|---|
| The topic is encrypted with a customer-managed KMS key | `kms:GenerateDataKey*` and `kms:Decrypt` on that key |
| A receiver sends SMS through `phone_number` | `sns:Publish` on `"*"`, since an SMS has no topic ARN |
| A receiver sets `sigv4.role_arn` to publish from another account | `sts:AssumeRole` on that role, whose trust policy names this one (and `sigv4.external_id`, if set) |

The identity reaches the pod one of two ways on EKS.
Both resolve to the ServiceAccount `alertmanager`, in the namespace Alertmanager runs in.

| Mechanism | Setup |
|---|---|
| IRSA | A role whose trust policy allows `sts:AssumeRoleWithWebIdentity` from the cluster's OIDC provider for `system:serviceaccount:<namespace>:alertmanager`, and `alertmanager.serviceAccount.annotations` naming it |
| EKS Pod Identity | A pod identity association for namespace `<namespace>` and ServiceAccount `alertmanager`. No annotation |

The chart's `automountServiceAccountToken: false` does not interfere with IRSA.
The EKS webhook projects a token volume of its own into the pod, whatever that setting says.

```yaml
alertmanager:
  serviceAccount:
    annotations:
      eks.amazonaws.com/role-arn: arn:aws:iam::123456789012:role/materialize-alertmanager
alerting:
  receivers:
    sns:
      class: [high, normal, low]
      config:
        sns_configs:
          - topic_arn: arn:aws:sns:us-east-1:123456789012:materialize-alerts
            sigv4:
              region: us-east-1
```

With no `access_key` or `secret_key`, Alertmanager uses the AWS SDK's default credential chain, which is what finds the
IRSA or Pod Identity credentials.
`sigv4` has no `_file` variant for static keys, so a cluster without workload identity supplies them as
`AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` through `alertmanager.extraEnv`, from a Secret, rather than inline.

An SNS topic with email subscriptions is also the AWS-native way to send alert email without SMTP credentials.

### Amazon SES

SES accepts mail over SMTP, and SMTP authenticates with a username and password, so workload identity cannot apply.
The credentials belong to an IAM user whose policy allows `ses:SendRawEmail`, which is the action SES's SMTP interface performs.
The SMTP password is derived from that user's secret access key; it is not the key itself.

```yaml
alerting:
  global:
    smtp_smarthost: email-smtp.us-east-1.amazonaws.com:587
    smtp_from: alertmanager@example.com
    smtp_auth_username: <ses-smtp-username>
    smtp_auth_password_file: /etc/alertmanager/secrets/alertmanager-receivers/ses-smtp-password
  receivers:
    ops-email:
      class: [high, normal, low]
      config:
        email_configs:
          - to: ops@example.com
```

The `smtp_from` address, or its domain, MUST be a verified SES identity.
An account still in the SES sandbox can also only send to verified recipients.

### Google Cloud

No Alertmanager integration authenticates to a Google API, so Workload Identity on GKE has nothing to authorize, and
`alertmanager.serviceAccount` needs no annotation there.

| Destination | Path |
|---|---|
| Email | An SMTP provider, with `email_configs` and credentials in a Secret |
| Google Chat, Pub/Sub, or another Google service | A relay the deployment runs, reached with `webhook_configs`. The relay holds the Google identity and permission, such as `roles/pubsub.publisher`, and Alertmanager authenticates to it with a bearer token from a Secret |

### Azure

No Alertmanager integration authenticates with an Entra token, so Azure Workload Identity has nothing to authorize either.

| Destination | Path |
|---|---|
| Email through Azure Communication Services | `email_configs` against `smtp.azurecomm.net:587`. The username is an SMTP Username resource linked to an Entra application; the password is one of that application's client secrets |
| Microsoft Teams | `msteamsv2_configs` with a Workflows webhook URL in a Secret, read through `webhook_url_file` |

The Entra application behind Azure Communication Services SMTP needs a role on the Communication Services resource.
The built-in **Communication and Email Service Owner** role works.
A custom role limited to `Microsoft.Communication/CommunicationServices/Read`,
`Microsoft.Communication/CommunicationServices/Write` and `Microsoft.Communication/EmailServices/write` is narrower.

```yaml
alerting:
  global:
    smtp_smarthost: smtp.azurecomm.net:587
    smtp_from: DoNotReply@alerts.example.com
    smtp_auth_username: materialize-alertmanager
    smtp_auth_password_file: /etc/alertmanager/secrets/alertmanager-receivers/acs-client-secret
  receivers:
    ops-email:
      class: [high, normal, low]
      config:
        email_configs:
          - to: ops@example.com
```

An Entra client secret expires, so the Secret holding it needs rotating before it does.
Alertmanager reads it on every send, so rotation needs no restart.

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
      - matchers: ['audience="workload"']
        receiver: data-team
        continue: true
```

| `continue` | An alert the route matches |
|---|---|
| `false` (default) | Goes to this route's receiver only |
| `true` | Goes to this route's receiver, then on through the preset as well |

Each `receiver` an extra route names, at any depth, MUST be defined under `alerting.receivers`; the render fails otherwise.
A top-level extra route that names no receiver and does not continue sends the alerts it matches to `mzmon-null`, and the render warns about it.

Every bundled rule carries an `audience` label: `platform` for the deployment and the platform under it, `workload` for what runs on it.
Here the workload alerts, such as a user cluster falling behind or running out of memory, reach `data-team`.
Because the route continues, they also reach whichever receivers the preset names for their severity.

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

The chart ships notification templates of its own, and by default they replace Alertmanager's built-in ones.
A receiver that sets no `title`, `text` or link of its own uses them, so a receiver needs nothing beyond its destination and credentials.
Alertmanager's built-ins send Slack an empty body, and link every notification to the pod's own address.

| Field | Integrations | The chart's default |
|---|---|---|
| Subject or title | Slack, email, PagerDuty, Opsgenie, Microsoft Teams, Discord, Jira and the rest | `[FIRING:2] env-uptime-sla — prod-us-east/materialize-environment (critical)`: the status and count, the alert, `cluster/namespace`, and the severity |
| Link | Slack's title, PagerDuty's client link, Opsgenie's source, the email button | The alert group in Grafana; see [Links in notifications](#links) |
| Body | Slack | The summary, the description while firing, one line per alert when there are several, and links to the runbook, the group in Grafana, and a pre-filled silence |
| Colour | Slack | Red for `critical`, amber for `warning`, blue for anything milder, green once resolved |
| Footer | Slack | The labels the alerts share that the subject does not already carry |

A firing Slack notification for two alerts reads:

```text
[FIRING:2] ThanosCompactHalted — prod-us-east/monitoring (warning)
Thanos Compact has failed to run and is now halted.
Thanos Compact thanos-compactor has failed to run and now is halted.
• instance=10.0.0.0:10902 pod=thanos-compactor-0 · since Today 3:11 PM
• instance=10.0.0.1:10902 pod=thanos-compactor-1 · since Today 3:10 PM
Runbook · Grafana · Silence
app=thanos  container=compactor  job=thanos-compactor  service=thanos-compactor
```

Times are shown in each reader's own time zone.
The per-alert lines stop at ten firing and ten resolved alerts, and the Grafana link shows the rest.
A description's hard-wrapped lines are rejoined, because Slack shows every newline; a blank line still separates paragraphs.

`alerting.defaultTemplates: false` restores Alertmanager's built-ins.
The chart's templates are still loaded, so a receiver can reference them by name.

### The chart's templates

The chart's templates live in `mzmon.gotmpl`, beside the configuration.
They are defined under the `mzmon.` prefix, which is reserved for them.

| Template | Renders |
|---|---|
| `mzmon.subject` | The one-line subject above |
| `mzmon.slack.text` | The Slack body |
| `mzmon.slack.links` | The body's last line: runbook, Grafana, silence |
| `mzmon.url.group` | The notification's alert group in Grafana, or in Alertmanager's own UI |
| `mzmon.url.silence` | A new silence, pre-filled with every label the notification's alerts share |

### A deployment's own templates

`alerting.templates` adds Go-template files that every receiver can reference.
Each key is a file name ending in `.tmpl`.
They load after `mzmon.gotmpl`, so a file that defines a name the chart also defines replaces the chart's definition.
That covers Alertmanager's built-in names as well, such as `slack.default.text`.

A template of a deployment's own can build on the chart's:

```yaml
alerting:
  templates:
    materialize.tmpl: |
      {{ define "materialize.title" }}{{ .CommonLabels.severity | toUpper }}: {{ .CommonAnnotations.summary }}{{ end }}
  receivers:
    chat:
      class: [high, normal, low]
      config:
        slack_configs:
          - channel: "#materialize"
            api_url_file: /etc/alertmanager/secrets/alertmanager-receivers/slack-url
            send_resolved: true
            title: '{{ template "materialize.title" . }}'
            actions:
              - type: button
                text: Silence
                url: '{{ template "mzmon.url.silence" . }}'
```

## Links in notifications {#links}

Notifications link to Grafana's alerting pages, which show the bundled Alertmanager's alerts and silences through the Alertmanager datasource (`connections.datasources.alertmanager`).
The chart takes Grafana's address from the setting Grafana already builds its own links from.

| `connections.grafana.mode` | Grafana's address comes from |
|---|---|
| `bundled` (default) | `grafana.grafana.ini.server.root_url` |
| `operator` | `connections.grafana.operator.spec.config.server.root_url` |
| `external` | `connections.grafana.external.url` |

`alerting.grafanaURL` overrides all three.
It SHOULD be set wherever the derived address is not the one people open Grafana at.

| Deployment | Set |
|---|---|
| The bundled Grafana, exposed through `grafana.ingress` | `grafana.grafana.ini.server.root_url`, which Grafana's own share links need too |
| A Grafana this chart does not deploy, addressed in-cluster | `alerting.grafanaURL`, to its browser-facing address |
| A `root_url` written with Grafana's `%(domain)s` placeholders | `alerting.grafanaURL`; the chart cannot resolve the placeholders |
| Grafana reached only through `kubectl port-forward` | `alerting.grafanaURL: http://localhost:3000`, or nothing |
| Alertmanager exposed, and Grafana not | `alertmanager.baseURL`, below |

With no Grafana address, the links fall back to Alertmanager's own UI when `alertmanager.baseURL` is set, and are left out otherwise.
A link nobody can open is worse than none.
The render warns when a receiver exists and notifications would carry no links, and when they would link to an in-cluster address.

The group link opens Grafana's active-notifications view, filtered to the notification's group labels and receiver.
The silence link opens Grafana's silence editor with every label the alerts share as a matcher, which is the narrowest silence covering everything in the notification.
The editor shows the matchers before anything is saved.
Silences created from it live in Alertmanager like any other; see [Maintenance Windows](../maintenance/).

### `alertmanager.baseURL`

`alertmanager.baseURL` is the address of Alertmanager's own UI, and is empty by default.
Alertmanager's built-in templates link to its alert list (`/#/alerts`) from it.

| Deployment | `alertmanager.baseURL` |
|---|---|
| Alertmanager is not exposed (default) | Empty |
| Alertmanager is exposed, behind authentication | The address operators reach it at. A sub-path is fine, provided the ingress strips it |

**`alertmanager.baseURL` MUST NOT point at Grafana.**
Grafana serves none of Alertmanager's paths, so every link lands on Grafana's home page, and the render warns when the two share a host.

The chart pins `alertmanager.extraArgs.web.route-prefix` to `/`.
Without that pin, Alertmanager takes its route prefix from the path of `baseURL`, which would move the API both rulers
post to, its probes, the reloader and the Grafana datasource.
The render fails on a `baseURL` with a path once the pin is removed.

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
| An `alerting.grafanaURL` that is not an absolute `http://` or `https://` URL | Send every notification with links nobody can open |

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
    --annotation=summary="Delivery test for the warning class"
```

The test alert is delivered after `group_wait`, resolves on its own after `resolve_timeout` (5m by default), and is a
real notification to whoever the route reaches.

## Another Alertmanager

Nothing on this page applies when the bundled Alertmanager is disabled.
A deployment that notifies an Alertmanager it already runs configures that Alertmanager's routing itself, and points
both rulers at it as described in [Configuring](../configuring/#what-an-operator-configures-today).

