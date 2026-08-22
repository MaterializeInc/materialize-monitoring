---
title: "Prometheus Scraper"
weight: 30
---

# Prometheus Scraper (classic `scrape_config`)

For a Prometheus that is not operator-managed, or where this stack is not in the collection path at all.

Prometheus Operator CRDs are the preferred integration, but plenty of Prometheus deployments are a config file and a binary.
For those, the repository ships classic `scrape_configs` covering the Materialize workloads, which you paste into your own `prometheus.yml`.

Download them from [Scraping > Classic ScrapeConfig Downloads](../../scraping/#classic).

> [!INFO]
>   These are best-effort convenience. If your Prometheus can consume `ServiceMonitor` resources — including via `prometheus-operator`, `kube-prometheus-stack`, or Alloy — [that path](../prometheus-operator/) is better maintained.

## Using them

Each file is a single `scrape_config` entry. Add it under the top-level `scrape_configs` key:

```yaml
scrape_configs:
  - job_name: materialize-environmentd
    # ... contents of the downloaded file
```

The configs use Kubernetes service discovery, so the Prometheus running them needs in-cluster RBAC to list pods in the Materialize namespaces.

The SQL-derived metrics need the `mz_support` username — the classic configs carry it inline, so unlike the operator path they need no Secret.
See [Authenticating the SQL metrics endpoint](../../scraping/#authenticating-the-sql-metrics-endpoint) for why the password does not matter.

## Getting the data into this stack

Scraping with your own Prometheus and *using* this stack are independent choices.

- **Keep the data in your Prometheus.** Point Grafana at it as a datasource and import the [dashboards](../../../dashboards/grafana/importing/). Nothing else here is required.
- **Ship it here as well.** Add a [remote-write](../prometheus-remote-write/) block pointing at the gateway. Your Prometheus keeps its local copy and this stack gets one too.

## See also

- [Overview](../overview/) — the four collection paths.
- [Prometheus Operator](../prometheus-operator/) — the preferred path when the CRDs are available.
- [Prometheus Remote Write](../prometheus-remote-write/) — forwarding what you scrape into this stack.
