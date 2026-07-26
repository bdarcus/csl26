# citum-refs

Multi-format bibliography reference *ingestion* for the Citum citation engine: parsing
and loading external bibliography input into `citum_schema::InputBibliography` /
`Reference`, without depending on `citum-engine`. Output/serialization of native Citum
formats stays in `citum-io`.

## Position in the dependency graph

Both `citum-engine` and `citum-io` depend on this crate. Surface crates
(`citum-server`, `citum-bindings`) may depend on it directly when they need reference
ingestion without pulling in the rendering engine.

## Supported formats

| Format | Module | `RefsFormat` variant |
|---|---|---|
| Citum YAML/JSON/CBOR | [`formats::native`](src/formats/native.rs) | `CitumYaml`, `CitumJson`, `CitumCbor` |
| CSL-JSON | [`formats::csl_json`](src/formats/csl_json.rs) | `CslJson` |
| BibLaTeX | [`formats::biblatex`](src/formats/biblatex/mod.rs) | `Biblatex` |
| RIS | [`formats::ris`](src/formats/ris.rs) | `Ris` |

Each format module is self-contained: it owns both the I/O (reading/parsing a file or
string) and any entry/field mapping into Citum's reference types. BibLaTeX splits this
across two files — `formats/biblatex/mod.rs` (loading: `load_biblatex`,
`parse_biblatex_str`) and `formats/biblatex/mapping.rs` (entry/field →
`InputReference` conversion: `input_reference_from_biblatex`,
`contributors_from_biblatex_persons`, re-exported from `mod.rs`) — because the mapping
logic is large enough to warrant its own file.

## Entry points

- [`load_refs`] / [`load_refs_with_sets`] — load a single native or CSL-JSON
  bibliography file, with optional compound-set metadata.
- [`load_merged_refs`] — load and merge multiple bibliography files.
- [`load_input_refs`] — load bibliography input in an explicitly specified
  [`RefsFormat`] (native, CSL-JSON, BibLaTeX, or RIS).
- [`infer_refs_input_format`] / [`infer_refs_output_format`] — infer a [`RefsFormat`]
  from a file path (JSON inputs are content-sniffed to distinguish native Citum JSON
  from CSL-JSON).
- [`validate_compound_sets`] — validate compound reference sets against a loaded
  bibliography.

[`load_refs`]: src/lib.rs
[`load_refs_with_sets`]: src/lib.rs
[`load_merged_refs`]: src/lib.rs
[`load_input_refs`]: src/lib.rs
[`infer_refs_input_format`]: src/lib.rs
[`infer_refs_output_format`]: src/lib.rs
[`validate_compound_sets`]: src/lib.rs
[`RefsFormat`]: src/lib.rs
