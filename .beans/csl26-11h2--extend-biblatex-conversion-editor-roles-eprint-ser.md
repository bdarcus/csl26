---
# csl26-11h2
title: 'Extend BibLaTeX conversion: editor roles, eprint, series, remaining types'
status: todo
type: task
priority: normal
tags:
    - conversion
    - schema
    - fidelity
created_at: 2026-07-24T11:28:35Z
updated_at: 2026-07-26T13:41:18Z
---

Deferred field/entry-type gaps found while investigating csl26-2mse, each genuinely blocked on a decision only Bruce can make (a new enum variant, a modeling choice, or separately-scoped effort) -- not just more of the same mapping work already done in that bean's fix.

- [ ] Editor sub-roles (annotator/commentator/foreword/introduction/afterword/holder): entry.editors() in citum-refs/src/formats/biblatex/mapping.rs flattens all editorial roles into one undifferentiated `editor` field. ContributorRole enum (crates/citum-schema-data/src/reference/contributor.rs:197-213) has no variants for these -- needs new variants added first.
- [ ] eprint/eprinttype -> MonographType::Preprint: MonographType::Preprint exists but nothing produces it. Needs a precedence rule: does an eprint field on an otherwise-typed entry (e.g. @article with eprint) override the entry-type-driven mapping, or only apply to generic/misc entries?
- [ ] series: no flat field on Monograph/Collection; only maps through `container: WorkRelation`, which would mean modeling a BibLaTeX series as a fully embedded parent Collection -- a real modeling decision, not a one-line field read.
- [ ] Remaining entry types with no obvious InputReference target: patent, dataset, software, standard, map, archive, periodical, reference/mvreference/inreference. Note: citum-schema-data/src/reference/types/specialized.rs already defines standalone Patent/Dataset/Standard/Software reference classes (not MonographType variants) -- mapping to these would follow the same pattern as build_inbook_reference/build_article_reference (a new builder function per class), not a schema change. Real, multi-struct implementation work, worth its own pass rather than folding into a single-field fix.
- [ ] eventtitle/venue (for @inproceedings), chapter: no obvious schema slot on CollectionComponent today -- needs a schema-field decision.
- [ ] Build a .bib fixture corpus + BibLaTeX conversion contract tests: no .bib fixtures exist anywhere in citum-core; a native-construction corpus (per this repo's test-coverage conventions) would let the gb7714-bench-derived exact-match-vs-Zotero gap be tracked and regression-tested locally instead of relying on an external, unpersisted CI artifact.
- [ ] Refactor field_str (citum-refs/src/formats/biblatex/mapping.rs) to use biblatex's typed field accessors instead of hand-rolled Chunk-to-string, which currently silently discards Chunk::Math content to an empty string. Touches every field extraction in the file -- a separate robustness concern from mapping breadth, not bundled here.

See csl26-2mse's Summary of Changes for what was already fixed (entry-type mapping for techreport/thesis/online/unpublished/proceedings, translator, institution/organization/school publisher fallback, subtitle, abstract/version/keywords, ISBN propagation to synthesized parent Collection).
