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
updated_at: 2026-08-05T17:32:21Z
parent: csl26-ccdt
---

Fix ieee's "Eds."/"Trans." punctuation -- it renders "Eds. The Handbook" instead of "Eds., The Handbook" (missing comma before the next field).

- Already tried the obvious fix (add a comma to the shared role-label preset) -- it corrected ieee but silently broke chicago-author-date-18th (172->164/540 exact parity) and american-medical-association (49->48/67), since the preset is shared identically across all three styles. Reverted.
- Don't patch the shared preset again. Needs either a per-style override, or separator logic that tells an abbreviation-ending period ("Eds.") apart from a sentence-ending one.
- Before landing any fix here, run report-core.js --all-features across every embedded-core style, not a guessed subset -- that's exactly how the last attempt caught its regression.

## Root cause (measured 2026-08-05)

`crates/citum-engine/src/render/bibliography.rs:377` sets
`ends_with_punctuation = is_final_punctuation(trimmed_last)`, true for `.`. Lines
404-421 then drop the configured separator and push a bare space unless the trailing
mark is a *strong* terminal (`!?...`). A period is `WeakTerminal`
(`render/punctuation.rs:38`), so `Eds.` + separator `", "` loses its comma.

`resolve_punctuation_collision` (`render/punctuation.rs:238`) already encodes the
correct answer for this pair: `(.,,) => ".,"` -- keep both. `append_rendered_component`
never consults it. The defect is in the engine, not the shared role-label preset the
earlier attempt patched -- which is why that attempt regressed chicago/AMA.

Affected ieee rows: sr-editor-only, sr-translator-only, sr-chapter-container-editors,
ITEM-4, ITEM-14, ITEM-7, TLIB-SEL-TREATY-1.

Blast radius: among embedded styles only `ieee` and
`chicago-shortened-notes-bibliography-core` use `separator: ", "`.

Baseline: ieee exact parity 88/149 (59.1%).

Plan: docs/../plans -- PR1 of three (engine -> schema docs -> style).

## Todo

- [x] Delegate the `ends_with_punctuation` suppression decision to `resolve_punctuation_collision`, scoped to `default_separator.core() == Some(char::from(44))`
- [x] rstest cases: Eds./Trans./et al./U.S.S.R. + ", " keep the comma
- [x] Regression cases: "Title." + ". " unchanged; "Title!" + ", " unchanged under both StrongTerminalCommaPolicy values
- [x] just pre-commit
- [x] Full embedded sweep vs baseline: chicago-author-date-18th 172/540 and american-medical-association 49/67 unmoved
- [x] CI green

## Result

Full embedded sweep (35 styles, `--all-features`), baseline 6595cf0b vs fix:

| style | before | after |
|---|---|---|
| ieee | 88/149 | **95/149** |
| american-society-of-mechanical-engineers | 0/67 | **4/67** |
| american-mathematical-society-label | 26/67 | **27/67** |

Total exact-parity rows 1546 -> 1558 (+12). **Zero regressions**; no fidelity score
moved. The styles the earlier preset attempt broke are unmoved:
chicago-author-date-18th 172/546, american-medical-association 33/67, and
chicago-shortened-notes-bibliography 13/473 (the only other embedded style using
`separator: ", "`).

Follow-up noted, not done here: `delimiter_suppressing_terminal_marks` (locale
`grammar-options`, default `"?!..."`) is threaded through `processor/setup.rs` into
`Config` and read by no renderer -- dead config naming this exact concept.
