#!/usr/bin/env python3
"""Merge Trivy SARIF reports into a single deduplicated SARIF run.

Called from `bin/security-scan.sh` for both report modes.

Two problems this solves.

**Code scanning rejects multiple runs under one category.** Trivy writes one run
per target -- per image, and per rendered scenario -- so uploading a directory
of them fails with "The CodeQL Action does not support uploading multiple SARIF
runs with the same category". A category per target would work but would
accumulate stale categories as scenarios and images come and go.

**The same finding is reported once per target.** The misconfiguration scan
renders three scenarios that overlap heavily (tier1 and mtls3 produce an
identical finding set), and an image CVE is reported once per binary that embeds
it -- 160 of Grafana's alerts are the Go stdlib recompiled into each bundled
plugin. Reported raw, one finding becomes many alerts and the dashboard reads as
far worse than it is.

So findings are deduplicated on what actually identifies them:

*   For an image, `(rule, image)`. Which of the fifteen binaries inside the
    image embeds the CVE does not change the remediation, which is to bump the
    image.
*   For a rendered scenario, `(rule, resource)`. The same ClusterRole flagged in
    tier1 and in azure is one thing to fix.

Merged findings record every target they were seen in, so collapsing them does
not hide where they came from.

Stdlib only, so the CI job needs no Python setup.
"""

import json
import re
import sys
from pathlib import Path

# Trivy puts a `Message: <resource>` line in each misconfiguration result,
# alongside an `Artifact: <file>` line naming the scenario it rendered from.
# The resource is what identifies the finding; the artifact is what we collapse.
_MESSAGE_LINE = re.compile(r"^Message: (.*)$", re.MULTILINE)
_ARTIFACT_LINE = re.compile(r"^Artifact: .*$\n?", re.MULTILINE)


def _scope(run: dict, source: Path) -> str:
    """Name the thing a run scanned: an image reference, or a scenario."""
    image = (run.get("properties") or {}).get("imageName")
    return image or source.stem


def _identity(result: dict, scope: str, *, run_is_image: bool) -> tuple:
    """Key a finding by what makes it distinct, ignoring where it was seen."""
    rule = result.get("ruleId")
    if run_is_image:
        # Scope is the image: the same CVE in two images is two findings.
        return (rule, scope)
    match = _MESSAGE_LINE.search(result.get("message", {}).get("text", ""))
    # Scope is deliberately absent: collapsing across scenarios is the point.
    return (rule, match.group(1) if match else json.dumps(result.get("locations")))


class _Accumulator:
    """Collects deduplicated results, keeping rule indices consistent."""

    def __init__(self) -> None:
        self.rules: list[dict] = []
        self.rule_index_by_id: dict[str, int] = {}
        self.results: list[dict] = []
        self.index_by_identity: dict[tuple, int] = {}
        self.scopes_by_identity: dict[tuple, list[str]] = {}
        self.driver: dict = {
            "name": "Trivy",
            "informationUri": "https://github.com/aquasecurity/trivy",
        }
        self.merged = 0

    def _remap_rules(self, run_rules: list[dict]) -> dict[int, int]:
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

    def _rewrite(
        self, result: dict, remap: dict[int, int], scope: str, *, is_image: bool
    ) -> None:
        """Point a result at the merged rule array, and at its image if any."""
        old_index = result.get("ruleIndex")
        if old_index is not None:
            if old_index not in remap:
                raise SystemExit(
                    f"result references ruleIndex {old_index}, "
                    f"which the run's rule array does not define"
                )
            result["ruleIndex"] = remap[old_index]

        if not is_image:
            return
        # The image lives in run.properties, which is dropped when runs are
        # combined. Without this a Thanos CVE reads as `bin/thanos`, naming no
        # image and looking like a path in this repository.
        for location in result.get("locations") or []:
            artifact = (location.get("physicalLocation") or {}).get("artifactLocation")
            if artifact and "uri" in artifact:
                artifact["uri"] = f"{scope}/{artifact['uri']}"

    def add_run(self, run: dict, source: Path) -> None:
        """Fold one run's results in, dropping ones already seen."""
        run_driver = (run.get("tool") or {}).get("driver") or {}
        if run_driver.get("version"):
            self.driver.update({k: v for k, v in run_driver.items() if k != "rules"})

        remap = self._remap_rules(run_driver.get("rules") or [])
        scope = _scope(run, source)
        is_image = bool((run.get("properties") or {}).get("imageName"))

        for result in run.get("results") or []:
            identity = _identity(result, scope, run_is_image=is_image)
            self.scopes_by_identity.setdefault(identity, [])
            if scope not in self.scopes_by_identity[identity]:
                self.scopes_by_identity[identity].append(scope)

            if identity in self.index_by_identity:
                self.merged += 1
                continue

            self._rewrite(result, remap, scope, is_image=is_image)
            self.index_by_identity[identity] = len(self.results)
            self.results.append(result)

    def _annotate(self) -> None:
        """Record every target a finding was seen in, and drop the stale one."""
        for identity, index in self.index_by_identity.items():
            scopes = self.scopes_by_identity[identity]
            result = self.results[index]
            text = result.get("message", {}).get("text", "")
            # The `Artifact:` line names one scenario, which is wrong once a
            # finding is known to occur in several.
            text = _ARTIFACT_LINE.sub("", text)
            result["message"]["text"] = f"{text.rstrip()}\nSeen in: {', '.join(scopes)}"

    def document(self) -> dict:
        """Render the accumulated state as a single-run SARIF document."""
        self._annotate()
        return {
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [
                {
                    "tool": {"driver": {**self.driver, "rules": self.rules}},
                    "results": self.results,
                    "columnKind": "utf16CodeUnits",
                }
            ],
        }


def merge(paths: list[Path]) -> tuple[dict, int]:
    """Merge every run in every given SARIF file into one deduplicated run."""
    acc = _Accumulator()
    for path in sorted(paths):
        doc = json.loads(path.read_text())
        for run in doc.get("runs") or []:
            acc.add_run(run, path)
    return acc.document(), acc.merged


def main(argv: list[str]) -> int:
    """Merge SARIF_DIR/*.sarif into OUTPUT_FILE."""
    if len(argv) != 3:
        print(f"usage: {argv[0]} SARIF_DIR OUTPUT_FILE", file=sys.stderr)
        return 2

    sarif_dir, output = Path(argv[1]), Path(argv[2])
    paths = sorted(sarif_dir.glob("*.sarif"))
    if not paths:
        # Every target failing to scan is already reported by the caller; an
        # empty merge would upload as a clean report, which is worse.
        print(f"{argv[0]}: no SARIF files in {sarif_dir}", file=sys.stderr)
        return 1

    merged, collapsed = merge(paths)
    kept = len(merged["runs"][0]["results"])
    print(
        f"merged {len(paths)} report(s): {kept + collapsed} findings -> "
        f"{kept} after deduplication ({collapsed} collapsed) -> {output}"
    )
    output.write_text(json.dumps(merged))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
