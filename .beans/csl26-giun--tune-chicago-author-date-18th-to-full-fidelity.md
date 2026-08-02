---
# csl26-giun
title: Tune chicago-author-date-18th to full fidelity
status: in-progress
type: task
priority: high
created_at: 2026-06-30T18:46:08Z
updated_at: 2026-08-02T14:19:30Z
parent: csl26-h7oc
blocked_by:
    - csl26-t0m4
    - csl26-4q7v
---

Tune `chicago-author-date-18th` to 100% fidelity + clean SQI via the
`style-tune` skill, against the shared Chicago corpus
(`chicago-18th-citations.json`, 15 items; `chicago-18th.json`, 402 refs).

## Baseline (measured 2026-06-30)
- citations: 11/15 (73%)
- bibliography: 298/402 (74%)
- gated via `chicago-shared-corpus`, `min_pass_rate: 0.73` (combined
  citation+bibliography rate; csl26-h7oc)

## Input contract (style-tune)
- Embedded style ID: `chicago-author-date-18th`
- Legacy CSL: `styles-legacy/chicago-author-date.csl`
- Citum YAML: `crates/citum-schema-style/embedded/styles/chicago-author-date-18th.yaml`
- Authority: CMOS 18 author-date system
- Extends: `chicago-18-base` (csl26-zs0f) — base options inherited, do not
  re-litigate base-level rules here

## Why first
Tuned first because `taylor-and-francis-chicago-author-date-core` extends
this style: fidelity gains here lift T&F's baseline before its Style-F deltas
are layered on.

## Todo
- [ ] Fidelity loop: oracle → classify failures → smallest correct YAML fix →
      re-run, until 100% on the shared corpus (citation + bibliography)
- [ ] SQI loop: `report-core` → hoist/preset/prune → re-check, until clean
- [ ] style-qa-reviewer handoff (tier: embedded-core)
- [ ] Confirm no regression on `references-expanded.json` / `core` fixture sets

- 2026-06-30 tune attempt: `webpage` citation variant added to render publisher/year for publisher-owned web pages. `node scripts/report-core.js --style chicago-author-date-18th` now reports headline fidelity 0.808, SQI 0.975, and `chicago-shared-corpus` 12/15 citations + 298/402 bibliography. Residual citation failures: `chi-article-magazine` (`Gourmet 2000b` vs `(2000)`), `chi-multi-source` classic book (`Augustine, De civitate Dei` vs `Augustine 1931`), and `chi-personal-communication` (`Johnson to O’Laughlin 1916` vs `Hiram Johnson`). Invalid/failed probes reverted: citation substitute keys `publisher`/`parent-serial` are schema-invalid; article-magazine `title: parent-serial` duplicates `Gourmet`; raw `container-title` is invalid in citation type-variants; personal_communication sender-recipient-year variant regresses `secondary-roles` recipient citation. Fidelity not green; SQI/QA/PR intentionally not run.
- 2026-06-30 recovery attempt on `codex/chicago-author-date-full-fidelity`: citation fidelity is now green for `chicago-shared-corpus` (`15/15`). Implemented bounded support for `parent-serial` substitutes, field-presence/absence `render-when` groups, classic conversion from CSL/Zotero `type: classic`, collection-editor substitution, custom contributor-role rendering, and audio-visual event/dimensions preservation. Bibliography improved from `298/402` to `331/400` in the shared oracle run; `node scripts/report-core.js --style chicago-author-date-18th` reports headline fidelity `0.868` and SQI `0.927`. Remaining bibliography failures are broad rich-reference clusters (book original/reprint/rich contributors, interviews, manuscripts, web pages, recordings/media, broadcasts, and manuscript/archive edge cases), so the original 100% + clean SQI + QA completion gate is not met. This draft PR is a substrate/progress PR, not a completed full-fidelity tune.

