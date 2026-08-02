# Performance Profile Report

- Date: 2026-05-24
- Branch: `perf/analysis`
- Scope: Citum CLI rendering, reference conversion, validation, and CSL migration
- Follow-up beans: `csl26-pmxa`, `csl26-tdcl`, `csl26-rtak`

## Summary

This pass bootstrapped the optional CSL corpora, captured a release-mode timing
baseline with `hyperfine`, generated CPU flamegraphs with `perf`/`flamegraph`,
and added an optional `dhat-heap` feature so heap profiling can be repeated
without ad-hoc source edits.

The slowest measured CLI path is CSL migration. `citum-migrate` spent most of
its time and heap traffic in allocation-heavy CSL node expansion, template diff
selection, and large temporary clone/drop chains. Large bibliography rendering is
fast enough for current CLI use, but still allocates heavily while collecting
rendered template components and building JSON output.

## Setup

The local checkout was bootstrapped with:

```bash
./scripts/bootstrap.sh full
```

That installed script dependencies and shallow-initialized:

- `styles-legacy`
- `tests/csl-test-suite`

Large CSL-JSON fixture wrappers were converted to temporary top-level arrays for
CLI conversion benchmarks:

```bash
jq '.items' tests/fixtures/test-items-library/apa-7th.json > /tmp/apa-7th-items.json
jq '.items' tests/fixtures/test-items-library/chicago-18th.json > /tmp/chicago-18th-items.json
target/release/citum convert refs /tmp/apa-7th-items.json --from csl-json --to citum-json -o /tmp/apa-7th-citum.json
target/release/citum convert refs /tmp/chicago-18th-items.json --from csl-json --to citum-json -o /tmp/chicago-18th-citum.json
```

CPU flamegraphs were captured from release binaries rebuilt with debug symbols
and no stripping:

```bash
env CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=false \
  cargo build --release -p citum -p citum-migrate --no-default-features
```

Heap profiles were captured with:

```bash
env CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=false \
  cargo build --release -p citum -p citum-migrate --features dhat-heap
```

The feature installs the DHAT allocator, while capture is explicitly enabled
per invocation with `CITUM_DHAT_HEAP=1`. This keeps ordinary
`--all-features` validation and reporting free of profiler output and
`dhat-heap.json` artifacts.

## Timing Baseline

| Workload | Mean | Stddev |
|---|---:|---:|
| `citum convert refs /tmp/apa-7th-items.json --from csl-json --to citum-json` | 6.2 ms | 1.2 ms |
| `citum check -s apa-7th -b tests/fixtures/references-expanded.json -c tests/fixtures/citations-expanded.json --json` | 7.5 ms | 0.6 ms |
| `citum render refs -b tests/fixtures/references-expanded.json -c tests/fixtures/citations-expanded.json -s apa-7th --json` | 10.2 ms | 1.7 ms |
| `citum render refs -b /tmp/chicago-18th-citum.json -s chicago-author-date-18th --json` | 44.3 ms | 6.8 ms |
| `citum render refs -b /tmp/apa-7th-citum.json -s apa-7th --json` | 46.2 ms | 4.4 ms |
| `citum-migrate styles-legacy/apa.csl --template-source xml` | 142.0 ms | 1.4 ms |

The exported `hyperfine` result for this run was
`/tmp/citum-hyperfine.json`.

## CPU Findings

Symbolized flamegraphs:

- `/tmp/citum-migrate-apa-xml.symbolized.svg`
- `/tmp/citum-render-apa-large.symbolized.svg`

Migration hotspots:

- `csl_legacy::model::CslNode` and `Choose` drop chains consumed a large part
  of sampled time, indicating substantial temporary tree construction.
- `citum_migrate::template_diff::component_selector` repeatedly cloned
  `serde_json::Value` into selector maps.
- `citum_migrate::template_diff::diff_resolves_to_template` cloned parent
  templates into temporary `TemplateVariant::Full` values.

Rendering hotspots:

