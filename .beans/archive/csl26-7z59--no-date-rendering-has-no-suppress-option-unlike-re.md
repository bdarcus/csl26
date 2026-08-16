---
# csl26-7z59
title: No-date rendering has no suppress option, unlike real CSL
status: completed
type: bug
priority: normal
tags:
    - rendering
    - dates
    - engine
    - citation
created_at: 2026-08-16T00:16:33Z
updated_at: 2026-08-16T10:48:01Z
---

Found while validating csl26-p7a8's title-quote flip for
taylor-and-francis-council-of-science-editors-author-date via
report-core.js exactParity comparison against citeproc-js.

Real CSL's year-date macro is a bare
<date variable="issued"><date-part name="year"/></date> with no
fallback branch. CSL 1.0 renders nothing when the date variable is
entirely absent unless the style explicitly authors a fallback (e.g.
<if variable="issued">...<else><text term="no date"/></else></if>).
This style doesn't. citeproc-js confirms: an author-less-except-for-a-
surname-that-happens-to-be-"Forthcoming" reference with no `issued`
field renders "(Forthcoming)" -- author only, no date placeholder.

Citum always injects a "n.d." (or "no date") term when a date:issued
template component's date is absent:
"(Forthcoming n.d.)" instead of "(Forthcoming)".

Confirmed via crates/citum-schema-style/src/options/dates.rs that
there is no schema field to suppress this -- `DateConfig.no_date_form`
only selects which term to render (`short` -> "n.d.", `long` -> "no
date"), not whether to render one. This is a pure engine-level
default with no per-style override, unlike the disambiguate and
substitute-title gaps in sibling beans csl26-8nrt/csl26-ecfn.

## Investigation needed
- [x] Confirm this is a genuine engine gap (not a schema field I
      missed) by grepping the full date-rendering path in
      citum-engine for where the no-date fallback gets applied.
- [x] Decide the fix shape: a new DateConfig field (e.g.
      `no-date-form: none` alongside `short`/`long`), or treat
      "no fallback" as the new default and require styles to opt
      IN to a no-date term (matches CSL's actual default -- likely
      the more correct direction, but is a broader behavior change
      needing its own before/after report-core.js sweep across the
      full embedded-style corpus, not just this one style).
- [x] Whichever direction: add tests and a DIVERGENCE_REGISTER.md
      entry documenting the decision.
- [x] Re-run report-core.js --style
      taylor-and-francis-council-of-science-editors-author-date
      after the fix to confirm impact.


## Summary of Changes

**The bean's premise was wrong, and that's the actual finding.** A suppress mechanism already existed — `TemplateDate.fallback: Some(vec![])` — it just wasn't a `DateConfig` field, which is where the original investigation looked. `citum-migrate` has emitted `fallback: []` for every issued date since commit `63fdb104` (2026-07-16); this style was migrated 2026-05-05, before that fix, so its five `date: issued` components carried no `fallback:` key and fell into the engine's implicit `n.d.` path.

Verified against the shipped legacy `.csl`: `year-date` is a bare `<date variable="issued"><date-part name="year"/></date>` with no fallback branch, used identically in `<citation>` and `<bibliography>` — confirming all five sites should suppress.

**Changes:**
- `taylor-and-francis-council-of-science-editors-author-date-core.yaml`: added `fallback: []` to all five `date: issued` components (citation template + 3 bibliography type-variants + bibliography default template), with a provenance comment.
- `crates/citum-engine/src/processor/tests.rs`: refactored the existing single-case no-date test into an `#[rstest]` with a third case pinning T&F CSE's `(Forthcoming)` output alongside the existing harvard/springer contrast cases.
- `docs/adjudication/DIVERGENCE_REGISTER.md`: added div-016 documenting the `fallback: []` opt-out as the deliberate per-style suppress mechanism, kept as an opt-out (not flipped as the default) because `harvard-cite-them-right` depends on the implicit term.

**No engine or schema change** — confirmed via `preferred_no_date_term_form`/`values/date.rs` that this is a documented, tested mechanism (`crates/citum-engine/src/processor/rendering/tests.rs:1387` `empty_fallback_list_leaves_the_date_position_blank_with_or_without_disambiguation`), not a gap.

**Verified via `report-core.js --style taylor-and-francis-council-of-science-editors-author-date`:**
- exactParity: 26/67 → 27/67 (the `(Forthcoming n.d.)` → `(Forthcoming)` divergence cleared)
- Fidelity gate unchanged: citations 20/20, bibliography 44/47 — no regression
- Manually confirmed clean bibliography rendering around the now-empty year slot (`Forthcoming A. Foundations of Declarative Bibliography. Cambridge: University Press` — no stray double-period)

`just pre-commit` green: fmt, clippy, and 2551 nextest tests all pass.

**Follow-up bean filed:** every embedded style migrated before 2026-07-16 has the same latent staleness (no shipped style currently uses `fallback: []`). Each needs its real `.csl` checked individually — some styles (e.g. APA) genuinely author an `n.d.` fallback, so this is not a blanket sweep.
