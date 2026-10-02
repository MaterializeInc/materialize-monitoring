#!/usr/bin/env python3
# Copyright Materialize, Inc. and contributors. All rights reserved.
"""Assert the module's alerting inputs land in the rendered chart.

Called from `bin/terraform-render-check.sh` with the example's plan and its
rendered chart. Silent and successful when the example declares no alerting.

The expectation is read from what the example *declared* — Terraform's own
attribute names, `for_duration` and `excluded_namespaces` — and checked against
the render, so the module's translation into chart keys is what gets tested.
Every one of these fails quietly when it is wrong:

  * a receiver written to a path the chart does not read renders an
    Alertmanager with no receiver, which routes every alert to nobody;
  * a `preset` that does not land leaves the chart's default, so `critical`
    reaches a chat channel where a pager was asked for;
  * a receiver Secret in the wrong namespace, or under keys the receivers do not
    read, is an optional mount with no file behind it, and Alertmanager fails
    the notification at the moment it is needed;
  * an override written to the wrong key leaves the rule's own `for`, so a
    cluster that hydrates for hours raises `cluster-hydration-stuck` every time.

Exit status: 0 when everything declared landed, 1 when something did not.
Anything else is a tooling failure the caller reports as such.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any

import yaml

# Where the chart mounts the `alertmanager-receivers` Secret. Fixed by the chart.
RECEIVER_MOUNT = "/etc/alertmanager/secrets/alertmanager-receivers"
CHART_VALUES = (
    Path(__file__).resolve().parent.parent / "charts/materialize-monitoring/values.yaml"
)


def declared(plan: dict, name: str) -> Any:
    """Return the constant value the example passed for `name`, or None."""
    expr = (
        plan.get("configuration", {})
        .get("root_module", {})
        .get("module_calls", {})
        .get("monitoring", {})
        .get("expressions", {})
        .get(name, {})
    )
    return expr.get("constant_value")


def planned_receiver_secret(plan: dict) -> dict | None:
    """Return the planned `alertmanager-receivers` Secret, if the module creates one."""
    for module in (
        plan.get("planned_values", {}).get("root_module", {}).get("child_modules", [])
    ):
        for res in module.get("resources", []):
            if (
                res.get("type") == "kubernetes_secret"
                and res.get("name") == "alertmanager_receivers"
            ):
                values = res.get("values") or {}
                return {
                    "namespace": (values.get("metadata") or [{}])[0].get("namespace"),
                    "data": values.get("data") or {},
                }
    return None


def rendered_documents(path: str) -> list[dict]:
    """Return every mapping document in the rendered chart."""
    with Path(path).open() as fh:
        return [d for d in yaml.safe_load_all(fh) if isinstance(d, dict)]


def alertmanager(docs: list[dict]) -> tuple[dict, str | None]:
    """Return the rendered Alertmanager configuration and the namespace its pods run in."""
    config, namespace = {}, None
    for doc in docs:
        meta = doc.get("metadata") or {}
        if doc.get("kind") == "Secret" and meta.get("name") == "alertmanager-config":
            config = (
                yaml.safe_load(
                    (doc.get("stringData") or {}).get("alertmanager.yml") or "{}"
                )
                or {}
            )
        if doc.get("kind") == "StatefulSet" and meta.get("name") == "alertmanager":
            namespace = meta.get("namespace")
    return config, namespace


def walk_routes(route: dict, path: tuple = ()) -> list[tuple[tuple, dict]]:
    """Return every route in the tree, with the matchers of every route above it."""
    here = path + tuple(route.get("matchers") or [])
    out = [(here, route)]
    for child in route.get("routes") or []:
        out.extend(walk_routes(child, here))
    return out


def receivers_for_severity(root: dict, severity: str) -> set[str]:
    """Return the receivers an alert of `severity` reaches through the severity routes."""
    want = f'severity="{severity}"'
    return {
        r["receiver"]
        for matchers, r in walk_routes(root)
        if want in matchers and r.get("receiver")
    }


def classes(receiver: dict) -> set[str]:
    """Return the classes a declared receiver serves, as a set."""
    cls = receiver.get("class") or []
    return {cls} if isinstance(cls, str) else set(cls)


# ---- alerting.* ---------------------------------------------------------------


def check_receivers(want: dict, config: dict, problems: list[str]) -> list[str]:
    """Check every declared receiver rendered, with every integration it declared."""
    rendered = {r.get("name"): r for r in config.get("receivers") or []}
    declared_receivers = want.get("receivers") or {}
    for name, receiver in declared_receivers.items():
        got = rendered.get(name)
        if got is None:
            problems.append(
                f"receiver {name!r} is not in the rendered Alertmanager configuration"
            )
            continue
        problems.extend(
            f"receiver {name!r} rendered without its {integration}"
            for integration in (receiver.get("config") or {})
            if integration not in got
        )
    return (
        [f"{len(declared_receivers)} receiver(s) rendered"]
        if declared_receivers
        else []
    )


def check_preset(want: dict, config: dict, problems: list[str]) -> list[str]:
    """Check the preset by its effect: `critical` reaches whoever serves its class.

    That is the only thing about a preset that matters. Left at the chart's
    default, the class differs, and so, in any deployment with more than one
    receiver, does who gets woken.
    """
    preset = want.get("preset")
    if not preset:
        return []
    chart_presets = (
        yaml.safe_load(CHART_VALUES.read_text()).get("alerting") or {}
    ).get("presets") or {}
    target = ({**chart_presets, **(want.get("presets") or {})}.get(preset) or {}).get(
        "critical"
    )
    if target in {None, "suppressed"}:
        return []
    serving = {
        n for n, r in (want.get("receivers") or {}).items() if target in classes(r)
    }
    if not serving:
        return [
            f"preset {preset}: no declared receiver serves {target!r}, so nothing to follow"
        ]
    reached = receivers_for_severity(config.get("route") or {}, "critical")
    if not reached & serving:
        problems.append(
            f"preset {preset} routes critical to class {target!r}, served by {sorted(serving)}, "
            f"but critical alerts reach {sorted(reached) or 'nobody'}"
        )
        return []
    return [f"preset {preset}: critical reaches {sorted(reached & serving)}"]


def check_extra_routes(want: dict, config: dict, problems: list[str]) -> list[str]:
    """Check every declared extra route sits at the top of the route tree."""
    extra = (want.get("routes") or {}).get("extra") or []
    top = (config.get("route") or {}).get("routes") or []
    for route in extra:
        found = any(
            r.get("receiver") == route.get("receiver")
            and list(r.get("matchers") or []) == list(route.get("matchers") or [])
            for r in top
        )
        if not found:
            problems.append(
                f"extra route to {route.get('receiver')!r} on {route.get('matchers')} is not in the route tree"
            )
    return [f"{len(extra)} extra route(s) ahead of the preset"] if extra else []


def check_secret(
    secret: dict,
    config: dict,
    am_namespace: str | None,
    rendered_text: str,
    problems: list[str],
) -> list[str]:
    """Check the receiver Secret is read, lives beside Alertmanager, and never reached the release."""
    keys = sorted(secret["data"])
    text = yaml.safe_dump(config)
    unread = [k for k in keys if f"{RECEIVER_MOUNT}/{k}" not in text]
    if unread:
        problems.append(f"the receiver Secret sets {unread}, which no receiver reads")
    if am_namespace and secret["namespace"] != am_namespace:
        problems.append(
            f"the receiver Secret is in {secret['namespace']!r} but Alertmanager runs in {am_namespace!r}; "
            "the optional mount finds nothing (set alertmanager_namespace)"
        )
    # The reason these are not values at all: anything in `values` is readable
    # with `helm get values`. A credential in the render got there through them.
    leaked = sorted(k for k, v in secret["data"].items() if v and v in rendered_text)
    if leaked:
        problems.append(f"receiver credentials reached the Helm release: {leaked}")
    return [
        f"receiver Secret read by path ({len(keys)} key(s)), in {secret['namespace']}, absent from the release"
    ]


# ---- rules.* ------------------------------------------------------------------


def rules_by_alert(docs: list[dict]) -> dict[str, dict]:
    """Return every rendered alerting rule, keyed by alert name."""
    out = {}
    for doc in docs:
        if doc.get("kind") != "PrometheusRule":
            continue
        for group in (doc.get("spec") or {}).get("groups") or []:
            for rule in group.get("rules") or []:
                if "alert" in rule:
                    out[rule["alert"]] = rule
    return out


def check_selection(want: dict, rules: dict[str, dict], problems: list[str]) -> None:
    """Check selected alerts installed and disabled ones did not.

    `selected` takes alert names (kebab-case), group names (snake_case) and
    `*`; only alert names name one rule to look for. A selected alert that
    needs a missing capability is the chart's to warn about, and the examples
    select only rules that apply.
    """
    problems.extend(
        f"selected alert {name!r} did not install"
        for name in want.get("selected") or []
        if "-" in name and name not in rules
    )
    problems.extend(
        f"disabled alert {name!r} still installed"
        for name in want.get("disabled") or []
        if name in rules
    )


def check_overrides(
    want: dict, rules: dict[str, dict], problems: list[str]
) -> list[str]:
    """Check each override's `for_duration` and labels reached its rule."""
    overrides = want.get("overrides") or {}
    for name, override in overrides.items():
        rule = rules.get(name)
        if rule is None:
            continue  # not installed; the chart warns about that itself
        duration = override.get("for_duration")
        if duration and rule.get("for") != duration:
            problems.append(
                f"{name}: for is {rule.get('for')!r}, overridden to {duration!r}"
            )
        labels = rule.get("labels") or {}
        problems.extend(
            f"{name}: label {label} is {labels.get(label)!r}, overridden to {value!r}"
            for label, value in (override.get("labels") or {}).items()
            if labels.get(label) != value
        )
    return [f"{len(overrides)} override(s) applied"] if overrides else []


