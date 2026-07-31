# Style Workflow Decision Rules

**Status:** Active
**Version:** 1.5
**Date:** 2026-07-31
**Related:** [STYLE_WORKFLOW_EXECUTION.md](../guides/STYLE_WORKFLOW_EXECUTION.md),
[SKILL_AGENT_REFACTOR.md](../architecture/SKILL_AGENT_REFACTOR.md),
[MIGRATION_STRATEGY_ANALYSIS.md](../architecture/MIGRATION_STRATEGY_ANALYSIS.md),
[CSL_TYPE_CONVERSION_CONTRACT.md](../specs/CSL_TYPE_CONVERSION_CONTRACT.md),
[2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)

## Rule
Shared style-workflow agents must classify each mismatch as `style-defect`, `migration-artifact`, `processor-defect`, or `intentional divergence`, and must stop iterating once a cluster is clearly outside the active workflow's scope.

Style-authority decisions for existing journals must follow this order:
1. current publisher or journal style guide
2. current publisher submission instructions or house rules
3. documented parent-family manual or base style reference
4. CSL implementation evidence
5. existing Citum YAML structure

Before editing YAML, shared style workflows must classify the target on **three** axes:
1. semantic class: `base`, `profile`, `journal`, or `independent`
2. implementation form: `alias`, `config-wrapper`, `structural-wrapper`, or `standalone`
3. portfolio tier: `embedded-core` or `dependent`

All three classifications are operational, not cosmetic.

### Conversion-layer pre-flight

Before classifying any mismatch whose reference type or field population
looks suspicious (wrong type-variant selected, fields missing that the
fixture clearly carries, output that reads like a generic default), verify
the reference converts truthfully **before** touching YAML:

```bash
cargo run --bin citum -- convert refs <item>.json --from csl-json -o /tmp/converted.yaml
```

Compare the converted class/type/fields against the canonicalization table
in [CSL_TYPE_CONVERSION_CONTRACT.md](../specs/CSL_TYPE_CONVERSION_CONTRACT.md)
(the authoritative statement of which `ref_type()` output each CSL 1.0.2
input type must produce, including the intentional divergences).

- Conversion wrong (type collapsed, fields dropped, note-field `type:`
  override misapplied) → classify `processor-defect` (conversion). It must
  reproduce in the conversion contract-test module
  (`citum_schema_data::reference::conversion::contract_tests`) before any
  Rust fix, and no YAML iteration happens on that cluster.
- Conversion correct → the defect is style- or migration-side; classify
  against the remaining categories as usual.

Two facts of the conversion contract matter when reading oracle output:
every CSL 1.0.2 type now reaches styles as its real `ref_type` (including
`collection`, `entry`, `figure`, `graphic`, `musical_score`, `pamphlet`,
`performance`, `periodical`, `review`, `review-book`, `post-weblog`), and
note-field `type:` overrides apply only for recognized types — a
misspelled override keeps the top-level type and stays visible in the
note. A style that predates the contract may simply lack a `type-variants`
entry for a newly-routed type; that is a `style-defect` (or a
migration-gap when the source CSL style handled the type), not a
processor problem.

This pre-flight is the interim manual procedure; bean `csl26-3r34`
(attributed fidelity reporting) will mechanize the tagging in
`oracle.js`/`report-core.js`.

### Portfolio tier

The **embedded-core** tier is the set of styles baked into the Citum binary at
compile time. The canonical registry is
[embedded/styles.rs](../../crates/citum-schema-style/src/embedded/styles.rs) (the `include_bytes!` match
arms). The CLI exposes the predicate: `citum style list --source embedded`.
All other styles are **dependent** (long-tail or external).

Tier determines the quality bar. Fidelity is a lenient structural check (see
[2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md)
for what it discards); exact parity is the primary tuning objective for
embedded-core styles:

| Tier | Fidelity | Exact Parity | SQI |
|---|---|---|---|
| `embedded-core` | Hard gate — 100% required | **Hard gate** — per-style floor in `scripts/report-data/embedded-parity-baseline.json` may never regress | **Hard gate** — clean SQI required alongside fidelity and exact parity |
| `dependent` | Hard gate | Diagnostic only | Advisory / tie-breaker only |

