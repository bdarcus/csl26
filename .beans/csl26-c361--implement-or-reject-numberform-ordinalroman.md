---
# csl26-c361
title: Implement or reject NumberForm ordinal/roman
status: todo
type: task
priority: normal
tags:
    - numbers
    - localization
created_at: 2026-07-04T17:11:33Z
updated_at: 2026-08-11T23:56:32Z
parent: csl26-8m2p
---

TemplateNumber.form (numeric|ordinal|roman) is documented schema surface but TemplateNumber::values never reads it — 'form: ordinal' on edition renders 2 instead of 2nd with no warning; gender agreement similarly limited to label terms. Implement ordinal/roman rendering via locale ordinal suffixes, or reject the option loudly at style load. docs/architecture/audits/2026-07-04_CITUM_ENGINE_REVIEW_PART2.md finding 10.

## GB/T evidence (csl26-huuz, 2026-08-11 session)

gb-t-7714-2025-author-date, gbt7714.7.4:5 (`manuscript,personal-communication,pamphlet` type-variant): oracle renders `5th editors` (ordinal edition), citum renders `5 editors` — the exact form-not-read gap this bean describes, on `TemplateNumber.form: ordinal` for the edition field.
