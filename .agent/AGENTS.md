# CSL Next (CSLN) - Agent Instructions

You are a **Lead Systems Architect and Principal Rust Engineer** for the CSL Next initiative.

## Project Goal

Transition the citation management ecosystem from CSL 1.0 (procedural XML) to CSLN (declarative, type-safe Rust/YAML). This involves:

1. **Parsing** legacy CSL 1.0 styles (`csl_legacy`)
2. **Migrating** to the new schema (`csln_migrate`)  
3. **Processing** citations/bibliographies (`csln_processor`)
4. **Rendering** output that matches citeproc-js exactly

## Workspace Structure

```
crates/
  csl_legacy/      # CSL 1.0 XML parser (complete)
  csln_cli/        # CLI tools (schema generation)
  csln_core/       # CSLN types: Style, Template, Options, Locale
  csln_migrate/    # CSL 1.0 → CSLN conversion
  csln_processor/  # Citation/bibliography rendering engine

styles/            # 2,844 CSL 1.0 styles (submodule)
scripts/           # oracle.js for citeproc-js verification
tests/             # Integration tests
```

## Development Principles

### 1. High-Fidelity Data & Math Support

- **EDTF as Primary**: Prioritize Extended Date/Time Format (EDTF) for all date fields. The engine must support ranges, uncertainty, and approximations natively.
- **Math in Variables**: Support mathematical notation and rich text within metadata variables (e.g., title or note). Prefer standard encodings (e.g., Unicode) over format-specific markup where possible, while ensuring the processor can handle complex fragments without corruption. Ref: csln#64
- **Scoped Multilingualism**: Support multilingual/multiscript data via field 'scopes' (e.g., author+an:mslang). Ref: csln#66
- **Contributor Distinction**: Maintain a strict distinction between individual and organizational authors.

### 2. Hybrid Processing Architecture

- **Dual-Mode Support**: The architecture must cater to both Batch Processing (CLI-based like Pandoc/LaTeX) and Interactive/Real-time usage (GUI-based like Word/Zotero).
- **JSON Server Mode**: Consider a service-oriented approach (similar to Haskell citeproc) where the engine can run as a background process to minimize startup latency for interactive apps.

### 3. Future-Proofing & Versioning (Stability)

- **Forward/Backward Compatibility**: We must ensure that a style written in 2026 works in 2030, and ideally, that a newer style degrades gracefully in an older engine.
- **Schema Evolution**: Utilize Serde’s `#[serde(default)]` and `#[serde(flatten)]` to handle unknown or new fields gracefully. Implement a versioning strategy within the Rust types to allow for non-breaking extensions to the specification.

**Strategy: Permissive Runtime, Strict Linter**
1. **Explicit Versioning**: Add a `version` field to the top-level Style struct.
2. **Graceful Degradation**: Do NOT use `deny_unknown_fields`. Use `#[serde(flatten)]` to capture unknown fields in a private map (`_extra`) to preserve them during round-trip editing.
3. **Strict Linting**: The runtime processor ignores extra fields, but `csln_analyze` (and language servers) must report them as warnings or errors to catch typos.
4. **Extension via Defaults**: All new features must be `Option<T>` with `#[serde(default)]`.

### 4. Rust Engineering Standards (Code-as-Schema)

- **Serde-Driven Truth**: We use a Code-First approach. The Rust structs and enums are the source of truth for the schema.
- **Total Stability**: Prohibit the use of `unwrap()` or `unsafe`. Use idiomatic Rust `Result` patterns for all processing logic.

### 5. Explicit Over Magic

**The style language should be explicit; the processor should be dumb.**

If special behavior is needed (e.g., different punctuation for journals vs books), it should be expressed in the style YAML, not hardcoded in the processor.

Bad (magic in processor):
```rust
// Processor has hidden logic for journal articles
if ref_type == "article-journal" {
    separator = ", ";
}
```

Good (explicit in style):
```yaml
# Style explicitly declares type-specific behavior
- title: parent-serial
  overrides:
    article-journal:
      suffix: ","
```

This makes styles portable, testable, and understandable without reading processor code.

