---
# csl26-h7oc
title: Drive all Chicago variants to full fidelity
status: in-progress
type: epic
priority: high
created_at: 2026-06-30T14:30:24Z
updated_at: 2026-08-23T20:41:23Z
parent: csl26-40n4
---

Coordinator for driving all four Chicago variants from their real baseline to
~100% fidelity + clean SQI, via the `style-tune` skill, one variant per child
bean. Supersedes the prior "final tuning pass" framing, which understated the
gap — baseline below was unchanged-codebase, freshly measured.

## Baseline (measured 2026-06-30, `node scripts/report-core.js`, `chicago-shared-corpus` run — 15 citations / 402 bibliography refs)

| variant | citations | bibliography | gated in CI? |
|---|---|---|---|
| chicago-author-date-18th | 11/15 (73%) | 298/402 (74%) | yes — `chicago-shared-corpus`, combined rate 309/417 = 0.741, `min_pass_rate: 0.73` |
| chicago-notes-18th | 7/15 (47%) | — (no bibliography surface) | yes — `chicago-shared-corpus`, citation-only rate 0.467, `min_pass_rate: 0.46` |
| chicago-shortened-notes-bibliography | 6/15 (40%) | 264/402 (66%) | yes — `chicago-shared-corpus`, combined rate 270/417 = 0.647, `min_pass_rate: 0.64` |
| taylor-and-francis-chicago-author-date | 11/15 (73%) | 298/402 (74%, inherits author-date) | yes — `chicago-shared-corpus`, combined rate 309/417 = 0.741, `min_pass_rate: 0.73` |

## Gating policy — implemented (this PR)

`chicago-shared-corpus` is now `count_toward_fidelity: true` on all four
variants in `scripts/report-data/verification-policy.yaml`, with interim
`min_pass_rate` floors set just below the 2026-06-30 baseline above (the
*combined* citation+bibliography match rate that `report-core.js`'s
`determineBenchmarkStatus` actually computes, not a naive average of the two
surface percentages). Floors lock against regression during tuning; ratchet
upward per variant as each is tuned toward 100%.

`chicago-author-date-18th`'s old `chicago-zotero-bibliography` run (same
402-ref `chicago-18th.json` fixture) is demoted to diagnostic
(`count_toward_fidelity: false`) — leaving both runs `true` would have
double-counted that fixture into the headline `fidelityScore`, since
`report-core.js` additively merges every `count_toward_fidelity: true` run
for a style. This also retires the fragmentation flagged on #987.

Note: none of this is a hard CI merge-blocking gate today —
`scripts/check-core-quality.js`'s hard `fidelityScore === 1.0` check only
applies to styles listed in `scripts/report-data/core-quality-baseline.json`,
which does not include any of the four Chicago variants. The `min_pass_rate`
floors above are scored and shown as pass/fail in `report-core.js` output
(visibility/regression-tracking), not wired into a failing CI step. Wiring a
hard gate is a separate decision, not made here — it would currently be
unmeetable (fidelityScore == 1.0) and would block unrelated engine/migrate PRs.

## Tuning order (recommended, encoded in the child-bean `blocked_by` graph below — not separately ratified)

author-date → T&F → notes → shortened. T&F-core extends
`chicago-author-date-18th`, so author-date gains lift T&F before its Style-F
deltas apply. `chicago-shortened-notes-bibliography-core` extends
`chicago-notes-18th`, so notes is tuned before shortened inherits its citation
gains, leaving shortened to only need its own bibliography surface tuned.
Author-date/T&F (73-74%) are closest to the bar; notes (47%) and shortened
(40%) need the most work but inherit upstream wins.

## Todo (restructured 2026-08-07, see docs/specs/CHICAGO_FAMILY_STRATEGY.md)
- [x] Decide and implement the gating-policy recommendation above
- [x] Restructure from per-style tuning (csl26-giun, csl26-7jht — both
      scrapped, evidence preserved) to per-defect-cluster children, since the
      per-style loop repeatedly deferred the same structural defects across
      six-plus sessions instead of converging
