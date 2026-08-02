---
# csl26-3az5
title: LocaleOverride needs per-style month-abbreviation override + date engine needs day-zero-pad
status: completed
type: feature
priority: normal
tags:
    - style
    - schema
    - engine
created_at: 2026-08-02T00:09:33Z
updated_at: 2026-08-02T23:02:35Z
parent: csl26-ccdt
---

Add two small date-formatting capabilities ieee needs and no style currently has:
- A way for a style to override specific month abbreviations (ieee wants "Jul."/"Jun."/"Sep.", not the engine defaults "July"/"June"/"Sept.").
- Zero-padded days ("Feb. 07" not "Feb. 7").

Example currently wrong: a patent entry renders "July 13, 2021" instead of "Jul. 13, 2021".

Both are schema+engine changes (LocaleOverride has no month-override field; the date formatter has no day-zero-pad option), reusable by any future style with the same need -- not ieee-specific. Regenerate schemas per CLAUDE.md if types change.

## Plan
- [x] Spec: docs/specs/LOCALE_DATE_NAME_KEYING.md
- [x] citum-edtf: expose season_code() (21-24)
- [x] citum-schema-style: SubYearCode key type + keyed MonthNames/seasons + raw compat
- [ ] Rewrite embedded locale YAMLs + hardcoded fixtures to canonical map form
- [x] LocaleOverride.dates (month/season name overrides), apply_override merge
- [x] DateConfig.day_zero_pad
- [x] citum-engine: resolve_date_pattern zero_pad_day param; wire 3 day-rendering surfaces; map-based month/season lookup
- [x] Tests (key parsing, round-trip, apply_override, engine day-zero-pad incl. range, bean's exact example)
- [x] just schema-gen + docs/schemas + data-model-reference regen (no data-model-reference changes needed)
- [x] docs/guides/AUTHORING_LOCALES.md update
- [x] just pre-commit green (fmt+clippy+nextest run individually under systemd-run MemoryMax=6G due to laptop memory pressure; all pass: 2373 tests)
- [x] Follow-up bean: wire ieee locale override + day-zero-pad (tagged, parented csl26-ccdt) -> csl26-fz2e
- [x] PR opened: https://github.com/citum/citum-core/pull/1133 -- CI green

## Summary of Changes

Both requested capabilities were implemented, plus a design change the review surfaced as a
prerequisite: month/season names were positional lists, so a sparse per-style override needed a
stable key first. Locale month and season names are now keyed by EDTF sub-year code (1-12
months, 21-24 seasons; 25-41 reserved for future EDTF Level 2 granularity), documented in a new
spec (docs/specs/LOCALE_DATE_NAME_KEYING.md). The legacy positional-list YAML shape still parses
and canonicalizes to the same codes, so no existing locale file needed to change.

- `LocaleOverride.dates` (new): sparse month/season name overrides, merged key-by-key by
  `apply_override`, reusing the same keyed `MonthNames` type as the base locale.
- `DateConfig.day_zero_pad` (new): zero-pads the rendered day across both single-date and
  same-year date-range rendering; independent of `month: numeric`/`iso`, which already zero-pad
  unconditionally.
- `citum_edtf::Edtf::season_code()` replaces `season()`, returning the raw EDTF code (21-24)
  instead of a flattened 1-based index, so the engine can use one lookup for months and seasons.
- Wired `day_zero_pad` through all three day-rendering surfaces in `citum-engine`'s date
  formatter, including the range-fragment path that's easy to miss.
- Deliberately did not wire IEEE's style YAML to use either capability in this PR, to avoid
  moving fidelity numbers in a schema/engine PR — tracked as follow-up bean csl26-fz2e.

Tests: key-type parsing (YAML int / JSON string / out-of-range), legacy-sequence-to-code
canonicalization, `apply_override` merge semantics, an embedded-locale round-trip completeness
check, and the bean's own reported example (en-US + short-month override + day-zero-pad renders
"Jul. 13, 2021"). `just schema-gen` regenerated `docs/schemas/locale.json` and `style.json`;
no `docs/reference/generated/` changes were needed. Full pre-commit gate (fmt, clippy, nextest)
passed, run in stages under `systemd-run --scope -p MemoryMax=6G` after a full run was OOM-killed.