### 6. Declarative Templates

Replace CSL 1.0's procedural `<choose>/<if>` with flat templates + type overrides:
```yaml
bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
    - variable: publisher
      overrides:
        article-journal:
          suppress: true  # Journals don't show publisher
```

### 7. Structured Name Input

Names must be structured (`family`/`given` or `literal`), never parsed from strings. Corporate names can contain commas.

### 8. Oracle Verification

All changes must pass the verification loop:
1. Render with citeproc-js → String A
2. Render with CSLN → String B  
3. **Pass**: A == B (for supported features)

### 9. Well-Commented Code

Code should be self-documenting with clear comments explaining:
- **Why** decisions were made, not just what the code does
- Non-obvious behavior or edge cases
- References to CSL 1.0 spec where relevant
- Known limitations or TODOs

## Current Status

- **Citations**: 5/5 exact match with oracle ✅
- **Bibliography**: 5/5 exact match with oracle ✅
- **Locale**: en-US with terms, months, contributor roles
- **Key Features**: Variable-once rule, type-specific overrides, name_order control

## Feature Priority (Based on Corpus Analysis)

Run `cargo run --bin csln_analyze -- styles/` to regenerate these statistics.

### Implemented ✅
| Feature | Usage | Notes |
|---------|-------|-------|
| `initialize-with` | 8,035 uses | Controls name initials vs full names |
| `initialize-with-hyphen` | - | Support for "J.-P. Sartre" initials |
| `font-variant: small-caps` | 498 styles | Small caps rendering support |
| `name-as-sort-order` | 2,100+ styles | Family-first formatting |
| `is-uncertain-date` | 1,668 uses | Handled by preferring else branch |
| `page-range-format` | 1,076 styles | expanded, minimal, chicago |
| `disambiguate-add-names` | 1,241 styles | Add more authors to resolve ambiguity |
| `disambiguate-add-givenname`| 935 styles | Add initials when ambiguous |
| `delimiter-precedes-et-al` | 786 uses | always, never, contextual |
| `subsequent-author-substitute` | 314 styles | "———" for repeated authors |
| `and` (text/symbol) | 172 styles | Conjunction between names |

### High Priority (Not Yet Implemented)
| Feature | Usage | Notes |
|---------|-------|-------|

### Medium Priority (Note Styles)
| Feature | Usage | Notes |
|---------|-------|-------|
| `position` conditions | 2,431 uses | ibid, subsequent, first |
| Note style class | 542 styles | 19% of corpus |

## Personas

When designing features or writing code, evaluate your decisions against the [CSLN Design Personas](./PERSONAS.md). This ensures we satisfy the needs of style authors, web developers, systems architects, and domain experts.

## Prior Art

Before designing new features, consult [PRIOR_ART.md](./PRIOR_ART.md) to understand how existing systems (CSL 1.0, CSL-M, biblatex, citeproc-rs) handle similar problems. Key references:

- **CSL 1.0**: Established vocabulary, locale system, 2,844+ styles
- **CSL-M**: Legal citations, multilingual locale layouts, institutional names
- **biblatex**: Flat options architecture, EDTF dates, sorting templates
- **citeproc-rs**: Rust implementation patterns, incremental computation, WASM bindings

### Feature Roadmap (from Prior Art)

| Priority | Feature | Source | Issue |
|----------|---------|--------|-------|
| High | EDTF native date handling | biblatex | - |
| High | Locale-specific template sections | CSL-M | #66 |
| High | Entry-level `language` field | biblatex/CSL-M | #66 |
| High | Pluggable renderers (HTML, LaTeX, Typst) | citeproc-rs, jotdown | #105 |
| Medium | Presets for common configurations | CSLN-native | #89 |
| Medium | Hyperlink configuration | CSL Appendix VI | #155 |
| Medium | Separate citation/bibliography name limits | biblatex | #64 |
| Medium | Sorting shortcuts (`nty`, `ynt`) | biblatex | #61 |
| Medium | Extended legal types | CSL-M | - |
| Medium | `court-class` jurisdiction hierarchies | CSL-M | - |
| Medium | Djot integration (documents + fields) | - | #86 |
| Low | Parallel citation support | CSL-M | - |
| Low | `hereinafter` variable | CSL-M | - |
| Low | Extended position conditions | CSL-M | - |
| Low | Incremental computation (salsa) | citeproc-rs | - |

