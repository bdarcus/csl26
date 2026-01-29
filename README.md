# CSL Next (CSLN)

**A next-generation citation styling system for the scholarly ecosystem.**

CSLN is a ground-up reimagining of the [Citation Style Language](https://citationstyles.org/) (CSL), designed to make citation styles easier to write, maintain, and reason about—while remaining fully compatible with the existing ecosystem of 10,000+ styles.

## Why CSLN?

CSL 1.0 has been tremendously successful. It powers citation formatting in Zotero, Mendeley, Pandoc, and countless other tools. But after 15+ years of evolution, the XML-based format has accumulated complexity that makes styles difficult to author and maintain.

### The Problem with CSL 1.0

Consider this excerpt from APA 7th edition (`apa.csl`):

```xml
<macro name="author">
  <names variable="author">
    <name and="symbol" initialize-with=". " delimiter=", "/>
    <label form="short" prefix=" (" suffix=")" text-case="capitalize-first"/>
    <substitute>
      <names variable="editor"/>
      <names variable="translator"/>
      <choose>
        <if type="report">
          <text variable="publisher"/>
          <text macro="title"/>
        </if>
        <else-if type="legal_case">
          <text variable="title"/>
        </else-if>
        <!-- ... 50 more lines of conditionals ... -->
      </choose>
    </substitute>
  </names>
</macro>
```

This is **procedural code disguised as data**. The style embeds:
- Control flow (`<choose>`, `<if>`, `<else-if>`)
- Iteration (implicit in `<names>`)
- Fallback logic (`<substitute>`)
- Type-specific overrides scattered throughout

When you multiply this across an entire style file, you get **3,000+ lines of XML** that are nearly impossible to diff, review, or extend.

### The CSLN Solution

CSLN separates **what** from **how**:

```yaml
# csln-apa.yaml
info:
  title: APA 7th Edition

options:
  processing: author-date
  substitute:
    template: [editor, translator, title]
    contributor-role-form: short
  contributors:
    display-as-sort: first
    and: symbol
    shorten:
      min: 3
      use-first: 1

citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year

bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
      emph: true
```

**50 lines instead of 3,000.** The same semantic information, expressed declaratively.

## Key Design Principles

### 1. Declarative Over Procedural

Instead of encoding logic in the style, CSLN styles declare *intent*. The processor implements the logic once, correctly.

| CSL 1.0 | CSLN |
|---------|------|
| `<choose><if type="book">...</if></choose>` | `overrides: { book: { emph: true } }` |
| `<names><substitute>...</substitute></names>` | `options.substitute.template: [editor, title]` |
| 20 lines of et-al logic | `shorten: { min: 3, use-first: 1 }` |

### 2. Options First

Common behaviors are extracted to configuration, not scattered through templates:

- **Contributor formatting**: initialization, sorting, et-al rules
- **Date formatting**: precision, localization
- **Substitution**: what to show when author is missing
- **Processing mode**: author-date vs. note-based

### 3. Type-Safe Schema

CSLN uses strongly-typed enums, not strings:

```rust
pub enum ContributorRole {
    Author, Editor, Translator, Director, // ...
}

pub enum TitleType {
    Primary, ParentSerial, ParentMonograph,
}
```

Typos become compile errors. Invalid combinations are impossible.

### 4. Full Backward Compatibility

Every CSL 1.0 style can be automatically migrated to CSLN. We verify correctness by comparing output against [citeproc-js](https://github.com/Juris-M/citeproc-js), the reference CSL implementation.

## Project Status

| Component | Status |
|-----------|--------|
| CSL 1.0 Parser (`csl_legacy`) | ✅ Complete - parses all 2,844 official styles |
| CSLN Schema (`csln_core`) | ✅ Complete - options, templates, locale, rendering |
| Migration Tool (`csln_migrate`) | ✅ Complete - extracts options, compiles templates |
| CSLN Processor (`csln_processor`) | ✅ APA 5/5 match - citations and bibliography verified |
| Oracle Verification | ✅ APA verified against citeproc-js |
| Corpus Analyzer (`csln_analyze`) | ✅ Complete - feature usage stats for 2,844 styles |

### Current Test Results

```
18 unit tests passing
APA 7th: 5/5 citations, 5/5 bibliography (exact match)

Features implemented:
✓ page-range-format (1,076 styles) - expanded, minimal, chicago
✓ delimiter-precedes-et-al (786 styles) - always, never, contextual  
✓ initialize-with (1,437 styles) - name initialization
✓ name-as-sort-order (2,100+ styles) - family-first ordering
✓ is-uncertain-date handling - [1962?] format
✓ disambiguate-add-givenname (935 styles) - name expansion
✓ disambiguate-add-names (1,241 styles) - et-al expansion

Remaining high-priority:
○ subsequent-author-substitute (314 styles)
```

## Architecture

```
crates/
├── csl_legacy/      # CSL 1.0 XML parser (read-only)
├── csln_core/       # CSLN schema and types
│   ├── options.rs   # Style configuration
│   ├── template.rs  # Template components
│   └── locale.rs    # Localization (terms, dates)
├── csln_migrate/    # CSL 1.0 → CSLN converter
│   ├── options_extractor.rs
│   └── template_compiler.rs
└── csln_processor/  # Citation/bibliography renderer
    ├── processor.rs # Core processing logic
    ├── values.rs    # Value extraction
    ├── render.rs    # String formatting
    └── main.rs      # CLI tool

.agent/              # LLM agent instructions
scripts/             # Oracle verification (citeproc-js)
styles/              # 2,844 CSL 1.0 styles (submodule)
```

## For Style Maintainers

If you maintain CSL styles, here's what CSLN means for you:

### Easier Maintenance

- **Readable diffs**: Changes are obvious in YAML
- **No macro hunting**: All behavior is visible in one place
- **Validation**: Schema catches errors before runtime

### Familiar Concepts

CSLN uses the same conceptual model as CSL:
- Contributors (author, editor, translator)
- Dates (issued, accessed)
- Titles (primary, container)
- Numbers (volume, issue, pages)

### Migration Path

```bash
# Convert an existing CSL style
cargo run --bin csln_migrate -- styles/apa.csl

# Output: csln-new.yaml with clean CSLN format
```

## For Developers

### Building

```bash
git clone https://github.com/bdarcus/csl26
cd csl26
cargo build --workspace
cargo test --workspace
```

### Running the Processor

```bash
# Run CSLN processor with a style
cargo run --bin csln_processor -- examples/apa-style.yaml
```

### Style Corpus Analysis

The `csln_analyze` tool scans all CSL 1.0 styles to identify patterns and gaps:

```bash
# Analyze all styles in the styles/ directory
cargo run --bin csln_analyze -- styles/

# Output as JSON for scripting
cargo run --bin csln_analyze -- styles/ --json
```

This helps prioritize which features to implement based on actual usage across 2,844 styles.

### Oracle Verification (citeproc-js)

The `scripts/` directory contains tools to verify CSLN output against citeproc-js, the reference CSL 1.0 implementation.

```bash
cd scripts
npm install   # First time only - installs citeproc

# oracle.js - Render citations/bibliography with citeproc-js
node oracle.js ../styles/apa.csl              # Both citations and bibliography
node oracle.js ../styles/apa.csl --cite       # Citations only
node oracle.js ../styles/apa.csl --bib        # Bibliography only
node oracle.js ../styles/apa.csl --json       # JSON output for scripting

# oracle-e2e.js - End-to-end migration test
# Migrates CSL 1.0 → CSLN → csln_processor, then compares with citeproc-js
node oracle-e2e.js ../styles/apa.csl
```

Example output from `oracle-e2e.js`:
```
=== End-to-End Oracle Test: apa ===

--- CITATIONS ---
  ✅ ITEM-1
  ✅ ITEM-2
  ✅ ITEM-3
  ✅ ITEM-4
  ✅ ITEM-5

Citations: 5/5 match
```

### Crate Documentation

```bash
cargo doc --workspace --open
```

## Roadmap

### Near-term
- [ ] Complete bibliography formatting (page ranges, punctuation)
- [ ] Full APA test suite verification
- [ ] Chicago author-date style support
- [ ] Bulk migration of all 2,844 styles

### Medium-term
- [ ] WASM build for browser use
- [ ] Additional locales (de-DE, fr-FR, etc.)
- [ ] Note-bibliography citation style support

### Long-term
- [ ] CSLN 1.0 specification
- [ ] Visual style editor
- [ ] Integration guides for reference managers

## Contributing

We welcome contributions! Areas where help is especially valuable:

- **Style testing**: Run your favorite styles through migration and report issues
- **Processor features**: Implement remaining template component types
- **Documentation**: Improve examples and guides
- **Locales**: Help with internationalization

## License

MPL-2.0 - see [LICENSE](LICENSE) for details.

## Acknowledgments

CSLN builds on the foundation laid by the CSL community over 15+ years. Special thanks to:
- Frank Bennett (citeproc-js)
- The CSL specification authors
- Thousands of style contributors

---

*CSLN: Citation styles should be data, not programs.*
