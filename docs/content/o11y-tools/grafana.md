---
title: "Grafana in a browser"
weight: 10
# custom parameters
params:
  author: Heather Lapointe
  agent: Claude Opus 5.5
  interface: "Browser"
  audience: "Everyone who reads dashboards"
  cluster-access: "Port-forward, if Grafana isn't exposed"
  grafana-access: "Browser login"
  ease-of-use: "Highest: nothing to install"
  stability: "Stable"
---

# Grafana in a browser

This page is for anyone who reads Materialize monitoring through Grafana's web UI.
The bundled Grafana is where every other tool in this section starts from:
the dashboards, the data sources, and the alerting views are already provisioned there.

{{< param-table keys="interface,audience,cluster-access,grafana-access,ease-of-use,stability" >}}

## Reach Grafana

How you reach Grafana depends on whether your install exposes it:

Exposed Grafana
: Open the URL your install serves, and sign in through your identity provider.
  For how an operator sets that up, see [Reaching Grafana](../../dashboards/grafana/architecture/#reaching-grafana)
  and [Authentication](../../dashboards/grafana/auth/).

Unexposed Grafana
: The Service is `ClusterIP` by default, so you need a port-forward and Kubernetes access to the Grafana namespace.

To reach an unexposed Grafana, follow these steps:

1. Forward a local port to the Grafana Service:

   ```bash
   kubectl --namespace monitoring port-forward svc/grafana 3000:80
   ```

1. Read the generated admin user name and password from their Secret:

   ```bash
   kubectl --namespace monitoring get secret grafana -o jsonpath="{.data.admin-user}" | base64 -d; echo
   kubectl --namespace monitoring get secret grafana -o jsonpath="{.data.admin-password}" | base64 -d; echo
   ```

1. Open `http://localhost:3000`, and sign in with that user name and password.

Under the `split-namespace` profile, Grafana runs in the `grafana` namespace instead of `monitoring`.

> [!WARNING]
> A Grafana role limits what you can change, not what you can read.
> Anyone who can sign in can query every data source, so a Viewer reads every metric in Thanos and every log in Loki from Explore.
> For more information, see [What Grafana roles do not do](../../dashboards/grafana/auth/#what-grafana-roles-do-not-do).

## Find your way around

Start from the following places:

Dashboards
: The **Materialize** folder holds the per-environment dashboards, and **Infrastructure** holds the platform ones.
  Start with **Materialize Environment Overview**.
  For what each dashboard answers, see [Available dashboards](../../dashboards/all/).

Explore
: Query the **Thanos** data source in PromQL and the **Loki** data source in LogQL.
  For queries to start from, see [Common queries](../../reference/common-queries/).

Alerting
: The **Alertmanager** data source shows firing alerts, and **Alerting** > **Silences** manages silences in it.
  **Alerting** > **Alert rules** lists the Loki ruler's rules, read-only.
  For more information, see [Alert architecture](../../alerting/architecture/) and [Maintenance windows](../../alerting/maintenance/).

When an investigation leads somewhere unfamiliar, [Troubleshooting Materialize](../../operating/troubleshooting-materialize/)
says which dashboard answers which question.

## Share what you find

A dashboard URL carries its UID, its time range, and its variables, so the link reproduces your view for anyone who can sign in.
Dashboard UIDs are on the [list of public interfaces](../../reference/stability/#public-interfaces),
so a dashboard's link changes only through a deprecation cycle.
Dashboard variables aren't, so a link that sets one can stop selecting what it did after an upgrade.

Share links are built from `grafana.ini`'s `server.root_url`.
On an exposed Grafana, a copied link that points at the wrong host means `root_url` doesn't match the URL users reach.
An operator needs to correct it.
