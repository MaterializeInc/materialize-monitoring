---
title: "gcx"
weight: 30
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
  interface: "Command line"
  audience: "Operators, scripts, and agents with a shell"
  cluster-access: "Port-forward, if Grafana isn't exposed"
  grafana-access: "Service account token"
  ease-of-use: "Moderate: PromQL and LogQL"
  stability: "Stable within a major version"
---

# gcx

This page is for operators who want Grafana from a terminal, and for anyone wiring Grafana into a script or an AI agent.
[`gcx`](https://grafana.com/docs/grafana/latest/ai-tools/grafana-cli/gcx/) is Grafana Labs' command-line tool, and it supports Grafana 13 and later.
It reaches dashboards, data sources, and alerting through Grafana's API, so it needs a Grafana login and no Kubernetes access.

{{< param-table keys="interface,audience,cluster-access,grafana-access,ease-of-use,stability" >}}

## Connect to your Grafana

`gcx` keeps each Grafana it talks to as a named context.

To connect a context to the bundled Grafana, follow these steps:

1. In Grafana, create a service account with the **Viewer** role, and add a token to it.
   For how, see [Service accounts](https://grafana.com/docs/grafana/latest/administration/service-accounts/).
   Give it **Editor** instead only if you're going to change dashboards with it.

1. Log in, and name the context:

   ```bash
   gcx login mzmon --server GRAFANA_ADDRESS --token SERVICE_ACCOUNT_TOKEN
   ```

1. Set the chart's data sources as the context's defaults, so queries don't need a `-d` flag:

   ```bash
   gcx config set contexts.mzmon.datasources.prometheus mzmon-thanos
   gcx config set contexts.mzmon.datasources.loki mzmon-loki
   ```

Replace the following:

- `GRAFANA_ADDRESS`: the URL your Grafana serves.
  For an unexposed Grafana, port-forward it first, as [Grafana in a browser](../grafana/#reach-grafana) shows, and use `http://localhost:3000`.
- `SERVICE_ACCOUNT_TOKEN`: the token from the first step.

## Query metrics and logs

`gcx metrics query` runs PromQL against a Prometheus-type data source, and `gcx logs query` runs LogQL against a Loki one:

```bash
gcx --context mzmon metrics query 'up{namespace="materialize-environment"}' --since 1h
```

```bash
gcx --context mzmon logs query '{namespace="materialize-environment", app="environmentd"} |= "error"' --since 1h
```

Add `--share-link` to print the same query as a Grafana Explore URL, which you can hand to someone working in a browser.
For queries worth running, see [Common queries](../../reference/common-queries/).

## Read dashboards

The shipped dashboards are v2 dashboards, and `gcx dashboards` reads and writes them through Grafana's resource API:

```bash
gcx --context mzmon dashboards search "Materialize"
```

```bash
gcx --context mzmon dashboards get mz-mon-env-top -o yaml
```

A dashboard's resource name is its UID.
To import a dashboard with `gcx`, see [Importing dashboards](../../dashboards/grafana/importing/#importing-dashboards-via-gcx-grafana-cli).

`gcx dashboards snapshot` renders a dashboard to PNG through the Grafana Image Renderer.
The chart leaves the renderer off, and it should stay off in production, so snapshots fail against the bundled Grafana.

## Use gcx from an AI agent

`gcx` detects common AI agents, such as Claude Code and Cursor, and switches to JSON output when one runs it.
It also bundles agent skills, which teach an agent its commands.
The following command installs the skills for setting up `gcx`, investigating with it, and reading dashboards into `~/.agents/skills`:

```bash
gcx agent skills install setup-gcx debug-with-grafana manage-dashboards
```

`gcx agent skills list` lists the rest.

An agent with a shell needs nothing else to use `gcx`.
For an agent without one, use the [Grafana MCP server](../grafana-mcp/) instead.