- [x] csl26-ey4f: Cluster 1 — contributor-role/pattern localization
      (completed; landed in PR #1151, no exact-parity movement by design —
      see csl26-ey4f/csl26-dfq0 for the localization-coverage numbers that
      metric can't show)
- [x] csl26-87yl: Cluster 2 — title quoting boundary by source type
      (landed article-newspaper + thesis quoting fixes; author-date/T&F
      172/546 -> 173/546; notes/shortened unchanged; map + notes-family
      dataset/report/thesis/webpage deferred, see csl26-87yl summary)
- [ ] csl26-vf5x: Cluster 3 — container-title terminal punctuation
      (re-verified 2026-08-08: described pattern not found in current
      failures, likely stale — needs fresh re-derivation before starting)
- [ ] csl26-yqma: Cluster 4 — name-list conjunction punctuation
      (re-verified 2026-08-08: same — largely unconfirmable in current
      author-date/T&F data, needs fresh re-derivation)
- [ ] csl26-cz0p: Cluster 5 — archival/manuscript/document-routed refs
      (re-verified 2026-08-08: strongly confirmed — document 35/35,
      manuscript 13/15 failing, identical across author-date-18th/T&F)
- [ ] csl26-rpza: Cluster 6 — broadcast/episode grammar
      (re-verified 2026-08-08: strongly confirmed — broadcast 8/8,
      motion_picture 8/8 failing everywhere; speech/song show the same
      shape and aren't yet in scope)
- [ ] csl26-s2kt: Cluster 7 — multi-volume/legal/patents/original-reprint trailers
      (re-verified 2026-08-08: strongly confirmed — legal_case,
      legislation, treaty, regulation, hearing, standard all at or near
      100% failure across all three comparable styles)
- [x] csl26-zl7f: chicago-shortened-notes-bibliography-core's
      bibliography.options.separator was ", " instead of ". " —
      landed 2026-08-08. Measured impact: exactParity 13/473 -> 20/473
      (+7), not the ~326 the original signature-match estimate implied;
      most of those entries carry a second, independent defect that
      this fix alone doesn't clear. Still worth keeping — every entry's
      top-level delimiter is now correct regardless.
- [ ] Triage the 13 additional near-100%-failing reference types found
      2026-08-08 (interview, thesis, webpage, entry-encyclopedia,
      entry-dictionary, dataset, post, post-weblog, article, software,
      map, graphic, personal_communication — see gap note below) into
      new cluster bean(s), or fold into existing ones if they share a
      root cause
- [ ] Final `report-core.js` sweep confirming all four styles' exact parity
      moved upward from the 2026-08-07 baseline (author-date 172/546, T&F
      172/546, notes 22/72, shortened 13/473); ratchet `min_pass_rate` floors
      upward as each cluster clears


## Gap found while re-verifying clusters 3-7 (2026-08-08)

Cross-referenced every currently-failing exactParity bibliography entry (chicago-author-date-18th, taylor-and-francis-chicago-author-date, chicago-shortened-notes-bibliography; `node scripts/report-core.js`, references-expanded + chicago-18th corpora) against reference type. Clusters 3-7 (csl26-vf5x/yqma/cz0p/rpza/s2kt, updated with fresh counts) cover document, manuscript, broadcast, motion_picture, legal_case, legislation, treaty, regulation, hearing, bill, patent, standard, report, article-journal, article-magazine, article-newspaper. The following types are also failing at or near 100% across all three styles and aren't in any of the five clusters' scope:

| type | author-date-18th | T&F | shortened |
|---|---|---|---|
| interview | 11/11 | 11/11 | 9/9 |
| thesis | 6/6 | 6/6 | 5/5 |
| webpage | 15/16 | 15/16 | 15/15 |
| entry-encyclopedia | 7/7 | 7/7 | 6/6 |
| entry-dictionary | 4/4 | 4/4 | 2/3 |
| dataset | 4/6 | 4/6 | 5/5 |
| post | 6/6 | 6/6 | 6/6 |
| post-weblog | 3/3 | 3/3 | 3/3 |
| article | 3/3 | 3/3 | 3/3 |
| software | 5/5 | 5/5 | 4/4 |
| map | 5/5 | 5/5 | 4/4 |
| graphic | 4/4 | 4/4 | 4/4 |
| personal_communication | 6/7 | 6/7 | 0/0 |

Not proposing new cluster beans unilaterally -- this needs the same "is it one shared-template gap or several" triage the existing five clusters got before being split out. Flagging so the next planning pass treats the 5-cluster list as a subset of the remaining gap, not the whole of it.

Also: chicago-shortened-notes-bibliography had a single-cause defect independent of all of the above -- a wrong `bibliography.options.separator` value (comma instead of period), tracked and fixed as csl26-zl7f. 326 of its 422 failing bibliography entries matched that signature, but the fix's measured impact was smaller than that count implied (exactParity 13/473 -> 20/473, +7) -- most of the flagged entries carry a second, independent defect the separator fix alone doesn't clear. See csl26-zl7f for the full before/after and the note on why the signature count overstated the gain.

## Restructured by the 2026-08-23 leverage audit

docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md
computed the first exact-parity leverage table for this family (10
defect classes explain 69% of 1,148 failing rows, concentrated on
book/article-journal rather than the exotic tail) and diagnosed why
PR #1218 moved only +17 entries in a day: clusters were ranked by
narrative rather than measured count, worked one source-type at a
time, and the tune loop's ordering let a fidelity-saturated family
stop before exact parity moved.

**New children, in leverage order (fix in this order for max return):**
1. csl26-4xr6 -- title case not applied (304 rows)
2. csl26-jxco -- title quote boundary, all types at once (300 rows)
3. csl26-4ndr -- date detail (month/day) dropped (264 rows)
4. csl26-wtaq -- terminal punctuation/delimiter grammar (221 rows)
5. csl26-i7nz -- contributor role and ordering (143 rows)
6. csl26-x61x -- volume/issue/series grammar (136 rows)
7. csl26-rrsb -- year-suffix letter, engine layer (97 rows)
8. csl26-78ay -- URL/DOI, genre label, edition, in-container tail (~190 rows)

A wave that does not move parity across every style it touches is
reverted, not argued for (carried from docs/specs/CHICAGO_FAMILY_STRATEGY.md).
Report before/after per-type parity for book and article-journal
specifically, not just family totals.

**Disposition of prior clusters:**
- csl26-vf5x (cluster 3), csl26-yqma (cluster 4): scrapped, superseded
  by csl26-wtaq/csl26-i7nz/csl26-x61x with fresh, larger, verified counts.
- csl26-87yl (cluster 2): stays completed; noted that the audit found
  its defect is 3x larger family-wide than its own estimate. csl26-jxco
  picks up the deferred scope.
- csl26-cz0p (cluster 5, archival/document), csl26-rpza (cluster 6,
  broadcast/episode), csl26-s2kt (cluster 7, legal/multi-volume/patents):
  kept as-is, still type-scoped and data-confirmed. Re-verify their
  residual counts with node scripts/analyze-parity-residuals.js
  --by-type AFTER waves 1-4 land above -- several of their sampled rows
  (e.g. the CMOS-18 corpus-annotation document rows) fail on title-case
  and quoting, not type-specific grammar, so their true remaining scope
  is likely smaller than currently recorded.
- csl26-gukd: all acceptance criteria were already checked with a
  Summary of Changes recorded; marked completed (was left in-progress).

**Adjudication:** scripts/report-data/parity-adjudication.json now
carries one entry (genre-slug fixture artifact, state 'unclear' --
see the audit for why the CMOS-18 corpus-annotation rows were
considered and NOT adjudicated the same way). Any citum-correct
candidate stays a human call; none written by this pass.

**In-progress cap:** csl26-s2kt remains the only in-progress cluster
child; all eight new leverage-class beans are filed todo, one at a
time per docs/guides/STYLE_WORKFLOW_EXECUTION.md's tune loop.
