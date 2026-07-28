# Style Inheritance & Portfolio — Audit

- **Date:** 2026-07-28
- **Beans:** `csl26-s2rw` (epic), resolves `csl26-8x90`, motivates `csl26-svfg`
- **Question:** Is the style-inheritance story coherent across specs and
  implementation, and does the checked-in `styles/` corpus still serve its
  purpose given the exact-parity results of the refactored compatibility
  report?
- **Instruments:** `/tmp/core-report.json` (report-core run of 2026-07-28,
  157 styles), a drift classifier over its paired entries (node one-off,
  reproducible from the JSON), the 2026-07-17 alias-band artifacts, and a
  docs sweep of the seven inheritance-adjacent specs.

## Portfolio shape

| Metric | Value |
|---|---|
| Public report styles | 157 |
| Embedded style files (`crates/citum-schema-style/embedded/styles/`) | 32 (12 public families + hidden `*-core`, `chicago-18-base`, `gb-t-7714-2025-base` layers) |
| Checked-in `styles/*.yaml` | 141 |
| Families (by ultimate `extends` root) | 134 |
| Implementation forms | 128 standalone, 18 structural-wrapper, 11 config-wrapper |
| Checked-in styles with top-level `extends` | 16 (all extend embedded parents; none extends another checked-in style) |

134 families across 157 styles means inheritance is essentially unused
outside the embedded set: the checked-in corpus is a flat field of standalone
conversions, not a family graph.

## Exact parity

Overall exact-text parity is 3,159 / 12,874 paired observations (**24.5%**),
against ~96% citation and ~91% bibliography lenient compatibility.

8,804 of the 9,715 exact failures occur on rows that lenient compatibility
grades as **matched** — the drift lives inside the normalization gap.
Classifying those 8,804 matched-row drifts:

| Class | Count |
|---|---|
| Punctuation/whitespace only | 803 |
| Citation-number value drift (numbering order) | 241 |
| Numbering + punctuation + case | 75 |
| Case only (incl. with punctuation) | 145 |
| Content drift | 7,540 |

Within the content-drift mass, systematic sub-classes recur across many
styles rather than being per-style noise: leading bibliography number
present in the oracle but absent in Citum output (≥560 rows), et-al
truncation thresholds, `&` vs `,` name delimiters, dropped quotation marks
around titles, missing role labels and contributor initials, and
disambiguation suffix differences.

Tuned embedded styles score far higher (`gb-t-7714-2025-numeric` 0.95,
`gb-t-7714-2025-note` 0.84) than the untuned long tail (mostly 0–0.3, twenty
styles at exactly 0). The low overall number therefore reflects **loosely
converted long-tail styles graded by a lenient metric**, not primarily
engine gaps — although the numbering-presence and numbering-order classes
are engine/converter-systematic and affect the embedded set too.

## Spec coherence

The docs sweep found four historical mechanism layers that were never
unified:

1. **Presets** — [STYLE_ALIASING.md](../../specs/STYLE_ALIASING.md)
   (Active, 2026-02-15) selects options-presets plus embedded templates and
   states "no parent/child aliasing".
2. **`extends` / StyleBase** —
   [STYLE_PRESET_ARCHITECTURE.md](../../specs/STYLE_PRESET_ARCHITECTURE.md)
   (Active, v2.0) adds structural inheritance and states (§3) structural
   deep merge for nested structs.
3. **Profile narrowing** — CONFIG_ONLY_PROFILE_OVERRIDES.md (Superseded) →
   [UNIFIED_SCOPED_OPTIONS.md](../../specs/UNIFIED_SCOPED_OPTIONS.md)
   (Active) restricts profiles to scoped options.
4. **Registry aliases** — [STYLE_REGISTRY.md](../../specs/STYLE_REGISTRY.md)
   (Active) provides name resolution orthogonal to `extends`.

Specific defects:

- **Spec–spec contradiction.** STYLE_ALIASING.md's "no parent/child
  aliasing" verdict is contradicted by STYLE_PRESET_ARCHITECTURE.md's
  `extends` and STYLE_REGISTRY.md's `aliases:`, yet the doc is still marked
  Active.
