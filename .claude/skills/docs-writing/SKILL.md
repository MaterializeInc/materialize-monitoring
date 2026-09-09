---
name: docs-writing
description: |
  This skill should be used when writing or editing Markdown prose in this repository, including any page under `docs/content/`, any `SKILL.md`, and `README.md` files.
  Covers the house voice, the RFC 2119 convention for normative pages, and the authorship provenance fields.
---

# Docs Writing

This skill covers **how prose reads**.
The mechanical Markdown rules — sentence-per-line, `_index.md` frontmatter-only, `(internal)` markers, agent notes in comments — live in `CLAUDE.md` and are not repeated here.

Two things this skill exists to prevent: prose that reads as machine-generated, and normative statements a reader cannot act on because their force is ambiguous.

## Voice

The house voice is plain, declarative, and third-person.
A reader should be able to extract the rule from a page without reading the argument for it.

| Rule | Instead of |
|---|---|
| Address the reader in the third person — "a consumer", "an operator", "a contributor" | "you", "your cluster", "if you are being asked to" |
| One claim per sentence; start a new sentence rather than appending a clause | an em-dash aside carrying a second claim mid-sentence |
| Define a term in its own sentence before using it | using it first and glossing it in parentheses later |
| State a consequence once, flatly | arguing for it, or restating it for emphasis |
| Open a section with its subject | "Two things are worth knowing before you change anything" |
| Use a bulleted list | a two-column table that is really a list |
| Let the reader judge severity from the facts | "this is not hypothetical", "worth stating plainly", "which is the point of the page" |

**Tables earn their place at three or more columns of genuinely parallel data.**
A two-column table is a list with extra syntax, and it re-wraps badly in diffs.

**Hedging is permitted and often correct.**
This repository is pre-1.0 and largely single-maintainer, and a promise that cannot be kept is worse than a narrow one.
Hedge by narrowing what is claimed, not by softening the verb — `SHOULD` over "will do its best to try to adequately".
A hedged verb wrapped around a broad claim reads as evasion; a firm verb over a narrow claim does not.

**Do not publish a guarantee to reduce the need for a caveat.**
The shortest path to a page that is easy to keep true is fewer commitments on it.

## RFC 2119

**This repository adopts [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119) keywords for normative prose.**
They exist to remove ambiguity about how much force a statement carries, which plain English does not reliably convey.

Use them on **normative pages** — stability guarantees, deprecation policy, authoring conventions, review criteria.
Do not use them in tutorials, architecture explanations, or troubleshooting prose, where nothing is being required of anyone.

A page using the keywords MUST invoke the shortcode once, immediately after the page's opening paragraph:

```markdown
{{< rfc-2119 >}}
```

Rules for using them:

- `MUST` is absolute. It MUST NOT be wrapped in a best-effort qualifier — `MUST do its best to` is unfalsifiable and cancels itself. Use `SHOULD`.
- `SHOULD` is the calibrated hedge. Reach for it when a rule holds in the general case and a documented exception is acceptable.
- `MAY` grants permission. It does not express likelihood — "a value MAY be absent" states that absence is allowed, not that it is probable.
- Prefer `MUST` over `SHALL`. RFC 2119 treats them as synonyms, and mixing both in one document invites the reader to hunt for a distinction that is not there.
- Keywords are uppercase only when normative. A lowercase "may" in ordinary prose is fine and carries no force.
- A statement of fact about the policy is not a requirement. "These interfaces are not subject to the cycle" is correct; "SHOULD NOT be subject" places an obligation on nobody.

## Authorship provenance

Every page under `docs/content/` carries two frontmatter params recording **who produced the first draft**:

```yaml
# custom parameters
params:
  author: Heather Lapointe
  agent: None
```

- `author` — the human accountable for the page.
- `agent` — the model that wrote the initial draft, or `None` if a human did.

**These record initial authorship only and MUST NOT be updated on later edits.**
Subsequent revisions are findable through git history, which is a better record than a field that has to be maintained by hand.

An agent creating a new page MUST stamp `agent` with its own model name.
An agent editing an existing page MUST leave both fields alone, including when its edit is substantial.

The fields are not rendered on normal pages.
`{{< param-table >}}` displays them where a page wants them visible, as the design docs do.

## Exemplars

**Follow:** `docs/content/reference/stability2.md` — the reference implementation of the voice and the RFC 2119 convention.

<!--
stability2.md is the in-progress rewrite and will replace stability.md on completion.
Until it does, the page at stability.md is pre-convention and MUST NOT be used as a style reference.
Update this section to name stability.md once the rename lands.
-->

**Do not imitate:** most pages under `docs/content/` predate this skill and were machine-drafted.
`operating/troubleshooting-materialize.md` and `operating/production-best-practices.md` are the clearest specimens of the register being replaced — second person throughout, em-dash asides, dramatized stakes, tables where lists belong.
They are accurate and useful; they are not style references.

Do not rewrite a page's voice as a side effect of an unrelated edit.
A voice rewrite is its own change, with its own diff.

## Review checklist

A page is ready when:

- [ ] No second-person address.
- [ ] No sentence carries a second claim after an em-dash.
- [ ] Every two-column table is a list.
- [ ] Normative force is carried by RFC 2119 keywords, and the shortcode is invoked.
- [ ] No `MUST` sits on a best-effort phrase.
- [ ] `params.author` and `params.agent` are set, and unchanged if the page already existed.
- [ ] Nothing on the page promises a surface the [policy of record](../../../docs/content/reference/internal/versioning.md) marks as no-promise.
