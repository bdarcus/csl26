---
# csl26-g6bi
title: Fix Eds./Trans. role-label trailing punctuation without regressing shared styles
status: todo
type: bug
priority: normal
tags:
    - engine
    - style
    - fidelity
created_at: 2026-08-02T00:09:00Z
updated_at: 2026-08-02T13:16:07Z
parent: csl26-ccdt
---

Fix ieee's "Eds."/"Trans." punctuation -- it renders "Eds. The Handbook" instead of "Eds., The Handbook" (missing comma before the next field).

- Already tried the obvious fix (add a comma to the shared role-label preset) -- it corrected ieee but silently broke chicago-author-date-18th (172->164/540 exact parity) and american-medical-association (49->48/67), since the preset is shared identically across all three styles. Reverted.
- Don't patch the shared preset again. Needs either a per-style override, or separator logic that tells an abbreviation-ending period ("Eds.") apart from a sentence-ending one.
- Before landing any fix here, run report-core.js --all-features across every embedded-core style, not a guessed subset -- that's exactly how the last attempt caught its regression.
