---
# csl26-l5oh
title: 'Bibliography reference-marker gap: springer/T&F-NLM numeric styles missing label-wrap/label-separator'
status: in-progress
type: bug
priority: normal
tags:
    - scorecard
    - styles
    - fidelity
    - regression
created_at: 2026-07-31T13:24:53Z
updated_at: 2026-08-13T11:12:23Z
parent: csl26-ccdt
---

Originally scoped to springer-vancouver-brackets only, discovered while regenerating scripts/report-data/embedded-parity-baseline.json at HEAD (940b461d) for csl26-6th8's exact-parity gate: passed count dropped from 28/67 (2026-07-30, 828cb9d2) to 20/67 (940b461d), fidelityScore unchanged at 1.0. Re-verified 2026-08-13 at 7ea608b9: the style has since fallen further, to 11/67 — and two sibling styles carry the identical, previously-untracked defect: `springer-basic-brackets` (20/67) and `taylor-and-francis-national-library-of-medicine` (0/67).

**Root cause (all three, confirmed via node scripts/report-core.js).** 100% of bibliography exact-parity failures in all three styles (47+47+47=141 rows) share one signature: the leading reference marker. citeproc-js emits `4. ` (springer pair) or `[4] ` (NLM), Citum emits a bare `4` glued to the entry body with no separator. Each style's `bibliography.options` sets `label-mode: numeric` but never sets `label-wrap`/`label-separator`, so the marker renders unwrapped and flush. `label_wrap`/`label_separator` are fully wired at `crates/citum-engine/src/processor/rendering/marker.rs:190-191` — this is a style-config gap, not an engine bug.

**Correct target values, derived from the shipped CSL source (not a judgement call):**
| Style | CSL marker | Target |
|---|---|---|
| `springer-basic-brackets` | `styles-legacy/springer-basic-brackets.csl:77` `suffix=". "` | `label-wrap: period`, `label-separator: " "` |
| `springer-vancouver-brackets` | `styles-legacy/springer-vancouver-brackets.csl:257` `suffix=". "` | `label-wrap: period`, `label-separator: " "` |
| `taylor-and-francis-national-library-of-medicine` | `styles-legacy/taylor-and-francis-national-library-of-medicine.csl:244` `prefix="[" suffix="] "` | `label-wrap: brackets`, `label-separator: " "` |

**General rule and corpus-wide check.** `label-separator` should equal the trailing whitespace of the CSL marker's `suffix`. Verified against every style `6595cf0b` ("feat(schema)!: own markers in the processor") converted to `label-mode`: `ieee`, `elsevier-with-titles`, `american-medical-association`, `american-medical-association-alphabetical`, `royal-society-of-chemistry`, `gb-t-7714-2025-numeric` (and CSL-source-less `alpha`, `numeric-comp`, `american-mathematical-society-label`) already have correct or absent config and are unaffected — these three are the complete affected set.

**Bisect answer (criterion 1) — two separate events, not one:**
- 28 → 20 (2026-07-30 → 07-31): the Chicago bibliography-link commits, as originally guessed (92fcfafe, 8a81ca58, 5b470906, 940b461d). Unrelated to this defect.
- 20 → 11 (springer-vancouver-brackets only, by 2026-08-13): `6595cf0b` (2026-08-04). The child YAMLs' bare `label-mode: numeric` predates that commit, but the commit replaced the old group-delimiter join with the new `label-separator` mechanism (its own spec, docs/specs/REFERENCE_MARKERS.md, documents this under Motivation: "Spacing became schema"). The three styles had been getting their `. `/`[ ]` from the bibliography-level component separator and lost it when marker joining moved to the dedicated option.

**Adjudication (criterion 2):** genuine defect, not an intentional divergence. No DIVERGENCE_REGISTER.md entry.

Fix: crates/citum-schema-style/embedded/styles/{springer-basic-brackets,springer-vancouver-brackets,taylor-and-francis-national-library-of-medicine}.yaml.

Acceptance criteria:
- [ ] Add label-wrap + label-separator per the table above to all three style YAMLs
- [ ] springer-vancouver-brackets-core.yaml titles: scientific (SentenceNlm) replaced with explicit as-is title case — the shipped CSL title macro has no text-case, so citeproc-js preserves title case; this core is the only user of the preset
- [ ] node scripts/report-core.js before/after confirms all three styles' exact-parity rises (target: springer-basic-brackets 53/67, taylor-and-francis-national-library-of-medicine 13/67, springer-vancouver-brackets ~40/67)
- [ ] cargo nextest run and just check-core-quality green
- [ ] Regenerate scripts/report-data/embedded-parity-baseline.json so the CI floor gate reflects the fix