- 2026-07-02 bounded cluster-lift PR (`fix/chicago-author-date-bib-clusters`):
  shared-corpus bibliography 331/400 -> 344/400 (citations stayed 15/15).
  Root-caused and fixed four clusters:
  - **Title casing**: the engine's legacy type->title-category fallback only
    recognized a small hardcoded set (component: article/chapter/entry;
    monograph: book/thesis/report); `broadcast`, `manuscript`, `motion-picture`,
    `song`, `webpage`, and the note-field-routed `collection` type fell through
    with no case transform. Fixed via a style-local `titles.type-mapping`
    (YAML) rather than widening the shared engine default (would affect every
    embedded style). Also found and fixed three Rust engine bugs surfaced by
    this cluster: (1) the CSL `substitute` chain (title standing in for a
    missing author) bypassed all text-case resolution entirely — added
    `resolve_substitute_text_case` (`citum-engine/src/values/title.rs`) and
    wired it into `resolve_title_substitute`
    (`citum-engine/src/values/contributor/substitute.rs`); (2) `to_title_case`
    hunted past leading digits to capitalize the next letter ("35 mm film" ->
    "35 Mm film") — `capitalize_first_word` now stops at a leading digit
    (`citum-engine/src/values/text_case.rs`); (3) title-case hyphenated-compound
    handling didn't recognize en dash as a hyphen substitute ("Aging-Disability"
    stayed uncapitalized on the second half) — added en-dash awareness to
    `capitalize_hyphenated`; (4) the bibliography sentence-initial
    auto-capitalization pass wrongly capitalized a contributor's literal name
    ("the author" -> "The author") when the component had a static YAML
    `prefix:` instead of a dynamic role-label prefix
    (`citum-engine/src/processor/rendering/grouped/sentence_initial.rs`).
  - **Original/reprint dates**: fixed the missing period before the
    parenthesized original year (`Fitzgerald, F. Scott (1925) 1992` ->
    `Fitzgerald, F. Scott. (1925) 1992.`) and the missing "Translated by"
    verb-label — `role.omit` was suppressing the verb-form label entirely
    instead of only the decorative default/preset label
    (`citum-engine/src/values/contributor/labels.rs`), plus wiring
    `text-case: capitalize-first` into `format_role_term`
    (`citum-engine/src/values/contributor/mod.rs`) since verb-form output is
    pre-formatted and skips the generic case pass. **Deferred**: the
    "Reprint"/"Originally published as X (publisher)" trailer text and
    `original-publisher`/`original-publisher-place` wiring — blocked on
    `TemplateConditionField` (a closed enum) not exposing those fields for
    `render-when`, plus each fixture item's free-text `edition` field has
    bespoke, non-systematic oracle capitalization that doesn't reduce to a
    simple rule. Deeper than "mostly YAML wiring"; needs its own pass.
  - **Manuscript/archive block**: the 7 manuscript-type refs in this corpus
    are tagged via a note-field `type: collection` override (routes to
    `ref_type() == "collection"` per the CSL type conversion contract), but
    the style had no `collection` type-variant at all, so they fell to the
    generic article-shaped default template. Merged `manuscript, collection`
    into one type-variant key (both need the same archival rendering:
    author/title substitute + archive-location/archive-collection/archive-name
    fields). **Deferred**: the CSL-`document`-routed archival refs (Purcell
    map, Agassiz, Henshaw, Johnson, Concerning-a-court-of-arbitration — 5
    refs) needed the same treatment, but `document` is also used by ~30
    unrelated annotation/placeholder items in this corpus; adding `document`
    to the merged type-variant dropped the oracle's total entry count from
    400 to 397 (some of those placeholder items stopped rendering
    distinguishably), so it was reverted as a real regression rather than a
    net win. Date ranges (`issued: 1790/1803` rendering as bare `1790`) and
    uncertain dates (`[1772?]` rendering as `n.d.`) are unresolved — likely
    engine-side date-form gaps, not investigated further in this PR.
  - **Contributor label punctuation**: `contributor-role-form: short` mapped
    to `RoleLabelPreset::ShortSuffix` (parenthetical: `Lattimore (eds.)`);
    CMOS wants the comma form. Switched to `short-comma` ->
    `RoleLabelPreset::ShortSuffixComma` (`Lattimore, eds.`).
  Verification: `just pre-commit` (fmt/clippy/nextest, 1707 tests) green;
  `node scripts/report-core.js` full run (154 embedded styles) against
  `core-quality-baseline.json` green (fidelity=1.0 for all, 0 warnings) —
  no regressions from the shared-engine changes. chicago-author-date-18th
  fidelity 0.875 -> 0.898, SQI 0.926 -> 0.925 (marginal miss of the stated
  0.926 floor: the `manuscript, collection` merge is scored as 2
  "variant selectors" by `report-core.js`'s sprawl-penalty concision metric,
  a ~0.001 mechanical cost for a functionally-correct DRY fix, not a
  maintainability regression — flagged for the reviewer rather than reverted
  the fix that lifted 6 bibliography refs). taylor-and-francis-chicago-author-date
  inherits identically (fidelity 0.875 -> 0.898, quality unaffected at 1.0).
  chicago-notes-18th and chicago-shortened-notes-bibliography unchanged (no
  regression). Ratcheted `chicago-shared-corpus.min_pass_rate` for both
  chicago-author-date-18th and taylor-and-francis-chicago-author-date from
  0.73 to 0.86 (combined rate 359/415 = 0.865, floor set just below) in
  `scripts/report-data/verification-policy.yaml`.
  **Target not fully met**: bibliography landed at 344/400, short of the
  ≥360/400 target — the two deferred pieces above (original/reprint trailer
  text, `document`-type archival refs) account for most of the gap and were
  each classified as genuinely deeper than "mostly YAML wiring" per the
  bounded-PR scope rule, rather than absorbed here.
  **Explicitly out of scope** (deferred structural clusters, unchanged from
  the prior note): broadcast/episode grammar (`Episode 6, "…," written
  by…`), hearings/legal, multi-volume chains (`Bk. 3 of…, vol. 2 of…`),
  review-of framing, patents (missing "filed" date / country prefix),
  dataset versioning (`Version 1.2. With … et al`), personal_communication
  edge cases (text/Facebook/email message genres), and the ~8-entry
  index-misaligned tail (classic/legal-citation placeholder items the
  oracle doesn't render at all). Bean stays in-progress; 100% + clean-SQI
  gate not met.

## 2026-07-30: oracle text-parity residuals (new class evidence, from citum-core#embedded-parity report at fb0a01af)

Full-portfolio oracle-parity clustering (not shared-corpus, the broader references-expanded run) surfaces residual classes on chicago-author-date-18th (byte-identical on taylor-and-francis-chicago-author-date, so any fix here pays twice — see csl26-gzwj):

- **D: title quoting differs (111 of this style's mismatches).** Oracle quotes titles/chapter-titles with curly quotes on types this style doesn't (e.g. archival letters, dictionary entries, newspaper articles): `O: Adams, John. 1797. \"Letter to James Smith.\" Adams Papers, reel 45.` vs `C: Adams, John. 1797. Letter to James Smith. Adams Papers, reel 45.` Also the inverse — Citum quotes a map/dataset the oracle doesn't: `O: Cable, D. 2013. The Racial Dot Map. Map. …` vs `C: Cable, D. 2013. \"The racial dot map.\" …` — suggests a type-variant boundary issue, not a uniform quoting rule.
- **C: stray period after container-title, before volume/issue locator (68 occurrences).** `O: … Urban Climate 15: 1–18.` vs `C: … Urban Climate. 15: 1–18` — container-title is getting a terminal period before the volume:issue:page block where CMOS wants none.
- **B: name-list conjunction dropped between two authors (subset of 62 punctuation-only diffs).** `O: Smith, Jane, and Robert Williams. 2020.` vs `C: Smith, Jane and Robert Williams. 2020.` — missing comma before \"and\" in a 2-author list.

Not independently investigated/fixed — recorded as residual-defect input for this bean's ongoing fidelity loop. Full clustering data: /tmp/prd_report.json (stale path, regenerate via `node scripts/report-core.js --styles chicago-author-date-18th --parallelism 2`). See [[csl26-arly]] for the harness-side finding that class A (bib-number prefix) is a separate, likely-unrelated measurement artifact — do not chase it from this bean.



## 2026-07-30: exact-parity punctuation pass

- Prioritized `exactParity` for milestone `csl26-w0hf` rather than the behavioral fidelity score.
- Corrected Chicago bibliography serial commas, grouped journal container/volume/page/identifier punctuation, and quoted archival manuscript/collection titles.
- Fresh `report-core` evidence: exact parity improved from **105/540 (19.4%)** to **156/540 (28.9%)**, a gain of 51 byte-identical observations.
- The behavioral fidelity score is diagnostic for this pass and remains below the embedded-style tuning gate; exact parity is still incomplete, so the task remains in progress.
- Remaining exact clusters include rich reference-type templates (legal, media, web, and document/archive variants) and author-date citation formatting.



- Follow-up exact-parity pass: enabled `entry-suffix-after-url` and `entry-suffix-after-doi` for Chicago bibliography output. Fresh `report-core` exact parity improved **156/540 -> 165/540** (+9) with SQI unchanged at 0.920. The remaining 133 URL/DOI-ending mismatches also have structural differences, so this global option is exhausted; next cluster is type-specific quote/template selection.



- 2026-07-31: extended the terminal-link policy to the other active Chicago bibliography head, `chicago-shortened-notes-bibliography-core` (`entry-suffix-after-url: true`, `entry-suffix-after-doi: true`). Author-date already carries both flags, and its dependent T&F variant retains the verified **165/540** exact-parity result. The isolated author-date report rerun hit a snapshot-oracle exit-2 infrastructure failure before producing a new comparison; this change does not modify author-date YAML. [[csl26-2nv1]] tracks the separate inheritance/engine feature required to replace Chicago notes’ `bibliography.template: []` sentinel with an explicit disabled bibliography.

2026-07-31 progress: fixed a shared engine title-case defect for Chicago's interior stop words and hyphenated compounds (including with, during, along, between, and Text-to-Speech). On a clean isolated Cargo target, shared-corpus bibliography improved 334/379 -> 339/379 with citations steady at 15/15; embedded report fidelity improved 0.917 -> 0.926, exact parity 165/540 -> 170/540, and SQI stayed 0.920. The inherited Taylor & Francis Chicago style matched the improvement with no regression. Full fidelity, clean-SQI completion, and final QA closure remain open; this is a bounded processor-defect progress pass.



2026-07-31 follow-up: reclassified the stale container-period cluster against current evidence and fixed the live paper-conference fallback template instead. Pages now use a colon when volume/issue is present and a comma when both are absent. Added a focused paper-conference fixture and bibliography regression test. Fresh selected report: exact parity improved 170/540 -> 172/540; embedded fidelity stayed 0.926 with citations 63/63 and bibliography 437/477; shared Chicago corpus stayed 15/15 citations and 339/379 bibliography; inherited Taylor & Francis matched 172/540 with no regression; SQI stayed 0.920. The escalated authoritative gate passed format, clippy, and 2,304 nextest tests. Full fidelity, clean SQI, and final QA remain open.
