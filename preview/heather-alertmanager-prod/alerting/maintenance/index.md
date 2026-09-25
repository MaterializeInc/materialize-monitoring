# Maintenance Windows




<!--
Agent note: the rollout inhibition the design doc settles on keys off `materialize.generations.active > 1`, not the
hydrating-collection count, which also spikes on an ordinary restart. Document it here when the rule ships; see
"Maintenance windows do not work when the customer picks the time" in the alerting design doc.
-->

# Maintenance Windows

This page describes the three ways to keep an alert from notifying anyone while it still fires: a silence, a recurring
mute window, and an inhibition rule.
None of them stops an alert from firing or from showing in Alertmanager and Grafana; each stops only the notification.

| Mechanism | Suppresses | Scope | Set with |
|---|---|---|---|
| Silence | Alerts matching a set of labels | One period, created by hand | Grafana, or `amtool` |
| Mute window | Every alert a route delivers | A recurring schedule | `alerting.timeIntervals`, referenced from a route |
| Inhibition | Alerts matching one set of labels, while an alert matching another fires | For as long as the source alert fires | `alerting.inhibitRules` |

A self-managed deployment upgrades on its own schedule, so a fixed weekly window is either useless or permanently open.
The intended mechanism for upgrade noise is inhibition on a rollout signal, which closes itself when the rollout ends.
That rule arrives with the shipped rule set and does not exist yet, so a silence is the mechanism today.

## Silences

A silence matches alerts by label and suppresses their notifications until it expires.
Silences gossip between the Alertmanager replicas and are kept on each replica's volume, so one created on either
replica survives the loss of any one of them.

### Through Grafana

The chart provisions an **Alertmanager** datasource in the bundled Grafana.
Under **Alerting → Silences**, with that datasource selected, a silence is created, extended or expired from the UI.
Grafana stores nothing itself; the silence lives in Alertmanager.

### Through `amtool`

`amtool` ships in the Alertmanager image and reaches it on localhost.

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- \
  amtool silence add namespace=materialize-environment \
    --duration=2h --author="$USER" --comment="Planned Materialize upgrade" \
    --alertmanager.url=http://127.0.0.1:9093
```

```bash
kubectl --namespace monitoring exec alertmanager-0 -c alertmanager -- \
  amtool silence query --alertmanager.url=http://127.0.0.1:9093
```

`amtool silence expire <id>` ends one early.

A silence with no end in sight hides the next incident on the same labels as well as the current one.
A duration that covers the work and no more, and a comment naming who to ask, keep it from outliving its purpose.

## Recurring mute windows

A mute window stops a route from notifying during a named, recurring schedule.
It is two parts: a time interval defined once, and a route that references it.

```yaml
alerting:
  timeIntervals:
    - name: change-window
      time_intervals:
        - weekdays: [saturday]
          times:
            - start_time: "02:00"
              end_time: "06:00"
          location: America/New_York
  receivers:
    tickets:
      class: [high, normal, low]
      route:
        mute_time_intervals: [change-window]
      config:
        webhook_configs:
          - url: https://tickets.example.internal/hooks/alertmanager
```

A receiver's `route.mute_time_intervals` mutes it wherever the preset routes to it.
A route under `alerting.routes.extra` takes `mute_time_intervals` the same way, and `active_time_intervals` inverts it:
the route notifies only during the interval.
The render fails when a route names an interval `alerting.timeIntervals` does not define.

A mute window mutes a route, not a condition.
Everything that route delivers is muted for the window, including an unrelated incident that happens to fall inside it.
That is the reason inhibition is preferred where a signal exists.

## Inhibition

An inhibition rule suppresses alerts matching the `target_matchers` while an alert matching the `source_matchers` fires,
optionally only where the listed labels are equal on both.

```yaml
alerting:
  inhibitRules:
    # A warning is noise while the critical form of the same condition fires.
    - source_matchers: ['severity="critical"']
      target_matchers: ['severity="warning"']
      equal: [alertname, namespace]
```

`alerting.inhibitRules` passes through to Alertmanager verbatim, so any rule its [inhibition
reference](https://prometheus.io/docs/alerting/latest/configuration/#inhibit_rule) describes works.
The chart ships no inhibition rules of its own yet.

