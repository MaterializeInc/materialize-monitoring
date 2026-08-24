#!/usr/bin/env python3
"""Assert every Prometheus remote-write destination the module declared actually lands.

Called from `bin/terraform-render-check.sh` with the destinations the module call
declared, a rendered chart, and the values documents the plan composed. Silent
and successful when the example declares none.

Rendering alone proves nothing here, for the usual reason and for one specific
to this input. The usual reason is that a destination written to a path no
template reads is still valid YAML. The specific one is that
`pipeline.metrics.gateway.destination.prometheusRemoteWrite` is a **map**, and
Helm deep-merges maps: a module writing to a mistyped path — or to the
pre-map singular shape — adds a key beside the chart's `thanos` rather than
replacing anything. The install keeps working, keeps writing to Thanos, and
never mentions the destination that went missing. That is the failure this
exists to catch, and it cannot be caught by checking that the render succeeded.

So each declared destination is checked in three places, because each one fails
differently and only the first is visible in the config at all:

  * the `prometheus.remote_write "<name>"` component — absent, nothing writes
    there and no error is raised anywhere;
  * its URL environment variable in the gateway env ConfigMap — absent,
    `sys.env` resolves to the empty string, which Alloy accepts at load and
    every write then fails at run time;
  * its tier allowlist variable — absent, the filter falls back to `.*` and a
    destination asked for `essential` silently receives the full firehose.
    On a backend that bills per sample, that last one is the expensive failure,
    and it is completely invisible from the cluster's side.

A `sigv4` destination is checked one step further, because it is the one auth
type with *nothing* in the gateway Secret: the ServiceAccount annotation is its
entire credential. Written to a path the subchart does not read, it renders a
ServiceAccount with no annotation, the AWS SDK finds no web-identity token, and
every write comes back 403 with an error naming neither the role nor the values
key that went missing.
"""

from __future__ import annotations

import json
import re
import sys
from typing import Any

import yaml

GATEWAY_CONFIG_KEY = "config.alloy"


def slug(name: str) -> str:
    """Return the env-var fragment for a destination name.

    Mirrors the chart's `regexReplaceAll "[^A-Za-z0-9]" "_" | upper` and the
    module's `upper(replace(name, "/[^A-Za-z0-9]/", "_"))`. A third copy is not
    ideal, but the alternative is asserting the derivation against itself.
    """
    return re.sub(r"[^A-Za-z0-9]", "_", name).upper()


def load_documents(paths: list[str]) -> list[dict]:
    """Read every YAML document in `paths`, skipping anything that is not a mapping."""
    out = []
    for path in paths:
        with open(path) as fh:
            for doc in yaml.safe_load_all(fh):
                if isinstance(doc, dict):
                    out.append(doc)
    return out


def gateway_configmaps(rendered: str) -> tuple[str, dict[str, str]]:
    """Return the rendered Alloy config and the gateway's environment ConfigMap."""
    config = ""
    env: dict[str, str] = {}
    with open(rendered) as fh:
        for doc in yaml.safe_load_all(fh):
            if not isinstance(doc, dict) or doc.get("kind") != "ConfigMap":
                continue
            data = doc.get("data") or {}
            if GATEWAY_CONFIG_KEY in data:
                config = data[GATEWAY_CONFIG_KEY]
            elif any(k.startswith("GATEWAY_") for k in data):
                env.update(data)
    return config, env


def gateway_service_account_annotations(rendered: str) -> dict[str, str]:
    """Return the annotations on the alloy-gateway ServiceAccount, if one rendered."""
    with open(rendered) as fh:
        for doc in yaml.safe_load_all(fh):
            if not isinstance(doc, dict) or doc.get("kind") != "ServiceAccount":
                continue
            meta = doc.get("metadata") or {}
            if meta.get("name") == "alloy-gateway":
                return meta.get("annotations") or {}
    return {}


def composed_destinations(paths: list[str]) -> dict[str, dict]:
    """Merge the remote-write destinations out of the values documents the plan composed.

    Read back rather than taken from the declaration, so the module's own
    document is what gets checked — that is the half most likely to be wrong.
    Later documents win, matching how Helm merges them.
    """
    out: dict[str, dict] = {}
    for doc in load_documents(paths):
        dests = (
            doc.get("pipeline", {})
            .get("metrics", {})
            .get("gateway", {})
            .get("destination", {})
            .get("prometheusRemoteWrite")
        )
        if not isinstance(dests, dict):
            continue
        for name, dest in dests.items():
            if isinstance(dest, dict):
                out.setdefault(name, {}).update(dest)
    return out


