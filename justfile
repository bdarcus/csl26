# Citum Project justfile
# See https://github.com/casey/just for documentation

# Default recipe: run the pre-commit gate
default: pre-commit

# Run the pre-commit gate (formatting check, clippy warnings as errors, and tests)
pre-commit:
    ./scripts/dev-env.sh cargo fmt --check
    ./scripts/dev-env.sh cargo clippy --all-targets --all-features -- -D warnings
    ./scripts/dev-env.sh cargo nextest run

# Run all tests via nextest
test:
    ./scripts/dev-env.sh cargo nextest run

# Validate tracked styles and locales against the published JSON schemas
schema-validate:
    node scripts/validate-schemas.js --scope=styles,locales

# Regenerate schemas when crates/citum-cli or schema crates change
schema-gen:
    ./scripts/dev-env.sh cargo run --bin citum --features schema -- schema --out-dir docs/schemas
    node scripts/build-data-model-reference.js
    git add docs/schemas/ docs/reference/

# Bootstrap the development environment (setup can be 'minimal' or 'full')
bootstrap setup="minimal":
    ./scripts/bootstrap.sh {{setup}}

# Build nodejs-target WASM bindings for local tooling (e.g. scripts/benchmark-wasm-workflow.js)
build-wasm-nodejs:
    wasm-pack build crates/citum-bindings --target nodejs --features full-wasm

# Regenerate docs/demo.html from docs/demo.djot (optionally override style)
demo style="styles/embedded/chicago-author-date-18th.yaml":
    ./scripts/build-demo.sh {{style}}

# Render bibliography references using a style
render-refs style="styles/embedded/apa-7th.yaml" refs="tests/fixtures/references-expanded.json":
    ./scripts/dev-env.sh cargo run --bin citum -- render refs -s {{style}} -b {{refs}}

# Validate a style YAML and reference library file
check-style style="styles/embedded/apa-7th.yaml" refs="tests/fixtures/references-expanded.json":
    ./scripts/dev-env.sh cargo run --bin citum -- check -s {{style}} -b {{refs}}

# Validate all production styles in the repository
validate-production-styles:
    ./scripts/validate-production-styles.sh

# Convert a bibliography reference library to another format (e.g. ris, csl-json)
convert-refs input output:
    ./scripts/dev-env.sh cargo run --bin citum -- convert refs {{input}} --output {{output}}

# Run the local oracle comparison for a specific style (e.g. styles-legacy/apa.csl)
oracle style:
    node scripts/oracle.js {{style}}

# Run the oracle + batch-impact workflow test on a legacy CSL file (e.g. styles-legacy/apa.csl)
workflow-test csl:
    ./scripts/workflow-test.sh {{csl}}

# Generate a core rendering report and validate it against baseline quality gates: fails if any
# gated style's fidelity drops below 1.0, or if any embedded-core style's exact-parity `passed`
# count drops below its recorded floor (see docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md).
check-core-quality:
    node scripts/report-core.js --all-features > /tmp/r.json
    node scripts/check-core-quality.js --report /tmp/r.json \
        --baseline scripts/report-data/core-quality-baseline.json \
        --parity-baseline scripts/report-data/embedded-parity-baseline.json \
        --parity-adjudication scripts/report-data/parity-adjudication.json

# Validate schemas, hashes, partitions, and byte-for-byte freshness for explicitly registered audits
check-style-coverage-audits:
    node scripts/check-style-coverage-audits.js

# Refresh Top-10 oracle aggregate baselines
oracle-refresh:
    node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10

# Discover recurring per-concern config shapes across the legacy CSL corpus that no named
# preset covers (contributors, dates, titles, locators) — a worklist for citum-schema-style presets.
analyze-presets styles="styles-legacy":
    ./scripts/dev-env.sh cargo run --quiet --bin citum-analyze -- {{styles}} --config-presets --json \
        | jq '.concerns[] | {concern, matched_style_count, unmatched_style_count, \
              candidate_count: (.candidates|length), candidates: .candidates[:5]}'

# Validate YAML frontmatter for local contributor AI skills and commands
validate-frontmatter flags='--copilot-strict':
    ./scripts/validate-frontmatter.sh {{flags}}