- **Spec–code contradiction.** STYLE_PRESET_ARCHITECTURE.md §3 promises
  structural deep merge for nested structs, but
  `crates/citum-schema-style/src/style/overlay.rs` merges option blocks via
  `Options::merge`, which replaces nested struct fields whole-value
  (`csl26-svfg`). A child style cannot add one field to an inherited nested
  block (e.g. `bibliography.options.dates`) without redeclaring the whole
  block — the cause of the triplicated GB/T dates blocks in PR #1068.
- **Undocumented conventions.** `chicago-18-base` and `gb-t-7714-2025-base`
  appear in no spec; the `*-core` hidden-layer convention is described only
  in passing in STYLE_EDITIONS_AND_FAMILIES.md and STYLE_TAXONOMY.md.
- **Undefined directory authority.** No spec states the relationship between
  checked-in `styles/` and `crates/citum-schema-style/embedded/styles/`
  (which `styles/embedded` merely symlinks).
- **Scattered decision rules.** The alias-vs-wrapper-vs-standalone choice
  exists only as STYLE_TAXONOMY.md's Profile Rule, disconnected from the
  mechanism specs.

## Checked-in corpus utility

- **Original purpose.** `styles/` existed to drive engine iteration during
  migration. That role is now served by `styles-legacy/` (2,844 styles) for
  breadth and by the tuned embedded set for depth.
- **Behavioral redundancy.** 102 of 141 checked-in styles sit in the ≥0.98
  alias band against a registered style
  ([2026-07-17_EXTENDS_DELTA_DERIVABILITY.md](2026-07-17_EXTENDS_DELTA_DERIVABILITY.md)),
  but the same audit's addendum found **zero safely auto-registrable
  aliases** — normalized similarity is too weak an equivalence.
- **Test blast radius.** Only six checked-in styles are referenced by Rust
  tests (`oscola`, `oscola-no-ibid`, `royal-society-of-chemistry`,
  `harvard-cite-them-right`,
  `thomson-reuters-legal-tax-and-accounting-australia`, `alpha`), plus files
  under `styles/experimental/`.

## Disposition (resolves `csl26-8x90`)

Per-style dispositions are recorded in
[`scripts/report-data/style-disposition-2026-07-28.tsv`](../../../scripts/report-data/style-disposition-2026-07-28.tsv):

| Disposition | Count | Rule |
|---|---|---|
| `keep-exemplar` | 16 | Rust-test fixtures (6); one wrapper exemplar per embedded parent family (6); unique-coverage picks (4: Chicago 17th notes, MHRA notes, Nature, ACS) |
| `move-to-community` | 125 | Everything else; 90 additionally flagged `alias_review: yes` (CSL-metadata dependency evidence) for later human raw-output review |

The `csl26-8x90` framing of consolidating via aliases and thin wrappers is
superseded: aliasing requires raw-output human review per pair (zero pairs
pass automatically), and re-deriving wrappers loses fidelity (1/28
delta-expressible in the 2026-07-17 sweep). Relocation, not consolidation,
is the maintenance-reduction lever. No re-run of the 6 GB derivability sweep
was needed; existing artifacts plus the exact-parity data decided every row.

## Decisions

1. **Unify the inheritance story in one spec** —
   [STYLE_INHERITANCE.md](../../specs/STYLE_INHERITANCE.md) (Draft) becomes
   the authoritative document for merge semantics and portfolio policy;
   STYLE_ALIASING.md is marked Superseded.
2. **Merge semantics: field-level deep merge** for nested option structs
   (scalars, arrays, and explicit `null` still replace whole), aligning the
   implementation with what STYLE_PRESET_ARCHITECTURE.md §3 already states.
   Implementation tracked in `csl26-svfg`.
3. **Split the long tail to a community repo** (`citum-styles`); citum-core
   keeps the embedded set plus the 16 exemplars and `styles/experimental/`.
4. **Refocus reporting** on the embedded set as the headline metric, with
   exemplars as a secondary tier.
5. **Embedded set unchanged** in this wave; promotion criteria live in the
   new spec, expansion is follow-up work.
