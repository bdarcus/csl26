---
# csl26-9a89
title: Refine bibliography rendering for delimiter and formatting edge cases
status: in-progress
type: task
priority: high
created_at: 2026-02-08T12:48:04Z
updated_at: 2026-02-08T14:00:00Z
---

The template resolver and per-component delimiter detection are working.

## Completed (2026-02-08)
- Trailing period after DOI/URL: always suppressed in renderer (no config needed)
- Per-component delimiter detection: predecessor frequency check prevents rare
  type-specific pairs (e.g. editors in chapters) from setting wrong prefixes
- Items group (volume+issue) predecessor lookup: tries both issue and volume
  names, fixing pages prefix detection for entries without issue numbers
- Elsevier-harvard bibliography: 0/28 → 6/28 match
- **Wrapped component prefix fix**: Template inferrer now skips setting 
  whitespace-only prefix for components with wrap (e.g. date in parentheses).
  This prevents "( 2019)" rendering. Springer bibliography: 0/28 → 2/28
- **Renderer separator simplification**: Refactored refs_to_string separator 
  logic to be clearer and more predictable. year:missing dropped 57 → 46.
- **Name ordering logic**: Fixed name order detection to correctly infer 
  `family-first` for bibliographic entries. Removed hardcoded `GivenFirst` 
  override for APA editors in the migrator. APA bibliography now uses correct 
  name order for both authors and editors.

1. **Issue number leaking**: Issue numbers render when citeproc-js suppresses
   them (e.g. "37, 1, 1-13" vs "37, 1-13"). Needs type/value-specific
   suppress logic.
2. **Entry suffix**: Some styles don't want trailing period (springer)
3. **Editor formatting**: "edited by" vs "(Eds.)" vs "In: Name (ed)"
5. **Conference papers**: Duplicate container titles
6. **Unsupported types**: 13 of 28 items undefined (legal, patent, film, etc.)

## Current scores (oracle-e2e)
| Style | Citations | Bibliography |
|-------|-----------|-------------|
| elsevier-harvard | 14/28 | 6/28 |
| springer-basic-author-date | 15/28 | 2/28 ✅ |
| chicago-author-date | 0/28 | 0/27 |
| ieee | 15/28 | 0/28 |
| elsevier-with-titles | 15/28 | 0/28 |

## Next steps (priority order)
1. Fix entry suffix to respect style config (no trailing "." for springer)
2. Address issue number leaking for styles that suppress issue
3. Clean up period delimiters in APA (e.g. fix "). " artifacts from inference)