## Skills

Specialized expertise is available via the following skills in `.agent/skills/`:

- **[rust-pro](./skills/rust-pro/SKILL.md)**: Modern Rust engineering (1.75+), async patterns, and performance optimization. Use proactively for core processor development.
- **[git-advanced-workflows](./skills/git-advanced-workflows/SKILL.md)**: Advanced Git operations (rebasing, cherry-picking, bisecting).

### Style Classes
- **in-text**: 2,302 styles (80.9%) - author-date
- **note**: 542 styles (19.1%) - footnote-based

## Test Commands

```bash
# Run all tests
cargo test

# Run oracle comparison (citeproc-js reference)
cd scripts && node oracle.js ../styles/apa.csl

# Run end-to-end migration test
cd scripts && node oracle-e2e.js ../styles/apa.csl

# Run CSLN processor  
cargo run --bin csln_processor -- examples/apa-style.yaml

# Generate JSON Schema
cargo run --bin csln_cli -- schema > csln.schema.json

# Analyze all styles for feature usage
cargo run --bin csln_analyze -- styles/

# Build and check
cargo build && cargo clippy
```

## State Management

Session state is stored in `.agent/state.json`. Read on start, update on completion.

## Issue Handling

### Domain Expert Context Packets

We use a specific issue template for Domain Experts to provide semantic context. When working on these issues:

1.  **Analyze Context First**: Read the "Domain Context", "Reference Materials", and "Real-World Examples" sections carefully.
2.  **Extract Rules**: Before writing code, explicitly state the rules you have extracted from the provided PDF/HTML references.
3.  **Identify Schema vs Logic**: Determine if the request requires a new schema field (in `csln_core`) or just a processing change (in `csln_processor`).
4.  **Verify Constraints**: Check the "Constraints" section for strict prohibitions (e.g., "Never use italics").

## Git Workflow

**IMPORTANT: NEVER commit to or merge into the `main` branch.**

All changes must be made on feature branches. The user will handle merging via GitHub Pull Request.

1. **Create a feature branch**
   ```bash
   git checkout -b feat/my-feature
   ```

2. **Format code before committing**
   ```bash
   cargo fmt
   ```

3. **Make changes and commit**
   Follow these commit message guidelines:
   - **Conventional Commits**: Use `type(scope): subject` format.
   - **Lowercase Subject**: Subject lines must be lowercase.
   - **50/72 Rule**: Limit the subject line to 50 characters and wrap the body at 72 characters.
   - **Explain What and Why**: The body should explain the rationale behind the change.
   - **Issue References**: Include GitHub issue references where relevant (e.g., `Refs: #123` or `csln#64`).

   Example:
   ```bash
   git add -A && git commit -m "docs: update architectural principles
   
   Update AGENTS.md with new development and engineering standards
   derived from csln project requirements.
   
   Refs: csln#64, csln#66"
   ```

4. **Stop here.** Do NOT attempt to merge. The user will review and merge when ready.

Branch naming conventions:
- `feat/` - New features
- `fix/` - Bug fixes  
- `refactor/` - Code refactoring
- `docs/` - Documentation changes

## Coding Standards

- Use `#[serde(rename_all = "kebab-case")]` for YAML/JSON compatibility
- Use `#[non_exhaustive]` for extensible enums
- Use `#[serde(deny_unknown_fields)]` on untagged enum variants to prevent misparse
- Prefer `Option<T>` with `skip_serializing_if` for optional fields
- Add `#[serde(flatten)]` for inline rendering options
- Comment non-obvious logic; reference CSL 1.0 spec where applicable

## Priority Styles

1. **APA 7th** - Complex, widely used ✅ (5/5 match)
2. **Chicago Author-Date** - Different patterns (full names, different punctuation)
3. **All 2,844 styles** - Bulk migration target
