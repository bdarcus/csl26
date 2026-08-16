---
# csl26-h9jy
title: disambiguate-add-givenname falsely resolves same-initial collisions
status: completed
type: bug
priority: high
tags:
    - engine
    - disambiguation
    - citation
created_at: 2026-08-16T19:46:19Z
updated_at: 2026-08-16T20:17:17Z
---

Found while investigating csl26-5753. `disambiguate-add-givenname` silently
under-disambiguates two different authors whose given names share the same
initial (e.g. Brandon vs Biff Bronchitis, both "B."), for every
`givenname-disambiguation-rule` value -- not just `by-cite`.

Reproduced with a style using `contributors.name-form: initials` +
`disambiguate.add-givenname: true`:

```
Two references: [..., ("Bronchitis", "Brandon"), ...] vs [..., ("Bronchitis", "Biff"), ...]
Rendered: "A Asthma, B Bronchitis, et al." for BOTH -- identical, no
disambiguator applied, no fallback to year-suffix.
```

Control (differing initials, e.g. Brandon vs Zach) resolves correctly
("B Bronchitis" vs "Z Bronchitis"), confirming the mechanism generally
works -- the bug is specific to same-initial collisions.

## Root cause
`check_givenname_resolution`/`append_givenname_resolution_key`
(processor/disambiguation.rs) build the collision-resolution key from the
raw full given-name string (`name.given`) unconditionally, assuming that
"expanding given names" reveals the full name and will discriminate. But
rendering (`values/contributor/names.rs::format_single_name`) only escalates
`ContributorForm::Short -> Long` when `expand_given_names` is set; the given
name's actual granularity (initials vs full) still comes from
`ctx.name_form` (the style's configured baseline), which stays at Initials.
So the collision-detection key and the rendered output diverge: the key
says "resolved" (Brandon != Biff as raw strings), the render says
"unresolved" (B. == B.), and the cascade never proceeds to year-suffix.

## Verified against real CSL
Official CSL test-suite fixtures confirm real citeproc-js resolves this via
a level ladder (family-only -> initials [only when initialize-with is
configured] -> full given name), escalating only as far as needed:
- tests/csl-test-suite/processor-tests/humans/disambiguate_ByCiteMinimalGivennameExpandMinimalNames.txt
- tests/csl-test-suite/processor-tests/humans/disambiguate_ByCiteBaseNameCountOnFailureIfYearSuffixAvailable.txt
- tests/csl-test-suite/processor-tests/humans/disambiguate_ByCiteGivennameShortFormInitializeWith.txt (+ NoInitializeWith,
  NoShortFormInitializeWith siblings)

The `-with-initials` rule variants (all-names-with-initials,
primary-name-with-initials) cap the ladder at initials -- they must never
escalate to full given names (confirmed by rule doc comments: only these
two say "(initials form)" explicitly).

## Scope
- [x] Add a givenname-level ladder (Initials -> Full, capped per
      `GivennameRule`) to the collision-key builder and threading it
      through to ProcHints/rendering.
- [x] Fix: same-initial collisions correctly escalate to full given names
      (matching the oracle) rather than silently rendering identically.
- [x] Note: per-position minimality (only escalating the ONE position that
      actually needs it, not every shown position) is still deferred to
      csl26-5753/PR4 -- this bug's fix may render more given names than the
      oracle at other positions in the same citation. That divergence is
      intentional and documented, not a regression.

## Summary of Changes

Added a given-name escalation ladder (Initials -> Full, per csl26-h9jy) to `crates/citum-engine/src/processor/disambiguation.rs`: `check_givenname_resolution`/`append_givenname_resolution_key` now build the collision-resolution key using the SAME representation the renderer would actually produce at each level (via `initialize_given_name`, made `pub(crate)`), instead of always assuming the raw full given-name string discriminates. `resolve_givenname_level` tries initials first (only when `initialize-with` is configured, mirroring real CSL's presence gate), then escalates to full given name -- capped for `all-names-with-initials`/`primary-name-with-initials`, which must never escalate past initials (new `DisambiguationFlags.givenname_full_allowed`).

`ProcHints` gained `expand_given_names_full: bool` alongside the existing `expand_given_names`. Rendering (`values/contributor/names.rs::resolve_given_part`) applies the escalated level, with one critical correction found via full-corpus verification: escalation only ever RAISES the given-name form (FamilyOnly < Initials < Full) relative to the rendering context's OWN configured baseline, never lowers it. Hints are computed once against citation-scope config and shared with bibliography rendering, which commonly configures a MORE revealing baseline (e.g. Chicago author-date: citation-scope `initials`, bibliography-scope default `full`) -- without the floor, escalating to "Initials" wrongly downgraded already-correct full-name bibliography entries. Caught by the sandboxed full-corpus sweep (chicago-18-base exactParity dropped 174->168/546), fixed, and reverified byte-identical (0 regressions across all 35 exemplar styles, before/after via detached worktree).

Added 3 native regressions: same-initial escalation to full name (mirrors the official CSL test suite's `disambiguate_ByCiteMinimalGivennameExpandMinimalNames.txt` fixture exactly, with the known/annotated PR3-vs-oracle divergence -- Citum reveals every shown position uniformly, not just the minimal one needed, which is csl26-5753's remaining scope); the `-with-initials` rule cap falling through to year-suffix; and re-expected an existing test (`disambiguation_initials_are_used_when_short_form_family_names_collide`) whose old assertion had encoded the bug ("T Smith, (2000); T Smith, (2000)" for Thomas/Ted Smith) as correct -- now matches the official `disambiguate_ByCiteGivennameShortFormInitializeWith.txt` fixture ("Thomas Smith; Ted Smith").

Verification: `just pre-commit` (2587 tests, fmt+clippy clean), full `cargo nextest -p citum-engine` (1329 tests), targeted report-core.js on all 8 styles that could reach `disambiguate-add-givenname` (apa-7th, elsevier-harvard, chicago-18-base, elsevier-vancouver, gb-t-7714-2025, harvard-cite-them-right, modern-language-association, taylor-and-francis-cse) -- all byte-identical exactParity/compat before/after, and the full sandboxed corpus sweep (`report-core.js --all-features`, systemd-run MemoryMax=6G) -- 0 regressions across 35 exemplar styles.
