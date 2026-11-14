---
title: "Grafana MCP server"
weight: 20
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
  interface: "AI agent, over MCP"
  audience: "Operators who work with an AI agent"
  cluster-access: "Port-forward, if Grafana isn't exposed"
  grafana-access: "Service account token"
  ease-of-use: "High after setup, but check its answers"
  stability: "Fast-moving: tools change in minor releases"
---

# Grafana MCP server

This page is for operators and on-call engineers who want an AI agent, such as Claude Code, Claude Desktop, or Cursor, to read Materialize monitoring.
The [Grafana MCP server](https://grafana.com/docs/grafana/latest/ai-tools/mcp/set-up/), `mcp-grafana`, connects an agent to Grafana
over the [Model Context Protocol](https://modelcontextprotocol.io/) (MCP).
The agent can then search and read dashboards, run PromQL and LogQL through Grafana's data sources, and read alerts, without Kubernetes access.

{{< param-table keys="interface,audience,cluster-access,grafana-access,ease-of-use,stability" >}}

> [!INFO]
> The shipped dashboards use Grafana's v2 dashboard schema, which `mcp-grafana` reads and writes natively from version 0.16.0.
> Use a 2.x release, which fixes several v2 bugs, including one that let `update_dashboard` replace a v2 dashboard with a v1 body.
> An older version converts each dashboard to the v1 schema before the agent sees it, which drops the panel queries in v2-only layouts.

## What the agent can do

`mcp-grafana` groups its tools into categories, and you can turn each category on and off.
The following categories work against the bundled Grafana:

Dashboards and search
: Find dashboards, summarize one, read any property of it, and list the queries behind its panels.

Prometheus
: Run PromQL against the Thanos data source, and list metric names, labels, and label values.

Loki
: Run LogQL against the Loki data source, list labels, and read stream statistics.

Alerting
: List alert rules and their state, including the Loki ruler's.

Navigation
: Build a Grafana link to a dashboard, a panel, or an Explore query, so you can check the agent's answer in a browser.

The rendering tools export a panel as an image through the Grafana Image Renderer.
The chart leaves the renderer off, and it should stay off in production, so those tools fail against the bundled Grafana.
Other categories, such as OnCall and Incident, need Grafana products that the stack doesn't include.

Grafana Cloud also runs a [hosted MCP server](https://grafana.com/docs/grafana-cloud/ai-tools/mcp-servers/cloud-mcp/),
but it serves only Grafana Cloud stacks.
To reach the bundled Grafana, run `mcp-grafana` yourself.

## Set up the server

To connect an agent to your Grafana, follow these steps:

1. In Grafana, create a service account with the **Viewer** role, and add a token to it.
   For how, see [Service accounts](https://grafana.com/docs/grafana/latest/administration/service-accounts/).

1. Install [uv](https://docs.astral.sh/uv/getting-started/installation/), which downloads and runs `mcp-grafana` on demand.
   For the container image, release binaries, and Helm chart,
   see [Set up the Grafana MCP server](https://grafana.com/docs/grafana/latest/ai-tools/mcp/set-up/).

1. Add the server to your agent.
   For Claude Code, run the following command:

   ```bash
   claude mcp add-json grafana '{
     "command": "uvx",
     "args": ["mcp-grafana", "--disable-write"],
     "env": {
       "GRAFANA_URL": "GRAFANA_ADDRESS",
       "GRAFANA_SERVICE_ACCOUNT_TOKEN": "SERVICE_ACCOUNT_TOKEN"
     }
   }'
   ```

   For Claude Desktop, Cursor, and other agents, put the same `command`, `args`, and `env` in the agent's MCP configuration.

Replace the following:

- `GRAFANA_ADDRESS`: the URL your Grafana serves.
  For an unexposed Grafana, port-forward it first, as [Grafana in a browser](../grafana/#reach-grafana) shows, and use `http://localhost:3000`.
- `SERVICE_ACCOUNT_TOKEN`: the token from the first step.
  To keep the token out of the agent's configuration, set `GRAFANA_SERVICE_ACCOUNT_TOKEN_FILE` to the path of a file that holds it instead.

> [!WARNING]
> A Viewer token reads every metric in Thanos and every log in Loki, the same as a Viewer who signs in.
> Create a service account for each person or agent, so you can revoke one without the rest.
> For more information, see [What Grafana roles do not do](../../dashboards/grafana/auth/#what-grafana-roles-do-not-do).

`mcp-grafana` reports anonymous usage statistics to Grafana Labs by default: which tools ran and how the server is configured.
The reports never include queries, log lines, credentials, or your Grafana's address.
To turn them off, add `--usage-stats=disabled` to `args`.

### Choose what the agent can change

`--disable-write` removes the tools that create, update, or delete anything, such as dashboards and annotations.
It also limits the generic `grafana_api_request` tool to reads.
Keep it on for investigation.

Log lines reach the agent as tool output, and a log line can carry text that reads like an instruction.
With `--disable-write`, an agent that follows one can't change anything through the server.

Grafana refuses most writes from a Viewer token anyway, but `--disable-write` keeps the agent from attempting them.
Give the token the **Editor** role, and drop the flag, only when you want the agent to change dashboards.

The operator re-pushes the dashboards the chart ships every `resyncPeriod`, so an agent's edit to one doesn't last.
To change one, copy it to a new UID first, as [Drift](../../dashboards/grafana/grafana-operator/#drift) describes.

## Give the agent context

The agent knows Grafana, not this stack.
Tell it the names in [Know the names the tools share](../overview/#know-the-names-the-tools-share):
the data source UIDs, the dashboard UIDs, and the environment label.

These docs are published for agents as well as for people.
[`llms.txt`](https://materializeinc.github.io/materialize-monitoring/llms.txt) indexes every page,
and [`llms-full.txt`](https://materializeinc.github.io/materialize-monitoring/llms-full.txt) inlines the whole site.
Point the agent at the index first, because it's about a hundredth of the full file's size and usually names the page the agent needs.

The following prompts are places to start:

- "Open the `mz-mon-env-top` dashboard in Grafana and summarize the health of environment `ENVIRONMENT`."
- "Which Materialize clusters have used more than 80% of their memory limit in the last 6 hours? Show me the PromQL you ran."
- "Find error lines from `environmentd` in the last hour, group them by message, and give me an Explore link for each group."

Replace `ENVIRONMENT` with the environment's `materialize_cloud_organization_name` value.

## Check what the agent tells you

An agent reads the same data you do, and it can misread it in the same ways.
The following habits keep an answer checkable:

- **Ask for the query.** A PromQL or LogQL expression, with its time range and scope, is something you can run again.
  A summary isn't.
- **Ask for a link.** The navigation tools return a Grafana link, so you can see the panel the agent read.
- **Confirm a label before building on it.** A query that filters on a label the series doesn't carry returns nothing, which looks healthy.
- **Don't read quiet as healthy.** Only a default set of the bundled alert rules installs,
  so an absence of firing alerts doesn't mean nothing is wrong.

For how this data misleads people and agents alike, see [Troubleshooting Materialize](../../operating/troubleshooting-materialize/).
