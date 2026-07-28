---
# csl26-svfg
title: 'Style extends: merges nested option blocks whole-value, not field-level'
status: todo
type: task
priority: high
tags:
    - style
    - architecture
created_at: 2026-07-20T17:50:16Z
updated_at: 2026-07-28T16:28:43Z
parent: csl26-s2rw
---

Style inheritance (extends:) merges BibliographyOptions/CitationOptions/Config fields whole-value-replace, including nested struct fields like dates (crates/citum-schema-style/src/style/overlay.rs:340-344, same merge_options! macro shape as the runtime global/citation/bibliography scope merge in crates/citum-schema-style/src/options/mod.rs). A child style that wants to add one field on top of an inherited nested config (e.g. note-wrap alongside era-labels/approximation-marker) has no partial-override path -- it must redeclare the entire block. This forced gb-t-7714-2025-author-date/numeric/note to each carry a duplicated bibliography.options.dates copy in PR #1068 rather than one shared change in gb-t-7714-2025-base.yaml, and will recur for any style family with a shared base and scope-level nested option blocks (dates confirmed; contributors/titles/locators likely share the same shape -- worth auditing). Examine whether field-level partial merge on inherit is desirable in general (vs today's unambiguous whole-block-or-nothing), and if so what it would take: a deep-merge variant of merge_options! for nested-struct fields, an opt-in merge: deep annotation per field, or just a documented convention. Use the GB/T duplication as the motivating example, not the scope -- this is about the general inheritance mechanism, not consolidating GB/T's copies specifically.

## Sharpened acceptance criteria (2026-07-28)

Decision made in docs/specs/STYLE_INHERITANCE.md (Draft): field-level deep merge for nested option structs; scalars/arrays/explicit-null replace whole; type-variants stay per-key.

- [ ] Deep-merge nested option structs in crates/citum-schema-style/src/style/overlay.rs (Options::merge impls; check the merge_options! macro shape shared with crates/citum-schema-style/src/options/mod.rs runtime scope merge — only the extends overlay changes, not runtime scope semantics unless spec'd)
- [ ] rstest coverage: child adds one field to inherited dates block, sibling fields survive; scalar/array/null replace cases
- [ ] Delete the three duplicated bibliography.options.dates blocks from gb-t-7714-2025-{author-date,numeric,note}, moving the shared change to gb-t-7714-2025-base.yaml; rendered output byte-identical
- [ ] Flip STYLE_INHERITANCE.md status Draft -> Active in the implementation commit
- [ ] Audit contributors/titles/locators nested blocks for the same duplication pattern

Good candidate for delegation to a smaller model; the spec is the contract.

## Spike findings (2026-07-28, branch wip/svfg-deep-merge)

A working implementation exists on local branch wip/svfg-deep-merge (red on one test, do not merge as-is). Approach: raw-YAML deep merge in overlay.rs (authored keys over serialized base options), preset-string expansion to resolved mappings (preserves biblatex layering, e.g. options.contributors: springer keeping the parent's demote-non-dropping-particle), and a post-parse mutation guard (typed overlay must round-trip from raw options; otherwise fall back to typed merge — required because Processor::new re-resolves and programmatic mutations carry stale raw_yaml). DateConfig.month needs #[serde(default)] for partial dates blocks to parse. GB/T dedup verified byte-identical on references-expanded fixtures for all three leaves.

Remaining before merge:
- [ ] Wrapper-compat pass: taylor-and-francis-chicago-author-date relies on its partial titles block suppressing the parent's type-mapping/title-class entries (domain_fixtures test red); add explicit ~ clears there and audit the other 15 in-tree wrappers + embedded families for the same reliance
- [ ] Full workspace nextest + report-core regression sweep vs baseline
- [ ] just schema-gen (month no longer required in JSON schema)
- [ ] Explicit-null semantics: null on defaulted non-Option scalars is a parse error; spec rule 3 updated accordingly — keep tests aligned

See STYLE_INHERITANCE.md Implementation Notes for the constraint list.
