# Contributing to materialize-monitoring

The canonical contributor guide lives in the docsite:

**→ [Internal Development](https://materializeinc.github.io/materialize-monitoring/reference/internal/)** *(or [the source on GitHub](docs/content/reference/internal/_index.md))*

The audience is SRE, Field Engineering, and customer infrastructure teams. Conventions, lint rule rationale, and the full pre-commit / pre-push wiring are documented there.

## Quickstart

If you just want to start hacking, run these four commands after cloning:

```sh
git lfs install                   # one-time per machine
uv sync                           # Python workspace + venv
uv run pre-commit install         # wires both pre-commit and pre-push stages
./bin/check-lfs.sh                # verify LFS state for this repo
```

Then build everything with `make all`, or jump to the docsite for the per-language and per-stack workflows.

## Reporting issues

Open an issue on [GitHub](https://github.com/MaterializeInc/materialize-monitoring/issues). If a pre-commit hook or convention surprised you, that's the right venue — surprises are bugs in this guide.
