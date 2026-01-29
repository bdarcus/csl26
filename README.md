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

### 5. High-Fidelity Data

CSLN prevents data loss by supporting:
- **EDTF Dates**: ranges, uncertainty, and approximations
- **Rich Text/Math**: mathematical notation and strict Unicode handling
- **Multilingualism**: scoped fields for multi-script data

### 6. Hybrid Architecture

The engine is built for dual-mode operation:
- **Batch**: High-throughput CLI for build systems (like Pandoc)
- **Interactive**: Low-latency JSON server mode for reference managers (like Zotero)

### 7. Stability & Forward Compatibility

CSLN is built for a long-lived ecosystem:
- **Explicit Versioning**: Styles include a `version` field for unambiguous schema identification.
- **Permissive Runtime**: The engine ignores unknown fields, allowing older versions of the processor to run newer styles gracefully.
- **Round-trip Safety**: Unknown fields are captured during parsing and preserved during serialization, ensuring no data loss when editing with different tool versions.
- **Strict Linting**: While the runtime is permissive, development tools (like `csln_analyze`) are strict, catching typos and deprecated fields.

## Project Status

| Component | Status |
|-----------|--------|
| CSL 1.0 Parser (`csl_legacy`) | ✅ Complete - parses all 2,844 official styles |
| CSLN Schema (`csln_core`) | ✅ Complete - options, templates, locale, rendering, versioning |
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
✓ initialize-with-hyphen - hyphenated initials support
✓ small-caps (498 styles) - font-variant: small-caps support
✓ name-as-sort-order (2,100+ styles) - family-first ordering
✓ is-uncertain-date handling - [1962?] format
✓ disambiguate-add-givenname (935 styles) - name expansion
✓ disambiguate-add-names (1,241 styles) - et-al expansion
✓ subsequent-author-substitute (314 styles) - "———" replacement

Remaining high-priority:
○ (None - Core feature set complete)
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

CSLN follows an **AI-first development model**. The majority of the codebase is authored by AI agents (like Gemini and Antigravity) guided by human contributors. This approach lowers the barrier to entry, allowing the most valuable contributions to come from **Domain Experts** and **Style Authors** rather than just systems programmers.

### How to Contribute

The most impactful way to contribute is by providing the "raw material" that the AI needs to understand and solve complex citation problems:

- **Surface Real-World Gaps**: Describe formatting requirements or edge cases that current systems (including CSL 1.0) handle poorly.
- **Provide Contextual Resources**: Shared style guides, official manuals, and sample documents are high-value inputs that allow the LLM to extract logic and implement it.
- **Refine Instructions**: Help improve the "identity" and "skills" of the AI agents by suggesting updates to the `.agent` directory.
- **Report Pain Points**: Use GitHub issues to describe what is difficult or counter-intuitive in the current CSLN model.

### AI-Augmented Workflow

We treat GitHub Issues as **Context Packets** for our AI agents. Here is the current lifecycle:

1. **Context Submission**: A Domain Expert submits an issue with dense context (e.g., "Legal citations in this jurisdiction require X, see attached PDF").
2. **Agent Activation**: A project maintainer activates an AI agent (using tools like `antigravity` or `gemini`) initialized with the [Domain Expert Persona](.agent/PERSONAS.md).
3. **Implementation**: The agent reads the issue, extracts the rules, and generates the necessary Rust code, YAML schema changes, or tests.
4. **Verification**: The Code and tests are verified against the Oracle (citeproc-js) to ensure correctness.

*Note: While maintainers currently trigger these agents manually, we are actively developing workflows to automate this loop directly from GitHub Actions.*

### For Developers

If you want to contribute code directly, focus on:
- **Core Engine Architecture**: Improving the performance and correctness of `csln_processor`.
- **Schema Design**: Ensuring `csln_core` remains robust and extensible.
- **Agent Tooling**: Developing new "skills" or scripts that enhance the autonomy and capabilities of the AI agents.

## License

MPL-2.0 - see [LICENSE](LICENSE) for details.

## Acknowledgments

CSLN builds on the foundation laid by the CSL community over 15+ years. Special thanks to:
- Frank Bennett (citeproc-js)
- The CSL specification authors
- Thousands of style contributors

---

*CSLN: Citation styles should be data, not programs.*
