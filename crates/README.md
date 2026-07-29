# Crate Map

The citum-core workspace. Read this first to orient — then descend into the relevant crate's `CLAUDE.md` (when present) for scoped rules and entry points.

## Naming

- `citum-*` (hyphenated): standard crate naming convention.
- `citum_store` (underscored, legacy): pre-dates the convention; kept stable to avoid churn. Treat the form as fixed.

## Pipeline crates (the citation lifecycle)

| # | Crate | Role |
|---|---|---|
| 1 | [`csl-legacy`](csl-legacy/CLAUDE.md) | CSL 1.0 XML parser — frozen, consumed by `citum-migrate`. |
| 2 | [`citum-migrate`](citum-migrate/CLAUDE.md) | Standalone CSL 1.0 -> Citum YAML migration crate/tool. Plateau reached on automatic mode; hand-authoring is canonical for top parents. |
| 3 | [`citum-schema`](citum-schema/CLAUDE.md) | Facade re-exporting `citum-schema-style` + `citum-schema-data`. Real types in `citum-schema-style/`. |
| 4 | `citum-schema-style` | Modular style model: `Style`, `Template`, `Options`, `Locale`, `Renderer`, presets, registry, and the exact-parity embedded style core; community styles resolve through the filesystem registry layer. Navigate via jcodemunch. |
| 5 | `citum-schema-data` | Reference / data model accessors. |
| 6 | [`citum-engine`](citum-engine/CLAUDE.md) | Rendering engine. Citation/bibliography output. Byte-for-byte targets vs citeproc-js / biblatex. |

## Surface crates (how external consumers reach the engine)

| Crate | Role |
|---|---|
| `citum-cli` | The `citum` binary. Schema gen lives behind `--features schema`. |
| `citum-bindings` | WASM/cdylib string API over `citum-engine`; optional TypeScript type export. |
| `citum-server` | Long-running server surface. |
| `citum-resolver-api` | API for reference resolution. |
| `citum-pdf` | PDF rendering surface. |
| `citum-io` | I/O helpers shared across surfaces. |
| `citum_store` | Storage layer. |

## Support crates

| Crate | Role |
|---|---|
| `citum-analyze` | Analytic passes used in tooling and tests. |
| `citum-edtf` | EDTF (Extended Date/Time Format) parsing. |
| [`citum-refs`](citum-refs/README.md) | Multi-format reference ingestion (Citum YAML/JSON/CBOR, CSL-JSON, BibLaTeX, RIS); no `citum-engine` dependency. |

## Where work usually happens

| Task | Crate(s) |
|---|---|
| Engine rendering bug | `citum-engine` (especially `src/render/`, `src/grouping/`) |
| Style YAML model change | `citum-schema-style` |
| Reference / data field addition | `citum-schema-data`, then bindings |
| New converter behavior | `citum-migrate` |
| New input format / ingestion bug | `citum-refs` |
| CLI command / flag | `citum-cli` |
| FFI / bindings | `citum-engine/src/ffi/` then `citum-bindings/` |

## Navigation rules

This workspace contains 2,300+ Rust symbols across 15 crates. **Read fewer files; query symbols instead.**

- Symbol body / call hierarchy / module API → **jcodemunch** (`get_symbol`, `get_call_hierarchy`, `get_repo_outline`).
- Type resolution, hover, go-to-def → **rust-analyzer** (LSP).
- Call sites, string literals, regex → **Bash `mgrep` / `grep`** (RTK-rewritten).

See root `CLAUDE.md` for the full tool priority table.
