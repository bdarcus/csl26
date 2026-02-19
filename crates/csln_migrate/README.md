# csln-migrate

`csln-migrate` converts a CSL 1.0 style (`.csl`) into a CSLN style (`.yaml`).

The migration pipeline is now output-driven first:

1. Extract global options from CSL XML.
2. Resolve citation and bibliography templates from inferred output artifacts.
3. Fall back to XML template compilation only when template artifacts are missing or rejected.

This keeps option extraction deterministic while scaling template migration to large style corpora.

## CLI Usage

```bash
cargo run --bin csln-migrate -- <style.csl> [flags]
```

Example:

```bash
cargo run --bin csln-migrate -- styles-legacy/apa.csl > styles/apa.yaml
```

## Flags

- `--template-source auto|hand|inferred|xml`
- `--template-dir <path>`
- `--min-template-confidence <0.0..1.0>`
- `--debug-variable <name>`

### `--template-source`

- `auto` (default): hand-authored -> inferred cache/live -> XML fallback
- `hand`: hand-authored only -> XML fallback
- `inferred`: inferred cache only -> XML fallback
- `xml`: XML templates only

Important: `inferred` mode is cache-only and never runs live Node/citeproc-js inference.

## Template Resolution Order

In `auto` mode:

1. `examples/<style-name>-style.yaml` (hand-authored templates)
2. `templates/inferred/<style-name>.bibliography.json`
3. `templates/inferred/<style-name>.citation.json`
4. Legacy cache compatibility: `templates/inferred/<style-name>.json` (bibliography)
5. Live inference via `scripts/infer-template.js` (auto mode only)
6. XML template compiler fallback

## Precompile Once, Migrate in Rust

For large-scale migration, precompute inferred templates once, then run Rust migrations without citeproc-js:

```bash
# 1) Precompute inferred template cache for all parent styles
./scripts/batch-infer.sh

# 2) Or precompute selected styles
./scripts/batch-infer.sh --styles "apa elsevier-harvard ieee"

# 3) Migrate using cache-only inferred mode (no live Node inference)
cargo run --bin csln-migrate -- styles-legacy/apa.csl --template-source inferred
```

## Cache Artifact Format

Section-keyed cache files:

- `templates/inferred/STYLE_NAME.bibliography.json`
- `templates/inferred/STYLE_NAME.citation.json`

Each file is produced by:

```bash
node scripts/infer-template.js styles-legacy/STYLE_NAME.csl --section=bibliography --fragment
node scripts/infer-template.js styles-legacy/STYLE_NAME.csl --section=citation --fragment
```

Fragment shape:

```json
{
  "meta": {
    "style": "apa",
    "confidence": 0.85,
    "delimiter": ". ",
    "entrySuffix": ".",
    "wrap": "parentheses"
  },
  "bibliography": {
    "template": []
  }
}
```

`citation` artifacts use the same shape with a `citation` section key.

## Confidence Gate

`--min-template-confidence` rejects inferred fragments below threshold before use.

Example:

```bash
cargo run --bin csln-migrate -- styles-legacy/apa.csl \
  --template-source auto \
  --min-template-confidence 0.80
```

When rejected, migration falls back to XML template compilation for that section.

## Notes

- Output is written to stdout; redirect to a file as needed.
- Options extraction remains XML-based by design.
- Template inference is output-driven to avoid procedural CSL template translation bottlenecks.
