---
title: "Overview"
weight: 1
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
---

# Observability tools

This page is for anyone who reads data from a `materialize-monitoring` install, and for the operators who decide who gets access.
Every tool in this section reads the same metrics, logs, and alerts.
The tools differ in who they suit, what access they need, and how much they ask of you.

## Compare the tools

The following table classifies each tool.
Each tool's page carries the same classification at its top.

{{< param-matrix keys="interface,audience,cluster-access,grafana-access,ease-of-use,stability" >}}

The table's rows mean the following:

Interface
: How you use the tool.

Audience
: Who the tool suits best.

Cluster access
: The Kubernetes access the tool needs.
  The Grafana-based tools need it only to port-forward a Grafana that your install doesn't expose.

Grafana access
: The Grafana credential the tool signs in with.

Ease of use
: How much setup and query-language knowledge the tool asks of you.

Stability
: How much the tool's own interface changes from one release to the next.

> [!WARNING]
> Every tool in this section reads all of the telemetry the stack holds.
> A Grafana role limits what you can change, not what you can read, so any Grafana login or service account token reads every metric and every log.
> The backend APIs have no authentication, so Kubernetes access to their namespace grants the same.
> For more information, see [Securing](../../operating/securing/).

## Choose a tool

Pick a tool by what you're doing:

- **Reading a dashboard, or handing someone a link:** use [Grafana in a browser](../grafana/).
- **Asking questions in plain language, or having an AI agent investigate:** use the [Grafana MCP server](../grafana-mcp/).
- **Scripting queries, or working from a terminal:** use [`gcx`](../gcx/).
- **Working while Grafana is down, or debugging the stack itself:** use the [backend APIs](../backend-apis/).

The tools work together.
An agent on the MCP server or `gcx` can return a Grafana link, so a person can check its answer in a browser.

If your install forwards metrics to Datadog, Datadog is another place to read them.
For more information, see [Datadog dashboards](../../dashboards/datadog/).

## Know the names the tools share

A tool that goes through Grafana reaches the data through the chart's data sources.
A script or an agent needs their UIDs, which the following table lists:

| Data source | Type | UID | Holds |
|---|---|---|---|
| Thanos | `prometheus` | `mzmon-thanos` | Metrics |
| Loki | `loki` | `mzmon-loki` | Logs and Kubernetes events |
| Alertmanager | `alertmanager` | `mzmon-alertmanager` | Firing alerts and silences |

The following names also come up often:

Dashboard UIDs
: Each shipped dashboard's UID starts with `mz-mon-`, such as `mz-mon-env-top` for **Materialize Environment Overview**.
  Dashboard UIDs are a [public interface](../../reference/stability/#public-interfaces), so a prompt or a script can name one.

Folder UIDs
: The dashboards live in the `materialize` and `infra` folders, and in `meta-o11y` inside `infra`.

Environment label
: `materialize_cloud_organization_name` scopes metrics to one Materialize environment, on self-managed installs as well as on Materialize Cloud.
