# Zone-spread adjustment.
#
# The chart's hard zone constraints assume a cloud cluster with at least two
# availability zones, and they are hard on purpose: a pod that cannot satisfy one
# goes Pending, which is the signal Karpenter uses to add a node in the deficient
# zone. That assumption does not hold everywhere, and where it fails it fails
# closed rather than degrading.
#
# `minDomains: 2` with fewer than two eligible domains is the sharp edge. When the
# number of eligible domains is below `minDomains`, Kubernetes treats the global
# minimum as 0 — so a single zone holding all three replicas computes a skew of 3
# against a `maxSkew` of 1 and every pod stays Pending. **One zone is exactly as
# broken as none.**
#
# Zero zones is worse still and more common than it sounds: `kind` labels no node
# with `topology.kubernetes.io/zone`, and neither do many on-premises
# distributions. A `DoNotSchedule` constraint whose topologyKey no node carries
# has no domain to place into, so the pods are unschedulable rather than merely
# unbalanced. Nobody should have to hand-label their nodes to install a
# monitoring stack.
#
# So `min_zones` rewrites the constraints rather than asking the operator to
# restate them:
#
#   * `null` (default) — leave the chart's defaults alone. Right for any cluster
#     with two or more zones, which is every managed cloud default.
#   * `0` — drop the hard zone constraints entirely. The soft host spread stays,
#     so pods still prefer to land on different nodes.
#   * `1` — keep them hard but set `minDomains: 1`, which is satisfiable with one
#     domain and becomes real protection the day a second zone appears.
#   * `2`+ — keep them hard with `minDomains` set to the real zone count, which
#     is stricter than the chart's floor of 2 whenever you have more.
#
# The constraint *shape* is read from the chart's own values rather than restated
# here, the same way `loki_schema_configs` is in values.tf. Only `minDomains`
# changes, so a constraint the chart adds later is carried along for free and
# there is no second copy of the spread policy to drift.

locals {
  zone_topology_key = "topology.kubernetes.io/zone"

  # The two workloads whose zone constraint is hard, keyed by the values path it
  # is written back to. Both are replication-factor-3 ring members, which is why
  # they are the two that are hard in the first place.
  zone_spread_source = {
    thanos_receive = local.chart_values.thanos.receive.topologySpreadConstraints
    loki_ingester  = local.chart_values.loki.ingester.topologySpreadConstraints
  }

  # Split, adjust, recombine — deliberately without a conditional expression
  # inside the comprehension. A ternary there would have to unify the types of a
  # patched constraint and an unpatched one, and the soft host rule carries
  # neither `minDomains` nor `nodeTaintsPolicy`, so the two objects have different
  # attribute sets and Terraform refuses. Two filtered comprehensions and a
  # concat sidestep it, and preserve the chart's hard-then-soft ordering.
  #
  # `minDomains` is only valid alongside `whenUnsatisfiable: DoNotSchedule` — the
  # API server rejects it otherwise — which is the other reason the soft rule must
  # not be swept up here.
  zone_spread_adjusted = {
    for name, constraints in local.zone_spread_source : name => concat(
      var.min_zones == 0 ? [] : [
        for c in constraints : merge(c, { minDomains = var.min_zones })
        if c.whenUnsatisfiable == "DoNotSchedule" && c.topologyKey == local.zone_topology_key
      ],
      [
        for c in constraints : c
        if !(c.whenUnsatisfiable == "DoNotSchedule" && c.topologyKey == local.zone_topology_key)
      ],
    )
  }

  zone_spread_document = var.min_zones == null ? [] : [yamlencode({
    thanos = {
      receive = { topologySpreadConstraints = local.zone_spread_adjusted.thanos_receive }
    }
    loki = {
      ingester = { topologySpreadConstraints = local.zone_spread_adjusted.loki_ingester }
    }
  })]
}
