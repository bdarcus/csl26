---
# csl26-4xg9
title: 'Proposal: make style-wide fallback templates patchable like reference-type templates'
status: todo
type: feature
priority: normal
tags:
    - schema
    - style
    - chicago
created_at: 2026-08-07T23:53:13Z
updated_at: 2026-08-07T23:53:13Z
---

Surfaced in docs/specs/CHICAGO_VARIANT_AXES.md (Gap B) while checking whether Citum's inheritance covers the CSL Chicago family's -no-url variant. Every style has one fallback bibliography template (and one fallback citation template) used for any reference type without a template of its own -- BibliographySpec.template (crates/citum-schema-style/src/style/sections/bibliography.rs:40) and CitationSpec.template (crates/citum-schema-style/src/style/sections/citation.rs:56). Both are a plain Option<Template> (bare component list), not the Full-or-Diff TemplateVariant (crates/citum-schema-style/src/template.rs:653) that reference-type templates in type-variants already use. So modify:/remove:/add: only work per reference type; under extends: the fallback template is whole-replace only (confirmed by test_explicit_null_clears_bibliography_template, crates/citum-schema-style/tests/bdd_inheritance.rs:448). Concretely: chicago-author-date-18th.yaml's own fallback bibliography template renders variable: doi and variable: url unconditionally at its final two components (lines 1041-1043), and a child style cannot remove just those two components -- only copy the entire 41-line template, which then stops tracking the parent's future changes.

This looks accidental, not intentional: nothing in STYLE_INHERITANCE.md's merge rules explains why the fallback template should be exempt from the same patch mechanism reference-type templates already have -- it reads as an oversight in how the two fields were typed, not a deliberate constraint.

Proposed direction: widen BibliographySpec.template and CitationSpec.template from Option<Template> to Option<TemplateVariant>, reusing the exact Full/Diff resolution already built and tested for reference-type templates (crates/citum-schema-style/src/template/resolution.rs), so a child's Diff patches the parent's already-resolved fallback template the same way it already patches a parent's reference-type template.

This is a schema and engine change. Per this project's policy, it needs its own docs-first spec PR (with rejected alternatives) before any implementation -- not done as part of the Chicago-axes mapping spec.

- [ ] Draft a docs/specs/*.md spec proposing the Option<TemplateVariant> widening, including how existing Full-typed fallback templates in shipped styles stay valid (Full is one of the two TemplateVariant cases, so no migration needed) and any interaction with template_ref
- [ ] Get the spec reviewed and merged as Draft
- [ ] Implement in a follow-up PR, status flips to Active in that same commit
