---
# csl26-d3kj
title: Djot title markup (e.g. nocase) leaks raw when emph:true bypasses TemplateTitle rendering
status: completed
type: bug
priority: normal
tags:
    - rendering
    - engine
    - title
created_at: 2026-07-23T23:23:29Z
updated_at: 2026-08-14T12:52:23Z
---

Discovered while verifying csl26-zaqk's fix (HTML->Djot conversion at CSL-JSON
ingestion). A NATIVE Citum reference with a Djot title
(`[Library of Congress]{.nocase}`) renders correctly (nocase stripped) through
gb-t-7714-2025-numeric, but leaks the raw markup verbatim through apa-7th when
the title's `TitleRendering.emph` resolves to `true` (apa-7th.yaml sets
`titles.monograph.emph: true`).

Repro (no CSL-JSON involved -- purely native ingestion, isolating this from
csl26-zaqk's ingestion fix):

    citum render refs -b refs.json -s apa-7th -m bib --json
    # refs.json: {"id":"x","class":"monograph","type":"book",
    #             "title":"[Library of Congress]{.nocase}", "issued":{"date-parts":[[2020]]}}
    # => "text": "[Library of Congress]{.nocase}. (2020)."   (should be "Library of Congress. (2020).")

    # Same reference through gb-t-7714-2025-numeric renders correctly:
    # => "Library of Congress[M]. ..."

**Root cause found -- the bean's original hypothesis was wrong.** `rendering.emph`
in `component.rs` was not the trigger. The actual trigger is a **missing
author**: apa-7th's `substitute` chain (editor -> title -> translator)
promotes the title into the author slot for author-less references, and
`resolve_title_substitute()` in
`crates/citum-engine/src/values/contributor/substitute.rs` never routed
through `TemplateTitle::values()`'s Djot-aware pipeline at all -- it applied
`apply_text_case_with_language` to the raw string and handed it straight to
`fmt.text(...)`. `gb-t-7714-2025-numeric` was unaffected only because it
renders the title through its own `title:` component, not via substitution;
the same reference WITH an author present also rendered correctly through
apa-7th, confirming the substitute chain (not `emph`) as the discriminator.

Fixed by adding `render_substitute_title_text()` in
`crates/citum-engine/src/values/title.rs` -- a thin wrapper around the
existing `render_part_with_case` used by the normal title path -- and
rewiring `resolve_title_substitute()` to use it, so the substitute path gets
Djot inline rendering, `.nocase` case-protection, and smart-quote
smartening for free, matching `TemplateTitle::values()`'s contract exactly.

## Summary of Changes

- Added `render_substitute_title_text()` (`crates/citum-engine/src/values/title.rs`), a thin wrapper around the existing `render_part_with_case` helper that the normal `TemplateTitle::values()` path already uses.
- Rewired `resolve_title_substitute()` (`crates/citum-engine/src/values/contributor/substitute.rs`) to use it instead of `apply_text_case_with_language` + `fmt.text(...)`. The substitute path now gets Djot inline rendering, `.nocase` case-protection, and smart-quote smartening -- matching the direct title path's contract, including citation-mode quoting and explicit-link URL suppression.
- Added 5 tests in `crates/citum-engine/src/values/tests.rs` (native `InputReference::Monograph` construction, no `csl_legacy` round-trip): an `rstest`-parameterised bibliography-context case (nocase span, emphasis span, plain-text control) and a citation-context quoting case.
- Corpus check (`node scripts/report-core.js --all-features`, current branch vs. a clean `main` worktree baseline at commit `1765777f`, paired by stable entry `id` across all 35 core styles): exactly 2 entries changed in the whole corpus, both `U.S. Const. [art. I]{.nocase}` in `chicago-author-date-18th` and its Taylor & Francis derivative, both flipping from a failing match (raw markup leaked) to a passing one. Zero quote-only side effects, zero regressions elsewhere.

**Two residual gaps found and deliberately left out of this fix** (filed as follow-ups, not fixed here):

- [csl26-0dca](csl26-0dca) -- substituted titles still don't get the title category's `emph`/`strong`/`small_caps`/`vertical_align` (`TitleRendering`) applied, since the substitute value flows through the *contributor* component, not the title category's rendering. This is the neighborhood this bean's original (incorrect) hypothesis pointed at.
- [csl26-4wts](csl26-4wts) -- title sort keys (`sort_support.rs`'s `title_sort_text`) still use the raw title string including Djot markup; this is a separate code path (sort-key derivation, not display rendering) untouched by this fix.
