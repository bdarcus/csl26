---
# csl26-i18h
title: Localizable bibliography group headings
status: completed
type: refactor
priority: normal
created_at: 2026-02-16T16:15:00Z
updated_at: 2026-02-19T11:26:41Z
---

The `heading` field in `BibliographyGroup` is currently a plain `String`, which prevents it from being localized.

**Requirements:**
- Refactor `heading` in `BibliographyGroup` to support localization.
- Options include:
  - Referencing a locale term key.
  - Using a map of language codes to strings.
  - A hybrid approach allowing both literal strings and localizable keys.
- Update rendering logic to resolve the localized string based on the current processor locale.

**Related:** csl26-group
