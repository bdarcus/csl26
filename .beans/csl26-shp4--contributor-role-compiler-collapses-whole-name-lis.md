---
# csl26-shp4
title: Contributor role compiler collapses whole name list (maps to Composer not Compiler)
status: todo
type: bug
priority: high
tags:
    - style
    - chicago
    - fidelity
    - contributors
created_at: 2026-09-04T13:02:52Z
updated_at: 2026-09-04T14:32:56Z
parent: csl26-h7oc
---

Deeper investigation (this session) found the fix is bigger than a
one-line role-name remap, and needs a spec before implementation
(per schema-changes-need-docs-pr-first).

Three separate string->role mapping sites for "compiler" exist:
- crates/citum-schema-style/src/locale/raw_conversion.rs:395
  `parse_role_name` maps CSL-legacy role key "compiler" to
  `ContributorRole::Composer` -- but this only affects LOCALE
  role-TERM display-string lookup (populates
  `locale.roles: HashMap<ContributorRole, ContributorTerm>`), not
  which role a contributor's own name gets classified as.
- crates/citum-engine/src/values/contributor/mod.rs:101 already
  correctly maps "compiler" -> `citum_schema::reference::ContributorRole::Compiler`.
- crates/citum-engine/src/values/contributor/substitute.rs:176
  already correctly maps "compiler" -> `DataRole::Compiler`.

The DATA-side enum (`citum_schema::reference::ContributorRole`,
crates/citum-schema-data/src/reference/contributor.rs:210) already
has a proper `Compiler` variant, and CSL-legacy ingestion already
carries it through correctly for the contributor's actual role.

The real gap: the STYLE/TEMPLATE-side enum
(`citum_schema::template::ContributorRole`, via `str_enum!` at
crates/citum-schema-style/src/template.rs:1101-1136) has NO
`Compiler` variant at all -- a style YAML author cannot write
`contributor: compiler` as a role selector today, and
`SubstituteField`'s fixed scalar enum (CollectionEditor/Editor/
ParentSerial/Title/Translator) has no room for it either, though
`SubstituteKey::Contributor(SubstituteContributor{contributor:
ContributorRoles})` architecturally could accept it once the
variant exists.

Fixing the "Austin, Tim, comp." collapse case for real needs:
1. Add a `Compiler` variant to `citum_schema::template::ContributorRole`
   (public schema change, needs `just schema-gen` regen).
2. Thread it through role-selector eligibility so a `contributor:
   author`-style selector (or a dedicated compiler selector) treats
   a compiler-only reference as having a primary contributor.
3. Add `compiler` to Chicago's `options.substitute.candidates` list
   (currently `[editor, translator, parent-serial, title]`) so a
   compiler stands in for a missing author via the existing
   substitute mechanism.
4. Separately fix raw_conversion.rs:395's Composer mismapping --
   real but narrower, affects only locale role-term display text
   for editor/compiler-labeled entries, not the name-dropping bug.

Given the schema-change scope, this needs a docs-only spec PR first
(status Draft in docs/specs/, reviewed before implementation) per
project convention, not a same-session engine fix. Root cause
confirmed via jcodemunch code reading, not yet verified with a live
CLI render of the Austin/Tim compiler fixture -- do that first when
resuming, before finalizing the spec's scope.
