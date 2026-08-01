---
# csl26-2vcg
title: Migrate embedded styles to semantic punctuation marks
status: todo
type: task
priority: normal
tags:
    - punctuation
    - multilingual
    - style
created_at: 2026-08-01T11:58:12Z
updated_at: 2026-08-01T11:58:12Z
---

Enabler for PUNCTUATION_NORMALIZATION.md phase 3: replace the character-sniffing in citum-engine's punctuation-in-quote join sites (default_separator.chars().next(), first_visible_char) with typed marks per docs/specs/PUNCTUATION_REALIZATION.md. Not required to fix punctuation-in-quote itself (csl26-1hya) -- realize_wrap returns None for WrapPunctuation::Quotes, so quote glyphs never touch the realization table. Follow-up from csl26-1hya.
