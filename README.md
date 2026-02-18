# CSLN

**A next-generation citation styling system for the scholarly ecosystem.**

CSLN is a ground-up reimagining of the [Citation Style Language](https://citationstyles.org/) (CSL), designed to make citation styles easier to write, maintain, and reason about—while remaining fully compatible with the existing ecosystem of 10,000+ styles.

## Table of Contents

- [Why CSLN?](#why-csln)
  - [The Problem with CSL 1.0](#the-problem-with-csl-10)
  - [The CSLN Solution](#the-csln-solution)
- [Key Design Principles](#key-design-principles)
- [Project Status](#project-status)
- [Architecture](#architecture)
- [For Style Maintainers](#for-style-maintainers)
- [For Developers](#for-developers)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

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

## Key Features

- **Declarative Templates**: High-level components (`contributor`, `date`, `title`) replace procedural logic.
- **Three-Tier Options**: Context-aware formatting (global, citation/bibliography, and type-specific).
- **Oracle Verification**: Built-in scripts to compare output against `citeproc-js` for exact fidelity.
- **Modern Input**: Native support for CSLN YAML/JSON bibliography format with EDTF date support.
- **High Performance**: Native support for **CBOR binary format** for lightning-fast style and bibliography loading.
- **Diverse Fixtures**: Built-in 10-item test dataset covering edge cases like massive author lists and missing dates.

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

### 4. Migration in Progress

CSLN uses a hybrid migration strategy combining XML options extraction, output-driven template inference, and hand-authored templates for the highest-impact styles. Every CSL 1.0 style can be processed, with correctness verified against [citeproc-js](https://github.com/Juris-M/citeproc-js), the reference CSL implementation. See [Migration Strategy](#migration-strategy) for details.

### 5. High-Fidelity Data

CSLN prevents data loss by supporting:
- **EDTF Dates**: ranges, uncertainty, and approximations
- **Rich Text/Math**: mathematical notation and strict Unicode handling
- **Multilingualism**: scoped fields for multi-script data

### 6. Hybrid Architecture

The engine is built for dual-mode operation:
- **Batch**: High-throughput CLI for build systems (like Pandoc)
- **Interactive**: Low-latency JSON server mode for reference managers (like Zotero). Supports binary formats (CBOR) to minimize startup latency.

### 7. Stability & Type Safety

CSLN is built for a long-lived ecosystem with strict type safety:
- **Explicit Versioning**: Styles include a `version` field for unambiguous schema identification.
- **Strict Validation**: The engine uses `deny_unknown_fields` to catch typos and invalid fields at parse time, providing clear error messages.
- **Explicit Extension Points**: Styles can use explicit `custom` fields for user-defined metadata and extensions, making the intent clear.
- **Type-Safe Schema**: Rust's type system ensures styles are validated at parse time, preventing runtime errors from malformed data.

## Project Status

> **Note**: This project is in active development. While the core architecture is solid, rendering fidelity across the full corpus of 2,844 styles is still a work in progress.

| Component | Status |
|-----------|--------|
| CSL 1.0 Parser (`csl_legacy`) | ✅ Complete - parses all 2,844 official styles |
| CSLN Schema (`csln_core`) | ✅ Complete - options, templates, locale, rendering |
| Migration Tool (`csln_migrate`) | ✅ Complete (hybrid) - XML options, output-driven templates, hand-authoring |
| CSLN Processor (`csln_processor`) | 🔄 In Progress - APA 7th verified (5/5 citation + bibliography), top 10 in progress |
| Oracle Verification | ✅ Infrastructure complete - citeproc-js comparison, template inference |
| Corpus Analyzer (`csln_analyze`) | ✅ Complete - feature usage stats for 2,844 styles |

## Style Management

To ensure high performance and maintainable history, CSLN follows a hybrid style management strategy:

- **Core Styles (In-Repo)**: This repository maintains the top ~20 "parent" styles (APA, Chicago, IEEE, Vancouver, etc.) and edge-case test styles. These serve as our primary integration test suite.
- **Community Styles (Submodule)**: The broader ecosystem of 2,000+ journal-specific styles is managed in a separate repository (e.g., `csln-styles`) and linked as a git submodule.

This approach keeps the core repository lean while providing a tight development loop for the most impactful styles.

### Current Test Results

The hybrid migration strategy has been validated with the following results:

**APA 7th Edition** (hand-authored): 5/5 citations ✅, 5/5 bibliography ✅ (exact match)

**Batch Testing** (50 styles):
- Citations: 74% with 5/5 match (XML options extraction)
- Bibliography: Output-driven inference tested on 6 major styles, validated component ordering and type-specific suppression logic
- Errors: 0 migration errors, 0 processor errors

**Features Implemented**:
- ✅ XML options extraction (87-100% citation accuracy): initialize-with, name-as-sort-order, et-al rules, page-range-format, delimiter logic
- ✅ Output-driven template inference: component ordering, delimiter detection, type-specific overrides
- ✅ Hand-authored styles: APA 7th as gold standard; top 5-10 parent styles in progress
- ✅ Type-specific overrides: publisher suppression, page formatting, contributor ordering per reference type
- ✅ Page label extraction: "pp." from CSL Label nodes
- ✅ Pluggable output formats: plain text, HTML, Djot with semantic class wrapping

**Known gaps** (documented in test fixture):
- Rare reference types (legal, patent, dataset) need expanded test coverage
- Locale term disambiguation (locale vs. hardcoded prefix)
- Latent features (substitute rules, disambiguation) require XML or hand-authoring

## Architecture

```
crates/
├── csl_legacy/      # CSL 1.0 XML parser (read-only)
│   └── src/
│       ├── csl_json.rs  # CSL JSON import/export
│       ├── model.rs     # Legacy XML schema types
│       └── parser.rs    # XML parsing logic
├── csln/            # Main CLI entry point
├── csln_analyze/    # Corpus-wide analysis and batch testing
│   └── src/
│       ├── analyzer.rs  # Style feature statistics
│       ├── batch_test.rs # Oracle comparison runner
│       ├── ranker.rs    # Parent style ranking logic
│       └── main.rs      # CLI entry point
├── csln_core/       # CSLN schema and core types
│   ├── src/
│   │   ├── citation.rs  # Citation model
│   │   ├── embedded/    # Style presets (APA, Chicago, etc.)
│   │   ├── legacy.rs    # CSL 1.0 legacy type bridge
│   │   ├── locale/      # Localization (terms, dates, raw mapping)
│   │   ├── options/     # Style configuration groups
│   │   ├── presets.rs   # Named configuration bundles
│   │   ├── renderer.rs  # Rendering orchestration
│   │   ├── template.rs  # Template components
│   │   └── reference/   # Internal reference model
├── csln_edtf/       # Extended Date/Time Format (EDTF) handling
├── csln_migrate/    # CSL 1.0 → CSLN converter
│   └── src/
│       ├── options_extractor/ # Extracts config from XML
│       ├── template_compiler/ # Compiles XML macros to CSLN templates
│       ├── upsampler.rs       # XML to CSLN Node mapping
│       ├── analysis/          # Style-specific feature detection
│       └── passes/            # Transformation passes
└── csln_processor/  # Citation/bibliography renderer
    └── src/
        ├── processor/   # Core logic (disambiguation, matching, sorting)
        ├── values/      # Field-level extraction and formatting
        └── render/      # String rendering (plain, HTML, Djot)

.beans/              # Local task management
.claude/             # Agent instructions and design documents
locales/             # CSLN YAML locale files (en-US, de-DE, etc.)
scripts/             # Oracle verification (citeproc-js) and automation
styles/              # CSLN YAML styles
styles-legacy/       # 2,844 CSL 1.0 styles (submodule)
```

## Migration Strategy

CSLN uses a **hybrid approach** combining the strengths of three migration strategies:

1. **XML Options Extraction** - The XML compiler excels at extracting global options (name formatting, et-al rules, initialization, date forms). This is why citations achieve 87-100% accuracy out of the box.

2. **Output-Driven Template Inference** - For template structure (which components, in what order, with which delimiters), observing actual rendered output is more reliable than parsing 126+ nested conditionals in CSL 1.0. The template inferrer has been validated to correctly identify component ordering, type-specific suppression, and delimiter consensus across 6 major styles.

3. **Hand-Authored Styles** - For the top 5-10 parent styles covering 60% of the ecosystem (like APA 7th), a human domain expert or LLM-assisted author creates gold-standard CSLN templates using official style guides. APA 7th has been validated with 5/5 citation and bibliography matches.

**Why hybrid?**
- XML options extraction handles what it does well (global config), while being abandoned where it fails (template structure)
- Output-driven inference bypasses the procedural-to-declarative translation bottleneck
- Hand-authored styles guarantee correctness for the highest-impact cases

See [MIGRATION_STRATEGY_ANALYSIS.md](./docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md) for detailed trade-off analysis and validation results.

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

For existing CSL 1.0 styles, CSLN provides multiple migration options:

**Automated Migration** (for low-priority styles):
```bash
# XML-based migration extracts options and compiles templates
cargo run --bin csln-migrate -- styles-legacy/apa.csl

# Output: csln-new.yaml with XML-derived options and compiled template
# Note: Options are accurate (87-100% citations), templates may need refinement
```

**LLM-Assisted Hand-Authoring** (for top parent styles):
```bash
# Prepare context for LLM-assisted authoring
./scripts/prep-migration.sh styles-legacy/apa.csl

# Use the /styleauthor skill or @styleauthor agent to create templates
# from the provided context (citeproc-js output + reference data)
```

**Validation** (all approaches):
```bash
# Verify against citeproc-js (the reference implementation)
node scripts/oracle.js styles-legacy/apa.csl
node scripts/oracle-e2e.js styles-legacy/apa.csl

# Generate styles with test fixture
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10
```

### Using Presets

CSLN includes embedded templates for common styles (APA, Chicago, Vancouver, IEEE, Harvard). Instead of defining a template from scratch, you can reference a preset:

```yaml
citation:
  use-preset: apa

bibliography:
  use-preset: vancouver
```

This effectively "inherits" the standard template for that style, which you can then customize with options.

## For Developers

### Building

```bash
git clone https://github.com/bdarcus/csl26
cd csl26
cargo build --workspace
cargo test --workspace

# Run functional integration tests for the processor
cargo nextest run --test citations
cargo nextest run --test bibliography
```

### Running the Processor

The `csln` binary is the primary entry point for processing and conversion.

```bash
# Render references with a style (default plain text, citations + bibliography)
csln render refs -b references.json -s styles/apa-7th.yaml

# Show reference keys/IDs for debugging (e.g. [ITEM-1])
csln render refs -b references.json -s styles/apa-7th.yaml --show-keys

# Generate semantic HTML
csln render refs -b references.json -s styles/apa-7th.yaml -O html

# Generate Djot with semantic attributes
csln render refs -b references.json -s styles/apa-7th.yaml -O djot

# Render a full Djot document with bibliography appended
csln render doc -i examples/document.djot -b examples/document-refs.json -s styles/apa-7th.yaml -I djot -O html

# Validate style/bibliography/citations files
csln check -s styles/apa-7th.yaml -b references.json -c citations.yaml
```

### Format Conversion

CSLN supports YAML, JSON, and CBOR. Use the `convert` command to switch between them.

```bash
# Convert a YAML style to binary CBOR (performance mode)
csln convert styles/apa-7th.yaml --output styles/apa-7th.cbor

# Convert a large JSON bibliography to CBOR
csln convert references.json --output references.cbor

# Convert a CBOR locale back to YAML for editing
csln convert locales/en-US.cbor --output locales/en-US.yaml
```

### Style Corpus Analysis

The `csln_analyze` tool scans all CSL 1.0 styles to identify patterns and gaps:

```bash
# Analyze all styles in the styles-legacy/ directory
cargo run --bin csln-analyze -- styles-legacy/

# Output as JSON for scripting
cargo run --bin csln-analyze -- styles-legacy/ --json
```

This helps prioritize which features to implement based on actual usage across 2,844 styles.

### Oracle Verification (citeproc-js)

The `scripts/` directory contains tools to verify CSLN output against citeproc-js, the reference CSL 1.0 implementation.

```bash
cd scripts
npm install   # First time only - installs citeproc

# oracle.js - Render citations/bibliography with citeproc-js
node oracle.js ../styles-legacy/apa.csl              # Both citations and bibliography
node oracle.js ../styles-legacy/apa.csl --cite       # Citations only
node oracle.js ../styles-legacy/apa.csl --bib        # Bibliography only
node oracle.js ../styles-legacy/apa.csl --json       # JSON output for scripting

# oracle-e2e.js - End-to-end migration test
# Migrates CSL 1.0 → CSLN → csln render refs, then compares with citeproc-js
node oracle-e2e.js ../styles-legacy/apa.csl
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

### Benchmarking

CSLN uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical performance benchmarking.

```bash
# Run all benchmarks
cargo bench

# Run format comparison benchmark (YAML vs JSON vs CBOR)
cargo bench -p csln_core --bench formats
```

Benchmarks are currently focused on deserialization performance for styles and bibliographies. Current results show CBOR and JSON outperforming YAML by 3-4x.

### JSON Schema Generation

You can generate formal JSON Schemas for all CSLN models using the CLI:

```bash
# Output specific schema to stdout (style, bib, locale, citation)
csln schema style
csln schema bib

# Save all schemas to a directory
csln schema --out-dir ./schemas
```

These schemas can be used to validate your files or provide intellisense in editors like VS Code.

## Roadmap

### Near-term
- [x] Bibliography formatting (page ranges, subsequent author substitute)
- [ ] Complete bibliography formatting (complex punctuation, affixes)
- [ ] Resolve high-frequency gaps identified by `csln_analyze`
- [ ] Automated verification pipeline for top 100 styles
- [x] Schema versioning and forward compatibility
- [ ] Bulk migration of all 2,844 styles

### Medium-term
- [ ] WASM build for browser use
- [x] Additional locales (de-DE, fr-FR, tr-TR, etc.)
- [x] Style presets vocabulary (see [STYLE_ALIASING.md](./docs/architecture/design/STYLE_ALIASING.md))
- [x] Embedded priority templates (APA, Chicago, Vancouver, IEEE, Harvard)
- [x] Preset-aware migration (emit preset names instead of expanded config)
- [ ] Note-bibliography citation style support

### Long-term
- [ ] CSLN 1.0 specification
- [ ] Visual style editor
- [ ] Integration guides for reference managers

## Contributing

CSLN follows an **AI-first development model**. The core CSLN schema and data model was designed by the project maintainer, and AI agents (like Claude Code) have adapted and extended this work to build out the migration tooling, processor, and analysis infrastructure. This approach lowers the barrier to entry, allowing the most valuable contributions to come from **Domain Experts** and **Style Authors** rather than just systems programmers.

### How to Contribute

The most impactful way to contribute is by providing the "raw material" that the AI needs to understand and solve complex citation problems:

- **Surface Real-World Gaps**: Describe formatting requirements or edge cases that current systems (including CSL 1.0) handle poorly.
- **Provide Contextual Resources**: Shared style guides, official manuals, and sample documents are high-value inputs that allow the LLM to extract logic and implement it.
- **Refine Instructions**: Help improve the "identity" and "skills" of the AI agents by suggesting updates to the `.agent` directory.
- **Report Pain Points**: Use GitHub issues to describe what is difficult or counter-intuitive in the current CSLN model.

### AI-Augmented Workflow

We treat GitHub Issues as **Context Packets** for our AI agents. Here is the current lifecycle:

1. **Context Submission**: A Domain Expert submits an issue with dense context (e.g., "Legal citations in this jurisdiction require X, see attached PDF").
2. **Agent Activation**: A project maintainer activates an AI agent (using tools like `antigravity` or `gemini`) initialized with the [Domain Expert Persona](./docs/architecture/PERSONAS.md).
3. **Implementation**: The agent reads the issue, extracts the rules, and generates the necessary Rust code, YAML schema changes, or tests.
4. **Verification**: The Code and tests are verified against the Oracle (citeproc-js) to ensure correctness.

*Note: While maintainers currently trigger these agents manually, we are actively developing workflows to automate this loop directly from GitHub Actions.*

### For Developers

If you want to contribute code directly, focus on:
- **Core Engine Architecture**: Improving the performance and correctness of `csln_processor`.
- **Schema Design**: Ensuring `csln_core` remains robust and extensible.
- **Agent Tooling**: Developing new "skills" or scripts that enhance the autonomy and capabilities of the AI agents.

## Task Management

Active development uses [beans](https://github.com/jdx/beans) for local task tracking (see `.beans/` directory). GitHub Issues remain open for:

- **Community bug reports**: Submit issues when you find rendering defects or incorrect output
- **Feature requests**: Propose new capabilities or improvements
- **Public discussion**: Comment on planned work and provide domain expertise

### For Contributors

Current development tasks are tracked locally as beans. If you see a GitHub issue marked with a migration note, the work is actively being tracked in the `.beans/` directory. The issue will be closed when the work is completed.

### For Maintainers

Use the `/bean` skill (see `.claude/skills/bean/SKILL.md`) for local task management:

```bash
/bean list              # Show all tasks
/bean next              # Get recommended task
/bean show BEAN_ID      # View details
/bean update BEAN_ID --status completed
```

All beans are git-tracked markdown files with dependency relationships and priority levels.

## License

MPL-2.0 - see [LICENSE](LICENSE) for details.

## Acknowledgments

CSLN builds on the foundation laid by the CSL community over 15+ years. Special thanks to:
- Frank Bennett (citeproc-js)
- The CSL specification authors
- Thousands of style contributors

---

*CSLN: Citation styles should be data, not programs.*
