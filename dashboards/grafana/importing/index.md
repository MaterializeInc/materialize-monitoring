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

{{< grafana-dashboards pattern="dashboards/grafana/*.json" apiVersion="dashboard.grafana.app/v2" >}}

> [!INFO]
> There are minor differences in Google Cloud Platform metrics exposed
> by GKE, so you should select a dashboard that has that particular cloud annotation.

### Grafana 10 and 11 (Dashboard Schema v1)

> [!WARNING]
> Not published yet. The dashboards are authored against Dashboard v2, and no v1 render ships — see the
> [roadmap](https://github.com/MaterializeInc/materialize-monitoring/blob/main/docs/content/reference/internal/roadmap.md)
> for the v1 gallery item. Grafana can import a v2 dashboard on 12+ only.