def pipeline_problems(name: str, config: str) -> list[str]:
    """Check the three components a destination needs in the rendered pipeline."""
    if f'prometheus.remote_write "{name}"' not in config:
        return [f'{name}: no prometheus.remote_write "{name}" component rendered']

    problems = []
    # The tier filter is a separate component upstream of the writer. Without it
    # the destination still writes, so nothing looks broken — it just ignores
    # its own min_importance.
    if f'prometheus.relabel "{name}"' not in config:
        problems.append(
            f"{name}: has a remote_write component but no prometheus.relabel tier filter, "
            f"so min_importance would be ignored"
        )
    # An orphan component is valid Alloy that never receives a sample.
    if f"prometheus.relabel.{name}.receiver" not in config:
        problems.append(
            f"{name}: nothing forwards to it — the prometheus.relabel.egress fan-out "
            f"does not list prometheus.relabel.{name}.receiver"
        )
    return problems


def env_problems(
    name: str, declared: dict[str, Any], composed: dict[str, Any], env: dict[str, str]
) -> list[str]:
    """Check the URL and tier variables the pipeline reads for this destination."""
    problems = []

    url_env = f"GATEWAY_PROM_DEST_{slug(name)}"
    expected_url = composed.get("url")
    if url_env not in env:
        problems.append(f"{name}: the gateway env ConfigMap sets no {url_env}")
    elif expected_url and env[url_env] != expected_url:
        problems.append(
            f"{name}: {url_env} is {env[url_env]!r}, expected {expected_url!r}"
        )

    tier_env = f"GATEWAY_UNFILTERED_PROM_METRICS_{slug(name)}"
    want_tier = declared.get("min_importance", "all")
    if tier_env not in env:
        problems.append(f"{name}: the gateway env ConfigMap sets no {tier_env}")
    elif want_tier == "all" and env[tier_env] != ".*":
        problems.append(f"{name}: min_importance is 'all' but {tier_env} is not '.*'")
    elif want_tier != "all" and env[tier_env] == ".*":
        problems.append(
            f"{name}: min_importance is {want_tier!r} but {tier_env} is '.*', "
            f"so the tier filter passes everything"
        )
    return problems


# The annotations that bind a pod to a cloud identity. A `sigv4` destination has
# no other source of credentials, so one of these has to be on the gateway's
# ServiceAccount for it to authenticate at all.
IDENTITY_ANNOTATIONS = (
    "eks.amazonaws.com/role-arn",
    "iam.gke.io/gcp-service-account",
    "azure.workload.identity/client-id",
)


def check(
    declared: dict[str, dict],
    config: str,
    composed: dict[str, dict],
    env: dict[str, str],
    sa_annotations: dict[str, str],
) -> list[str]:
    """Collect every way the declared destinations failed to reach the render."""
    problems: list[str] = []
    for name, dest in declared.items():
        if dest.get("enabled") is False:
            if f'prometheus.remote_write "{name}"' in config:
                problems.append(
                    f"{name}: declared enabled=false but a component still rendered"
                )
            continue

        if name not in composed:
            problems.append(
                f"{name}: the module composed no "
                f"pipeline.metrics.gateway.destination.prometheusRemoteWrite.{name} document"
            )
            continue

        problems += pipeline_problems(name, config)
        problems += env_problems(name, dest, composed[name], env)

        if dest.get("auth_type") == "sigv4" and not any(
            a in sa_annotations for a in IDENTITY_ANNOTATIONS
        ):
            problems.append(
                f"{name}: auth_type is sigv4, but the alloy-gateway ServiceAccount carries none of "
                f"{', '.join(IDENTITY_ANNOTATIONS)} — there is nothing for the AWS SDK to sign as, "
                f"so every write is refused with a 403"
            )
    return problems


def main(argv: list[str]) -> int:
    """Run the check; 0 on success, 1 on a destination that did not land, 2 on misuse."""
    if len(argv) < 3:
        print(
            "usage: check_remote_write_destinations.py <declared-json> <rendered> <values...>",
            file=sys.stderr,
        )
        return 2

    declared = json.loads(argv[0]) or {}
    if not declared:
        return 0

    config, env = gateway_configmaps(argv[1])
    if not config:
        print("     no alloy gateway config in the rendered chart", file=sys.stderr)
        return 1

    problems = check(
        declared,
        config,
        composed_destinations(argv[2:]),
        env,
        gateway_service_account_annotations(argv[1]),
    )
    for problem in problems:
        print(f"     {problem}", file=sys.stderr)
    if problems:
        return 1

    landed = ", ".join(sorted(declared))
    print(f"    remote-write destinations landed with their own tier filters: {landed}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
