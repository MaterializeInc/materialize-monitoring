---
title: "Importing Dashboards"
weight: 1
---

# Importing Grafana Dashboards

This page covers the ways to get a Materialize dashboard into a Grafana instance.
For the dashboards themselves — what each one is for, and where to download it — see [Available Dashboards]({{< relref "../all.md" >}}).

## Importing Dashboards via the Grafana UI

Download the corresponding `.json` file from [Available Dashboards]({{< relref "../all.md" >}}).

From a Grafana instance, navigate to the "New" menu on any page
or within a specific folder.
Then, select "Import" and choose the "Upload .json file" option.

You may opt to change the title or UID at time of import.
The recommended filters are already in place.

## Importing Dashboards via `gcx` (grafana-cli)

[gcx](https://grafana.com/docs/grafana/latest/as-code/observability-as-code/grafana-cli/gcx/) is a CLI interface for
managing Grafana from the command line.
It is built for Dashboards-as-Code workflows and AI agent usage.

1. Download the corresponding `.json` file from [Available Dashboards]({{< relref "../all.md" >}}).
2. If you are not logged in via `gcx`, run `gcx login --server YOUR_GRAFANA` to authenticate with your Grafana instance.
3. Run `gcx dashboards create -f DOWNLOADED_FILE.json` to import the dashboard to your Grafana instance.

## Importing Dashboards via Grafana Operator

This is the recommended path when the dashboards are installed by the `materialize-monitoring` Helm chart, since it
keeps them in sync rather than importing a point-in-time copy.
Refer to the [Grafana Operator documentation](../grafana-operator/).

For how the chart wires Grafana, the Grafana Operator, and the datasources together, see [Grafana Architecture](../architecture/).
