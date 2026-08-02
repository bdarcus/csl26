---
# csl26-3az5
title: LocaleOverride needs per-style month-abbreviation override + date engine needs day-zero-pad
status: todo
type: feature
priority: normal
tags:
    - style
    - schema
    - engine
created_at: 2026-08-02T00:09:33Z
updated_at: 2026-08-02T14:19:29Z
parent: csl26-ccdt
---

Add two small date-formatting capabilities ieee needs and no style currently has:
- A way for a style to override specific month abbreviations (ieee wants "Jul."/"Jun."/"Sep.", not the engine defaults "July"/"June"/"Sept.").
- Zero-padded days ("Feb. 07" not "Feb. 7").

Example currently wrong: a patent entry renders "July 13, 2021" instead of "Jul. 13, 2021".

Both are schema+engine changes (LocaleOverride has no month-override field; the date formatter has no day-zero-pad option), reusable by any future style with the same need -- not ieee-specific. Regenerate schemas per CLAUDE.md if types change.
