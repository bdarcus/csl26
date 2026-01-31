# Style Aliasing and Presets Design

## Problem Statement

The CSL 1.0 ecosystem contains **7,987 dependent styles** that alias approximately **300 parent styles**. Analysis shows that the top 10 parent styles cover **60% of all dependent styles**:

| Parent Style | Dependents | Format | Coverage |
|-------------|------------|--------|----------|
| apa | 783 | author-date | 9.8% |
| elsevier-with-titles | 672 | numeric | 8.4% |
| elsevier-harvard | 665 | author-date | 8.3% |
| elsevier-vancouver | 502 | numeric | 6.3% |
| springer-vancouver-brackets | 472 | numeric | 5.9% |
| springer-basic-author-date | 460 | author-date | 5.8% |
| springer-basic-brackets | 352 | numeric | 4.4% |
| springer-socpsych-author-date | 317 | author-date | 4.0% |
| american-medical-association | 293 | numeric | 3.7% |
| taylor-and-francis-chicago-author-date | 234 | author-date | 2.9% |

**Current CSLN migration produces fully independent styles with no aliasing mechanism.** This document evaluates whether and how CSLN should address style reuse.

---

## Design Goals

1. **Explicit over magic**: Styles should be self-documenting without chasing parent references
2. **Simple authoring**: Creating a new style should be easy for librarians and publishers
3. **No hidden complexity**: A style file should contain everything needed to understand its behavior
4. **Forward compatibility**: New presets can be added without breaking existing styles
5. **Engine independence**: Styles should work without external style library dependencies

---

## Option Analysis

### Option A: Replicate CSL 1.0 Dependent Styles

CSL 1.0's dependent style mechanism:
```xml
<link href="http://www.zotero.org/styles/apa" rel="independent-parent"/>
```

The dependent style contains only metadata and a parent reference. All formatting comes from the parent.

**Pros:**
- Direct migration path from CSL 1.0 dependent styles
- Familiar to existing CSL users
- Minimal style files (just metadata + parent reference)
- Preserves existing ecosystem structure

**Cons:**
- Very limited customization (locale-only override in CSL 1.0)
- No partial customization—must accept all parent behavior
- Hidden complexity (must chase parent reference to understand behavior)
- **Violates "explicit over magic" principle**
- Requires maintaining a style registry/repository
- Version management: what if parent changes?
- Runtime dependency: style files aren't self-contained

**Verdict:** Does not align with CSLN's design philosophy.

---

### Option B: Presets Everywhere (Issue #89)

Presets are named bundles of configuration that encode common patterns. Instead of inheriting from a parent style, styles compose presets for different concerns:

```yaml
title: ABC Journal
info:
  id: abc-journal

# Preset-based configuration
options:
  processing: author-date         # Preset: disambiguation, grouping, sorting
  contributors: apa               # Preset: name formatting, et al., order
  dates: year-only                # Preset: date display format
  titles: sentence-case           # Preset: title casing rules

citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
```

**Pros:**
- Explicit configuration in each style—no hidden parent chains
- **Composable**: mix contributor formatting from one tradition with date formatting from another
- **Self-documenting**: reading a style file tells you everything
- Enables simple style authoring for common cases
- **Bundled with engine**: no external dependencies
- Easy versioning: presets are engine features, not external files
- Gradual adoption: presets are optional, not required

**Cons:**
- Some duplication across styles (preset names repeated)
- Requires defining and documenting preset vocabulary
- Migration must map CSL 1.0 macro patterns to preset names
- Presets may not cover all edge cases (escape hatches needed)

**Implementation Considerations:**

Presets would expand to `Config` values at parse time or processing time:

```rust
impl ContributorPreset {
    pub fn config(&self) -> ContributorConfig {
        match self {
            ContributorPreset::Apa => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Symbol),
                shorten: Some(ShortenListOptions {
                    min: 21,
                    use_first: 19,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::Chicago => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Text),
                ..Default::default()
            },
            // ...
        }
    }
}
```

**Verdict:** Strongly aligned with CSLN's "explicit over magic" principle.

---

### Option C: Shared Template Libraries

Allow styles to import template components from external libraries:

```yaml
title: ABC Journal
imports:
  - library: csln-templates
    templates:
      - apa-bibliography-article
      - apa-bibliography-book

bibliography:
  type_templates:
    article-journal:
      use: apa-bibliography-article
    book:
      use: apa-bibliography-book
```

