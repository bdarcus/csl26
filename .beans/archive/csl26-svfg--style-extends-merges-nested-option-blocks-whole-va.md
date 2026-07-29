---
# csl26-svfg
title: 'Style extends: merges nested option blocks whole-value, not field-level'
status: completed
type: task
priority: high
tags:
    - style
    - architecture
created_at: 2026-07-20T17:50:16Z
updated_at: 2026-07-29T15:39:48Z
parent: csl26-s2rw
---

Style inheritance (extends:) merges BibliographyOptions/CitationOptions/Config fields whole-value-replace, including nested struct fields like dates (crates/citum-schema-style/src/style/overlay.rs:340-344, same merge_options! macro shape as the runtime global/citation/bibliography scope merge in crates/citum-schema-style/src/options/mod.rs). A child style that wants to add one field on top of an inherited nested config (e.g. note-wrap alongside era-labels/approximation-marker) has no partial-override path -- it must redeclare the entire block. This forced gb-t-7714-2025-author-date/numeric/note to each carry a duplicated bibliography.options.dates copy in PR #1068 rather than one shared change in gb-t-7714-2025-base.yaml, and will recur for any style family with a shared base and scope-level nested option blocks (dates confirmed; contributors/titles/locators likely share the same shape -- worth auditing). Examine whether field-level partial merge on inherit is desirable in general (vs today's unambiguous whole-block-or-nothing), and if so what it would take: a deep-merge variant of merge_options! for nested-struct fields, an opt-in merge: deep annotation per field, or just a documented convention. Use the GB/T duplication as the motivating example, not the scope -- this is about the general inheritance mechanism, not consolidating GB/T's copies specifically.

## Sharpened acceptance criteria (2026-07-28)

Decision made in docs/specs/STYLE_INHERITANCE.md (Draft): field-level deep merge for nested option structs; scalars/arrays/explicit-null replace whole; type-variants stay per-key.

