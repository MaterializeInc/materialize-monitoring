---
title: "Common Alerts"
weight: 40
aliases:
  - /reference/stable-metrics/common-alerts/
---

# Common Alerts

These are the alerting rules the chart bundles, generated from the query registry.
Each is installed only where the deployment has the capabilities it requires, and only the default set installs without
being selected; see [Configuring Alerting]({{< relref "../alerting/configuring.md" >}}#the-bundled-rules).

> [!WARNING]
> Rules outside the default set have not all been checked against a self-managed install.
> Evaluate one against the deployment before relying on it.

{{% list-alerts %}}