**Pros:**
- Maximum reuse of complex template logic
- Similar conceptually to CSL 1.0 macro system
- Allows sharing sophisticated type-specific templates

**Cons:**
- **External dependencies**: where do template libraries live?
- Version management complexity: library updates could break styles
- Less explicit than presets—must look up external templates
- Harder to validate/lint without resolving dependencies
- Network/filesystem access needed at parse time

**Verdict:** Adds too much complexity for the benefit. Most reuse needs are better served by presets (configuration) rather than shared templates (structure).

---

### Option D: Hybrid (Recommended)

Combine **presets for options** with **embedded priority templates**:

1. **Presets for configuration**: `processing`, `contributors`, `dates`, `titles` per Issue #89
2. **Embedded priority templates**: Top 10 parent style templates as Rust constants (optional, for migration convenience)
3. **No parent/child aliasing**: Each style is self-contained

```yaml
title: ABC Journal
options:
  processing: author-date
  contributors: apa
  dates: year-only
  titles:
    component:
      quote: true
    periodical:
      emph: true

# Full template for customization (or use defaults)
bibliography:
  template:
    - contributor: author
    - date: issued
      wrap: parentheses
    - title: primary
    # ...
```

For simple journal styles that exactly match a parent, migration could emit:

```yaml
title: Journal of Example Studies
info:
  description: Uses APA 7th edition formatting

# Minimal configuration using presets
options:
  processing: author-date
  contributors: apa
  dates: apa
  titles: apa

# Could reference embedded template (optional feature)
citation:
  use-template: apa-citation
bibliography:
  use-template: apa-bibliography
```

**Rationale:**
- **Presets** handle 90% of configuration without duplication
- **Embedded templates** (optional) provide battle-tested defaults for priority styles
- **No aliasing** keeps styles explicit and portable
- Aligns with "explicit over magic" principle
- Presets are self-documenting (enum values, not opaque strings)

---

## Recommendation

**Implement Option D (Hybrid)** with the following phases:

### Phase 1: Define Preset Vocabulary

Add preset enums to `csln_core::options`:

```rust
/// Processing mode presets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessingPreset {
    /// Disambiguation, year-suffix, author-year grouping
    AuthorDate,
    /// Citation-order, no disambiguation
    Numeric,
    /// Position tracking, ibid handling
    Note,
}

/// Contributor formatting presets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContributorPreset {
    /// First author family-first, "&" symbol, et al. after 20 authors
    Apa,
    /// First author family-first, "and" text, no serial comma
    Chicago,
    /// All authors family-first, no conjunction
    Vancouver,
    /// All authors family-first, "and" before last
    Ieee,
    /// First author family-first, "and" text (Elsevier/Springer variant)
    Harvard,
}

/// Date formatting presets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DatePreset {
    /// Year only: (2024)
    YearOnly,
    /// Full date: (January 15, 2024)
    Full,
    /// Month and year: (January 2024)
    MonthYear,
    /// ISO format: (2024-01-15)
    Iso,
}

/// Title formatting presets
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TitlePreset {
    /// Component titles plain, monograph titles italic
    Apa,
    /// Component titles quoted, monograph titles italic
    Chicago,
    /// Component titles quoted, monograph titles italic
    Ieee,
    /// All titles italic
    Scientific,
}
```

**Important:** Use doc comments (`///`) not regular comments (`//`) for enum variants. With `schemars`, doc comments become `description` fields in the JSON schema, which editors show as tooltips. This helps style authors who may not know style guide names but understand the formatting behavior.

Each preset expands to concrete `Config` values. Style authors can:
1. Use a preset name for defaults
2. Override individual fields as needed
3. Skip presets entirely and specify everything explicitly

### Phase 2: Embed Priority Templates

For the top 10 parent styles, embed templates as Rust constants:

```rust
pub mod embedded {
    use super::*;

    pub const APA_BIBLIOGRAPHY: &[TemplateComponent] = &[
        TemplateComponent::Contributor(ContributorComponent {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            ..Default::default()
        }),
        // ...
    ];
}
```

This is **core infrastructure** for:
- Faster migration of simple dependent styles
- Reference implementations for testing
- Fallback defaults when template is omitted


### Phase 3: Migration Updates

