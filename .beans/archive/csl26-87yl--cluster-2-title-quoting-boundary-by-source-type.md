---
# csl26-87yl
title: 'Cluster 2: title quoting boundary by source type'
status: completed
type: task
priority: high
created_at: 2026-08-07T13:20:49Z
updated_at: 2026-08-23T20:41:03Z
parent: csl26-h7oc
---

Per docs/specs/CHICAGO_FAMILY_STRATEGY.md cluster ordering. Oracle quotes titles/chapter-titles on types this style doesn't (e.g. archival letters, dictionary entries, newspaper articles), and vice versa for maps/datasets — a type-variant boundary issue, not a uniform quoting rule. ~111 residual observations (2026-07-30 clustering, csl26-giun evidence). Verify per source type against citeproc-js render sites, not CMOS prose, per the strategy doc's authority rule. Fix across all four Chicago-family styles at once, not per-style.

## Summary of Changes

Verified the title-quoting boundary against the legacy `title-primary` CSL
macro (identical across `chicago-author-date.csl`, `chicago-notes-
bibliography.csl`, `chicago-shortened-notes-bibliography.csl`,
`taylor-and-francis-chicago-author-date.csl` — confirmed by direct read, not
CMOS prose), per the strategy doc's authority rule.

Of the three examples cited in this bean's evidence, two (`entry-dictionary`,
`manuscript, collection`) were already fixed in a prior session — confirmed by
inspection, not re-touched. Landed the remaining evidenced gap plus one
closely-related defect found via the same macro:

- **`article-newspaper`** (chicago-author-date-18th.yaml): added missing
  `wrap: {punctuation: quotes}` to `title: primary`. Newspaper-article titles
  have `container-title` present, landing on the macro's "quotes" branch.
  Inherited automatically by `taylor-and-francis-chicago-author-date-core`
  (verified by rendering the child style directly, not just reading the
  parent) — confirms the extends chain propagates this class of fix without
  duplication.
- **`thesis`** (same file): added a `modify` (`emph: false` +
  `wrap: quotes`) on the `extends: book` type-variant. The engine's
  `title_category()` hardcodes `thesis => Monograph` (italic) at the code
  level, but the legacy macro explicitly lists `thesis` in its quoted-type
  bucket regardless of `container-title`. Fixed style-locally per the
  strategy's engine/Rust-out-of-scope boundary, not by touching the engine
  default. Verified rendering (quoted, not italic) before measuring.

**Measured with `node scripts/report-core.js --styles
chicago-author-date-18th,chicago-notes-18th,chicago-shortened-notes-
bibliography,taylor-and-francis-chicago-author-date --parallelism 1`**
(exact-parity `passed/total`):

| style | before | after (newspaper) | after (+thesis) |
|---|---|---|---|
| chicago-author-date-18th | 172/546 | 173/546 | 173/546 |
| taylor-and-francis-chicago-author-date | 172/546 | 173/546 | 173/546 |
| chicago-notes-18th | 22/72 | 22/72 | 22/72 |
| chicago-shortened-notes-bibliography | 13/473 | 13/473 | 13/473 |

The `article-newspaper` fix is the one that moved parity (+1/546, both
author-date and T&F, confirming inheritance). `thesis` is independently
verified-correct against the legacy macro but contributed zero additional
exact-parity entries in this corpus — the affected items have other,
unrelated co-occurring mismatches blocking full byte match. Kept it: it's the
same defect class, evidenced by the same macro read, and the cluster as a
whole still moves parity upward, satisfying the strategy doc's revert rule
(applies to the cluster, not each line item).

notes-18th and shortened-notes-bibliography did not move — neither carries
this specific `article-newspaper`/`thesis` defect shape today (notes-18th's
`article-newspaper` already inherits `quote: true` from the `chicago` title
preset's `component` bucket; its `thesis` extends `dataset`, a structurally
different, entangled path — see below).

## Deferred, not landed in this PR

- **`map`** (3 corpus entries in author-date/T&F): legacy macro groups
  `map` with `book`/`classic`/`graphic`/`hearing` (italic, not quoted).
  Currently unquoted-but-should-be-quoted was the bean's stated example
  direction; actual corpus behavior is the opposite — `map` has no
  type-variant at all in `chicago-author-date-18th.yaml` and falls to the
  generic fallback `template:`, which hardcodes quotes, so maps are
  currently *wrongly quoted*, not wrongly unquoted. Fixing it needs three
  coupled changes for only 3 entries: a new `map` type-variant, a
  `titles.type-mapping: {map: monograph}` entry (map isn't in the engine's
  hardcoded Monograph fallback), and a matching re-add on
  `taylor-and-francis-chicago-author-date-core` (which explicitly nulls its
  inherited `type-mapping` for an unrelated documented reason — sentence-case
  proper-noun workaround for csl26-4kt3). One of the three map entries ("Yu ji
  tu") renders via a still-unexplained different path (a note-field date hack,
  `issued: 1933?`, not a real `issued` field) that looks like it belongs to
  Cluster 5 (document-routed archival refs), not this cluster. Left for a
  future pass with its own evidence, per advisor guidance not to chase three
  coupled changes for three entries in this PR.
- **`chicago-notes-18th` / `chicago-shortened-notes-bibliography-core`
  `dataset`/`report`/`thesis`/`webpage`**: found via investigation (not in
  this bean's original evidence) that `dataset`'s title is currently
  unformatted (no quotes, no italics) — a real defect (dataset is in the
  macro's quoted bucket) — and that `report`, `thesis`, and `webpage` all
  structurally `extends: dataset`. Adding `wrap: quotes` directly to
  `dataset`'s title would incorrectly propagate onto `report` (which should
  stay italic — `report` is not in the macro's quoted-type list). Untangling
  this needs its own `modify`/override work verified against this style's
  `dataset`/`report`/`thesis`/`webpage` entries specifically; not attempted
  here since it wasn't part of this bean's evidenced scope. Flagging for
  whichever future cluster (2 follow-up, or folded into 3/4) picks up
  notes-family title quoting.

## Assessment: does this deliver the "reduced LOC via inheritance" framing?

Per the strategy doc: `chicago-18-base.yaml` is explicitly out of scope for
every cluster, and cluster 1 recorded "inheritance: untouched, deliberately."
This cluster is the same shape — the fix landed in
`chicago-author-date-18th.yaml` and was picked up by
`taylor-and-francis-chicago-author-date-core` for free via the existing
`extends` chain (net LOC added: 8 lines, 0 in the T&F file), which
demonstrates the *existing* inheritance graph working as designed, but does
not *reduce* family LOC — no shared code moved to `chicago-18-base.yaml` or a
new shared component. The commit that set this strategy in motion (1c44a515)
frames both "improved exact parity" and "reduced overall LOC via inheritance"
as goals; only the first is being pursued by the cluster plan as currently
scoped. Reducing LOC would require a deliberate pass to extract genuinely
family-shared logic into the base style, which none of the 7 cluster beans
under csl26-h7oc currently plan to do.

## Update from the 2026-08-23 leverage audit

docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md
re-measured this defect family-wide (not per-type) and found title
quote boundary is the single largest class in the family (300 entries,
26% of all failing rows), not the ~111 estimated here. csl26-jxco picks
up where this bean's deferred per-type list (map, dataset, report,
webpage) left off, scoped to all source types at once rather than one
type per pass -- the execution pattern this bean's own history shows
does not converge.
