# Style Author Guide

This guide is for people who write and maintain CSLN styles.

## What Success Looks Like

Use two quality signals, with clear priority:

1. Fidelity: output matches the citeproc-js oracle.
2. SQI: style quality, maintainability, and fallback robustness.

Fidelity is the hard gate. SQI helps choose between equally correct solutions.

## Core Principles

- Keep behavior explicit in style YAML.
- Prefer declarative templates plus type-specific `overrides`.
- Avoid hidden logic in processor code for style-specific formatting.
- Keep contributor names structured (`family`/`given` or `literal`).
- Preserve multilingual fallback behavior (original -> transliterated -> translated).
- Prefer readable, reusable style definitions over one-off hacks.

## Practical Workflow

1. Start from a nearby style in `/styles`.
2. Implement the target style guide rules in YAML (`options`, `citation`, `bibliography`).
3. Run oracle checks to confirm rendered output.
4. Fix fidelity mismatches first.
5. Improve SQI only when output stays unchanged.
6. Re-run checks before finishing.

## Preset Catalog

Use presets first, then override only what is style-specific.

Option presets:

- `contributors`: `apa`, `chicago`, `vancouver`, `ieee`, `harvard`, `springer`, `numeric-compact`, `numeric-medium`
- `dates`: `long`, `short`, `numeric`, `iso`
- `titles`: `apa`, `chicago`, `ieee`, `humanities`, `journal-emphasis`, `scientific`
- `substitute`: `standard`, `editor-first`, `title-first`, `editor-short`, `editor-long`, `editor-translator-short`, `editor-translator-long`, `editor-title-short`, `editor-title-long`, `editor-translator-title-short`, `editor-translator-title-long`

Template presets:

- `citation.use-preset: numeric-citation` for numeric styles that render citation numbers via style-level wrapping (`[1]`, `(1)`, or superscript contexts).

Example:

```yaml
options:
  contributors: numeric-compact
  dates: long
  titles: humanities
  substitute: editor-translator-title-short

citation:
  use-preset: numeric-citation
  wrap: brackets
```

## Verification Commands

Run from repository root:

```bash
# Compare a style against oracle output
node scripts/oracle.js styles-legacy/apa.csl

# Check core fidelity + SQI drift
node scripts/report-core.js > /tmp/core-report.json
node scripts/check-core-quality.js \
  --report /tmp/core-report.json \
  --baseline scripts/report-data/core-quality-baseline.json
```

If your style work includes Rust code changes, run:

```bash
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo nextest run
```

Use `cargo test` if `cargo nextest` is unavailable.

## How to Use SQI Well

SQI is most useful for improving style quality after correctness is established.

Target improvements such as:

- Better type coverage.
- Stronger fallback behavior.
- Less duplication across templates and overrides.
- Cleaner use of shared options and presets.

Do not trade fidelity for a higher SQI score.

## Common Mistakes

- Putting style-specific punctuation rules into processor code.
- Solving one style with hardcoded exceptions instead of declarative overrides.
- Duplicating variable rendering when substitution/fallback can do it cleanly.
- Accepting small oracle regressions for “cleaner” YAML.

## Definition of Done

A style update is complete when:

- Oracle fidelity target is met.
- No fidelity regressions are introduced in affected core styles.
- SQI is stable or improved.
- Style YAML remains explicit, readable, and maintainable.

## Related Reading

- [Rendering Workflow](./RENDERING_WORKFLOW.md)
- [SQI Refinement Plan](../architecture/SQI_REFINEMENT_PLAN.md)
- [Type Addition Policy](../architecture/TYPE_ADDITION_POLICY.md)
- [CSLN Personas](../architecture/PERSONAS.md)
