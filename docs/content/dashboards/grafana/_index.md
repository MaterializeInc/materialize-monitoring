---
title: "Grafana"
weight: 10
bookCollapseSection: true
---

# Grafana Dashboards

This section contains documentation for Grafana Dashboards
that are recommended for managing Materialize and its infrastructure.

## Importing Dashboards via the Grafana UI

Download one of the corresponding `.json` files from the "Download Dashboards" section below.

From a Grafana instance, navigate to the "New" menu on any page
or within a specific folder.
Then, select "Import" and choose the "Upload .json file" option.

You may opt to change the title or UID at time of import.
The recommended filters are already in place.

TODO: expose other toggles upon import.
TODO: move some filters off of hidden variables.

## Importing Dashboards via `gcx` (grafana-cli)

TODO

## Importing Dashboards via Grafana Operator

Refer to the [Grafana Operator documentation](./grafana-operator).

## Download Dashboards

Below you will find links to download the Grafana dashboards in JSON format.
These dashboards use features from latest Grafana versions, so be sure to check the compatibility of your Grafana instance before importing.

### Checking your Grafana Version

To check your Grafana version, navigate to the Grafana instance and click on the "Help" menu (represented by a question mark icon) in the left sidebar.
Selecting it will show the current version.

### Dashboard v2 (Grafana 13)

TODO: environment table

[Environment Overview](../../downloads/dashboards/grafana/env-top.json)

### Dashboard v2beta1 (Grafana 12)

TODO: add links