- [x] Deep-merge nested option structs in crates/citum-schema-style/src/style/overlay.rs (Options::merge impls; check the merge_options! macro shape shared with crates/citum-schema-style/src/options/mod.rs runtime scope merge — only the extends overlay changes, not runtime scope semantics unless spec'd)
- [x] rstest coverage: child adds one field to inherited dates block, sibling fields survive; scalar/array/null replace cases
- [x] Delete the three duplicated bibliography.options.dates blocks from gb-t-7714-2025-{author-date,numeric,note}, moving the shared change to gb-t-7714-2025-base.yaml; rendered output byte-identical
- [x] Flip STYLE_INHERITANCE.md status Draft -> Active in the implementation commit
- [x] Audit contributors/titles/locators nested blocks for the same duplication pattern (full render diff across all embedded + in-repo styles vs main; findings in Summary)

Good candidate for delegation to a smaller model; the spec is the contract.

## Spike findings (2026-07-28, branch wip/svfg-deep-merge)

A working implementation exists on local branch wip/svfg-deep-merge (red on one test, do not merge as-is). Approach: raw-YAML deep merge in overlay.rs (authored keys over serialized base options), preset-string expansion to resolved mappings (preserves biblatex layering, e.g. options.contributors: springer keeping the parent's demote-non-dropping-particle), and a post-parse mutation guard (typed overlay must round-trip from raw options; otherwise fall back to typed merge — required because Processor::new re-resolves and programmatic mutations carry stale raw_yaml). DateConfig.month needs #[serde(default)] for partial dates blocks to parse. GB/T dedup verified byte-identical on references-expanded fixtures for all three leaves.

Remaining before merge:
- [x] Wrapper-compat pass (partial): taylor-and-francis-chicago-author-date-core fixed. Root cause was type-mapping inheritance routing motion-picture/broadcast/collection/manuscript/song/webpage into 'component', combined with T&F's own sentence-case override, hitting the open proper-noun bug (csl26-4kt3). Fix (per user decision): made TitlesConfig.type_mapping Option-wrapped so `type-mapping: ~` can null-clear it; wrapper now clears it explicitly. Also found + fixed a stale fixture assertion (Metamorphosis expected no italics -- an artifact of the old whole-replace bug losing monograph.emph:true; deep merge now correctly applies it, which is the desired Chicago behavior). Still need: audit the other 15 in-tree wrappers for the same reliance pattern.
- [x] Full workspace nextest + report-core regression sweep vs baseline (nextest: 2285 passed; render diff of every embedded+in-repo style vs main used instead of report-core for wrapper-compat since the oracle's own text normalization is insensitive to quote/emphasis differences — see memory report-core-baseline-worktree)
- [x] just schema-gen (month no longer required, type-mapping now nullable, substitute doc comment updated)
- [x] Explicit-null semantics: null on defaulted non-Option scalars is a parse error; spec rule 3 updated accordingly — keep tests aligned (covered by null_on_defaulted_non_option_scalar_is_a_parse_error)

See STYLE_INHERITANCE.md Implementation Notes for the constraint list.

## Ingest prerequisite (2026-07-29)

Blocked by csl26-j3zy: the raw-presence basis must be format-neutral and load-path-uniform before deep merge lands. The store resolver currently bypasses Style::from_yaml_bytes (no raw_yaml for store-resolved styles in any format), which already makes shipped explicit-null clearing load-path-dependent. Deep merge must read authored presence from the unified raw tree (yaml/json/cbor), so:

- [x] Depends on the single raw-preserving constructor from csl26-j3zy (completed 2026-07-29)
- [x] Deep-merge tests must cover a JSON-authored child (and ideally CBOR) proving format-identical merge results (json_authored_child_deep_merges_identically_to_yaml_equivalent; CBOR not added — JSON round-trips through the same generic value tree as CBOR per csl26-j3zy, so it exercises the same code path)

## Summary of Changes

Implemented field-level deep merge for nested option structs in the
`extends` overlay (`crates/citum-schema-style/src/style/overlay.rs`),
per `docs/specs/STYLE_INHERITANCE.md` (now Active): a child's authored
`options`/`citation.options`/`bibliography.options` mapping deep-merges
onto the resolved parent using the raw YAML tree, falling back to the
existing typed whole-field merge when no raw tree is available or the
typed overlay no longer round-trips from it (programmatic mutation after
parse). `DateConfig.month` gained `#[serde(default)]` so partial `dates`
blocks parse.

Consolidated the three duplicated `bibliography.options.dates` blocks
across `gb-t-7714-2025-{author-date,numeric,note}` into
`gb-t-7714-2025-base.yaml` — verified byte-identical via a full render
diff of every embedded and in-repo style against `main`.

**Two real bugs found via that render diff, both fixed in this change**
(not wrapper-authoring issues — gaps in the deep-merge machinery itself):

1. `substitute` (`Config`/`CitationOptions`/`BibliographyOptions`) lacked
   the eager preset-resolution deserializer `dates`/`contributors`/
   `titles`/`multilingual`/`locators` already had, so a child writing
   `substitute: <preset>` over a richer parent whole-replaced the
   inherited block instead of field-merging — silently dropping
   `role-substitute` (found via `gb-t-7714-2025-author-date`, which
   lost its `container-author` editor substitution). Fixed with
   `deserialize_substitute_config`; regression test
   `preset_string_override_preserves_inherited_sibling_fields_not_covered_by_the_preset`.
2. `TitlesConfig.type_mapping` was a non-`Option` `HashMap`, so it could
   never be null-cleared at all. Made it `Option<HashMap<String, String>>`.

**Wrapper-compat pass** (full render diff across all 32 embedded + 141
in-repo styles against `main`, not just a manual review):
- `taylor-and-francis-chicago-author-date-core`: deep merge correctly
  inherits the parent's `titles.type-mapping`, routing motion-picture/
  broadcast/collection/manuscript/song/webpage into `component` — which,
  combined with the wrapper's own sentence-case override, hits the open
  proper-noun text-case bug (`csl26-4kt3`, still todo). Per user decision
  (asked directly, since this is a content/style-correctness call, not a
  parity call): added an explicit `type-mapping: ~` clear rather than
  accept the regression or guess per-category remapping. Also fixed a
  stale fixture assertion (`Metamorphosis` expected no italics — an
  artifact of the old whole-replace bug losing `monograph.emph: true`;
  deep merge now correctly applies it).
- `chicago-shortened-notes-bibliography(-core)` and four exemplar styles
  (`american-institute-of-aeronautics-and-astronautics`,
  `american-mathematical-society-label`,
  `american-society-of-mechanical-engineers`,
  `inter-research-science-center`): regained inherited title-class
  quoting/emphasis the whole-replace bug had been silently dropping.
  Verified as correctness improvements (standard Chicago/AIAA/ASME/Vancouver
  conventions, no open bug involved, no existing test asserted the old
  behavior) — left as-is, not suppressed.
- No other embedded or in-repo style's rendered output changed.

Added rstest coverage in `bdd_inheritance.rs`: nested-option partial
override (bibliography- and global-scope), scalar replace, explicit-null
clear, preset-string layering (dates), preset-string-preserves-inherited-
sibling-fields (contributors + substitute, the regression above), a
JSON-authored-child-matches-YAML-equivalent test, and a null-on-defaulted-
non-Option-scalar-is-a-parse-error test.

`just schema-gen`: `month` no longer required, `type-mapping` now
nullable, `substitute` doc comment updated in `docs/schemas/style.json`.

Flipped `docs/specs/STYLE_INHERITANCE.md` Draft → Active in this commit;
ticked all acceptance criteria (STYLE_ALIASING.md supersession and the
141-style disposition TSV were already satisfied by prior commits on
`main`).

Full `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D
warnings`, and `cargo nextest run` (2285 tests) all pass.
