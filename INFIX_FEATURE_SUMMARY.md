# Infix Citation Support - Implementation Summary

## Feature Overview

Added support for **infix citations** in integral citations, allowing narrative text to appear between the author name and year while ensuring proper author name formatting according to style rules.

## Syntax

```djot
@key(infix text)[locator]
```

Where:
- `@key` - citation key (required)
- `(infix text)` - narrative text inserted between author and year (optional)
- `[locator]` - page/chapter/section locator (optional)

## Examples

**Basic infix:**
```djot
@smith(argues that x).
```
Output: `Smith argues that x (2023)`

**Infix with locator:**
```djot
@jones(suggests that y)[ch. 3].
```
Output: `Jones suggests that y (2023, ch. 3)`

**Traditional integral (unchanged):**
```djot
@smith argues that x.
```
Output: `Smith (2023) argues that x`

## Value Proposition

**Why infix-as-citation-metadata vs. document prose?**

1. **Automatic author formatting** - The processor applies style-specific rules:
   - Small caps (e.g., "SMITH argues that x (2023)")
   - Transliteration for multilingual names
   - Institutional name formatting

2. **Style portability** - Switching citation styles automatically updates author formatting throughout the document without manual changes

3. **Multilingual support** - Works correctly with CSL-M multilingual names and transliteration rules

## Technical Details

- **Parser**: Extended `parse_narrative_citation` to recognize `(...)` for infix and `[...]` for locators
- **Data model**: Added `infix: Option<String>` field to `CitationItem`
- **Renderer**: Modified integral citation formatting to insert infix between author and year
- **Constraint**: Infix only applies to single-item citations (not multi-cite groups)

## Testing

See `examples/document.djot` for comprehensive examples demonstrating:
- Basic infix citations
- Infix with structured/unstructured locators
- Integration with existing citation patterns

## Implementation

- **Files changed**: 3
- **Lines added**: ~60
- **Tests added**: 4 new parser tests
- **Status**: All tests passing ✅
