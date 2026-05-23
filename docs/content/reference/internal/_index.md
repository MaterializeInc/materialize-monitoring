---
title: "Internal Development"
weight: 200
bookCollapseSection: true
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

Optional but commonly useful: the [`helm-unittest`](https://github.com/helm-unittest/helm-unittest) plugin (`helm plugin install https://github.com/helm-unittest/helm-unittest`), `yq`, `jq`.

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
```

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

**Runs additionally on `pre-push` (slower, before publishing):**

- `cargo clippy --workspace --all-targets -- -D warnings`

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
4. Open a PR against `main`. Keep PRs focused — one concern per PR reviews faster than a sweep.
5. The `pre-push` hook runs `cargo clippy` before `git push` succeeds.

## Where to go next

The pages under this section are the authoritative reference for their respective topics:

- [Repo Layout](repo-layout/) — where everything lives and why
- [Dashboard development](dashboard/) — Grafana dashboards-as-code (SDKs, generating/pushing, style guidelines, testing)
- [Pipelines](pipelines/) — alloy logging and metrics pipelines
- [Helm](helm/) — chart conventions (in progress)
- [Skills](skills/) — overview of the `.claude/skills/` system used for AI-agent context (in progress)
- [Releasing](releasing/) — release process (in progress)
- [Roadmap](roadmap/) — what's planned next (in progress)

Cross-cutting authoring conventions are bundled in [`.claude/skills/`](https://github.com/MaterializeInc/materialize-monitoring/tree/main/.claude/skills) — they're consumed by both contributors and AI agents, and link back into the pages above.

## Reporting issues and asking questions

Open an issue on the [GitHub repository](https://github.com/MaterializeInc/materialize-monitoring/issues). For ambiguous cases — a hook that surprises you, a convention you want to change, a build target that fails on your platform — an issue is the right venue. These tend to surface mismatches between repo guidance and real-world contributor workflows, which is exactly what this page should reflect.