def check_scopes(want: dict, exprs: str, problems: list[str]) -> None:
    """Check namespaces and workload tiers reached the rule expressions.

    A tier only lands where an installed rule reads it, so an example
    replacing a tier also selects a rule that reads that tier.
    """
    for ns in want.get("excluded_namespaces") or []:
        if not re.search(r'namespace!~"[^"]*\b' + re.escape(ns) + r"\b", exprs):
            problems.append(f"excluded namespace {ns!r} reaches no rule's exclusion")
    for ns in want.get("environment_namespaces") or []:
        if not re.search(r'namespace=~"[^"]*\b' + re.escape(ns) + r"\b", exprs):
            problems.append(f"environment namespace {ns!r} reaches no rule's scope")
    for tier, names in (want.get("infra_workloads") or {}).items():
        problems.extend(
            f"infra_workloads.{tier} entry {entry!r} reaches no rule"
            for entry in names or []
            if entry not in exprs
        )


def check_rules(want: dict, docs: list[dict], problems: list[str]) -> list[str]:
    """Check the declared rule selection, overrides and scopes reached the PrometheusRules."""
    rules = rules_by_alert(docs)
    if want.get("enabled") is False:
        if rules:
            problems.append(
                f"alert_rules.enabled is false but {len(rules)} rule(s) rendered"
            )
        return ["rules off, none rendered"]
    check_selection(want, rules, problems)
    notes = check_overrides(want, rules, problems)
    check_scopes(want, "\n".join(r.get("expr", "") for r in rules.values()), problems)
    return [*notes, f"{len(rules)} rule(s) installed"]


def main() -> int:
    """Run every check the example's declarations call for, and report."""
    plan = json.loads(Path(sys.argv[1]).read_text())
    rendered_path = sys.argv[2]
    want_alerting = declared(plan, "alerting") or {}
    want_rules = declared(plan, "alert_rules") or {}
    secret = planned_receiver_secret(plan)
    if not (want_alerting or want_rules or secret):
        return 0

    docs = rendered_documents(rendered_path)
    config, am_namespace = alertmanager(docs)
    problems: list[str] = []
    notes: list[str] = []
    if want_alerting:
        notes += check_receivers(want_alerting, config, problems)
        notes += check_preset(want_alerting, config, problems)
        notes += check_extra_routes(want_alerting, config, problems)
    if secret:
        notes += check_secret(
            secret, config, am_namespace, Path(rendered_path).read_text(), problems
        )
    if want_rules:
        notes += check_rules(want_rules, docs, problems)

    for p in problems:
        print(f"  !! {p}", file=sys.stderr)
    if problems:
        return 1
    for n in notes:
        print(f"    {n}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
