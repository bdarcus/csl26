# TODO: Delimiter Enum Refactoring

## Issue

Current delimiter handling mixes structural delimiters with decorative affixes, making migration from CSL 1.0 challenging.

### Current Problems

1. **String-based delimiters**: Delimiters are mostly stored as strings (`, `, `. `, etc.)
2. **Compound patterns**: Some styles use compound patterns like `, in: ` (comma-space-term-colon-space) that don't fit clean enum values
3. **CSL 1.0 mixing**: Original CSL styles treat structural delimiters (like colons) as suffix affixes (e.g., `suffix=": "`)

### Examples

- Elsevier Harvard: Uses `, in: ` as a compound delimiter pattern
- Chicago: Uses `: ` colon-space between journal title and volume
- Standard: `, ` comma-space between components

## Proposed Solution

### Option A: Expand DelimiterPunctuation Enum

Add more enum variants for common compound patterns:

```rust
pub enum DelimiterPunctuation {
    Comma,        // ", "
    Period,       // ". "
    Colon,        // ": "
    Semicolon,    // "; "
    Space,        // " "
    None,         // ""
    CommaIn,      // ", in: " (Elsevier pattern)
    // ... other compound patterns as needed
}
```

**Pros:**
- Type-safe, compile-time checked
- Clear semantic meaning
- Easier to document and reason about

**Cons:**
- Need to enumerate all possible patterns
- Edge cases still need string fallback

### Option B: Keep Strings with Better Extraction

Keep delimiter as `String` but improve extraction logic:

- Better pattern recognition in template compiler
- Clearer separation between structural delimiters and decorative affixes
- Helper functions for common delimiter patterns

**Pros:**
- Handles all edge cases
- More flexible

**Cons:**
- No compile-time checking
- Less semantic clarity

### Option C: Hybrid Approach

Use enum for common cases with a `Custom(String)` variant for edge cases:

```rust
pub enum Delimiter {
    Comma,
    Period,
    Colon,
    Semicolon,
    Space,
    None,
    Custom(String),
}
```

**Pros:**
- Best of both worlds
- Clean for common cases
- Flexible for edge cases

**Cons:**
- Still need to handle Custom variant everywhere

## Recommendation

Start with **Option C (Hybrid Approach)** because:

1. Covers 95% of cases with enum clarity
2. Handles edge cases with Custom variant
3. Can add new enum variants as patterns emerge from corpus analysis
4. Migration path: analyze styles to find common patterns, promote Custom patterns to enum variants

## Migration Strategy

1. **Analyze corpus**: Run analyzer to find all unique delimiter patterns
2. **Identify common patterns**: Promote top 10-20 patterns to enum variants
3. **Refactor incrementally**: Start with new code, migrate existing code gradually
4. **Add validation**: Linter can warn about non-standard Custom delimiters

## Related Issues

- #89 (Presets for common configurations)
- #64 (Math in variables, delimiters in metadata)

## Priority

Medium - Not blocking tier 1 rendering, but would improve code clarity and migration quality for tier 2+.
