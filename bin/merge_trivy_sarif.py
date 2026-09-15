#!/usr/bin/env python3
"""Merge per-image Trivy SARIF reports into a single SARIF run.

Called from `bin/security-scan.sh` in images-report mode.

Code scanning refuses an upload that contains several runs sharing one category:
"The CodeQL Action does not support uploading multiple SARIF runs with the same
category." Trivy writes one run per image, so uploading the directory of them
fails. Giving each image its own category instead would mean a category per
image, and the image set changes whenever a subchart is enabled or bumped --
categories would accumulate and go stale as images come and go.

So the runs are merged into one. Two details make that lossless:

*   The image a finding came from lives in `run.properties.imageName`, which is
    dropped when runs are combined. Each result's artifact URI is rewritten to
    `<image>/<path in image>` so the image survives on the finding itself.
    Without this a Thanos CVE reads as `bin/thanos`, which looks like a repo
    path and names no image at all.
*   `ruleIndex` is an index into the run's own rule array, so rules are
    deduplicated by id and every index is remapped. A stale index would
    mislabel findings rather than fail loudly.

Stdlib only, so the CI job needs no Python setup.
"""

import json
import sys
from pathlib import Path


class _Accumulator:
    """Collects rules and results across runs, keeping rule indices consistent."""

    def __init__(self) -> None:
        self.rules: list[dict] = []
        self.rule_index_by_id: dict[str, int] = {}
        self.results: list[dict] = []
        self.driver: dict = {
            "name": "Trivy",
            "informationUri": "https://github.com/aquasecurity/trivy",
        }

    def remap_rules(self, run_rules: list[dict]) -> dict[int, int]:
        """Add a run's rules, returning its local rule index -> merged index."""
        remap: dict[int, int] = {}
        for i, rule in enumerate(run_rules):
            rule_id = rule.get("id")
            if not isinstance(rule_id, str):
                # Every result points at a rule by index, so a rule we cannot
                # key would silently mislabel findings downstream.
                raise SystemExit(f"rule at index {i} has no usable id: {rule!r}")
            if rule_id not in self.rule_index_by_id:
                self.rule_index_by_id[rule_id] = len(self.rules)
                self.rules.append(rule)
            remap[i] = self.rule_index_by_id[rule_id]
        return remap

    def add_run(self, run: dict, image: str, source: Path) -> None:
        """Fold one run's results in, rewriting rule indices and artifact URIs."""
        run_driver = (run.get("tool") or {}).get("driver") or {}
        # Keep the richest driver metadata we see; identical across runs in
        # practice, but the version is worth carrying through.
        if run_driver.get("version"):
            self.driver.update({k: v for k, v in run_driver.items() if k != "rules"})

        remap = self.remap_rules(run_driver.get("rules") or [])

        for result in run.get("results") or []:
            old_index = result.get("ruleIndex")
            if old_index is not None:
                if old_index not in remap:
                    raise SystemExit(
                        f"{source}: result references ruleIndex {old_index}, "
                        f"which the run's rule array does not define"
                    )
                result["ruleIndex"] = remap[old_index]

            for location in result.get("locations") or []:
                artifact = (location.get("physicalLocation") or {}).get(
                    "artifactLocation"
                )
                if artifact and "uri" in artifact:
                    artifact["uri"] = f"{image}/{artifact['uri']}"

            self.results.append(result)

    def document(self) -> dict:
        """Render the accumulated state as a single-run SARIF document."""
        driver = {**self.driver, "rules": self.rules}
        return {
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [
                {
                    "tool": {"driver": driver},
                    "results": self.results,
                    "columnKind": "utf16CodeUnits",
                }
            ],
        }


def merge(paths: list[Path]) -> dict:
    """Merge every run in every given SARIF file into one single-run document."""
    acc = _Accumulator()
    for path in sorted(paths):
        doc = json.loads(path.read_text())
        for run in doc.get("runs") or []:
            image = (run.get("properties") or {}).get("imageName") or path.stem
            acc.add_run(run, image, path)
    return acc.document()


def main(argv: list[str]) -> int:
    """Merge SARIF_DIR/*.sarif into OUTPUT_FILE."""
    if len(argv) != 3:
        print(f"usage: {argv[0]} SARIF_DIR OUTPUT_FILE", file=sys.stderr)
        return 2

    sarif_dir, output = Path(argv[1]), Path(argv[2])
    paths = sorted(sarif_dir.glob("*.sarif"))
    if not paths:
        # Every image failing to scan is already reported by the caller; an
        # empty merge would upload as a clean report, which is worse.
        print(f"{argv[0]}: no SARIF files in {sarif_dir}", file=sys.stderr)
        return 1

    merged = merge(paths)
    output.write_text(json.dumps(merged))
    print(
        f"merged {len(paths)} report(s), "
        f"{len(merged['runs'][0]['results'])} result(s) -> {output}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
