---
# csl26-l5oh
title: Investigate springer-vancouver-brackets exact-parity regression (28/67 -> 20/67)
status: todo
type: bug
priority: normal
tags:
    - scorecard
    - styles
    - fidelity
    - regression
created_at: 2026-07-31T13:24:53Z
updated_at: 2026-08-02T14:19:29Z
parent: csl26-ccdt
---

Discovered while regenerating scripts/report-data/embedded-parity-baseline.json at HEAD (940b461d) for csl26-6th8's exact-parity gate. The style's exact-parity passed count dropped from 28/67 (recorded 2026-07-30, commit 828cb9d2) to 20/67 at 940b461d, with fidelityScore unchanged at 1.0 both times. Confirmed the drop is real and unrelated to the summarizeExactParity divergence-masking fix in the same PR (both pre-fix and post-fix code report 20/67 at 940b461d). Likely a side effect of one of the intervening Chicago bibliography-link commits (92fcfafe, 8a81ca58, 5b470906, 940b461d) touching shared bibliography rendering. See docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md for context.

Acceptance criteria:
- [ ] Bisect which commit introduced the regression (likely one of the four recent Chicago-link commits).
- [ ] Determine whether the change is a genuine defect or an unrecorded intentional divergence; if the latter, register it in docs/adjudication/DIVERGENCE_REGISTER.md.
- [ ] Fix or document, then confirm springer-vancouver-brackets' exact-parity floor in embedded-parity-baseline.json can be raised back toward 28/67.
