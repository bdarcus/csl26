---
# csl26-huuz
title: Disambiguation collision grouping is variable-based, not render-text-based
status: completed
type: bug
priority: normal
tags:
    - engine
    - fidelity
created_at: 2026-08-06T12:43:05Z
updated_at: 2026-08-12T00:48:29Z
parent: csl26-ccdt
blocking:
    - csl26-q67h
---

citeproc-js groups year-suffix disambiguation collisions by RENDERED TEXT
equality (the CSL <sort>/disambiguation macro's actual output), while citum's
Disambiguator groups by abstract variable equality (same author-slot key,
same date value) regardless of what text the style's template actually
renders for that reference. For most references these coincide, but they
diverge whenever a style's date/name macro is type-conditional.

## Evidence (csl26-m8la, 2026-08-06 session)

gb-t-7714-2025-author-date's upstream CSL date-intext macro is conditional:
for article-journal/article-magazine types without volume/issue it falls
through to a plain <date variable="issued"> branch (never reaching the outer
<else> that renders the locale's "no date" term), while all other types with
no issued date DO reach that outer <else> and render "无日期"/"n.d.".

Two article-journal references (gbt7714.7.2.1:7, gbt7714.7.2.3:7) with no
issued date therefore render as bare "Anon，b." / "Anon，c." in the oracle —
no "n.d." term at all — and citeproc-js treats them as a SEPARATE,
non-colliding disambiguation sequence from the "Anon，n.d.-X" sequence the
other undated anonymous references share (confirmed: both
gbt7714.7.2.1:7="Anon，b." and gbt7714.7.3:7="Anon，n.d.-b." independently
reach letter 'b' in the oracle output — only possible if they're in separate
groups).

citum's Disambiguator has no notion of the rendered text — it only sees
"author=None (ANONYMOUS_FALLBACK_KEY), date=None" for both, so it lumps all
12 English-language anonymous-undated references (plus a 13th,
gbt7714.8.11.2.2:2, a webpage the oracle excludes from this bucket entirely
for a similarly unreplicated reason) into ONE shared collision group,
producing a systematic letter-count mismatch (citum's group has 13 members;
oracle's equivalent groups have 10 + 2 = 12, with different membership) that
manifests as a consistent +2 letter offset for every entry after the first
divergence point.

Full before/after diagnostic data (entry IDs, letters, rendered text) is in
this bean's parent PR discussion.

## Why this is architectural, not a bounded fix

Fixing this properly means giving Disambiguator's collision-key computation
awareness of what the ACTIVE TEMPLATE would actually render for a reference
(type-conditional branches, available-date/accessed fallbacks, etc.) — or
else switching collision detection to compare rendered text directly, closer
to citeproc-js's own algorithm. Either is real design work spanning the
template-resolution and disambiguation modules, not a targeted patch.

## Scope note

csl26-m8la's shipped fix (registry-order year-suffix ties for a resolved
`group_sort`) is unaffected by this and already brings
gb-t-7714-2025-author-date's adjusted bibliography oracle failures from 42 to
30 (out of 203), with zero regressions in citum-engine's test suite. A
follow-up bean (`csl26-q67h`, "restore gb-t-7714-2025-author-date's own
bibliography.sort") covers the still-missing explicit sort — this bean covers
only the residual ~9-entry gap in the English anonymous-undated bucket that
traces to the grouping/rendering mismatch described above, which is
independent of whether that sort gets restored.

## Summary of Changes

Collision-group **membership** now reflects what a reference's resolved date
slot actually renders, instead of assuming every undated reference renders
uniform "no date" text.

**Engine:**
- `sorting.rs`: `first_date_component_for_bibliography`/`_for_citation`,
  mirroring the existing contributor-resolution helpers.
- `disambiguation.rs`: `build_group_key` falls through to a new
  `date_slot_discriminant`/`date_component_discriminant` when there's no
  parseable issued year. Prefers the bibliography spec (confirmed empirically
  — GB/T's `citation:` template is undifferentiated by type, unlike its
  `bibliography:` template; preferring it collapsed every undated reference
  onto one discriminant). An access date never contributes identity, whether
  primary or fallback, and the search stops (doesn't fall through) once an
  access-date candidate would be selected — mirrors the if/else-if/else shape
  the fallback chain represents.
- `values/date.rs`: extracted `resolve_date_variable` (shared date-variable →
  reference-field mapping) and `render_date_fallback_chain`. The fallback
  render path now inlines a disamb suffix into a `date:` fallback candidate's
  raw text *before* its wrap is applied (so the letter lands inside brackets:
  `[2020a]`, not `[2020]a`), and renders a standalone suffix when nothing in
  the fallback chain resolves at all (previously: silently no letter).

**Style:** `gb-t-7714-2025-author-date.yaml` — `article-journal,article-magazine`
lost its `term.no-date` fallback (upstream's date-intext macro never reaches
it for that branch without volume/issue); `webpage,post,post-weblog` gained
an access-year fallback ahead of the no-date term (upstream's
`<else-if variable="accessed">` branch).

**Spec:** `docs/specs/DISAMBIGUATION.md` §1 — new "Date-slot discriminant"
subsection with the four-case rule, the bibliography-preference rationale,
and the rendering-side fixes required alongside it.

**Measured:** GB/T's diagnostic upstream-corpus bibliography scope
(`count_toward_fidelity: false`, no fidelity-gate impact) went 147/203 →
176/203 (+29), citations 8/8 unchanged. Zero regressions across the 35-style
exemplar corpus (`report-core.js --all-features`, before/after diffed via a
clean detached worktree) or `cargo nextest run` (2463/2463). 9 new unit tests
in `disambiguation.rs` (BDD `given/when/then`, `#[rstest]`, `assert_eq!` on
captured discriminants/group keys).

**Scope note:** this bean closes group *membership*. Five entries whose
membership is now correct still show the wrong *letter* because their
ordering depends on `gb-t-7714-2025-author-date`'s own `bibliography.sort`,
which is still missing (inherits `citation-number` registry order from its
numeric base) — that gap is `csl26-q67h`, linked below, not closed here.

Two unrelated defects surfaced while measuring were not filed as new beans —
existing siblings already covered them and got the corpus evidence appended
instead: `csl26-yyrs` (standards-body contributor promotion, now 5 entries
not 3) and `csl26-c361` (NumberForm::Ordinal not implemented, new GB/T
evidence: `5th editors` vs `5 editors`).
