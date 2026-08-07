---
# csl26-h7oc
title: Drive all Chicago variants to full fidelity
status: in-progress
type: epic
priority: high
created_at: 2026-06-30T14:30:24Z
updated_at: 2026-08-07T15:24:49Z
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
- [ ] csl26-87yl: Cluster 2 — title quoting boundary by source type
- [ ] csl26-vf5x: Cluster 3 — container-title terminal punctuation
- [ ] csl26-yqma: Cluster 4 — name-list conjunction punctuation
- [ ] csl26-cz0p: Cluster 5 — archival/manuscript/document-routed refs
- [ ] csl26-rpza: Cluster 6 — broadcast/episode grammar
- [ ] csl26-s2kt: Cluster 7 — multi-volume/legal/patents/original-reprint trailers
- [ ] Final `report-core.js` sweep confirming all four styles' exact parity
      moved upward from the 2026-08-07 baseline (author-date 172/546, T&F
      172/546, notes 22/72, shortened 13/473); ratchet `min_pass_rate` floors
      upward as each cluster clears
