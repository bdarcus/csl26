---
# csl26-g6bi
title: Fix Eds./Trans. role-label trailing punctuation without regressing shared styles
status: completed
type: bug
priority: normal
tags:
    - engine
    - style
    - fidelity
created_at: 2026-08-02T00:09:00Z
updated_at: 2026-08-05T14:07:23Z
parent: csl26-ccdt
---

Fix ieee's "Eds."/"Trans." punctuation -- it renders "Eds. The Handbook" instead of "Eds., The Handbook" (missing comma before the next field).

- Already tried the obvious fix (add a comma to the shared role-label preset) -- it corrected ieee but silently broke chicago-author-date-18th (172->164/540 exact parity) and american-medical-association (49->48/67), since the preset is shared identically across all three styles. Reverted.
- Don't patch the shared preset again. Needs either a per-style override, or separator logic that tells an abbreviation-ending period ("Eds.") apart from a sentence-ending one.
- Before landing any fix here, run report-core.js --all-features across every embedded-core style, not a guessed subset -- that's exactly how the last attempt caught its regression.

## Summary of Changes

Fixed in PR #1140 (merged 2026-08-05, commit 05bfcf89), at the engine layer rather than the shared role-label preset the earlier attempt patched.

`append_rendered_component` treated any final punctuation on the accumulated output as a sentence end and replaced the style's separator with a bare space. A period is a *weak* terminal (`render/punctuation.rs`), so an abbreviation-ending period ate the comma: `Eds. The Handbook` instead of `Eds., The Handbook`.

`resolve_punctuation_collision` already encoded the correct answer for the pair (`('.', ',') => ".,"`) and simply was not consulted. The branch now delegates to it, which also folds the hand-rolled StrongTerminalCommaPolicy special case back into the matrix that implements that policy. Scoped to comma separators, since an empty separator has no core and the surrounding code coerces it to '.'.

Full embedded sweep, 35 styles: exact-parity rows **1546 -> 1558**, zero regressions, no fidelity movement. ieee 88->95/149, ASME 0->4/67, ams-label 26->27/67. chicago-author-date-18th (172/546) and american-medical-association (33/67) -- the styles the earlier preset attempt broke -- unmoved, as was chicago-shortened-notes-bibliography (13/473), the only other embedded style using `separator: ", "`.
