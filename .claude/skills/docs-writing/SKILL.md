---
name: docs-writing
description: |
  This skill should be used when writing or editing Markdown prose in this repository, including any page under `docs/content/`, any `SKILL.md`, and `README.md` files.
  Covers the Google developer documentation style guide the repository follows, the house conventions that depart from it,
  the gaps filled from Grafana's AI quick reference, the RFC 2119 convention for normative pages, and the authorship provenance fields.
---

# Docs writing

This skill covers how prose reads.
The mechanical Markdown rules, such as sentence-per-line and frontmatter-only `_index.md` files, live in `CLAUDE.md`.

The skill exists to prevent prose that reads as machine-generated, and normative statements whose force a reader can't judge.

Most of the repository doesn't follow these conventions.
When you update content, bring the part you touch into line with them; a localized rewrite is encouraged, not scope creep.
Leave the rest of the page for the changes that touch it.

## Sources and precedence

The [Google developer documentation style guide](https://developers.google.com/style) is the style authority for this repository.
Where it doesn't settle a question,
the [Grafana Labs AI quick reference](https://grafana.com/docs/writers-toolkit/write/style-guide/ai-quick-reference/) fills the gap.

When sources disagree, resolve the conflict in this order:

1. The house conventions in this skill.
2. The Google guide.
3. The Grafana quick reference.
4. The third-party references that Google's [About this guide](https://developers.google.com/style) names for questions the guide doesn't cover.

The house conventions depart from the Google guide in the following places:

- **Normative keywords:** RFC 2119 keywords carry defined force on policy pages, where Google avoids "should" and reserves "may".
  See [RFC 2119](#rfc-2119).
- **Exceptions in callouts:** an exception to a rule can sit in a `NOTE` or `WARNING` callout,
  although Google keeps required information out of notes.
  See [Callouts](#callouts).
- **Plans and status:** the roadmap, design docs, and changelog describe plans and changes, and work in an open PR is described as done.
  See [Time and status](#time-and-status).
- **Description lists outside the docsite:** files that GitHub renders use bulleted lists with bold run-in terms instead.
  See [How the highlights apply here](#how-the-highlights-apply-here).

Where a Grafana rule conflicts with Google, follow Google:

- **Cross-references:** write "For more information, see [Page title](…)", not "refer to".
- **Placeholders:** write `UPPER_SNAKE_CASE` without angle brackets, and italicize an inline placeholder as ``*`NAMESPACE`*``.
  After a code sample, explain each placeholder with "Replace `NAMESPACE` with…" or "Replace the following:" and a list.
  Grafana writes `<VARIABLE_NAME>`.

## Highlights

The following list is copied from [Highlights](https://developers.google.com/style/highlights) without changes to its wording.
`CLAUDE.md` carries the same list, so an agent has it without loading this skill.
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

### How the highlights apply here

The following notes scope some of the highlights to this repository:

- **Second person:** "you" is the reader of the page.
  Say who that is near the top of the page and keep it consistent.
  In skills and under `docs/content/reference/development/`, "you" is a contributor or an agent working for one.
  Elsewhere, it's the person who installs or operates the stack.
  When the subject is something the project does, name the project or component.
  Use "we" only for the maintainers, and only where the context makes that clear.
- **Pre-announcing:** see [Time and status](#time-and-status) for where plans belong.
- **Description lists:** Hugo renders the `Term` and `: Description` syntax under `docs/content/`.
  GitHub doesn't render it, so in `SKILL.md`, `README.md`, and `CLAUDE.md` files, use a bulleted list with a bold run-in term, as this list does.
  Because Hugo has description lists turned on, a wrapped line that starts with `: ` turns the paragraph before it into a term.
  Keep the colon at the end of the previous line.

## Voice

Write so that a reader can find the rule without reading the argument for it.
The Google guide sets the register; the following rules address the habits it doesn't name:

- **One claim per sentence.** Start a new sentence rather than appending a second claim after an em dash.
- **Define a term before you use it.** Give the definition its own sentence rather than glossing the term in parentheses later.
- **State a consequence once, flatly.** Don't argue for it, and don't restate it for emphasis.
- **Open a section with its subject.** Skip warm-ups such as "Two things are worth knowing before you change anything".
- **Let the reader judge severity from the facts.**
  Cut phrases such as "this is not hypothetical", "worth stating plainly", and "which is the point of the page".

**Hedge by narrowing the claim, not by softening the verb.**
A promise that can't be kept is worse than a narrow one.
On a normative page, write `SHOULD` rather than "will do its best to try to adequately".
A soft verb around a broad claim reads as evasion; a firm verb over a narrow claim doesn't.

**Don't publish a guarantee to reduce the need for a caveat.**
The shortest path to a page that's easy to keep true is fewer commitments on it.

## Time and status

Write as though the work is complete once every PR it needs has merged, including downstream PRs.
Describe work that's in an open PR as done, and check off its roadmap row in that PR.
A page that calls the work "in progress" goes stale the moment the PR merges, and correcting it takes another PR.

The roadmap, the design docs, and the changelog exist to describe plans and changes, so the Google rule against pre-announcing doesn't apply to them.
Everywhere else, describe what ships, and drop the time-bound words that
[timeless documentation](https://developers.google.com/style/timeless-documentation) lists,
such as "currently", "new", "now", "soon", and "not yet".

When you know content is going to change in a future version, describe the present behavior and record the expected change in an agent note beside it:

```markdown
<!-- TODO: drop this caveat in the next major version, after the deprecated value is removed. -->
```

Use `TODO` for a planned change and `FIXME` for content you know is wrong but can't correct in this change.
Name the version or issue the change waits on, so a later reader can tell when it's due.

## Lists, tables, and callouts

Use a table when each item has three or more related pieces of data, such as a keyword, what it's for, and how it's misused.
Use a description list when each item is a pair, such as a field and what it records.
Introduce every table and list with a complete sentence, because some screen readers don't announce a table.
Omit the introduction only when the heading directly before a list already says what it holds.

Don't state how many items a list or section holds, as in "the following four steps".
The count goes stale when someone adds an item.
If you edit a sentence that carries a count, rephrase the count out rather than updating it.

Parallel content is laborious to structure while drafting, so it often stays as prose or a loose list.
Converting it to the right list or table is a welcome edit, not scope creep, whenever a change already touches that section.
An explanation or an argument stays a paragraph; don't break it into a list.

### Callouts

You can set an exception to a rule apart in a `> [!NOTE]` callout.
Use `> [!WARNING]` instead when acting on the exception risks data loss or an outage.
Keep the rule itself in the main text, so a reader who skips the callout still has it.

```markdown
> [!NOTE]
> The exception, and why it holds.
```

Hugo and GitHub both render this syntax.
Use callouts sparingly, and don't stack one directly on another.

## Gaps filled from Grafana

The following rules come from the
[Grafana Labs AI quick reference](https://grafana.com/docs/writers-toolkit/write/style-guide/ai-quick-reference/), adapted to this repository.
Most restate parts of the full Google guide that the highlights leave out.

### Ground every claim

Check every claim about behavior, defaults, values, or UI against a source:
the code, the chart's `values.yaml`, generated output, or upstream documentation.
If you can't verify a claim, mark it `[UNVERIFIED]` instead of guessing.
Resolve every marker before the change merges.

### Audience

Write each page so that it stands on its own.
A reader who arrives from a search engine gets the context they need on the page, or a link to it.
Keep contributor material out of operator pages; it belongs under `docs/content/reference/development/`.

### Tone and sentences

The following rules cover tone and sentence length:

- Use present tense, and avoid "will".
- Use contractions, such as "don't", "isn't", and "you're", but never for an RFC 2119 keyword.
- Be confident, not boastful.
  Don't call anything "easy", "simple", or "quick", and cut "just" and "simply".
- Say what to do rather than what to avoid, where both work.
- Keep sentences and paragraphs short.
- Cut filler, such as "there is", "there are", "in order to", "it is important to", and "please note".
- Avoid idioms and directional language, such as "on the left" or "the table below".
  Write "the following table" or "earlier on this page".

### Headings

The following rules apply to headings:

- Start a task heading with a bare-infinitive verb: "Install the chart", not "Installing the chart".
- Don't number headings or prefix them with "Step 1:".
- Follow every heading with content, and don't stack a subheading directly under its parent heading.
- Put most sections at h2 and their subsections at h3, and don't skip a level.

### UI elements, code, and links

The following rules cover formatting:

- Bold the label, not the element type: "Click **Apply**", not "Click the **Apply** button".
- Match the UI's capitalization.
- Write a navigation path with `>`, as in "Go to **Alerting** > **Alert rules**".
- Put filenames, configuration keys, commands, metric names, and status codes in code font.
- Use the linked page's title, or a short description of it, as the link text.
- Link from one `docs/content/` page to another with a relative path that ends in `/`, not `.md`.

### Names and terms

The following terms have a fixed form here; the [Google word list](https://developers.google.com/style/wordlist) covers the rest:

- Write "self-managed", not "self-hosted".
- Write "Materialize Cloud" in full, never "Cloud" alone.
- List signal types in the order metrics, logs, traces, profiles.
- Write "data source" as two words.
- Write "allowlist" and "denylist", not "whitelist" and "blacklist".
- Write "for example" and "that is", not "e.g." and "i.e.".
- Write "because" for a reason and "after" for a sequence, not "since" and "once".
- Don't write "&" for "and" unless the UI does.

## RFC 2119

**This repository adopts [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119) keywords for normative prose.**
The keywords give "must", "should", and "may" a defined force, which plain English doesn't reliably convey.
The Google word list makes the same point from the other side: it calls "should" ambiguous and reserves "may" for policy.
The keywords narrow that guidance for policy pages; they don't contradict it.

Use the keywords on **normative pages**: stability guarantees, deprecation policy, authoring conventions, and review criteria.
Don't use them in tutorials, architecture explanations, or troubleshooting prose, where nothing is required of anyone.

A page that uses the keywords MUST invoke the shortcode once, immediately after the page's opening paragraph:

```markdown
{{< rfc-2119 >}}
```

The following table gives each keyword's use and its common misuse:

| Keyword | Use it for | Don't |
|---|---|---|
| `MUST` / `MUST NOT` | An absolute requirement, with no acceptable exception | Wrap it in a best-effort qualifier. `MUST do its best to` is unfalsifiable and cancels itself; use `SHOULD` |
| `SHOULD` / `SHOULD NOT` | The calibrated hedge: a rule that holds in the general case, where a documented exception is acceptable | Reach for it when the rule is absolute, which weakens every other `SHOULD` on the page |
| `MAY` | Permission. Something is allowed | Read it as likelihood. "A value MAY be absent" says absence is permitted, not that it's probable |
| `RECOMMENDED` | A `SHOULD` where the subject is a practice rather than an actor | Mix it with `SHOULD` in one list of parallel rules |
| `SHALL` | Nothing. Prefer `MUST` | Use both in one document. RFC 2119 treats them as synonyms, and mixing them invites a hunt for a distinction that isn't there |

The following rules apply to the keywords as a whole.

Keywords are uppercase only when normative.
Everywhere else, follow the Google word list: "must" or "you need" for a requirement, "can" for permission or ability, and "might" for possibility.
Avoid a lowercase "should" or "may".

Never contract a keyword.
Write `MUST NOT` and `SHOULD NOT`, not "mustn't" or "shouldn't": a contraction can't be uppercase, so it loses the keyword's force.

Address the party the rule binds.
When it binds the reader, write "you MUST".
When it binds the project, name the project: "`materialize-monitoring` MUST provide notice of a deprecation."

A statement of fact about the policy isn't a requirement.
"These interfaces aren't subject to the cycle" is correct; "SHOULD NOT be subject" places an obligation on nobody.

## Authorship provenance

Every page under `docs/content/` carries frontmatter params that record **who produced the first draft**:

```yaml
# custom parameters
params:
  author: Heather Lapointe
  agent: None
```

The params record the following:

- **`author`:** the human accountable for the page.
- **`agent`:** the model that wrote the initial draft, or `None` if a human did.

**These record the initial writer, not the current one.**
Git history records ordinary revisions better than a field maintained by hand.

What you do with the params depends on the edit:

- **Creating a page:** stamp `agent` with your own model name.
  An agent MUST do this.
- **Editing a page:** leave the params alone.
  An agent MUST NOT touch them.
- **Rewriting more than half the page in one edit:** you MAY re-stamp the params.
  At that point the page has a new initial writer in every sense that matters.

The threshold is per edit, not cumulative.
A page reworked over ten small commits keeps its original stamp, because none of those edits replaced it.

The params aren't rendered on normal pages.
`{{< param-table >}}` displays them where a page wants them visible, as the design docs do.

## Exemplars

Use the following page for the convention it gets right:

- **`docs/content/reference/stability.md`:** the reference for RFC 2119 keywords on a normative page.
  It's written in the third person, so don't take its voice as a model.

`operating/troubleshooting-materialize.md` and `operating/production-best-practices.md` show what to fix when you touch them:
em-dash asides that carry a second claim, and dramatized stakes.
Their second person is correct.
They're accurate and useful; they aren't style references.

## Review checklist

A page is ready when it meets every item in the following checklist:

- [ ] "You" is the reader, identified near the top of the page, and instructions use the imperative.
- [ ] Sentences use active voice and present tense, without "will".
- [ ] Work in an open PR is described as done.
- [ ] Outside the roadmap, design docs, and changelog, nothing is pre-announced and no time-bound words such as "currently" or "new" remain.
- [ ] A change expected in a future version is recorded in a `TODO` or `FIXME` agent note, not in the prose.
- [ ] Headings use sentence case, and task headings start with a bare-infinitive verb.
- [ ] Every table and list has an introductory sentence, and no sentence states how many items follow.
- [ ] Items with three or more parts are in a table, and pairs are in a description list.
- [ ] Link text is descriptive, and cross-references use "see".
- [ ] Code-related text is in code font, and UI labels are bold.
- [ ] Every image has alt text.
- [ ] No sentence carries a second claim after an em dash.
- [ ] No `[UNVERIFIED]` marker remains.
- [ ] Normative force is carried by uncontracted RFC 2119 keywords, and the shortcode is invoked.
- [ ] No `MUST` sits on a best-effort phrase.
- [ ] `params.author` and `params.agent` are set, and unchanged unless this edit rewrote more than half the page.
- [ ] Nothing on the page promises a surface the [policy of record](../../../docs/content/reference/development/versioning.md) marks as no-promise.
