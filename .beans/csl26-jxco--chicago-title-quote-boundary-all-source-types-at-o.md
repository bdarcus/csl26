---
# csl26-jxco
title: 'Chicago: title quote boundary, all source types at once'
status: in-progress
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - title
    - punctuation
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-08-24T00:52:05Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 300 entries -- the single largest class family-wide. Supersedes/extends completed cluster csl26-87yl, which fixed article-newspaper + thesis quoting one type at a time (+1 entry) and explicitly deferred map/dataset/report/webpage. This bean's scope is every source type at once, verified per-type against citeproc-js render sites per docs/specs/CHICAGO_FAMILY_STRATEGY.md's authority rule. Touches all four Chicago variants.

## Update from wave 1 (2026-08-23)

Wave 1 (csl26-4xr6, title case) discovered this class's scope is larger
than originally estimated. Adding `manuscript`/`motion-picture`/
`broadcast`/`collection`/`song`/`webpage` to `chicago-notes-18th`'s
and `chicago-shortened-notes-bibliography-core`'s `titles.type-mapping`
(mirroring chicago-author-date-18th's existing list) regressed 3
previously-passing archival/manuscript entries: those types picked up
this family's `titles.component.quote: true`, which is correct for
genuine article titles but wrong for bare collection titles like
'Revere Family Papers' and 'Landscapes of Zambia, Central Africa'. Wave
1 landed narrower (map/dataset only) and reverted that list; this bean
now also owns:

- Extending `chicago-notes-18th`/`chicago-shortened-notes-bibliography-core`'s
  type-mapping to manuscript/motion-picture/broadcast/collection/song/
  webpage, per-type, with the same regression verification wave 1 used
  (per-entry exactMatch diff, not just aggregate count).
- `map`/`dataset` in `chicago-author-date-18th`/
  `taylor-and-francis-chicago-author-date` are now correctly title-cased
  (wave 1) but still fail exact match because of this class's quote
  boundary -- these are concrete, ready-to-verify entries to start from
  (previously: 'The Racial Dot Map' rendered quoted when the oracle does
  not quote map titles at all).

See docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md's
postscript for the full wave-1 writeup.

## Wave 2 progress (2026-08-23, session 1)

First verified increment landed: `document: component` added to
`chicago-author-date-18th`/`taylor-and-francis-chicago-author-date-core`,
plus `document: component` + `thesis: component` added to
`chicago-notes-18th`/`chicago-shortened-notes-bibliography-core`. Both
`document` and `thesis` are quoted+title-cased per the shipped
`chicago-author-date.csl`'s title choose-block (`article dataset document
interview manuscript paper-conference personal_communication speech thesis
webpage`), but the engine's hardcoded type→category fallback either has no
entry for `document` (falls to `Default`, no transform at all) or wrongly
hardcodes `thesis` to `Monograph` (italic, not quoted).

Per-entry exactMatch diff (not aggregate) confirmed **zero regressions**
across all four styles. Real text-level progress verified (title-case +
quoting now correct) for 26 `document` rows in
`chicago-author-date-18th`/T&F, 36 `document`+`thesis` rows in
`chicago-shortened-notes-bibliography`, and the `chi-thesis` citation
fixture in `chicago-notes-18th`. **Zero rows flipped to full exactMatch**
this pass -- expected per the audit postscript's entanglement pattern:
these rows carry additional un-landed defects (acronym case `Phd`/`PhD`,
missing ProQuest/genre detail, missing "archived" date phrasing) that must
also clear before the row counts.

Also ran and reverted a probe: adding `quote: true` to
`chicago-author-date-18th`'s `titles.component` (to try to reach `document`
rows via the category system, matching how the notes-family styles already
do it) produced **zero gain** (quote still didn't reach `document` rows) and
**one regression** (`song` type "Selected Poems" -- a standalone recording
with no `container-title`, wrongly quoted). Reverted; not part of the
landed commit.

### Engine gap found (filed, not fixed here -- YAML-only wave)

`chicago-author-date-18th`'s bibliography renders anonymous/no-author
entries (`document` type -- CMOS annotation titles with no author field) by
substituting the title into the author position
(`crates/citum-engine/src/values/contributor/substitute.rs`,
`resolve_author_substitute` / `resolve_category_quote`). Title-case
propagates correctly through this path (confirmed: `get_title_category_
title_rendering` is called and the category's `text-case` applies), but
**quote does not**, even with `titles.component.quote: true` set. By
contrast the notes-family styles (`chicago-notes-18th`,
`chicago-shortened-notes-bibliography-core`), which already carry `quote:
true` on `titles.component` from wave 1, DO correctly quote their own
anonymous/substituted entries -- confirmed via the same before/after diff
(the `document`/`thesis` rows above). So the gap is specific to how
`chicago-author-date-18th`'s bibliography reaches the substitute path, not
a general engine limitation. Root cause not isolated further (would need
tracing why this style's bibliography-side author-substitution differs from
the notes-family's); scoped as follow-up, not fixed in this YAML-only wave
per the audit's own layering rule (engine changes need their own reviewed
scope, family-wide blast radius).

### Deliberately not touched this pass

- `webpage` in `chicago-author-date-18th`: already mapped to `component`
  with an explicit per-node `wrap: quotes` on its "title present" branch,
  but corporate-authored entries (e.g. "City of Chicago") render with title
  BEFORE the organization name and no quotes -- evidence the entry is
  taking the substitute path (organization stored in `publisher`, not
  `author`; `publisher` isn't in `bibliography.options.substitute.candidates`,
  so `title` substitutes instead). This is a contributor-substitution
  gap (class E, not class B) -- out of this bean's scope.
- `collection` in the notes-family styles' `type-mapping`: intentionally
  left out. Per the shipped `.csl`, `collection` is in the never-quote set
  (`bill collection legislation regulation treaty` -> title case, no
  quotes), so adding it to `component` (which now carries `quote: true` in
  both notes-family styles) would regress archival entries exactly as wave
  1 found. `manuscript`/`motion-picture`/`broadcast`/`song`/`webpage`
  extension into these styles' type-mapping is still open -- each needs the
  same per-type verification against the `.csl` choose-block this session
  used for `document`/`thesis`, not a bulk copy of author-date-18th's list.
- `map`/`graphic` in `chicago-author-date-18th`: still wrongly quoted
  (e.g. "The Racial Dot Map"). Per the `.csl`, both are in the
  never-quote, always-italic set (`book classic graphic hearing map`).
  Fix needs a dedicated bibliography type-variant (`title: primary, emph:
  true`, no wrap) -- category reassignment can't remove a node-level
  `wrap: quotes` that a resolved type-variant doesn't carry (these two
  currently fall through to the file's default `template:` block, which
  hardcodes the wrap). Not yet attempted.

Remaining open work for this bean: map/graphic type-variant, webpage
corporate-author substitution (may need its own bean, class E territory),
manuscript/motion-picture/broadcast/song/webpage extension into the
notes-family type-mapping (per-type verified), and tracing the
chicago-author-date-18th substitute-quote engine gap.
