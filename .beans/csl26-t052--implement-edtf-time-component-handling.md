---
# csl26-t052
title: Implement EDTF time component handling
status: todo
type: feature
priority: normal
created_at: 2026-02-13T11:26:58Z
updated_at: 2026-02-13T11:26:58Z
---

Add support for rendering time components from EDTF datetime values.

## Context
EDTF supports full datetimes (e.g., 1985-04-12T23:20:30Z) but CSLN currently ignores the time component. Times are relevant for:
- Blog posts, social media (timestamp precision)
- Dataset versions (ISO 8601 timestamps)
- Legal documents (filed timestamps)
- 'accessed' dates for URLs (some styles show full timestamp)

## Schema Extensions (csln_core)

Add to DateConfig:
- time_format: Option<TimeFormat> (12h, 24h, none)
- show_seconds: bool (default: false)
- show_timezone: bool (default: false)

Add TimeFormat enum:
- TwelveHour (11:30 PM)
- TwentyFourHour (23:30)
- None (suppress time component)

## Locale Terms

Add to LocaleDates:
- am: "AM"
- pm: "PM"
- timezone_utc: "UTC"

## Processor Logic

Add to EdtfString:
- has_time() -> bool
- hour() -> Option<u8>
- minute() -> Option<u8>
- second() -> Option<u8>
- timezone_offset() -> Option<i8>

Update values/date.rs rendering to:
- Extract time from DateTime variant
- Format based on TimeFormat config
- Apply locale terms (AM/PM)
- Handle timezone offsets

## Test Cases

- 1985-04-12T23:20:30Z -> "April 12, 1985, 11:20 PM UTC" (12h)
- 1985-04-12T23:20:30Z -> "April 12, 1985, 23:20 UTC" (24h)
- 2004-01-01T10:10:10+05:00 -> with timezone offset
- DateTime with seconds suppressed
- DateTime with time suppressed (date only)

## Dependencies

Blocked by: EDTF date handling implementation (ranges, uncertainty)

Refs: #64
