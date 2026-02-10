# Testing Context

Working on **oracle verification** and ensuring rendering fidelity.

## Core Principle
All changes must pass the oracle loop:
1. Render with citeproc-js → String A
2. Render with CSLN → String B
3. **Pass**: A == B (for supported features)

## Oracle Scripts
| Script | Purpose |
|--------|---------|
| `scripts/oracle.js` | Structured oracle comparison (component-level diff) |
| `scripts/oracle-e2e.js` | End-to-end migration test (migrate + render + compare) |
| `scripts/oracle-simple.js` | Legacy simple string comparison |
| `scripts/oracle-batch-aggregate.js` | Batch analysis across top N styles |

## Recommended Workflow
```bash
# Full workflow test (structured oracle + batch impact)
./scripts/workflow-test.sh styles-legacy/apa.csl

# Structured oracle for a single style
node scripts/oracle.js styles-legacy/apa.csl

# Batch analysis across top 10 styles
node scripts/oracle-batch-aggregate.js styles-legacy/ --top 10
```

## Rust Tests
```bash
# Run all unit and integration tests
cargo test

# Run tests for a specific crate
cargo test -p csln_processor
```

## Pre-Commit Checks (Mandatory)
```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test
```

## Reference Docs
- [RENDERING_WORKFLOW.md](../../docs/RENDERING_WORKFLOW.md) — detailed workflow guide
- [TEST_STRATEGY.md](../../docs/architecture/design/TEST_STRATEGY.md) — testing philosophy
