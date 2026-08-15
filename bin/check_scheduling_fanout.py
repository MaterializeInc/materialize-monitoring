#!/usr/bin/env python3
"""Assert the scheduling fan-out reached every workload it should, and no others.

Also checks one topology-spread invariant, since it parses the same manifests:
`minDomains` is only valid alongside `whenUnsatisfiable: DoNotSchedule`, and the
API server rejects the pod outright otherwise. That is worth catching at render
time because the rejection surfaces as an admission error on a controller nobody
is watching, not as a failed `helm upgrade`.

Called from `bin/terraform-render-check.sh` with a rendered chart and the values
documents a Terraform plan composed. Silent and successful when the example sets
no scheduling.

The point of this check is that a `nodeSelector` written to a path no subchart
reads renders perfectly well and is silently ignored, so rendering alone proves
nothing. Counting is not enough either: the per-node DaemonSets are supposed to
*lack* the selector, so this asserts the exact set in both directions.

The subcharts disagree about where scheduling goes — `thanos.global`,
`loki.defaults`, `alloy-*.controller`, top level for the rest — and three Loki
components render from their own templates rather than `_pod.tpl`, so they fall
out of `defaults` and must be named individually. That map lives in two places
(`terraform/modules/materialize-monitoring/scheduling.tf` for Terraform,
`charts/materialize-monitoring/profiles/scheduling.values.yaml` for Helm), and
this is what keeps them honest against the pinned subchart versions.
"""

from __future__ import annotations

import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

import yaml

WORKLOAD_KINDS = {"Deployment", "StatefulSet", "DaemonSet"}

# Workloads that must NOT carry a node selector, by name prefix.
#
# Both are per-node collectors. A node selector *narrows* where a pod may run, so
# on a DaemonSet whose job is to observe every node it is a silent blind spot —
# the pods simply stop landing on the excluded nodes and no dashboard shows a
# hole. Tolerations are the opposite (they widen placement), which is why
# `alloy-agent` still takes those and only `node-exporter` is excluded from both.
NO_NODE_SELECTOR = ("alloy-agent", "node-exporter")
NO_TOLERATIONS = ("node-exporter",)

# The toleration the chart ships on the agent, which a caller's own list must not
# cost them: keyless `Exists` on `NoSchedule`, i.e. every NoSchedule taint. Helm
# overwrites lists, so `alloy-agent.controller.tolerations` is replaced wholesale
# by whatever the module writes there; `daemonset_tolerations` in scheduling.tf
# exists to append instead. The failure that guards against is invisible from the
# values alone — the rendered pod carries a toleration list that looks entirely
# correct while covering only the taints the caller happened to name.
CHART_AGENT_TOLERATION = {"key": None, "operator": "Exists", "effect": "NoSchedule"}


def find_value(obj: Any, key: str) -> Any:
    """First non-empty value for `key` anywhere in the composed values.

    Every fan-out site carries the same value, so the first is representative.
    """
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k == key and v:
                return v
            if found := find_value(v, key):
                return found
    elif isinstance(obj, list):
        for item in obj:
            if found := find_value(item, key):
                return found
    return None


def has_selector(spec: dict[str, Any], expected: dict[str, str]) -> bool:
    """Whether a pod spec carries every expected node-selector label.

    Subset rather than equality: node-exporter ships its own
    `kubernetes.io/os: linux`, and maps merge rather than replace.
    """
    got = spec.get("nodeSelector") or {}
    return all(got.get(k) == v for k, v in expected.items())


def has_tolerations(spec: dict[str, Any], expected: list[dict[str, Any]]) -> bool:
    """Whether a pod spec tolerates every expected taint key."""
    keys = {t.get("key") for t in (spec.get("tolerations") or [])}
    return all(t.get("key") in keys for t in expected)


def audit(
    workloads: list[tuple[str, dict[str, Any]]],
    expected: Any,
    predicate: Callable[[dict[str, Any], Any], bool],
    excluded_prefixes: tuple[str, ...],
) -> tuple[list[str], list[str]]:
    """Split workloads into (wrongly missing, wrongly present)."""
    missing, unexpected = [], []
    for name, spec in workloads:
        carries = predicate(spec, expected)
        excluded = name.startswith(excluded_prefixes)
        if carries and excluded:
            unexpected.append(name)
        elif not carries and not excluded:
            missing.append(name)
    return missing, unexpected


def bad_min_domains(workloads: list[tuple[str, dict[str, Any]]]) -> list[str]:
    """Workloads carrying `minDomains` on a constraint that is not DoNotSchedule.

    `min_zones` in the Terraform module patches `minDomains` onto the hard zone
    rule only, for exactly this reason; sweeping up the soft host rule alongside
    it would render valid YAML that the API server refuses.
    """
    offenders = []
    for name, spec in workloads:
        for c in spec.get("topologySpreadConstraints") or []:
            if "minDomains" in c and c.get("whenUnsatisfiable") != "DoNotSchedule":
                offenders.append(f"{name} ({c.get('topologyKey')})")
    return offenders


def dropped_chart_tolerations(
    workloads: list[tuple[str, dict[str, Any]]],
) -> list[str]:
    """Agent DaemonSets whose blanket NoSchedule toleration was overwritten."""
    offenders = []
    for name, spec in workloads:
        if not name.startswith("alloy-agent"):
            continue
        if not any(
            all(t.get(k) == v for k, v in CHART_AGENT_TOLERATION.items())
            for t in (spec.get("tolerations") or [])
        ):
            offenders.append(name)
    return offenders


def load_docs(paths: list[str]) -> list[Any]:
    """Parse every YAML document from the paths that exist."""
    docs: list[Any] = []
    for p in paths:
        path = Path(p)
        if not path.exists():  # an unmatched shell glob arrives literally
            continue
        docs.extend(d for d in yaml.safe_load_all(path.read_text()) if d)
    return docs


def main(argv: list[str]) -> int:
    """Compare the rendered chart against the scheduling the values asked for."""
    rendered_path, *values_paths = argv
    values = load_docs(values_paths)

    expected_selector = find_value(values, "nodeSelector")
    expected_tolerations = find_value(values, "tolerations")

    workloads = [
        (doc["metadata"]["name"], doc["spec"]["template"]["spec"])
        for doc in yaml.safe_load_all(Path(rendered_path).read_text())
        if doc and doc.get("kind") in WORKLOAD_KINDS
    ]

    problems: list[tuple[str, list[str]]] = [
        (
            "carry minDomains on a non-DoNotSchedule constraint",
            bad_min_domains(workloads),
        ),
    ]
    if expected_selector:
        missing, unexpected = audit(
            workloads, expected_selector, has_selector, NO_NODE_SELECTOR
        )
        problems += [
            ("missing the node selector", missing),
            ("carry a node selector they must not", unexpected),
        ]
    if expected_tolerations:
        missing, unexpected = audit(
            workloads, expected_tolerations, has_tolerations, NO_TOLERATIONS
        )
        problems += [
            ("missing the tolerations", missing),
            ("carry tolerations they must not", unexpected),
            (
                "lost the chart's blanket NoSchedule toleration",
                dropped_chart_tolerations(workloads),
            ),
        ]

    failed = False
    for label, names in problems:
        if names:
            failed = True
            print(
                f"    !! {len(names)} workload(s) {label}: {', '.join(sorted(names))}",
                file=sys.stderr,
            )
    if failed:
        return 1

    if expected_selector or expected_tolerations:
        print(
            "    scheduling reached every workload it should, and no DaemonSet it should not"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
