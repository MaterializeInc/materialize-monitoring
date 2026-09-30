---
title: "Terraform"
weight: 25
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Configuring Alerting through Terraform

The `materialize-monitoring` Terraform module takes the same alerting configuration as the chart, as four inputs.
This page shows how each one is written in HCL.
What the settings mean is covered on [Configuring Alerting](../configuring/) for the rules and [Alert Channels](../channels/) for where alerts go, and applies unchanged here.

<!-- more -->
{{< rfc-2119 >}}

<!--
Agent note: written for the per-cloud wrappers passing these four inputs
through under the same names (DEP-339). Keep "Through the per-cloud wrappers"
and the Alerting section of getting-started/terraform.md in step with them.
-->

## The inputs

| Input | Chart values | Configures |
|---|---|---|
| `alert_rules` | `rules.*` | Which bundled rules install, and how they are tuned |
| `alerting` | `alerting.*` | The routing preset, receivers, routes, inhibit rules, time intervals, templates and Alertmanager's `global` block |
| `alerting_receiver_secrets` | The `alertmanager-receivers` Secret | Receiver credentials |
| `alertmanager_namespace` | None | The namespace that Secret is created in |

Every attribute of `alert_rules` and `alerting` is optional.
An attribute left unset is left out of the Helm values, so the chart's default applies.
A map the chart already fills, such as `alerting.presets`, merges with the keys a caller sets rather than being replaced by them.

A default install configures no receiver, so every alert reaches `mzmon-null`, which notifies nobody.

## Receivers and routing

The Alertmanager-native parts of `alerting` pass through as written: `receivers`, `routes`, `inhibit_rules`, `time_intervals`, `templates` and `global`.
Their field names are Alertmanager's own, so every example on [Alert Channels](../channels/) translates by writing its YAML as HCL.
The module types these parts as `any`, so an integration Alertmanager supports needs no module release.

The Alert Channels pager-and-channel example, with a route sending workload alerts to the team that owns the clusters:

```hcl
module "monitoring" {
  # ...

  alerting = {
    preset = "critical-infrastructure"

    receivers = {
      oncall = {
        class = "page"
        route = { group_wait = "10s", repeat_interval = "1h" }
        config = {
          pagerduty_configs = [{
            routing_key_file = "/etc/alertmanager/secrets/alertmanager-receivers/pagerduty-key"
            send_resolved    = true
          }]
        }
      }
      platform = {
        class = ["high", "normal"]
        config = {
          slack_configs = [{
            channel       = "#platform-alerts"
            api_url_file  = "/etc/alertmanager/secrets/alertmanager-receivers/slack-url"
            send_resolved = true
          }]
        }
      }
      data-team = {
        config = {
          slack_configs = [{
            channel      = "#data-platform"
            api_url_file = "/etc/alertmanager/secrets/alertmanager-receivers/data-team-slack-url"
          }]
        }
      }
    }

    routes = {
      extra = [{
        matchers = ["audience=\"workload\""]
        receiver = "data-team"
        continue = true
      }]
    }
  }

  alerting_receiver_secrets = {
    "pagerduty-key"       = var.pagerduty_routing_key
    "slack-url"           = var.platform_slack_webhook
    "data-team-slack-url" = var.data_team_slack_webhook
  }
}
```

Every bundled alert carries an `audience` label, `platform` or `workload`, which is what the extra route matches.
A matcher is an Alertmanager string, so the quotes inside it are escaped in HCL.

| Attribute | Chart value | Notes |
|---|---|---|
| `preset` | `alerting.preset` | `critical-infrastructure`, `important` (the default), `evaluation`, or a key of `presets` |
| `presets` | `alerting.presets` | Severity-to-class maps, keyed by preset name. A cell set here overrides that cell of a shipped preset |
| `unknown_severity` | `alerting.unknownSeverity` | The severity an alert without a known one is routed as |
| `receivers` | `alerting.receivers` | Keyed by name, each with `class`, `config`, and optional `route` |
| `routes.root` | `alerting.routes.root` | Grouping and timing for the top-level route |
| `routes.extra` | `alerting.routes.extra` | Routes ahead of the preset's severity routes |
| `inhibit_rules` | `alerting.inhibitRules` | Alertmanager `inhibit_rules`, verbatim |
| `time_intervals` | `alerting.timeIntervals` | Alertmanager `time_intervals`, verbatim; see [Maintenance Windows](../maintenance/) |
| `templates` | `alerting.templates` | Notification templates keyed by file name, each ending in `.tmpl` |
| `global` | `alerting.global` | Alertmanager's `global` block, verbatim |

## Credentials

**A receiver's credentials MUST NOT be written into `alerting`.**
Anything in the Helm values is readable with `helm get values` by anyone who can read the release Secret, and the chart fails the render on an inline credential.

A receiver instead names a file under `/etc/alertmanager/secrets/alertmanager-receivers/`, through its field's `_file` variant: `api_url_file`, `routing_key_file`, `credentials_file`.
`alerting_receiver_secrets` supplies those files.
Each of its keys becomes a key of the `alertmanager-receivers` Secret, which the chart mounts at that path, so a receiver reads the key `slack-url` as `/etc/alertmanager/secrets/alertmanager-receivers/slack-url`.

