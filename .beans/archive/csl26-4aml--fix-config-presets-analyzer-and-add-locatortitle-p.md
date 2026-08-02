---
# csl26-4aml
title: Fix config-presets analyzer and add locator/title presets it justifies
status: completed
type: task
priority: normal
created_at: 2026-08-02T17:53:33Z
updated_at: 2026-08-02T18:17:02Z
---

csl26-qqdt audit found the --config-presets analyzer produces a correct frequency report but a misleading preset worklist: ContributorConfig's delimiter:null skip_serializing_if artifact fragments 217/454 contributor candidate styles, TitlePreset never covers the 'default' rendering category (830+208 unreachable styles), and the locator candidate is an unnoticed one-field diff from author-date. Fix the analyzer (null normalization, nearest-preset diff, enum-derived ALL const to prevent silent drift) and add the presets the corrected report justifies: LocatorPreset::Numeric, TitlePreset::EmphasisAll, TitlePreset::TitleCase. Add a justfile recipe for the analyzer. Contributor presets are explicitly out of scope pending re-run and family/convention review.

## Todo

- [x] 1a: normalize `null` fields before hashing (both accumulate and preset_keys)
- [x] 1b: nearest-preset diff on PresetCandidate (nearest_preset, differing_fields)
- [x] 1c: pub const ALL on preset enums; drop hand-listed named_keys arrays; dispatch test
- [x] 1d: share_of_unmatched on PresetCandidate + text output
- [x] Re-run analyzer, confirm contributor fragmentation drops, capture output for PR
- [x] LocatorPreset::Numeric (author-date + strip-label-periods)
- [x] TitlePreset::EmphasisAll (default.emph)
- [x] TitlePreset::TitleCase (default.text-case: title)
- [x] Schema parse/resolve tests for new presets + analyzer reverse-match coverage
- [x] justfile: analyze-presets recipe
- [x] just schema-gen (citum-schema-style changed)
- [x] just pre-commit
- [x] File follow-up bean: suspected migrate emph+quote title artifact (~68 styles)
- [x] File follow-up bean: deferred analyzer improvements (savings ranking, array normalization, substitute/sort concerns)
- [x] Update csl26-qqdt with audit outcome

## Summary of Changes

Evaluated citum-analyze --config-presets against csl26-qqdt: found two structural defects (delimiter:null normalization artifact fragmenting contributor candidates; TitlePreset never covering the `default` rendering category, making default-only shapes structurally unreachable) plus a silent-drift risk (hand-maintained preset-enum mirror lists with no coverage test). Fixed all three, added nearest-preset diff + share_of_unmatched to the report, re-ran, and added only the presets the corrected report justifies: LocatorPreset::Numeric (120/120 styles now matched) and TitlePreset::EmphasisAll / TitlePreset::TitleCase (matched 643→1681, unmatched 1697→659). Contributor presets deliberately not added — re-run showed matched_style_count unchanged even after the normalization fix, meaning no candidate shape was blocked by delimiter:null alone; remaining candidates scatter across many presets with ≥2 differing fields each, i.e. no nameable family/convention cluster. Added justfile analyze-presets recipe. just pre-commit and just check-core-quality both pass with no regression.
