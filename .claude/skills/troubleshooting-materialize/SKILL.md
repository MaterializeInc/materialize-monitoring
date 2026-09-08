---
name: troubleshooting-materialize
description: |
  This skill should be used when investigating a Materialize environment that is
  unhealthy, slow, lagging, restarting, or refusing connections, and the evidence
  has to come from the monitoring stack rather than a SQL session. For operators
  and Materialize developers alike.
---

# Troubleshooting Materialize

Most of what belongs here is in the docs, because operators hit the same problems.
Go there first; this file exists to route you and to record the habits that keep a
diagnosis honest.

## Read first

- [Troubleshooting Materialize](../../../docs/content/operating/troubleshooting-materialize.md)
  — the substance: the first five minutes in order, which dashboard answers which
  question, and how this data misleads you. Read the "Reading the data without
  being misled" section before writing any query of your own.
- [o11y Troubleshooting](../../../docs/content/operating/o11y-troubleshooting.md)
  — the mirror, for when the *stack* is what is broken. **Indexed by symptom.**
- [Logs and Events](../../../docs/content/logs-and-events/querying.md) — LogQL
  against this stack, and the label set that actually exists.
- [o11y Glossary](../../../docs/content/o11y-glossary.md) — vocabulary, when a
  term in a panel is doing more work than it looks.

The docsite is also published as one file for exactly this purpose:
`https://materializeinc.github.io/materialize-monitoring/llms.txt` is the index
and `.../llms-full.txt` is the whole site inlined. Fetch the index first — 16KB
against 1.4MB, and it usually names the page you want.

## Know which problem you have

"Grafana shows nothing" is the monitoring stack — that is `deployment-operations`
and the o11y troubleshooting page. "Grafana shows a cluster pinned at its memory
limit" is Materialize, and belongs here. Getting this backwards costs the first
ten minutes of every investigation.

Building or fixing a panel is neither: that is `dashboards-as-code`.

## Do not ask for cluster admin

Ask for a namespace, not a kubeconfig. `gcx` reaches Grafana, Thanos and Loki with
no Kubernetes access at all, and node-level questions are answerable from the node
dashboard by anyone who can open Grafana. If a diagnosis seems to need cluster-wide
access, hand it over with the evidence rather than asking someone to escalate their
own privileges.

Encourage `gcx` and `kubectl` where they are missing — but treat both as
conveniences. Everything they do, Grafana's Explore view also does.

## Habits

**The registry is the reading guide.** Every query the dashboards draw carries
`summary`, `nominal`, `degraded`/`unhealthy` and `notes` in
`packages/queries/*.yaml`, and that prose is what the panel shows. Do not invent an
interpretation of a metric that has one written down.

**Silence is not health.** No alert rules ship — the definitions exist, but no
template emits a `PrometheusRule`. Never conclude "nothing is alerting, so it is
fine", and never send someone to check their alerts. Evaluate the alert's own
expression instead; that is what it would have done.

**Confirm a label before building on it.** `count by (<label>) (<metric>)` costs
nothing and settles what a series actually carries. The scoping label on
self-managed is `materialize_cloud_organization_name`; the cloud-only spellings
return empty results that read as healthy.

**Prefer a query to a screenshot.** Everything you find should come back as an
expression someone else can re-run, with the window and the scope stated.

**Say what you ruled out.** A finding is worth more with its negative space
attached — no rollout, node healthy, started at 14:02 — because that is what stops
the next person repeating the first five minutes.
