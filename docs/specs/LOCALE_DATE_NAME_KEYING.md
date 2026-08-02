# Locale Date Name Keying Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-08-02
**Supersedes:** (none)
**Related:** bean `csl26-3az5`, [`LOCALE_MESSAGES.md`](./LOCALE_MESSAGES.md),
[`DATE_MODEL.md`](./DATE_MODEL.md)

## Purpose

Give styles two date-formatting capabilities they currently lack:

- Overriding specific month or season names on top of a base locale (e.g. a
  style that wants `"Jul."` where the base locale ships `"July"`).
- Zero-padding the day-of-month in rendered dates (`"Feb. 07"` instead of
  `"Feb. 7"`).

A sparse per-name override needs a stable key per calendar month/season. The
base locale format stores month and season names as parallel positional
lists (`months.long`, `months.short`, `seasons`), so a naive override would
introduce a second, index-based shape alongside the existing list shape.
This spec keys the base locale data the same way an override would need it,
so both use one shape.

## Scope

**In scope:**
- A key space for calendar months and seasons based on EDTF sub-year codes.
- The canonical (map) and accepted-legacy (sequence) YAML shapes for
  `dates.months.{long,short}` and `dates.seasons`.
- A `dates` block on `LocaleOverride` for sparse month/season name overrides,
  merged key-by-key into the base locale.
- The `day-zero-pad` option on `DateConfig`.

**Out of scope:**
- Wiring any specific style (e.g. IEEE) to use these capabilities — tracked
  as a separate follow-up bean.
- EDTF Level 2 sub-year granularity beyond reserving key space for it
  (quarters, semesters, quadrimesters, hemisphere-qualified seasons).
- Locale-specific season vocabularies beyond the four EDTF Level 1 seasons
  already supported.

## Design

### Key space: EDTF sub-year codes

EDTF (Extended Date/Time Format) defines a "sub-year" concept: a value more
specific than a year but not necessarily a calendar month. Level 0 covers
calendar months as `01`–`12`. Level 1 adds seasons as `21` (Spring), `22`
(Summer), `23` (Autumn), `24` (Winter). Level 2 extends further to quarters,
semesters, quadrimesters, and hemisphere-qualified seasons in the `25`–`41`
range.

Citum already parses these codes: `crates/citum-edtf/src/lib.rs` models
`MonthOrSeason` with the `21`–`24` season values, though its public
`Locale::season()` accessor currently flattens them to a `1`–`4` index. This
spec uses the EDTF codes themselves as map keys, both for months (`1`–`12`)
and seasons (`21`–`24`), reserving `25`–`41` for future Level 2 support
without another shape change.

A `SubYearCode` newtype wraps a validated `u8` in this combined range.

### Canonical shape

Month and season names are keyed maps, not lists:

```yaml
dates:
  months:
    long:
      1: January
      2: February
      # ...
      12: December
    short:
      1: "Jan."
      2: "Feb."
      # ...
  seasons:
    21: Spring
    22: Summer
    23: Autumn
    24: Winter
```

Serialization always emits this form.

### Accepted legacy shape

The sequence form used by every locale file prior to this spec continues to
parse:

```yaml
dates:
  months:
    long: [January, February, ..., December]
  seasons: [Spring, Summer, Autumn, Winter]
```

Sequence index `i` (0-based) maps to key `i + 1` for months and `i + 21` for
seasons. This mapping is applied once, at the raw-YAML → canonical-`Locale`
conversion boundary (`crates/citum-schema-style/src/locale/raw_conversion.rs`);
nothing downstream of `Locale` ever sees the sequence form. No
`locale-schema-version` bump is needed — both v1 and v2 message-syntax locale
files, and third-party locale files outside this repo, keep working
unchanged.

### Sparse overrides

`LocaleOverride` (see `LOCALE_MESSAGES.md` §"LocaleOverride") gains a `dates`
field reusing the same keyed types:

```yaml
dates:
  months:
    short:
      6: "Jun."
      7: "Jul."
      9: "Sep."
```

`Locale::apply_override` merges each supplied key into the corresponding base
map, leaving unmentioned months/seasons untouched — the same key-by-key
insertion/replacement semantics `apply_override` already uses for `messages`
and `legacy-term-aliases`.

### `day-zero-pad`

`DateConfig` (`crates/citum-schema-style/src/options/dates.rs`) gains
`day-zero-pad: bool`, default `false`. When `true`, every day-bearing
rendering path pads the day to two digits (`"07"` instead of `"7"`),
including the range-fragment path used by date-range rendering.

This is independent of `month: numeric` / `iso` presets, which already
zero-pad the day unconditionally as part of their fixed
`YYYY-MM-DD`-style output — `day-zero-pad` only affects the
textual month-name rendering paths (`long`, `short` month formats).

## Implementation Notes

- `crates/citum-edtf/src/lib.rs`: add a `season_code()` accessor returning the
  raw `21`–`24` EDTF code (existing `season()` flattens to `1`–`4`; retire it
  if the engine was its only caller).
- `crates/citum-schema-style/src/locale/types.rs`: `MonthNames` and
  `DateTerms::seasons` become `BTreeMap<SubYearCode, String>`.
- `crates/citum-schema-style/src/locale/raw.rs` /
  `raw_conversion.rs`: an untagged raw enum accepts sequence or map on read;
  canonicalizes to the map.
- `crates/citum-engine/src/values/date.rs`: month/season lookup switches from
  slice indexing to map lookup by code; `resolve_date_pattern` gains a
  `zero_pad_day: bool` parameter applied at all day-rendering call sites,
  including `format_abbreviated_month_day_fragment`'s range path.
- Embedded locale YAML files (12 locales + the `fr-CA` overlay) and the
  hardcoded `Locale::en_us()` / test fixtures are rewritten to the canonical
  map form; the round-trip and legacy-sequence-parsing tests cover both
  representations.

## Acceptance Criteria

- [ ] A `LocaleOverride` can replace a single month's short name without
      affecting the other eleven.
- [ ] The legacy sequence form for `months`/`seasons` still parses correctly
      into the same codes the map form would produce.
- [ ] `day-zero-pad: true` zero-pads the day in both single-date and
      date-range rendering.
- [ ] All embedded locales round-trip through parse → canonical map without
      losing or reordering any month or season name.
- [ ] `en-US` + a `short: {7: "Jul."}` override + `day-zero-pad: true` renders
      `2021-07-13` as the bean's reported expectation (`Jul. 13, 2021`), not
      the base locale's `July 13, 2021`.

## Changelog

- v1.0 (2026-08-02): Initial version.
