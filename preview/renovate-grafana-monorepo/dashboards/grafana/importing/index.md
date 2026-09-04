# Importing Dashboards




# Importing Grafana Dashboards

The Grafana dashboards below are recommended for managing Materialize and its infrastructure.
This page covers the ways to import them and where to download them.

## Importing Dashboards via the Grafana UI

Download one of the corresponding `.json` files from the [Download Dashboards](#download-dashboards) section below.

From a Grafana instance, navigate to the "New" menu on any page
or within a specific folder.
Then, select "Import" and choose the "Upload .json file" option.

You may opt to change the title or UID at time of import.
The recommended filters are already in place.

## Importing Dashboards via `gcx` (grafana-cli)

[gcx](https://grafana.com/docs/grafana/latest/as-code/observability-as-code/grafana-cli/gcx/) is a CLI interface for
managing Grafana from the command line.
It is built for Dashboards-as-Code workflows and AI agent usage.

1. Download one of the corresponding `.json` files from the [Download Dashboards](#download-dashboards) section below.
2. If you are not logged in via `gcx`, run `gcx login --server YOUR_GRAFANA` to authenticate with your Grafana instance.
3. Run `gcx dashboards create -f DOWNLOADED_FILE.json` to import the dashboard to your Grafana instance.

## Importing Dashboards via Grafana Operator

This is the recommended path when the dashboards are installed by the `materialize-monitoring` Helm chart, since it
keeps them in sync rather than importing a point-in-time copy.
Refer to the [Grafana Operator documentation](../grafana-operator/).

For how the chart wires Grafana, the Grafana Operator, and the datasources together, see [Grafana Architecture](../architecture/).

## Download Dashboards

Below you will find links to download the Grafana dashboards in JSON format.
These dashboards use features from latest Grafana versions, so be sure to check the compatibility of your Grafana instance before importing.

### Checking your Grafana Version

To check your Grafana version, navigate to the Grafana instance and click on the "Help" menu (represented by a question
mark icon) in the left sidebar.
Selecting it will show the current version.

### Grafana 12 and 13 (Dashboard Schema v2)

> [!SUCCESS]
> Right now, these are the only supported dashboards.


<table class="grafana-dashboards">
  <thead>
    <tr>
      <th>Dashboard</th>
      <th>Description</th>
      <th>Annotations</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Materialize Logs and Events
        <br />(<a href="/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/env-logs.json?xxhash=943c1b701937d70b" download="mz-mon-env-logs-943c1b701937d70b.json"><code>env-logs.json</code></a>)
      </td>
      <td><p>Logs and Kubernetes events for a Materialize deployment.</p>
<p>What the workloads said, as opposed to what they measured.</p>
</td>
      <td>
            <strong>min-mz-version</strong>: v26.24.0<br />
            <strong>rec-mz-version</strong>: v26.24.0<br />
            <strong>sql-metric-prefix</strong>: mz_<br />
            <strong>target-export</strong>: generic<br />
      </td>
    </tr>
    <tr>
      <td>Materialize Environment Overview
        <br />(<a href="/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/env-top.json?xxhash=6be77f70e14054d1" download="mz-mon-env-top-6be77f70e14054d1.json"><code>env-top.json</code></a>)
      </td>
      <td><p>Overview of a Materialize Environment.</p>
<p>This provides a high-level summary to catch more obvious issues
that may require further investigation.</p>
</td>
      <td>
            <strong>min-mz-version</strong>: v26.24.0<br />
            <strong>rec-mz-version</strong>: v26.29.0<br />
            <strong>sql-metric-prefix</strong>: mz_<br />
            <strong>target-export</strong>: generic<br />
      </td>
    </tr>
    <tr>
      <td>Materialize Upgrade
        <br />(<a href="/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/env-upgrade.json?xxhash=11e79750fdfde4cc" download="mz-mon-env-upgrade-11e79750fdfde4cc.json"><code>env-upgrade.json</code></a>)
      </td>
      <td><p>What happened during a Materialize upgrade.</p>
<p>The operator&rsquo;s own account of a rollout, alongside
what the cluster did underneath it.</p>
</td>
      <td>
            <strong>min-mz-version</strong>: v26.41.0<br />
            <strong>rec-mz-version</strong>: v26.41.0<br />
            <strong>sql-metric-prefix</strong>: mz_<br />
            <strong>target-export</strong>: generic<br />
      </td>
    </tr>
    <tr>
      <td>Infrastructure Logs and Events
        <br />(<a href="/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/infra-logs.json?xxhash=d283fbfb8a6e250f" download="mz-mon-infra-logs-d283fbfb8a6e250f.json"><code>infra-logs.json</code></a>)
      </td>
      <td><p>Logs and Kubernetes events for the platform a Materialize deployment runs on.</p>
<p>The monitoring stack, the Kubernetes system components, and the node journal.</p>
</td>
      <td>
            <strong>min-mz-version</strong>: v26.24.0<br />
            <strong>rec-mz-version</strong>: v26.24.0<br />
            <strong>sql-metric-prefix</strong>: mz_<br />
            <strong>target-export</strong>: generic<br />
      </td>
    </tr>
    <tr>
      <td>Infrastructure Node Detail
        <br />(<a href="/materialize-monitoring/preview/renovate-grafana-monorepo/dashboards/grafana/infra-nodes.json?xxhash=3fb217a8dd0baf5b" download="mz-mon-infra-nodes-3fb217a8dd0baf5b.json"><code>infra-nodes.json</code></a>)
      </td>
      <td><p>Everything about one node a Materialize deployment runs on.</p>
<p>What the machine is, how hard it is working, how much of it is already promised to pods, and what it and Kubernetes have said about it.</p>
</td>
      <td>
            <strong>min-mz-version</strong>: v26.24.0<br />
            <strong>rec-mz-version</strong>: v26.24.0<br />
            <strong>sql-metric-prefix</strong>: mz_<br />
            <strong>target-export</strong>: generic<br />
      </td>
    </tr>
  </tbody>
</table>


> [!INFO]
> There are minor differences in Google Cloud Platform metrics exposed
> by GKE, so you should select a dashboard that has that particular cloud annotation.

### Grafana 10 and 11 (Dashboard Schema v1)

> [!WARNING]
> Not published yet. The dashboards are authored against Dashboard v2, and no v1 render ships — see the
> [roadmap](https://github.com/MaterializeInc/materialize-monitoring/blob/main/docs/content/reference/internal/roadmap.md)
> for the v1 gallery item. Grafana can import a v2 dashboard on 12+ only.