| Behaviour | Why |
|---|---|
| The plan fails when a receiver reads a key the map does not set | Otherwise the mount exists, the file does not, and the notification fails when it is needed |
| An empty map creates no Secret, and the key check is skipped | Leaves `alertmanager-receivers` free for External Secrets Operator, Vault Agent or a CSI driver to own |
| Rotating a credential needs no restart | Alertmanager re-reads a credential file on every send, and the kubelet refreshes the mount within about a minute |
| The variable is `sensitive` | The values do not appear in plan output |

The credentials are still stored in Terraform state, as every Terraform-managed Secret is.
State SHOULD be kept where only the people who could read the Secret itself can read it.

A credential kept in a Secret the module does not create is mounted through `alertmanager.extraSecretMounts` in `additional_values`; see [Further Secrets](../channels/#further-secrets).

### Under `split-namespace`

The chart's `split-namespace` profile runs Alertmanager in a namespace of its own, `alertmanager`.
The Secret has to be created there, so `alertmanager_namespace = "alertmanager"` MUST be set alongside the profile.
The module cannot infer it, because the profile is applied through `additional_values`, and the namespace has to exist before apply.

## Tuning the bundled rules

`alert_rules` maps onto the chart's `rules.*`.
[Configuring Alerting](../configuring/#the-bundled-rules) describes the default set, capabilities, the audience split and the workload tiers.

```hcl
module "monitoring" {
  # ...

  alert_rules = {
    # This deployment runs a dedicated CockroachDB for its metadata.
    capabilities = ["crdb-dedicated"]

    # Beyond the default set: the rules for that database, user-cluster
    # freshness, which is opt-in because some clusters are behind by design,
    # and the infrastructure memory rules, which read the workload tiers.
    selected = ["crdb", "cluster-falling-behind", "cluster-stale", "infra_memory"]
    disabled = ["pods-stuck-in-waiting"]

    overrides = {
      # Large clusters here take hours to hydrate with nothing wrong.
      cluster-hydration-stuck = { for_duration = "6h" }
      cluster-cpu-high        = { labels = { severity = "notice" } }
    }

    excluded_namespaces = ["materialize-scratch"]

    # Replaces the chart's list for this tier only.
    infra_workloads = {
      core = ["coredns", "kube-proxy", "aws-node", "karpenter"]
    }
  }
}
```

| Attribute | Chart value | Notes |
|---|---|---|
| `enabled` | `rules.enabled` | `false` installs none of the bundled rules |
| `capabilities` | `rules.capabilities` | What the deployment contains beyond what the chart derives |
| `selected` | `rules.selected` | Alert names, rule-group names, or `"*"` |
| `disabled` | `rules.disabled` | Alert names never to install |
| `overrides` | `rules.overrides` | Per alert, `for_duration` and `labels`. An override never changes an expression |
| `environment_namespaces` | `rules.namespaces.environment` | Where Materialize environments run. Unset uses the module's `materialize_instance_namespace` |
| `excluded_namespaces` | `rules.namespaces.exclude` | Namespaces no rule alerts on |
| `infra_workloads` | `rules.infraWorkloads` | Per tier: `core`, `important`, `nonessential`, `daemonset`. Read only by rules outside the default set, so a tier has an effect once one of its groups is selected |

An override's duration is `for_duration`, the rule's `for`.
HCL reads `{ for = "6h" }` as a `for` expression, so the attribute has a name of its own.
An override's `severity` MUST stay `critical`, `warning` or `notice`, and its `audience` `platform` or `workload`, so the routes still match.

## Checking the result

Most mistakes fail before anything is installed, and the rest fail the Helm render at apply.

| Mistake | Fails at |
|---|---|
| A receiver reads a key `alerting_receiver_secrets` does not set | Plan, naming the key |
| An override's `for_duration` is not a duration | Plan |
| `preset` is not a shipped preset or a key of `presets` | Plan |
| A template name does not end in `.tmpl` | Plan |
| An unknown capability, alert or rule-group name | Apply, from the chart's validators |
| An inline credential in a receiver | Apply, from the chart's validators |
| The preset names a class no receiver serves | Apply, from the chart's validators |

`terraform plan` shows the composed Helm values in the `helm_release` resource, with `alerting_receiver_secrets` as `(sensitive value)`.
After apply, [Checking a configuration](../channels/#checking-a-configuration) shows how to confirm what the running Alertmanager loaded and which receiver an alert would reach.

## Through the per-cloud wrappers

The per-cloud wrappers in `materialize-terraform-self-managed` pass all four inputs through under the same names.
A deployment built from one of their example roots sets them on the wrapper's `monitoring` module block, written exactly as in the examples above.
`alertmanager_namespace` defaults to the wrapper's `namespace`, and is set to `alertmanager` under the `split-namespace` profile, as described [above](#under-split-namespace).
