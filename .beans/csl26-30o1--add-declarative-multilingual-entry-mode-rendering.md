---
# csl26-30o1
title: Add declarative multilingual entry-mode rendering
status: todo
type: feature
priority: normal
tags:
    - multilingual
    - schema
    - engine
    - rendering
created_at: 2026-08-03T15:30:45Z
updated_at: 2026-08-03T15:30:45Z
parent: csl26-0ugp
---

Add a declarative entry-level multilingual rendering mode for bilingual bibliographies.

Proposed style shape:

```yaml
options:
  multilingual:
    entry-mode:
      pattern:
        - view: translated
        - view: original-script
          wrap: brackets
```

Checklist:

- [ ] Add `entry-mode` to `MultilingualConfig` and generated schemas.
- [ ] Define bibliography-only projection and precedence semantics with field-level multilingual modes.
- [ ] Render translated/original entry bodies without duplicating citation numbers, type labels, URLs, or entry suffixes.
- [ ] Preserve punctuation, rich-text markup, missing-translation fallback, and simple English deduplication.
- [ ] Add multilingual engine and inheritance regression coverage.
- [ ] Enable the Acta Psychologica Sinica overlay to use the entry-level mode once supported.
