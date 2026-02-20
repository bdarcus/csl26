# CSLN

CSLN is a Rust-based, declarative citation styling system.

It is the successor-focused evolution of CSL 1.0: styles are expressed as YAML templates and options, then rendered by a type-safe processor with oracle verification against `citeproc-js`.

## Status

CSLN is in active development.

For current, generated metrics, use these as source of truth:

- Compatibility dashboard: [`docs/compat.html`](./docs/compat.html)
- Tier status snapshot: [`docs/TIER_STATUS.md`](./docs/TIER_STATUS.md)
- Core fidelity/SQI baseline: [`scripts/report-data/core-quality-baseline.json`](./scripts/report-data/core-quality-baseline.json)

Do not treat hard-coded README percentages as canonical.

## What CSLN Includes

- `csl_legacy`: CSL 1.0 XML parser
- `csln_core`: schema/types and shared models
- `csln_processor`: citation and bibliography rendering engine
- `csln_migrate`: CSL 1.0 -> CSLN migration pipeline (hybrid)
- `csln`: main CLI (`render`, `check`, `convert`)
- `csln_analyze`: corpus analysis tooling

## Quick Start

```bash
git clone https://github.com/bdarcus/csl26
cd csl26
cargo build --workspace
cargo test --workspace
```

Render references:

```bash
cargo run --bin csln -- render refs \
  -b tests/fixtures/references-expanded.json \
  -s styles/apa-7th.yaml
```

Render a document:

```bash
cargo run --bin csln -- render doc \
  -i examples/document.djot \
  -b examples/document-refs.json \
  -s styles/apa-7th.yaml \
  -I djot -O html
```

Validate inputs:

```bash
cargo run --bin csln -- check \
  -s styles/apa-7th.yaml \
  -b tests/fixtures/references-expanded.json \
  -c tests/fixtures/citations-expanded.json
```

Convert formats:

```bash
cargo run --bin csln -- convert styles/apa-7th.yaml --output /tmp/apa-7th.cbor
```

## CLI Surface

`csln` currently exposes:

- `render` (subcommands: `doc`, `refs`)
- `check`
- `convert`

Schema generation is available with the feature-enabled build:

```bash
cargo run --bin csln --features schema -- schema style
cargo run --bin csln --features schema -- schema --out-dir ./schemas
```

## Migration Workflow (Hybrid)

CSLN migration combines three approaches:

1. XML options extraction for global behavior.
2. Output-driven template inference for structure.
3. Hand-authored high-impact styles for top parent fidelity.

Run migration:

```bash
cargo run --bin csln-migrate -- styles-legacy/apa.csl
```

Prepare high-fidelity authoring context:

```bash
./scripts/prep-migration.sh styles-legacy/apa.csl
```

Detailed migration docs:

- [`crates/csln_migrate/README.md`](./crates/csln_migrate/README.md)
- [`docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md`](./docs/architecture/MIGRATION_STRATEGY_ANALYSIS.md)

## Verification Workflow

Single-style oracle checks:

```bash
node scripts/oracle.js styles-legacy/apa.csl
node scripts/oracle-e2e.js styles-legacy/apa.csl
```

Top-style aggregate:

```bash
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10
```

Core fidelity + SQI gate:

```bash
node scripts/report-core.js > /tmp/core-report.json
node scripts/check-core-quality.js \
  --report /tmp/core-report.json \
  --baseline scripts/report-data/core-quality-baseline.json
```

## Repository Layout

```text
crates/
  csl_legacy/
  csln/
  csln_analyze/
  csln_core/
  csln_edtf/
  csln_migrate/
  csln_processor/

docs/
styles/
styles-legacy/
scripts/
tests/
```

## Documentation Map

- Rendering workflow: [`docs/guides/RENDERING_WORKFLOW.md`](./docs/guides/RENDERING_WORKFLOW.md)
- Style tier tracking: [`docs/TIER_STATUS.md`](./docs/TIER_STATUS.md)
- Design and architecture docs: [`docs/architecture/`](./docs/architecture/)
- Web docs entry point: [`docs/index.html`](./docs/index.html)

## Contributing

- For roadmap/design context, start in [`docs/architecture/`](./docs/architecture/).
- For rendering issues, follow [`docs/guides/RENDERING_WORKFLOW.md`](./docs/guides/RENDERING_WORKFLOW.md).
- For local task tracking, see `.beans/` and project workflow docs.

If your change touches Rust code (`.rs`, `Cargo.toml`, `Cargo.lock`), run:

```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run
```

If `cargo nextest` is unavailable, use:

```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

## License

MPL-2.0. See [`LICENSE`](./LICENSE).
