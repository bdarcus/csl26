---
# csl26-oqgf
title: Tighten LocatorValue::is_plural heuristic (A1)
status: todo
type: bug
priority: normal
created_at: 2026-06-02T12:33:29Z
updated_at: 2026-06-02T12:33:29Z
---

Heuristic currently treats any hyphen as a range marker, causing false plurals for identifiers like 'figure A-3' or 'sec. 3-2'. Decision A1 from LOCATOR_INPUT.md: only fire on digit/roman-numeral boundaries.\n\n- [ ] Change is_plural in crates/citum-schema-data/src/citation.rs to match /\d\s*[–-]\s*\d/ (plus comma and ampersand branches unchanged)\n- [ ] Add test cases: 'A-3' → singular, 'sec. 3-2' → singular, '42-45' → plural, '42–45' → plural\n- [ ] Update doc comment on LocatorValue
