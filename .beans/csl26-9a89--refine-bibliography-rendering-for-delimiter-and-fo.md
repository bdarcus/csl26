---
# csl26-9a89
title: Refine bibliography rendering for delimiter and formatting edge cases
status: todo
type: task
priority: high
created_at: 2026-02-08T12:48:04Z
updated_at: 2026-02-08T12:48:04Z
---

The template resolver and per-component delimiter detection are working, but the renderer needs refinement:

Issues to fix:
1. Trailing periods after DOI/URL (suppress_period_after_url not working)
2. Issue numbers appearing when they shouldn't be shown
3. Name ordering (given-first vs family-first) not respecting style requirements
4. Volume-pages delimiter varies by style (comma vs colon vs space)
5. Editor name-order varies by style
6. 'In' prefix before editors sometimes missing proper spacing

The core infrastructure is in place:
- Per-component prefixes work (author-year gets ', ', pages gets ', ')
- Renderer checks first char for punctuation and skips separator
- Config options for suppress_period_after_url exist

Next steps:
- Debug why suppress_period_after_url not working
- Fix issue number visibility logic
- Improve name_order detection and application
- Add volume-pages delimiter handling
- Test across top 10 parent styles