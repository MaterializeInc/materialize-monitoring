# Overview




# Observability tools

This page is for anyone who reads data from a `materialize-monitoring` install, and for the operators who decide who gets access.
Every tool in this section reads the same metrics, logs, and alerts.
The tools differ in who they suit, what access they need, and how much they ask of you.

## Compare the tools

The following table classifies each tool.
Each tool's page carries the same classification at its top.


<table class="param-matrix">
  <thead>
    <tr>
      <td></td>
      <th scope="col"><a href="/materialize-monitoring/preview/renovate-grafana-alloy-1-x/o11y-tools/grafana/">Grafana in a browser</a></th>
      <th scope="col"><a href="/materialize-monitoring/preview/renovate-grafana-alloy-1-x/o11y-tools/grafana-mcp/">Grafana MCP server</a></th>
      <th scope="col"><a href="/materialize-monitoring/preview/renovate-grafana-alloy-1-x/o11y-tools/gcx/">gcx</a></th>
      <th scope="col"><a href="/materialize-monitoring/preview/renovate-grafana-alloy-1-x/o11y-tools/backend-apis/">Backend APIs</a></th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <th scope="row">Interface</th>
      <td>Browser</td>
      <td>AI agent, over MCP</td>
      <td>Command line</td>
      <td>HTTP API</td>
    </tr>
    <tr>
      <th scope="row">Audience</th>
      <td>Everyone who reads dashboards</td>
      <td>Operators who work with an AI agent</td>
      <td>Operators, scripts, and agents with a shell</td>
      <td>Operators debugging the stack</td>
    </tr>
    <tr>
      <th scope="row">Cluster access</th>
      <td>Port-forward, if Grafana isn&rsquo;t exposed</td>
      <td>Port-forward, if Grafana isn&rsquo;t exposed</td>
      <td>Port-forward, if Grafana isn&rsquo;t exposed</td>
      <td>Port-forward or exec in each backend&rsquo;s namespace</td>
    </tr>
    <tr>
      <th scope="row">Grafana access</th>
      <td>Browser login</td>
      <td>Service account token</td>
      <td>Service account token</td>
      <td>None</td>
    </tr>
    <tr>
      <th scope="row">Ease of use</th>
      <td>Highest: nothing to install</td>
      <td>High after setup, but check its answers</td>
      <td>Moderate: PromQL and LogQL</td>
      <td>Lowest: <code>kubectl</code>, and PromQL or LogQL</td>
    </tr>
    <tr>
      <th scope="row">Stability</th>
      <td>Stable</td>
      <td>Fast-moving: tools change in minor releases</td>
      <td>Stable within a major version</td>
      <td>Stable upstream APIs</td>
    </tr>
  </tbody>
</table>


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

