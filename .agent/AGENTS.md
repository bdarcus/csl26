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
  csln_core/       # CSLN types: Style, Template, Options, Locale
  csln_migrate/    # CSL 1.0 → CSLN conversion
  csln_processor/  # Citation/bibliography rendering engine

styles/            # 2,844 CSL 1.0 styles (submodule)
scripts/           # oracle.js for citeproc-js verification
tests/             # Integration tests
```

## Key Design Principles

### 1. Explicit Over Magic

**The style language should be explicit; the processor should be dumb.**

If special behavior is needed (e.g., different punctuation for journals vs books), 
it should be expressed in the style YAML, not hardcoded in the processor.

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

### 2. Type Safety
Use Rust enums for controlled vocabularies. No string typing.
```rust
pub enum ContributorRole { Author, Editor, Translator, ... }
pub enum DateForm { Year, YearMonth, Full, MonthDay }
```

### 3. Declarative Templates
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

### 4. Structured Name Input
Names must be structured (`family`/`given` or `literal`), never parsed from strings. Corporate names can contain commas.

### 5. Oracle Verification
All changes must pass the verification loop:
1. Render with citeproc-js → String A
2. Render with CSLN → String B  
3. **Pass**: A == B (for supported features)

### 6. Well-Commented Code
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

# Build and check
cargo build && cargo clippy
```

## State Management

Session state is stored in `.agent/state.json`. Read on start, update on completion.

## Coding Standards

- Use `#[serde(rename_all = "kebab-case")]` for YAML/JSON compatibility
- Use `#[non_exhaustive]` for extensible enums
- Use `#[serde(deny_unknown_fields)]` on untagged enum variants to prevent misparse
- Prefer `Option<T>` with `skip_serializing_if` for optional fields
- Add `#[serde(flatten)]` for inline rendering options
- Comment non-obvious logic; reference CSL 1.0 spec where applicable

## Priority Styles

1. **APA 7th** - Complex, widely used
2. **Chicago Author-Date** - Different patterns
3. **All 2,844 styles** - Bulk migration target
