---
# csl26-9a89
title: Refine bibliography rendering for delimiter and formatting edge cases
status: in-progress
type: task
priority: high
created_at: 2026-02-08T12:48:04Z
updated_at: 2026-02-08T13:30:52Z
---

The template resolver and per-component delimiter detection are working.

## Completed (2026-02-08)
- Trailing period after DOI/URL: always suppressed in renderer (no config needed)
- Per-component delimiter detection: predecessor frequency check prevents rare
  type-specific pairs (e.g. editors in chapters) from setting wrong prefixes
- Items group (volume+issue) predecessor lookup: tries both issue and volume
  names, fixing pages prefix detection for entries without issue numbers
- Elsevier-harvard bibliography: 0/28 → 6/28 match

## Remaining issues
1. **Renderer separator vs prefix conflict**: When inferred templates set
   prefix (e.g. " " for date with wrap:parentheses), the renderer's
   default separator logic conflicts. The prefix goes inside the wrap
   giving "( 2019)" instead of "(2019)". Need to teach refs_to_string
   to skip default separator when component has its own rendering prefix.
2. **Issue number leaking**: Issue numbers render when citeproc-js suppresses
   them (e.g. "37, 1, 1-13" vs "37, 1-13"). Needs type/value-specific
   suppress logic.
3. **Name ordering**: given-first vs family-first not matching some styles
4. **Entry suffix**: Some styles don't want trailing period (springer)
5. **Editor formatting**: "edited by" vs "(Eds.)" vs "In: Name (ed)"
6. **Conference papers**: Duplicate container titles
7. **Unsupported types**: 13 of 28 items undefined (legal, patent, film, etc.)

## Current scores (oracle-e2e)
| Style | Citations | Bibliography |
|-------|-----------|-------------|
| elsevier-harvard | 14/28 | 6/28 |
| springer-basic-author-date | 15/28 | 0/28 |
| chicago-author-date | 0/28 | 0/27 |
| ieee | 15/28 | 0/28 |
| elsevier-with-titles | 15/28 | 0/28 |

## Next steps (priority order)
1. Fix renderer prefix/separator conflict (biggest bang - affects all styles)
2. Fix entry suffix to respect style config (no trailing "." for springer)
3. Fix contributor trailing period for styles with non-period separator