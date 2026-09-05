---
# csl26-on47
title: Non-Chicago embedded exact-parity sweep
status: completed
type: task
priority: high
created_at: 2026-09-05T20:27:49Z
updated_at: 2026-09-05T21:56:34Z
parent: csl26-ccdt
---

One PR fixing the 12 non-Chicago, non-GB/T embedded-core styles' exact-parity
residuals. These styles carry 426 failing rows (587/1013, 58%) of which 362
(85%) are single-label defects (per scripts/analyze-parity-residuals.js) --
the opposite shape from Chicago's entangled residual. Order by sole-cause
descending: T&F-NLM(50) > MLA(46) > IEEE(44) > APA(41) > AMA(33) >
T&F-CSE(31) > elsevier-with-titles(29) > springer-vancouver-brackets(22) >
elsevier-harvard(21) = elsevier-vancouver(21) > springer-basic-author-date(12)
= springer-basic-brackets(12).

Fix in `-core` parent YAML where one exists. Two cross-style residuals get an
engine check first: (1) AMA/elsevier-vancouver citation-group affix
placement (`[1],see also [2]` vs oracle `[1,see also 2]`), (2) IEEE stray `:`
after empty component (`Eds.:`, `vol. 5:`).

Picks up existing epic children where they overlap: csl26-x8hb, csl26-y49d,
csl26-lhrl, csl26-fz2e (IEEE). Out of scope: Chicago family (csl26-h7oc),
GB/T family, genre-slug fixture repair (references-expanded.json, 45 rows/16
styles -- separate PR, own baseline regen), schema-touching residuals,
Z-unclassified label taxonomy (csl26-ww77 already covers investigating that
bucket).

Plan: /home/bruce/.claude/plans/the-compat-html-report-shows-hidden-ullman.md

## Todo
- [x] T&F-NLM (13/67 -> 27/67, +14, zero regressions; remainder needs online-source template work, filed as follow-up)
- [x] MLA (44/115 -> 55/115, +11, zero regressions; citation-disambiguation engine bug filed as follow-up)
- [x] IEEE (97/149 -> 105/149, +8, zero regressions)
- [ ] APA-7th (93/146 -> target ~125/146)
- [x] AMA (34/67 -> 34/67; bracket->superscript fix is CSL-correct but 0 net rows, superscript-vs-oracle-plain-text mismatch filed as follow-up)
- [x] T&F-CSE (32/67 -> 46/67, +14, zero regressions)
- [x] elsevier-with-titles (35/67 -> 50/67, +15, zero regressions)
- [x] springer-vancouver-brackets (40/67 -> 49/67, +9, zero regressions)
- [x] elsevier-harvard (45/67 -> 51/67, +6, zero regressions; date-position engine interaction filed as follow-up)
- [x] elsevier-vancouver (46/67 -> 55/67, +9, zero regressions)
- [x] springer-basic-author-date (54/67 -> 55/67, +1, zero regressions)
- [x] springer-basic-brackets (54/67 -> 55/67, +1, zero regressions)
- [x] Engine check: AMA/elsevier-vancouver citation-group affix placement -- was YAML (label-wrap vs whole-citation wrap), not engine; fixed in the numeric-marker-wrap commit
- [x] Engine check: IEEE stray colon after empty component -- was YAML (hardcoded prefix instead of group+delimiter), not engine; fixed in the chapter colon commit
- [x] Regenerate embedded-parity-baseline.json + docs/compat.html
- [x] Filed follow-ups: csl26-lm0z (MLA disambiguation, engine), csl26-mp44 (oracle plain-text comparison, architecture/policy), csl26-zs9y (URL-conditional template primitive, schema), csl26-qkvz (date-position scrambling, engine), appended to csl26-fz2e (springer et-al locale override)

## Summary of Changes

Portfolio: 1687/3242 (52.0%) -> 1775/3242 (54.8%), +88 rows, zero regressions across the whole embedded tier (verified via check-core-quality.js --parity-baseline).

10 commits, each independently verified with a before/after oracle diff showing 0 newly-failing rows:
- T&F-NLM: double-bracketed citations (redundant label-wrap on top of citation wrap) + locator delimiter collision (+14)
- MLA: missing entry-suffix-after-doi + uncapitalized translator label (+10), then title-case on primary titles (+1)
- IEEE: leaked colon in chapter publisher (+2), then publisher order + date form fixed properly per ieee.csl's real branching, with book given its own type-variant to preserve its original order (+6)
- AMA/elsevier-vancouver/springer-vancouver-brackets: wrong marker-wrap mechanism (label-wrap vs whole-citation wrap) (+0/+9/+9)
- T&F-CSE: entirely missing entry-suffix (+14)
- Springer (both): entry-encyclopedia container title used the wrong title-type (parent-serial instead of parent-monograph) (+1 each)
- elsevier-with-titles: article-journal wrongly wrapped its container in an "in: " (book-chapter) message pattern, plus a stray unconditional publisher field (+15)
- elsevier-harvard: redundant date-position directive scrambled entries with a leading empty contributor (+6)

Every fix traced to a concrete root cause (verified against the style's own legacy CSL source or a standalone CLI render) before landing -- no speculative pattern-matching fixes.

5 follow-ups filed for findings that need Rust/schema/architecture work rather than a style edit: csl26-lm0z (MLA citation-disambiguation engine bug), csl26-mp44 (oracle plain-text-vs-superscript comparison policy), csl26-zs9y (missing URL-presence template condition, blocks T&F-NLM/IEEE online-source markers), csl26-qkvz (date-position engine interaction), and an addition to csl26-fz2e (springer et-al locale override, same shape as IEEE's month-abbreviation gap).
