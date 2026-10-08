# CLAUDE.md

Guidance for Claude and other AI agents working in this repository.

## Sources of truth

The [roadmap](docs/content/reference/development/roadmap.md) is the current source of truth for what is built, what is in flight, and what is planned next.
Read it before reasoning about direction or priorities.

The [repository layout](docs/content/reference/development/repo-layout.md) is a cache of where things live in the repo.

## Stale content is common

This repository is under active development, so docs, comments, and tickets go stale quickly.
When you notice content that no longer matches reality, always offer to update it.
This includes checking off or updating the status of items that are now done — for example, roadmap milestone statuses.
Treat work as done once every PR it needs has merged, and have the PR that finishes an item check it off.
Content that calls work in an open PR "in progress" goes stale when that PR merges, and correcting it takes another PR.
Prefer fixing stale content in passing over leaving it wrong.

## Markdown style

Break Markdown lines on sentence ends — write one sentence per line.
You may soft wrap at 80-120 characters if you like, but lines should not exceed 150 characters.
Sentence-per-line keeps diffs small and avoids rewrapping churn when a sentence changes.

## Writing style

This repository follows the [Google developer documentation style guide](https://developers.google.com/style), and its voice is second person.
Most of the repository doesn't follow it.
When you update content, bring the part you touch into line; a localized rewrite is encouraged.

The [`docs-writing`](.claude/skills/docs-writing/SKILL.md) skill covers the rest of how prose should *read*.
That includes how the highlights apply here, the gaps filled from Grafana's AI quick reference,
the RFC 2119 keywords used on normative pages, and the `params.author` / `params.agent` provenance fields.
Read it before writing or editing prose anywhere in this repository, including skills and READMEs.

The following highlights are copied from [Highlights](https://developers.google.com/style/highlights) without changes to their wording.
Content © Google, licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); the page was last updated 2025-04-02.

### Tone and content

- [Be conversational and friendly](https://developers.google.com/style/tone) without being frivolous.
- [Don't pre-announce anything](https://developers.google.com/style/future) in documentation.
- [Use descriptive link text](https://developers.google.com/style/cross-references#descriptive-link-text).
- [Write accessibly](https://developers.google.com/style/accessibility).
- [Write for a global audience](https://developers.google.com/style/translation).

### Language and grammar

- [Use second person](https://developers.google.com/style/person): "you" rather than "we."
- [Use active voice](https://developers.google.com/style/voice): make clear who's performing the action.
- [Use standard American spelling](https://developers.google.com/style/spelling) and punctuation.
- [Put conditions before instructions](https://developers.google.com/style/sentence-structure), not after.
- [For usage and spelling of specific words, see the word list](https://developers.google.com/style/wordlist).

### Formatting, punctuation, and organization

- [Use sentence case](https://developers.google.com/style/capitalization) for document titles and section headings.
- [Use numbered lists](https://developers.google.com/style/lists#types-of-lists) for sequences.
- [Use bulleted lists](https://developers.google.com/style/lists#types-of-lists) for most other lists.
- [Use description lists](https://developers.google.com/style/lists#types-of-lists) for pairs of related pieces of data.
- [Use serial commas](https://developers.google.com/style/commas-serial).
- [Put code-related text in code font](https://developers.google.com/style/code-in-text).
- [Put UI elements in bold](https://developers.google.com/style/ui-elements).
- [Use unambiguous date formatting](https://developers.google.com/style/dates-times).

### Images

- [Provide alt text](https://developers.google.com/style/images#text-associated-with-images).
- [Provide high-resolution or vector images](https://developers.google.com/style/images#high-resolution-images) when practical.

## Docsite section indexes

`_index.md` files under `docs/content/` carry frontmatter only — never prose.
A section's landing content goes in a regular page inside that section at `weight: 1` so it sorts first: `logs-and-events/architecture.md`, `reference/development/contributing.md`, `overview.md` where nothing more specific fits.
This way a reader never has to guess whether a directory in the sidebar is also a page.
The site home, `docs/content/_index.md`, is the one exception.

Remember that a page one level below its old `_index.md` needs one more `../` on every relative link, and that inbound links written as `section/#anchor` or `../#anchor` have to be retargeted at the new page.

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
