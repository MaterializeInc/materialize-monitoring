---
title: "Contributing"
weight: 1
---

# Contributing to materialize-monitoring

This is the canonical entry point for repo contributors. The audience is SRE, Field Engineering, and customer infrastructure teams — the setup below assumes Unix-shell comfort but not full-time backend-developer familiarity. If a step is surprising or feels wrong, please file an issue rather than working around it; surprises are bugs in this guide.

The root [`CONTRIBUTING.md`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/CONTRIBUTING.md) is a thin pointer at this page.

## Prerequisites

Install the toolchain once:

| Tool | Required version | Install |
|---|---|---|
| [`uv`](https://docs.astral.sh/uv/) | latest | `brew install uv` (or the [official installer](https://docs.astral.sh/uv/getting-started/installation/)) |
| Python | 3.12+ (managed by `uv`) | `uv` reads `.python-version` and installs as needed |
| Rust | 1.95+ via [`rustup`](https://rustup.rs/) | `curl https://sh.rustup.rs -sSf \| sh` |
| Go | 1.22+ | `brew install go` — drives `hugo` and `helm-docs` via the `tool` directive in `go.mod` |
| Helm | 3.x / 4.x | `brew install helm` |
| Git LFS | latest | `brew install git-lfs` |
| [Terraform](https://developer.hashicorp.com/terraform) | 1.3+ | `brew install terraform` — needed for `make terraform-check`; `terraform-docs` (`brew install terraform-docs`) regenerates the module reference |

Optional but commonly useful: the [`helm-unittest`](https://github.com/helm-unittest/helm-unittest) plugin (`helm plugin install https://github.com/helm-unittest/helm-unittest`), `yq`, `jq`.

Only for the E2E tiers, which are the one part of the suite that needs a cluster:

| Tool | Required version | Install |
|---|---|---|
| Docker | latest | Docker Desktop, Colima, or equivalent — must be running |
| [`kind`](https://kind.sigs.k8s.io/) | 0.32+ | `brew install kind` |
| `kubectl` | matching the cluster | `brew install kubectl` |

## First-time setup

```sh
git clone https://github.com/MaterializeInc/materialize-monitoring.git
cd materialize-monitoring

# Initialize Git LFS for your user (one-time per machine)
git lfs install

# Verify LFS state for this repo
./bin/check-lfs.sh

# Sync the Python workspace (creates .venv/ via uv)
uv sync

# Install pre-commit hooks (wires both pre-commit and pre-push stages)
uv run pre-commit install
```

## Day-to-day commands

Everything is wired through [the top-level `Makefile`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/Makefile). The most-used targets:

```sh
make all              # build everything (charts + dashboards)
make charts           # package the Helm charts
make dashboards       # regenerate Grafana dashboards from Python sources
make helm-docs        # regenerate chart README + docsite values reference
make serve-docs       # serve this Hugo docsite locally
make terraform-check  # fmt + validate + the tier-0 render check (no cluster)
make terraform-docs   # regenerate the module README + docsite variable reference
```

### Terraform

The cloud-agnostic module lives in [`terraform/modules/materialize-monitoring`](https://github.com/MaterializeInc/materialize-monitoring/tree/main/terraform/modules/materialize-monitoring), next to the chart whose value paths it encodes. Per-cloud wrappers live downstream in `materialize-terraform-self-managed`.

`make terraform-check` is the gate, and the part worth understanding is what it adds over `terraform validate`. Validate accepts **any** well-formed HCL, including a value written to a path the chart never reads — that renders perfectly and is silently ignored. So the check also plans each example, extracts the composed Helm values from the plan, and renders the chart against them. It needs no cluster: the examples plan against a kubeconfig that does not exist, and every resource is a create, so the providers are never asked to connect.

Two examples are rendered, `aws` and `gcp`, and that is not redundancy — the chart's storage defaults are S3-shaped, so an AWS-only example agrees with every default it fails to set.

### E2E (kind)

```sh
make e2e-cluster          # kind cluster + the namespaces a real install has
make e2e-tier1            # chart, hermetic shape
make e2e-verify-tier1     # assert the logging round trip
make e2e-tier1-down       # remove tier 1 so the same cluster can host tier 2
make e2e-tier2            # rustfs + CNPG substrate, then the module onto it
make e2e-verify-tier2     # same assertions, with Thanos live
make e2e-cluster-down
```

The tiers do not coexist — both name their CRDs release `mzmon-crds`, so switching needs `make e2e-tier1-down` first (or a fresh cluster, which is slower).

The assertions themselves live in `packages/mz-monitoring-e2e`, a Rust workspace member, and there is one binary for every tier.
It reads the release's own coalesced Helm values to decide which assertions apply, so the tier is a property of the cluster rather than a flag — which means the same binary answers "is this stack healthy?" against a live cluster too:

```sh
make e2e-verify E2E_CONTEXT=<kube-context> E2E_NAMESPACE=monitoring
cargo run -p mz-monitoring-e2e -- --context <kube-context> --list   # what applies there
```

It only asserts; it never installs. Lifecycle stays with `make`, Terraform, or you.

See [`test/e2e/README.md`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/test/e2e/README.md) for what each tier covers and the traps worth knowing before extending them. Two are worth repeating here because they cost real debugging time:

- **Alloy needs a restart after any config change.** Its config arrives through `envFrom` ConfigMaps, and environment variables are fixed at container start, so neither Helm nor Alloy's `/-/reload` picks up a change. `make e2e-tier1` does the rollout restart explicitly.
- **Assert on recent data, not on any data.** Loki's filesystem store survives a pod restart, so an unbounded query passes against a stack that has stopped ingesting.

Direct invocations when iterating:

```sh
# Python
uv run pytest                                            # tests
uv run ruff check                                        # lint (auto-fixes)
uv run ruff format                                       # format
uv run pyright                                           # type-check

# Rust
cargo build                                              # build the workspace
cargo test --workspace                                   # all tests
cargo clippy --workspace --all-targets -- -D warnings    # lint

# Helm
helm unittest charts/materialize-monitoring              # template unit tests
```

## Iterating against a live cluster

The default loop — release the chart, release the module, bump the wrapper's ref, apply — is correct for consumers and far too slow for development. Three shortcuts remove a release from the cycle. All three are temporary: revert them before you commit.

### Point a Terraform wrapper at this repo

A per-cloud wrapper in `materialize-terraform-self-managed` pins the common module by Git ref, so an unreleased change is invisible to it. Swap the `source` for a relative path to your checkout:

```hcl
# source = "github.com/MaterializeInc/materialize-monitoring//terraform/modules/materialize-monitoring?ref=materialize-monitoring/vX.Y.Z"
source = "../../../../materialize-monitoring/terraform/modules/materialize-monitoring"
```

Local-path modules are used **in place** rather than copied, so edits take effect on the next `plan` with no `terraform init`.

It has to stay *relative*. An absolute path makes Terraform copy the module into `.terraform/modules/` without the chart directory beside it, and the sizing profiles silently stop resolving — the module carries a precondition for that case precisely because the failure is otherwise invisible.

### Point the module at a local chart

`chart_registry` does not have to be an OCI registry. Set it to a local directory and the module installs the chart from your working tree, which closes the loop on chart changes as well as module changes:

```hcl
chart_registry = "/path/to/materialize-monitoring/charts"
```

The module reads its chart version from that directory's `Chart.yaml` too, so the pair stays consistent.

### Install one component at a time

A full stack is a slow way to iterate on one piece. Because the tags are OR'd and each subchart has an `enabled` circuit breaker, a second release can bring up a single component beside your existing one:

```bash
helm install mzmon-scratch charts/materialize-monitoring \
  --namespace monitoring --skip-crds \
  --set tags.default=false --set tags.loki=true
```

Give it a different release name so it does not fight the full install. Mind what is *not* release-scoped: the subcharts use static `fullnameOverride`s, so two releases in one namespace will collide on Service and ServiceAccount names. Use a separate namespace when the component you are iterating on has either — which is most of them. See [Namespace layout](../../../operating/production-best-practices/#namespace-layout) for what else assumes one instance per namespace.

## Pre-commit

A single `uv run pre-commit install` wires both `pre-commit` and `pre-push` stages.

**Runs on `pre-commit` (every commit, fast):**

- generic hygiene (trailing whitespace, EOF newline, line endings, merge conflicts, large-file guard, shebang/executable consistency)
- `ruff check --fix` + `ruff format` (Python)
- `pyright` (Python types)
- `shellcheck` + `shfmt` (shell)
- `yamllint` (YAML and KYAML)
- `cargo fmt`
- `helm-docs` regeneration (only when chart sources change)
- `terraform fmt` and `terraform-docs` regeneration (only when `terraform/` changes)

**Runs additionally on `pre-push` (slower, before publishing):**

- `cargo clippy --workspace --all-targets -- -D warnings`
- `make terraform-render` — the tier-0 render check. Triggered by chart changes as well as Terraform ones, since the module writes into the chart's value paths and either side can break the pair.

The hooks are **fixers wherever possible** — re-`git add` after a hook rewrites a file and the next commit will pass. If a hook surfaces something that feels wrong, that's a bug in the configuration; please open an issue rather than reaching for `--no-verify`.

Configuration lives in [`.pre-commit-config.yaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.pre-commit-config.yaml).

### Lint rule notes

For YAML / KYAML conventions and the intentional yamllint relaxations (`quoted-strings` demoted to a warning, `empty-values: forbid-in-block-mappings` disabled to allow `pull_request:` and helm-docs sentinels), see the [`yaml-development`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.claude/skills/yaml-development/SKILL.md) skill.

For Python, ruff is configured with `select = ["ALL"]` and roughly 40 deliberate ignores in [`pyproject.toml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/pyproject.toml). KYAML is linted with `--strict` against [`.yamllint-kyaml.kyaml`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.yamllint-kyaml.kyaml); none of the lower-toil relaxations apply.

## Working with Git LFS

LFS is used for packaged Helm subcharts (`charts/*/charts/*.tgz`); the pattern is declared in [`.gitattributes`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/.gitattributes).

- If you add a binary artifact that would otherwise commit as a regular blob, `check-added-large-files` blocks commits over 512 KB — use LFS instead, or open a discussion if you think the file belongs in-tree as plain content.
- Run [`./bin/check-lfs.sh`](https://github.com/MaterializeInc/materialize-monitoring/blob/main/bin/check-lfs.sh) if LFS-tracked files look wrong locally (`--fix` can auto-install Git LFS via brew/apt/apk).
- Once installed, the canonical push-time LFS check is git-lfs's own pre-push hook, set up by `git lfs install`.

## Submitting changes

1. Branch from `main`. Use a name or topic prefix (e.g. `heather/pre-commit`, `topic/lfs-docs`).
2. Make changes; let pre-commit fixers run. Stage the resulting files.
3. Run the relevant tests:
   - Python: `uv run pytest`
   - Rust: `cargo test --workspace`
   - Helm: `helm unittest charts/materialize-monitoring`
   - Terraform: `make terraform-check`
   - E2E, when touching the chart or the module in a way a render cannot cover: `make e2e-cluster && make e2e-tier1 && make e2e-verify-tier1`
4. Open a PR against `main`. Keep PRs focused — one concern per PR reviews faster than a sweep.
5. The `pre-push` hook runs `cargo clippy` before `git push` succeeds.

## Where to go next

The pages under this section are the authoritative reference for their respective topics:

- [Repo Layout](../repo-layout/) — where everything lives and why
- [Dashboard development](../dashboard/) — Grafana dashboards-as-code (SDKs, generating/pushing, style guidelines, testing)
- [Queries](../queries/) — the query registry under `packages/queries/`, templating, and the per-engine translations
- [Pipelines](../pipelines/) — alloy logging and metrics pipelines
- [Helm](../helm/) — chart conventions (in progress)
- [Skills](../skills/) — overview of the `.claude/skills/` system used for AI-agent context (in progress)
- [Releasing](../releasing/) — release process (in progress)
- [Roadmap](../roadmap/) — what's planned next (in progress)

Cross-cutting authoring conventions are bundled in [`.claude/skills/`](https://github.com/MaterializeInc/materialize-monitoring/tree/main/.claude/skills) — they're consumed by both contributors and AI agents, and link back into the pages above.

## Reporting issues and asking questions

Open an issue on the [GitHub repository](https://github.com/MaterializeInc/materialize-monitoring/issues). For ambiguous cases — a hook that surprises you, a convention you want to change, a build target that fails on your platform — an issue is the right venue. These tend to surface mismatches between repo guidance and real-world contributor workflows, which is exactly what this page should reflect.
