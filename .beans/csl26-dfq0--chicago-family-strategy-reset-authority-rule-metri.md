---
# csl26-dfq0
title: 'Chicago family strategy reset: authority rule, metric re-framing, localization integrity'
status: in-progress
type: feature
priority: high
created_at: 2026-08-07T13:15:34Z
updated_at: 2026-08-07T15:51:33Z
parent: csl26-40n4
---

Reframes the Chicago-family effort per the plan developed in an
interactive session on 2026-08-07 (local plan-mode artifact, not
committed — see docs/specs/CHICAGO_FAMILY_STRATEGY.md for the durable
record and PR #1151 for the implementation).

Origin: reviewing chicago-author-date-18th and
taylor-and-francis-chicago-author-date-core (FIX/TODO comments on the branch)
surfaced that localization is adopted inconsistently within the same family —
chicago-notes-18th uses 17 locale `message:` refs, chicago-author-date-18th
hardcodes 27 literal English strings, T&F-core hardcodes 17 with zero message
refs — and that no existing metric (fidelity or exact parity) can see this,
because the oracle is English-only. Also established: fidelity
(report-core.js `computeFidelityScore`) is strictly weaker than exact parity
and already non-binding for this family; the per-entry tuning loop in
csl26-giun has deferred the same structural clusters across ~6 sessions.

## Todo
- [x] Commit 1 (docs): strategy doc + localization policy + csl26-h7oc
      restructured to cluster beans + csl26-giun/csl26-7jht scrapped with
      pointers (evidence preserved)
- [x] Commit 2 (scripts): localization-integrity detector implemented as
      STYLE010 in scripts/style-structure-lint.js (NOT wired into
      report-core.js — docs/reference/SQI.md documents style-structure-lint.js
      as the separate deterministic style-shape linter, distinct from the
      oracle-driven fidelity/SQI pipeline; better fit than the original plan's
      report-core.js wiring). Report-only, not in FATAL_RULE_IDS. Tests added
      in style-structure-lint.test.js (24/24 passing). Verified against the
      Chicago family: 14 hits in chicago-author-date-18th.yaml, 3 in
      taylor-and-francis-chicago-author-date-core.yaml. Portfolio-wide dry run
      (embedded + in-repo styles): 92 total hits, non-fatal, no gate broken.
      Metric re-framing done: report-core.js HTML explainer + table
      (Oracle Text column now before Fidelity, relabeled from
      'Compatibility') + docs/reference/SQI.md priority order rewritten
      (exact parity primary, fidelity a tripwire, corrected the doc's stale
      description of what check-core-quality.js actually gates).
- [x] Commit 3 (styles): 3 locale additions to en-US.yaml
      (chicago-recorded-date/chicago-released-date MF2 patterns,
      term.track-label-long MF2 plural term — illustrator/narrator/performer
      verbs turned out to already exist, found via a wider grep window) +
      converted all 36 hardcoded role-label sites across both files to locale
      messages/verb-forms, oracle-checked (0 entries changed against either
      style's baseline) + wrap:parentheses cleanup (3 sites) + resolved the
      non-defect FIX/TODO comments + fixed a real pre-existing 'Released.
      Released.' duplication bug found via oracle diff + fixed a real Rust
      ContributorRole::Narrator gap (see csl26-ey4f summary)
- [x] Non-regression check: check-core-quality.js against the 19-style
      embedded-core baseline (full portfolio, --all-features) — gate passed,
      0 exact-parity regressions, only a pre-existing unrelated ieee
      preset-usage warning (ieee.yaml untouched by this stack). Narrower than
      the originally-planned full 32+141 render diff, but covers every
      exact-parity-gated style plus fidelity==1.0 for all 35 core styles;
      judged sufficient given the Rust change's blast radius is proven nil
      (only chicago-author-date-18th references contributor: narrator
      anywhere in the portfolio).
- [x] Multilingual proof: `citum render refs -L fr-FR`/`-L de-DE` —
      narrator "Narrated by" -> "Lu par"; translator "Translated by" ->
      "Übersetzt von" (de-DE) / "Traduit par" (fr-FR, T&F)
- [x] gh stack init + submit --auto: single PR instead (no real stacking
      need — 3 commits land together). PR #1151. Commit 3 later split
      into fix(engine) + fix(styles) per review feedback (the engine
      fix stands alone and shouldn't be bundled with the style content
      it enables).

## What this PR actually moves (per review question 2026-08-07)

- **Exact parity: unchanged, by design.** author-date and T&F both stay
  at 172/546, 0 entries changed in either direction (verified
  entry-by-entry, not just aggregate). The converted text was already
  oracle-correct in English; a pure locale-source swap of already-
  correct English text cannot move an English-only oracle's pass/fail
  count. Parity movement is cluster 2-7's job, not done in this PR.
- **Inheritance: untouched, deliberately.** chicago-18-base.yaml has
  zero diff. This PR doesn't touch the shared base or either style's
  extends chain — Section A/B of the 2026-06-30 audit already covers
  what's safely shareable there, and nothing in this cluster needed
  more.
- **What did move, measured directly (STYLE010 hit count, the actual
  before/after for this cluster):** author-date 25 -> 1 hardcoded
  sites, T&F-core 12 -> 1. Locale message: pattern.* uses: author-date
  8 -> 24, T&F-core 0 -> 4 (undercounts — form: verb conversions don't
  show up as message: calls). This is the metric that answers whether
  localization coverage improved, since exact parity structurally
  cannot show it.
- **Two incidental bugs fixed:** ContributorRole::Narrator (engine,
  now split into its own commit) and a real 'Released. Released.'
  duplication in the software type-variant, found via oracle
  comparison — the latter does NOT flip that entry to a parity match
  (it has a separate, untouched author/title ordering defect), so it
  doesn't appear in the exact-parity numbers despite being a real fix.

Related, filed separately: csl26-629e (DOI/URL literal-prefix migration onto
existing `links:` config — out of scope for this bean's commits).
