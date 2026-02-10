# Processor Context

Working on the citation and bibliography **rendering engine**.

## Focus Areas
- Template resolution: matching reference types to templates, applying overrides
- Name formatting: initials, sort order, et-al, disambiguation
- Date formatting: EDTF support, date parts, ranges
- Output formats: HTML, Djot, PlainText (pluggable `OutputFormat` trait)
- Locale handling: terms, months, contributor roles
- Semantic classes: configurable via `--no-semantics` flag

## Key Crates
| Crate | Role |
|-------|------|
| `csln_processor` | Rendering engine (entry point: `cargo run --bin csln-processor`) |
| `csln_core` | Types consumed by the processor (Style, Template, Locale) |

## Core Principles
- **Declarative Over Procedural**: Flat YAML templates, no `<choose>/<if>` logic.
- **Oracle Parity**: Output must match `citeproc-js` for supported features.
- **Zero Magic**: Style behavior is explicit in YAML, never hardcoded in Rust.
- **Three-tier options**: Global → context-specific (citation/bibliography) → template-level overrides.

## Key Binaries & Scripts
- `cargo run --bin csln-processor -- styles/apa-7th.yaml` — main rendering entry point
- `node scripts/oracle.js styles-legacy/apa.csl` — verify against citeproc-js
- `./scripts/workflow-test.sh styles-legacy/apa.csl` — end-to-end impact analysis

## Reference Docs
- [RENDERING_WORKFLOW.md](../../docs/RENDERING_WORKFLOW.md)
- [PUNCTUATION_NORMALIZATION.md](../../docs/architecture/design/PUNCTUATION_NORMALIZATION.md)

## Recent Changes
- 2026-02-09: feat: add structured html output for bibliography
- 2026-02-09: feat: migrate and process localized terms
- 2026-02-09: feat: pluggable output formats (HTML, Djot, PlainText)
