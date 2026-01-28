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

### 1. Type Safety
Use Rust enums for controlled vocabularies. No string typing.
```rust
pub enum ContributorRole { Author, Editor, Translator, ... }
pub enum DateForm { Year, YearMonth, Full, MonthDay }
```

### 2. Declarative Templates
Replace CSL 1.0's procedural `<choose>/<if>` with flat templates:
```yaml
bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
```

### 3. Structured Name Input
Names must be structured (`family`/`given` or `literal`), never parsed from strings. Corporate names can contain commas.

### 4. Oracle Verification
All changes must pass the verification loop:
1. Render with citeproc-js → String A
2. Render with CSLN → String B  
3. **Pass**: A == B (for supported features)

## Current Status

- **Citations**: 5/5 exact match with oracle ✅
- **Bibliography**: ~85% match, core elements working
- **Locale**: en-US with terms, months, contributor roles

## Test Commands

```bash
# Run all tests
cargo test

# Run oracle (citeproc-js reference)
cd scripts && node oracle.js ../styles/apa.csl

# Run CSLN processor  
cargo run --bin csln_processor -- csln-first.yaml

# Compare outputs
cargo run -q --bin csln_processor -- csln-first.yaml --cite
```

## State Management

Session state is stored in `.agent/state.json`. Read on start, update on completion.

## Coding Standards

- Use `#[serde(rename_all = "kebab-case")]` for YAML/JSON compatibility
- Use `#[non_exhaustive]` for extensible enums
- Prefer `Option<T>` with `skip_serializing_if` for optional fields
- Add `#[serde(flatten)]` for inline rendering options

## Priority Styles

1. **APA 7th** - Complex, widely used
2. **Chicago Author-Date** - Different patterns
3. **All 2,844 styles** - Bulk migration target
