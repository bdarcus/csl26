# Schema Context

Working on the **CSLN data model** — types, schemas, and the specification itself.

## Philosophy
- **Code-First**: Rust structs and enums are the source of truth for the schema.
- **Permissive Runtime, Strict Linter**: Processor ignores unknown fields; `csln_analyze` reports them as warnings.
- **Extension via Defaults**: New features use `Option<T>` with `#[serde(default)]`.

## Key Crate: `csln_core`
| Module | Responsibility |
|--------|----------------|
| `style.rs` | Top-level `Style` struct (with `version` field) |
| `template.rs` | Template components, overrides, contributor/date/title types |
| `options.rs` | Three-tier options: global → context (citation/bibliography) → template |
| `locale.rs` | Locale terms, months, contributor roles |
| `reference.rs` | Reference types and metadata fields |

## Serde Conventions
- `#[serde(rename_all = "kebab-case")]` — YAML/JSON field naming
- `#[non_exhaustive]` — extensible enums
- `#[serde(deny_unknown_fields)]` — on untagged enum variants to prevent misparse
- `Option<T>` + `skip_serializing_if` — optional fields
- `#[serde(flatten)]` — inline rendering options and `_extra` map for round-trip preservation

## Three-Tier Options
```
Global options:        style.options
Context options:       style.citation.options / style.bibliography.options
Template overrides:    component-level overrides (per reference type)
```
Context-specific options override global for their context.

## Schema Generation
```bash
cargo run --bin csln-cli -- schema > csln.schema.json
```

## Reference Docs
- [STYLE_ALIASING.md](../../docs/architecture/design/STYLE_ALIASING.md)
- [PUNCTUATION_NORMALIZATION.md](../../docs/architecture/design/PUNCTUATION_NORMALIZATION.md)
- [PRIOR_ART.md](../../docs/architecture/PRIOR_ART.md)
