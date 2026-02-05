# Punctuation Normalization Design

## Current State

CSLN currently handles punctuation placement with:
- Boolean `punctuation-in-quote: true/false` (American style only)
- Ad-hoc logic scattered throughout `render.rs` that moves periods inside/outside quotes during rendering
- Only handles curly and straight double quotes
- Tightly coupled to the rendering process

**Problems:**
- Hard to reason about correctness (3 separate locations in render.rs doing similar logic)
- Only supports American English convention
- Fragile: quote style changes (straight → curly) require updating all punctuation logic
- Not locale-aware
- Cannot support other language conventions (German, French, etc.)

## Better Approach: Separate Normalization Phase

Based on org-cite's `org-cite-adjust-punctuation` design (see [mailing list post](https://lists.nongnu.org/archive/html/emacs-orgmode/2021-05/msg00714.html) and [source code comments](https://github.com/bzg/org-mode/blob/main/lisp/oc.el)), punctuation normalization should be:

1. **A separate processing phase** that runs after component assembly but before final rendering
2. **Language-aware** based on document locale
3. **Configurable** with three orthogonal parameters instead of one boolean

### Three-Parameter Model

```yaml
punctuation:
  movement: inside | outside | strict
  citation-position: inside | outside  # relative to quotes
  citation-order: before | after       # relative to punctuation
```

**Language conventions:**
- **American English**: `movement: inside, citation-position: outside, citation-order: after`
  - "Text." → citation → more text
  - Periods/commas move inside closing quotes

- **British English**: `movement: outside, citation-position: outside, citation-order: after`
  - "Text". → citation → more text
  - Punctuation stays outside quotes

- **German**: `movement: strict, citation-position: outside, citation-order: after`
  - Punctuation doesn't move
  - Citation comes after quotes

- **French**: `movement: strict, citation-position: inside, citation-order: before`
  - Punctuation doesn't move
  - Citation comes inside quotes before punctuation

### Processing Order

Nicolas's key insight: **"Call adjust-punctuation first, before wrap-citation"**

This suggests the pipeline should be:
1. Assemble components with their content
2. **Normalize punctuation** (separate phase, locale-aware)
3. Wrap citations in delimiters
4. Apply formatting (italics, quotes, etc.)
5. Concatenate with separators

Currently we do #2 and #4 together, which is why quote style changes break punctuation logic.

## Migration Path

### Phase 1: Refactor current code (no breaking changes)
- Extract punctuation logic into a single `normalize_punctuation()` function
- Make it handle both straight and curly quotes uniformly
- Keep existing `punctuation-in-quote: bool` as interface

### Phase 2: Extend for multilingual (breaking schema change)
- Replace boolean with three-parameter model
- Add locale-awareness (derive from document `lang` field or style metadata)
- Default to current behavior for backwards compatibility

### Phase 3: Separate phase (architectural)
- Move punctuation normalization to its own processing step
- Run after template assembly, before formatting
- Easier to test, reason about, and extend

## Related Work

### CSL 1.0
Has `punctuation-in-quote` attribute but it's underspecified:
- Only handles periods and commas
- No guidance on interaction with citations
- Assumes American convention

### CSL-M (legal citations)
Extended for legal citations but still American-centric.

### biblatex
Has `autopunct` feature that's more sophisticated:
- Handles multiple punctuation marks
- Language-aware via babel/polyglossia integration
- Separate from formatting logic

## Implementation Notes

### Current bugs to watch for:
1. **Quote character assumptions**: Any code that checks `ends_with('"')` must also check `ends_with('\u{201D}')`
2. **Separator conflicts**: Default separator `. ` interacts with quote normalization
3. **Multiple punctuation**: What if title ends with `?` or `!` - do we still add `.`?
4. **Nested quotes**: Single quotes inside double quotes not currently handled

### Testing strategy:
- Unit tests for `normalize_punctuation()` with all language conventions
- Integration tests with real styles (APA, Chicago, German DIN, French CNRS)
- Regression tests for current American behavior

## References

- org-cite design: https://github.com/bzg/org-mode/blob/main/lisp/oc.el
- CSL 1.0 spec: https://docs.citationstyles.org/en/stable/specification.html#punctuation-in-quote
- biblatex autopunct: https://www.ctan.org/pkg/biblatex (sec 3.9)

## Priority

**Medium-High** for multilingual support (csln#66)
**Low-Medium** for current English-only work

However, refactoring current ad-hoc code into a clean function would prevent bugs and make the codebase more maintainable even before multilingual support.

## Related Issues

- csln#66 - Multilingual/multiscript support
- Current PR #51 - Curly quote rendering (exposed fragility of current approach)