For `embedded-core` styles, the `citum-migrate` converter is a **seed and evidence
source**, not the canonical authoring path. The correct authoring path is `create`
or `tune` (iterative LLM authoring against oracle, guide, and SQI), as documented
in [MIGRATION_STRATEGY_ANALYSIS.md](../architecture/MIGRATION_STRATEGY_ANALYSIS.md). A migrated candidate is
acceptable only after it has been hand-tuned to satisfy both gates.

This distinction is operationally important: a correct-but-bulky migrated YAML is
not a finished embedded style. SQI matters here because the embedded set ships
with the binary and defines the maintainability standard for the whole portfolio.

## Rationale
Style work in Citum repeatedly follows the same decision logic: determine whether the defect belongs in YAML, migration, engine behavior, or adjudication, then route the work accordingly. Putting that logic in one policy keeps the Claude and Codex wrappers thin and reduces drift between hosts.

## Application
- Run the conversion-layer pre-flight before classifying a type- or
  field-population-shaped mismatch; a `processor-defect` (conversion)
  claim needs the pre-flight evidence attached.
- `style-defect` routes to style-local YAML repair.
- `migration-artifact` stays in migration-focused work.
- `processor-defect` routes to engine or processor follow-up.
- `intentional divergence` is recorded and excluded from fix counts.
- `profile` means both a semantic relationship and a config-wrapper contract.
  Profile work must preserve that contract: local metadata and scoped options
  are allowed, but local templates, local `type-variants`, and
  template-clearing `null` values are not.
- `journal` is semantic only. A journal descendant may legitimately be an
  `alias`, `config-wrapper`, or `structural-wrapper` depending on the evidence
  and the current infrastructure.
- If a publisher guide says a journal follows a known parent style with a few house tweaks, treat the parent preset as the baseline and keep only the documented deltas.
- Treat CSL XML and migrated standalone YAML as implementation evidence, not the canonical authority for wrapper thickness.
- If a guide-confirmed parent-plus-deltas relationship cannot yet be expressed compactly because preset merge semantics are too coarse, record that as an infrastructure constraint instead of preserving CSL duplication as the source of truth.
- If guide-backed parentage is real but the current merge mechanics still force
  a bulky child file, stop forcing compression. Record the infrastructure
  constraint and keep the style structural until the override model improves.
- If the same scenario fails with identical output after two distinct approaches, stop iterating on that scenario and reclassify it.
- If a registered divergence explains the failure, record the divergence ID instead of treating it as a fresh bug.
- A style wave is a bounded cohort executed through repeated `upgrade`,
  `migrate`, `create`, or `tune` passes under these same rules. Keep waves scoped
  to one family or one clearly related cohort per PR.

## Exceptions
- Host-specific routing, model choice, and permission semantics stay in the wrapper files.
- Rich-input evidence ordering and per-skill output phrasing live in the execution guide.

## Changelog
- v1.5 (2026-07-31): Added exact parity as a hard gate for `embedded-core`
  styles in the portfolio-tier table, alongside fidelity and SQI. See
  [2026-07-31_EXACT_PARITY_REFOCUS.md](../architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md).
- v1.4 (2026-07-02): Added the conversion-layer pre-flight (verify the
  reference converts truthfully against `CSL_TYPE_CONVERSION_CONTRACT.md`
  before classifying) and the newly-routed-types reading guidance,
  following the conversion contract landing in PR #993. Interim manual
  procedure until `csl26-3r34` mechanizes the tagging.
- v1.3 (2026-06-24): Added the third classification axis (portfolio tier:
  `embedded-core` vs `dependent`), tier-dependent quality bar (SQI is a hard
  gate for embedded-core), and the embedded-tier authoring rule (migrate as
  seed, not deliverable). Cross-linked `MIGRATION_STRATEGY_ANALYSIS.md`.
  Added `tune` to the list of wave pass types.
- v1.2 (2026-04-23): Added the two-axis taxonomy as an operational workflow
  rule, made the config-wrapper profile contract explicit, clarified that
  journal descendants may remain structural, and added the infrastructure-stop
  rule for guide-backed wrappers blocked by current merge mechanics.
- v1.1 (2026-04-19): Added explicit source-of-truth ordering for preset-wrapper work and clarified that CSL artifacts are verification evidence, not authority.
- v1.0 (2026-04-04): Established shared style-workflow decision rules.