- `render_refs_json` and `print_json_with_format` were the dominant CLI frames
  for the large APA workload.
- `render_template_components` collected per-entry `ProcTemplateComponent`
  vectors and cloned component options.
- JSON value construction/drop was visible in the output path.

## Heap Findings

DHAT artifacts:

- `/tmp/citum-migrate-apa-xml.dhat-heap.json`
- `/tmp/citum-render-apa-large.dhat-heap.json`

| Workload | Total allocated | Allocation blocks | Peak heap | End heap |
|---|---:|---:|---:|---:|
| `citum-migrate styles-legacy/apa.csl --template-source xml` | 333,974,675 B | 1,576,462 | 88,248,058 B | 112,385 B |
| `citum render refs -b /tmp/apa-7th-citum.json -s apa-7th --json` | 76,253,287 B | 555,624 | 9,392,310 B | 117,045 B |

Largest migration allocation sites:

- `template_diff::component_selector` at `template_diff.rs:239`.
- `TemplateVariant::clone`, `diff_resolves_to_template`, and
  `resolve_template_variant` around type-variant resolution.
- `MacroInliner::expand_macros_no_increment` at `lib.rs:122`, with peak
  allocations from cloned CSL node subtrees.

Largest render allocation sites:

- `render_template_components` at `grouped/core.rs:867`, collecting rendered
  components.
- `ProcEntry::clone` in JSON output assembly.
- `BibliographySpec::resolve_template` and cloned citation/bibliography
  options.
- High-frequency small allocations in `key_base` and `get_variable_key`.

## Improvement Plan

1. Reduce `citum-migrate` macro expansion cloning.
   Track in `csl26-pmxa`. Rewrite macro expansion to avoid cloning whole
   `CslNode` trees when only child vectors change, and add a benchmark or
   profiling fixture proving reduced peak heap.

2. Reduce `citum-migrate` template diff cloning.
   Track in `csl26-tdcl`. Avoid cloning `serde_json::Value` selector fields and
   full parent templates in diff equivalence checks.

3. Reduce render hot-path allocation.
   Track in `csl26-rtak`. Avoid per-component `RenderOptions` clones where
   possible, pre-size or stream rendered component output, and remove avoidable
   `String` allocation from key-base handling.

## Reproduction Commands

```bash
cargo build --release --bins

hyperfine --warmup 5 --min-runs 20 --export-json /tmp/citum-hyperfine.json \
  'target/release/citum render refs -b tests/fixtures/references-expanded.json -c tests/fixtures/citations-expanded.json -s apa-7th --json -o /tmp/citum-render-expanded.json' \
  'target/release/citum render refs -b /tmp/apa-7th-citum.json -s apa-7th --json -o /tmp/citum-render-apa-large.json' \
  'target/release/citum render refs -b /tmp/chicago-18th-citum.json -s chicago-author-date-18th --json -o /tmp/citum-render-chicago-large.json' \
  'target/release/citum check -s apa-7th -b tests/fixtures/references-expanded.json -c tests/fixtures/citations-expanded.json --json > /tmp/citum-check-expanded.json' \
  'target/release/citum convert refs /tmp/apa-7th-items.json --from csl-json --to citum-json -o /tmp/citum-convert-apa-large.json' \
  'target/release/citum-migrate styles-legacy/apa.csl --template-source xml > /tmp/citum-migrate-apa-xml.yaml'

flamegraph -o /tmp/citum-migrate-apa-xml.symbolized.svg -- \
  target/release/citum-migrate styles-legacy/apa.csl --template-source xml

flamegraph -o /tmp/citum-render-apa-large.symbolized.svg -- \
  target/release/citum render refs -b /tmp/apa-7th-citum.json -s apa-7th --json \
  -o /tmp/citum-render-apa-large-symbolized.json
```

For heap profiling, run the target binary from `/tmp` with
`CITUM_DHAT_HEAP=1` so `dhat-heap.json` does not land in the repository root,
then rename the generated file.
