---
# csl26-arly
title: Embedded-tier oracle text parity
status: in-progress
type: epic
priority: high
created_at: 2026-07-30T14:16:05Z
updated_at: 2026-07-30T14:55:38Z
---

Raise oracle text parity for the embedded style tier (currently 38.7% overall, 1260/3255). Baseline: docs/compat.html generated at fb0a01af. Disjoint defect taxonomy over 2501 parity mismatches (Z unclassified 925/37%, D title-quoting 421/16.8%, D2 title/term case 392/15.7%, A oracle-only number prefix 318/12.7% split into A1 harness-join + A2 rendering-omission, B punctuation-only 200/8%, J citum-only number prefix 82/3.3%, I number-prefix format 64/2.6%, F bracket medium label 30/1.2%, H year-suffix 30/1.2% (csl26-m8la), E name separator 26/1%, G/M accessed+label terms 13/0.5%).

**Scope correction (discovered after epic creation):** the chicago-18-base family (1392/2501, 55.7% of all mismatches) is already under active, in-progress tuning at [[csl26-40n4]] -> [[csl26-h7oc]] -> per-variant children ([[csl26-giun]] chicago-author-date-18th, [[csl26-7jht]] chicago-shortened-notes-bibliography, [[csl26-gzwj]] T&F chicago, [[csl26-lxy3]] chicago-notes-18th), with deep implementation history (title-case engine fixes, original/reprint dates, manuscript/archive block, contributor label punctuation). This epic does NOT duplicate that work. Instead: (a) my new D/D2/C-class parity evidence (title quoting, term case, stray container-title period) is appended to csl26-giun and csl26-7jht as fresh residual-defect input for their existing tuning loops; (b) this epic covers the genuinely uncovered territory: measurement-harness integrity (class A1/A2), the baseline/ratchet gate gap, and non-Chicago style fixes (elsevier-vancouver-author-date numeric-prefix leak, class J).

Plan: /home/bruce/.claude/plans/ok-now-that-we-ve-iridescent-cat.md (revised scope per this note).

## PR-1 landed on fix/embedded-parity-wave-1 (3 commits)

1. chore(report): snapshot embedded parity -- scripts/report-data/embedded-parity-baseline.json, all 19 embedded styles' fidelity/quality/citations/bibliography/exactParity at HEAD (828cb9d2). Non-gating (see [[csl26-q42m]] summary for why the original 'add to core-quality-baseline.json' plan was wrong and corrected).
2. docs(report): spike class-A parity gap -- root-caused the 318-mismatch class A2 (bibliography numeric-label rendering gap for second-field-align-derived styles), corrected the false 'A1 join-space bug' hypothesis with a regression test, found and filed a separate RSC report-evidence bug. See docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md and [[csl26-lf68]].
3. fix(styles): elsevier-vancouver label-mode leak -- fixed class J (47 mismatches) in elsevier-vancouver-author-date via bibliography.options.label-mode: author-date. See [[csl26-w1vf]].

**Honest scope note:** PR-1 does not move the headline 38.7% oracle-parity number much. The elsevier-vancouver-author-date fix is exemplar-tier (not embedded), and none of the 19 embedded styles' fidelity or exactParity changed in this PR -- so no baseline ratchet was needed, and docs/compat.html was not regenerated (would require a full community-corpus sweep, out of proportion to what changed here; left as a separate concern). PR-1's real deliverables are the parity-tracking gap closed (embedded-parity-baseline.json) and the class-A2 root cause that unblocks the largest deferred wave, not a parity-percentage win.

**Remaining open children:** [[csl26-j7uc]] (A2 rendering-gap fix, largest deferred wave, 318 mismatches / unblocks 8 exemplars at 0% parity) and [[csl26-waly]] (RSC report-evidence bug). Chicago-family work (55.7% of all mismatches) is intentionally not tracked under this epic -- see the scope-correction note above; residual-defect evidence was fed into the existing in-progress [[csl26-giun]] and [[csl26-7jht]] beans instead.

**Correction (2026-07-30, post-PR-1):** the framing above overstates csl26-40n4/csl26-h7oc as active. Its last landed PR merged 2026-07-04 (26 days idle at time of writing), csl26-giun has sat 'in-progress' with no landed work since then, and csl26-7jht/csl26-gzwj are unstarted todos. Feeding evidence into them was still correct (avoids a second competing tracking structure for the same styles), but resuming Chicago work is separate, unstarted effort -- not something already in flight. User flagged this after reading the PR; next work picked was csl26-j7uc instead.
