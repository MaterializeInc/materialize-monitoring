# Skills

`.claude/skills/` holds authoring and operating conventions consumed by **both** contributors and AI agents.

**The index lives in the docsite**, not here, so it stays beside the pages the skills route into:

- [Skills](https://materializeinc.github.io/materialize-monitoring/reference/internal/skills/) — what each skill is
  for, how they are picked up in code review, and why they are deliberately thin.

Source: [`docs/content/reference/internal/skills.md`](../../docs/content/reference/internal/skills.md).

Each skill is a directory containing a `SKILL.md` with front matter naming when it applies, plus optional
`references/`, `assets/`, and `scripts/` beside it.
