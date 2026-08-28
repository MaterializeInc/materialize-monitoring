# Test fixtures

## `env-top.python-baseline.yaml`

The last `env-top.yaml` the Python generator produced, frozen at commit `f2bfce2e5`.

Two test suites read it, for two reasons:

- [`mzmon-lib/tests/grafana_golden.rs`](../grafana_golden.rs) uses it as a real-world Dashboard v2 document, to check the generated models can round-trip one.
- [`dashboards/tests/env_top_parity.rs`](../../../dashboards/tests/env_top_parity.rs) uses it as the baseline the Rust port is compared against.

It is a fixture rather than a read of `charts/materialize-monitoring/pre-rendered/dashboards/grafana/env-top.yaml` because the Rust generator now writes that file.
Comparing the output to itself would make every parity assertion vacuous, and would invert the ones that assert a *deliberate* divergence from the Python — those start failing the moment the switchover lands.

The file is byte-identical to its source, so the provenance is checkable:

```bash
git show f2bfce2e5:charts/materialize-monitoring/pre-rendered/dashboards/grafana/env-top.yaml | shasum
```

Expected: `65066cd37525e66119af81c765eb96d06504617a`.

Do not regenerate this file.
It is a record of what the Python emitted, and its value is that it does not move.
When a deliberate change to the Rust dashboard diverges from it further, add the divergence to the allow-lists at the top of `env_top_parity.rs` rather than updating the fixture.

Once the Python generator is removed, this fixture is what remains of it — keep it until the port is no longer worth pinning.
