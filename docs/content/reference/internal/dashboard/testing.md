---
title: "Testing"
weight: 40
---

# Testing

Dashboard code is tested with `cargo test`.
Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the module they cover; the suites that need the whole
dashboard live under `packages/dashboards/tests/`.

For broader code-quality tooling see
[Generating and Pushing Dashboards > Code quality]({{< relref "generating.md#code-quality" >}}).

## What each suite is for

| Suite | Covers |
| --- | --- |
| `dashboards/tests/env_top_parity.rs` | the port against the frozen Python render: titles, plugin/unit, queries, transformations, tab/row skeleton, variable set |
| `dashboards/tests/registry_contract.rs` | that every registry id a panel names resolves, and that a bad id or legend mismatch is reported rather than rendered |
| `mzmon-lib/tests/grafana_golden.rs` | that the generated models round-trip a real Dashboard v2 document |
| `mzmon-lib/tests/grafana_end_to_end.rs` | a complete dashboard built through every layer, then deserialized back through the models |
| `mzmon-lib/tests/grafana_query_bridge.rs` | the query bridge against the real registry under `packages/queries/` |
| `charts/materialize-monitoring/tests/dashboards_test.yaml` | that the chart ships what `dashboards.selected` selects — the patterns are globbed against the pre-rendered tree, so a dashboard joins or leaves the release silently otherwise |
| in-module tests | per-tab structure, selector shapes, threshold ladders, panel presets |

The models carry `deny_unknown_fields`, so anything invented along the way fails at deserialization rather than at push
time.

## The frozen baseline

`packages/mzmon-lib/tests/fixtures/env-top.python-baseline.yaml` is the last render the Python generator produced,
frozen and byte-identical to its source.
Two suites read it: the parity tests compare the port against it, and the golden test wants a real v2 document to
round-trip.

It is a fixture rather than a read of the checked-in artifact under `charts/` because the Rust generator now writes that
file — comparing the output to itself would make every assertion vacuous, and would *invert* the ones that assert a
deliberate divergence.
That is not hypothetical: those assertions started failing the moment the switchover landed, which is how the problem
surfaced.

**Do not regenerate the fixture.**
Its value is that it does not move.
When a deliberate change diverges from it further, add an entry to the allow-list at the top of `env_top_parity.rs`
rather than updating the fixture.

## Allow-listed divergences

Both parity allow-lists — shell fields and queries — are checked **in both directions**.
An entry that no longer diverges fails too, so the list cannot rot into a set of stale excuses.

That bidirectional check is what proved the four registry fixes landed: twenty-one entries went stale at once, and the
test named every one of them.

## Freshness of the checked-in artifacts

The rendered dashboards are checked in, so the `dashboards` workflow runs `make -B dashboards` and asserts `git status`
is clean.

`-B` matters. The Makefile targets are the output *directories*, and a fresh checkout's mtimes are arbitrary, so a
plain `make` can consider them up to date and skip rendering — which would make the check vacuous.
Deleting the outputs first does not fix it either: removing files inside a directory target updates that directory's
mtime, making it look *newer* than its prerequisites rather than missing.
