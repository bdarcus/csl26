---
# csl26-7e6l
title: Embedded exact-parity sweep, wave 2
status: completed
type: task
priority: high
tags:
    - style
    - fidelity
    - styles
created_at: 2026-09-06T15:27:17Z
updated_at: 2026-09-06T16:18:09Z
parent: csl26-ccdt
---

Second wave of embedded-tier non-Chicago exact-parity fixes, follow-on to csl26-on47 (completed, PR #1263). Targets APA-7th (untouched by on47, 783 dependents), gb-t-7714-2025-author-date (lowest non-Chicago embedded fidelity, 0.881), MLA/IEEE second-pass residuals, elsevier-with-titles, and tail styles as budget allows. Plan: /home/bruce/.claude/plans/let-s-do-another-comprehensive-sunny-wombat.md

## Todo
- [x] Backfill tags on csl26-on47/PR 1263 follow-ups (csl26-lm0z, csl26-mp44, csl26-zs9y, csl26-qkvz were filed untagged)
- [x] apa-7th (93/146 -> 106/146, +13, zero regressions; filed csl26-873v, csl26-7652, csl26-wt9e)
- [x] gb-t-7714-2025-author-date: investigated, root cause is engine bug csl26-873v (legal-type author suppression), not style-fixable; deferred
- [x] modern-language-association second pass (55/115 -> 62/115, +7, zero regressions)
- [x] ieee second pass: investigated, remaining sole-cause N rows blocked on csl26-zs9y/csl26-fz2e; one publisher-prefix attempt (entry-encyclopedia) regressed 2 rows for +1, reverted. csl26-lhrl/csl26-y49d untouched, still open
- [x] elsevier-with-titles (50/67 -> 51/67, +1, zero regressions; type-specific date-parens exception left as residual)
- [x] Tails: skipped this wave, time-boxed after elsevier-with-titles per plan
- [x] Regenerate embedded-parity-baseline.json + docs/compat.html
- [x] Tag every new follow-up bean at creation

## Summary of Changes

Embedded-tier non-Chicago sweep, wave 2. Portfolio: 1775/3242 -> 1796/3242
(+21 rows), zero regressions across the whole embedded tier (verified via
check-core-quality.js --parity-baseline).

- apa-7th: 93/146 -> 106/146 (+13). Six root causes: suffix carried by an
  optional trailing component instead of the enclosing group (article-newspaper,
  interview, post/webpage title); lowercase "eds." from a missing
  text-case override on four editor/container-author labels, plus the
  editor-substituted-into-author-slot case (options.substitute.contributor-role-case);
  missing archive-place component in the base template.
- modern-language-association: 55/115 -> 62/115 (+7). MLA's works-cited
  container-chain (translator/editor/container-title/volume/publisher/date)
  needed to be one comma-joined group with a single period prefix separating
  it from Title, not a flat list of individually-prefixed components.
- elsevier-with-titles: 50/67 -> 51/67 (+1). Fallback (no-type-variant)
  template's terminal date needed parenthesizing + a nested space-joined
  group with the URL/accessed-date, for the several types with no
  dedicated type-variant.
- gb-t-7714-2025-author-date, ieee: investigated, no safe style-YAML fix
  found this pass (see follow-ups below). IEEE's one attempted fix
  (entry-encyclopedia publisher prefix) regressed 2 rows for +1 gain and
  was reverted.

## Follow-ups filed (need Rust/schema/architecture work, not a style edit)

- `csl26-873v` — GB/T (and likely any author-date style): legal-type
  references (bill/legislation/regulation/hearing) suppress the author
  contributor slot entirely, bypassing `substitute.otherwise` -- an
  engine bug, not a style config gap. This is 27/28 of gb-t-7714-2025-author-date's
  residual, the lowest-fidelity non-Chicago embedded style.
- `csl26-7652` — No schema primitive lets template delimiter/label choice
  condition on locator kind (page/line/timestamp vs everything else).
  Blocks 10 MLA citation rows (the single largest untapped cluster found
  this wave) and 1 APA row.
- `csl26-wt9e` — Software references drop `genre` entirely in schema-data
  conversion (struct has no field for it); map's genre field is present
  but still doesn't reach the render in one case, needs separate tracing.

## PR

https://github.com/citum/citum-core/pull/1264