Update `csln_migrate` to:
1. Detect when a style matches a known preset pattern
2. Emit preset names instead of fully-expanded configuration
3. For simple dependent styles, emit preset-only styles

Example migration output for a simple dependent style:

**Before (CSL 1.0 dependent style):**
```xml
<style>
  <info>
    <title>Journal of Example Studies</title>
    <link href="http://www.zotero.org/styles/apa" rel="independent-parent"/>
  </info>
</style>
```

**After (CSLN with presets):**
```yaml
info:
  title: Journal of Example Studies
  id: journal-of-example-studies
options:
  processing: author-date
  contributors: apa
  dates: apa
  titles: apa
citation:
  use-preset: apa
bibliography:
  use-preset: apa
```

---

## Design Decisions

### 1. Presets are Enums

**Decision:** Presets are implemented as Rust enums.
- Type-safe, IDE autocomplete
- Compile-time validation
- Clear documentation of available options
- Uses `#[non_exhaustive]` for forward compatibility.

### 2. Preset Versioning

**Decision:** Use suffixed names (e.g., `apa-6`) with un-suffixed names defaulting to the latest version (e.g., `apa` = APA 7). This is explicit and grep-able.

### 3. Embedded Templates are Core

**Decision:** Embedded templates are core infrastructure, not optional.
- Ensures consistent behavior across all implementations
- Simplifies migration tooling
- Always included (no feature gate)

### 4. Explicit Configuration Overrides Presets

**Decision:** When a preset is specified alongside explicit fields, explicit fields override the preset values.

```yaml
options:
  contributors: apa          # Preset: and = symbol
  contributors:
    and: text                # Override: and = text
```

This matches biblatex's option layering pattern.

---

## Comparison with CSL 1.0 Macros

CSL 1.0 macros are procedural code blocks that can be called from templates:

```xml
<macro name="author">
  <names variable="author">
    <name name-as-sort-order="first" and="symbol"/>
    <substitute>
      <names variable="editor"/>
      <text variable="title"/>
    </substitute>
  </names>
</macro>
```

**Macros are templates.** They define the structure of output.

**Presets are configuration.** They define options that affect how templates render.

This is the key distinction:
- CSL 1.0: Reuse through procedural macro calls
- CSLN: Reuse through declarative preset configuration

The CSLN approach separates "what to render" (templates) from "how to render" (options/presets), making styles more understandable and customizable.

---

## Impact on Personas

### Style Author
- Simpler style creation using preset names
- Override individual settings as needed
- No need to understand parent style internals

### Web Developer
- Preset enums are enumerable for dropdowns
- Schema validates preset names at parse time
- Clear documentation of available presets

### Systems Architect
- Type-safe Rust enums, not stringly-typed
- Presets expand to `Config` at well-defined points
- No external dependencies to manage

### Domain Expert
- Presets encode domain knowledge (APA rules, Chicago conventions)
- Versioned presets preserve historical accuracy
- Escape hatches available for edge cases

---

## Implementation Status

### Completed ✅

1. **Phase 1: Preset vocabulary** (PR #37)
   - Added `ContributorPreset`, `DatePreset`, `TitlePreset` enums in `csln_core::presets`
   - Each preset has a `config()` method to expand to concrete values

2. **Phase 2: Embedded templates** (PR #38)
   - Added `csln_core::embedded` module (always included)
   - Pre-built citation and bibliography templates for APA, Chicago, Vancouver, IEEE, Harvard

3. **Phase 3: Migration updates** (PR #40)
   - Added `csln_migrate::preset_detector` module
   - Detection functions: `detect_contributor_preset()`, `detect_title_preset()`, `detect_date_preset()`

### Remaining Work

1. **Expose embedded templates to style authors** (Issue #39)
   - Add `use-preset: apa` syntax to `CitationSpec` and `BibliographySpec`
   - Currently embedded templates are internal infrastructure only

2. **Implement preset expansion in processor**
   - Resolve preset names to concrete configs at runtime

3. **Document preset vocabulary** for style authors
   - Add examples to README and/or schema descriptions

---

## References

- [PRIOR_ART.md](../.agent/PRIOR_ART.md) - Presets concept (Issue #89)
- [STYLE_PRIORITY.md](../docs/STYLE_PRIORITY.md) - Parent style analysis
- [CLAUDE.md](../CLAUDE.md) - "Explicit over magic" principle
- Issue #89: Presets proposal
- biblatex manual: Option layering architecture
