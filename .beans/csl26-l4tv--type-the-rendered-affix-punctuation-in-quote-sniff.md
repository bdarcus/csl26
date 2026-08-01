---
# csl26-l4tv
title: Type the rendered-affix punctuation-in-quote sniffing (first_visible_char)
status: todo
type: task
priority: normal
tags:
    - punctuation
    - multilingual
    - engine
created_at: 2026-08-01T19:25:19Z
updated_at: 2026-08-01T19:25:27Z
---

Follow-up from csl26-2vcg: first_visible_char (render/bibliography.rs) and punctuation.rs's part.chars().next() sniff the already-rendered component string for a self-supplied leading period/comma (e.g. Chicago's prefix: ". Aired "). This can't be typed by widening config -- the affix is baked into the string by the time the join site sees it. Requires ProcTemplateComponent to carry structured affixes and render_component_with_format to return more than a bare String, touching all seven output formats. This is the architectural core of docs/specs/PUNCTUATION_NORMALIZATION.md phase 3. Note: first_visible_char also handles '(' and value-supplied leading characters, so even full phase-3 work likely retains part of it.
