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

### Contributor presets (`options.contributors`)

Each preset encodes name formatting conventions for a style family. Pick the closest match; add explicit overrides for fields that differ.

| Preset | Format | Initials | Conjunction | et al. | Example |
|--------|--------|----------|-------------|--------|---------|
| `apa` | First family-first | `. ` (period-space) | `&` symbol | 21/19 | `Smith, J. D., & Jones, M. K.` |
| `chicago` | First family-first | none (full names) | `and` text | none | `Smith, John D., and Mary K. Jones` |
| `vancouver` | All family-first | none | none | 7/6 | `Smith JD, Jones MK` |
| `ieee` | All given-first | `. ` (period-space) | `and` text | none | `J. D. Smith, M. K. Jones` |
| `harvard` | All family-first | `.` (period only) | `and` text | none | `Smith, J.D., Jones, M.K.` |
| `springer` | All family-first | none | none | 5/3 | `Smith JD, Jones MK` |
| `numeric-compact` | All family-first | none | none | 7/6 | `Smith J, Jones M` |
| `numeric-medium` | All family-first | none | none | 4/3 | `Smith J, Jones M` |
| `numeric-tight` | All family-first | none | none | 7/3 | `Smith J, Jones M, Brown A, et al.` |
| `numeric-large` | All family-first | none | none | 11/10 | `Smith J, … [10 authors], et al.` |
| `numeric-all-authors` | All family-first | none | none | none | `Smith JD, Jones MK, Brown AB` |
| `numeric-given-dot` | All given-first | `.` (period only) | none | none | `J.D. Smith, M.K. Jones, A.B. Brown` |
| `annual-reviews` | All family-first | none | none | 7/5, demote-never | `van der Berg J, Smith M, Jones A, Brown B, White C, et al.` |
| `math-phys` | All family-first | `.` (period only) | none | none (set separately) | `Smith, J., Jones, M., Brown, A.` |
| `soc-sci-first` | First family-first, rest given-first | `. ` (period-space) | none | none (set separately) | `Smith, J. D., M. K. Jones` |
| `physics-numeric` | All given-first | `. ` (period-space) | none | none (set separately) | `J. Smith, M. Jones, A. Brown` |

**Choosing between similar presets:**

- Compact initials (`""`): `vancouver`, `numeric-compact`, `numeric-medium`, `numeric-tight`, `numeric-large`, `numeric-all-authors`, `annual-reviews`, `springer`
- Period-only initials (`"."`): `harvard`, `math-phys`, `numeric-given-dot`
- Period-space initials (`". "`): `apa`, `ieee`, `soc-sci-first`, `physics-numeric`
- Given-first (no inversion): `ieee`, `physics-numeric`, `numeric-given-dot`
- First-author-only inversion: `apa`, `chicago`, `soc-sci-first`
- All inverted: everything except `ieee`, `physics-numeric`, `numeric-given-dot`

When you need a different et al. threshold than the preset provides, use the preset and add a `shorten:` override at the context level:

```yaml
options:
  contributors: math-phys       # all family-first, period initial, comma sort-sep
bibliography:
  options:
    contributors:
      shorten: { min: 11, use-first: 10 }   # override et al. threshold
```

### Date presets (`options.dates`)

| Preset | Month format | EDTF markers | Example |
|--------|-------------|--------------|---------|
| `long` | Full names | Yes (`ca.`, `?`, en-dash ranges) | `January 15, 2024` |
| `short` | Abbreviated | Yes | `Jan 15, 2024` |
| `numeric` | Numbers | Yes | `1/15/2024` |
| `iso` | Numbers | No | `2024-01-15` |

### Title presets (`options.titles`)

| Preset | Article/component | Book/monograph | Journal/periodical |
|--------|------------------|---------------|--------------------|
| `apa` | plain | *italic* | *italic* |
| `chicago` | "quoted" | *italic* | *italic* |
| `ieee` | "quoted" | *italic* | *italic* |
| `humanities` | plain | *italic* | *italic* |
| `journal-emphasis` | plain | plain | *italic* |
| `scientific` | plain | plain | plain |

### Substitute presets (`options.substitute`)

Controls what replaces the author when it is missing:

- `standard`: Editor → Title → Translator (most styles)
- `editor-first`: Editor → Translator → Title
- `title-first`: Title → Editor → Translator
- `editor-short` / `editor-long`: Editor only, with short or long role label
- `editor-translator-short` / `editor-translator-long`: Editor then Translator
- `editor-title-short` / `editor-title-long`: Editor then Title
- `editor-translator-title-short` / `editor-translator-title-long`: Full chain

### Template presets

- `citation.use-preset: numeric-citation` for numeric styles that render citation numbers via style-level wrapping (`[1]`, `(1)`, or superscript contexts).

### Example combining presets

```yaml
options:
  processing: author-date
  contributors: math-phys         # Springer math/physics family-first with period initial
  dates: short
  titles: apa
  substitute: standard
bibliography:
  options:
    contributors:
      shorten: { min: 3, use-first: 1 }   # override et al. threshold only
```

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
