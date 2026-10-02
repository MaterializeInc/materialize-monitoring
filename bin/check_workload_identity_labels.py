#!/usr/bin/env python3
# Copyright Materialize, Inc. and contributors. All rights reserved.
"""Assert every pod bound to an Azure workload identity carries the webhook's label.

Called from `bin/terraform-render-check.sh` with a rendered chart. Silent and
successful when no ServiceAccount carries `azure.workload.identity/client-id`.

The Entra webhook injects the projected token and the `AZURE_*` variables only
into pods labelled `azure.workload.identity/use: "true"`, and each subchart takes
that label through a different value path (see
`terraform/modules/materialize-monitoring/azure.tf`). A label written to a path
the subchart does not read renders fine and does nothing: the annotation is
there, the pod is not mutated, and the Azure SDK falls through to the node's
managed identity. Nothing fails until the first call to Azure, so rendering
alone proves nothing.

So: for every workload whose pod template runs as an annotated ServiceAccount,
require the label on that pod template.
"""

from __future__ import annotations

import sys
from pathlib import Path

import yaml

CLIENT_ID = "azure.workload.identity/client-id"
USE = "azure.workload.identity/use"
WORKLOAD_KINDS = {"Deployment", "StatefulSet", "DaemonSet"}


def main(rendered: Path) -> int:
    """Return 1 when an annotated workload lacks the label, else 0."""
    docs = [d for d in yaml.safe_load_all(rendered.read_text()) if isinstance(d, dict)]

    # An empty annotation names no identity, as the chart's own validator treats it.
    bound = {
        (d["metadata"].get("namespace"), d["metadata"]["name"])
        for d in docs
        if d.get("kind") == "ServiceAccount"
        and ((d.get("metadata") or {}).get("annotations") or {}).get(CLIENT_ID)
    }
    if not bound:
        return 0

    checked: list[str] = []
    problems: list[str] = []
    for d in docs:
        if d.get("kind") not in WORKLOAD_KINDS:
            continue
        meta = d.get("metadata") or {}
        template = (d.get("spec") or {}).get("template") or {}
        sa = (template.get("spec") or {}).get("serviceAccountName")
        if (meta.get("namespace"), sa) not in bound:
            continue
        name = f"{d['kind']}/{meta['name']}"
        checked.append(name)
        if (
            str(((template.get("metadata") or {}).get("labels") or {}).get(USE))
            != "true"
        ):
            problems.append(
                f'{name} runs as {sa}, which names an Azure identity, but its pods lack {USE}: "true"'
            )

    if not checked:
        print(
            f"  !! ServiceAccounts {sorted(n for _, n in bound)} carry {CLIENT_ID} but no workload runs as them",
            file=sys.stderr,
        )
        return 1
    for p in problems:
        print(f"  !! {p}", file=sys.stderr)
    if problems:
        return 1
    print(
        f"    Azure workload identity label on {len(checked)} workloads: {', '.join(sorted(checked))}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
