# EDTF Date-Range Formatting Specification

**Status:** Active
**Version:** 1.1
**Date:** 2026-07-26
**Related:** `.beans/archive/csl26-n54z--generalize-edtf-date-range-formatting.md`, [CSL forum discussion](https://discourse.citationstyles.org/t/date-ranges-should-support-abbreviated-condensed-year-formatting-cmos-18-9-63-9-66/2048)

## Purpose

Allow styles and locales to control closed EDTF date intervals without changing canonical EDTF input, date parsing, or page-range configuration.

## Scope

This specification covers closed EDTF intervals. It excludes page ranges and open intervals. Distinct-year ranges retain endpoint-by-endpoint assembly; locale interval messages apply when both endpoints share a year.

## Design

`options.dates.range-format` accepts:

```yaml
options:
  dates:
    range-format: expanded | chicago
```

- `expanded` is the default and renders both endpoints in full: `2021/2026` becomes `2021–2026`.
- `chicago` applies the existing Chicago inclusive-number algorithm: `2021/2026` becomes `2021–26`.

Chicago abbreviation applies only to fully specified, year-only intervals in the
same era. It formats displayed historical numbers, so the same rule supports
BCE values (`-0326/-0020` becomes `327–21 BCE` when BCE/CE labels are active).
Cross-era, unspecified, reversed, and open intervals retain endpoint-by-endpoint
rendering.

This option is independent of `page-range-format`. The Chicago 18 shared style base opts into `chicago`; all other styles retain `expanded` until explicitly configured.

### Shared-year locale messages

Locales may customize shared-year intervals with `pattern.date-range-<form>`
MF2 messages. The engine supplies reduced endpoint fragments as `$start` and
`$end`, plus `$year` when the selected date form displays a year. For example:

```yaml
messages:
  pattern.date-range-year-month: "{$start} a {$end}, {$year}"
```

This renders `2026-05/2026-06` as `mayo a junio, 2026` in Spanish. If no
locale pattern is authored, existing English fallback layouts remain unchanged.

## Acceptance Criteria

- [x] Styles can deserialize `options.dates.range-format: chicago`.
- [x] Unconfigured styles retain expanded year ranges.
- [x] Chicago rendering condenses `2021/2026` to `2021–26`.
- [x] Chicago rendering supports same-era BCE intervals without changing cross-era output.
- [x] Locales can configure shared-year interval grammar through MF2 messages.
- [x] Non-year and exceptional EDTF ranges retain their existing rendering unless a matching locale interval pattern is authored.

## Changelog

- v1.0 (2026-07-26): Initial specification.
- v1.1 (2026-07-26): Add same-era BCE Chicago abbreviation and shared-year MF2 interval patterns.
