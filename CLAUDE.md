# CLAUDE.md

Guidance for Claude and other AI agents working in this repository.

## Sources of truth

The [roadmap](docs/content/reference/internal/roadmap.md) is the current source of truth for what is built, what is in flight, and what is planned next.
Read it before reasoning about direction or priorities.

The [repository layout](docs/content/reference/internal/repo-layout.md) is a cache of where things live in the repo.

## Stale content is common

This repository is under active development, so docs, comments, and tickets go stale quickly.
When you notice content that no longer matches reality, always offer to update it.
This includes checking off or updating the status of items that are now done — for example, roadmap milestone statuses.
Prefer fixing stale content in passing over leaving it wrong.

## Markdown style

Break Markdown lines on sentence ends — write one sentence per line.
Do not hard-wrap a single sentence across multiple lines at a fixed column width.
Sentence-per-line keeps diffs small and avoids rewrapping churn when a sentence changes.

## No customer information

Customer information must never be committed to this repository.
That includes customer names, organization or environment identifiers, and any other customer-identifying details.
Keep examples generic.

## Internal references

Mark references to internal-only content as `(internal)` when it is not already handled implicitly.
This applies to things an external reader cannot access, such as Linear or internal infrastructure repositories.
Linear links inside `docs/content/` are marked with 🔒 automatically by the render-link hook, so they do not need a manual `(internal)` marker.

## Agent notes in docs

Put agent-specific notes inside `<!-- Markdown comments -->` within docs.
These notes should not repeat information that already lives in this file.
