---
# csl26-4xg9
title: Implement inherited fallback-template diffs
status: in-progress
type: feature
priority: normal
tags:
    - schema
    - style
    - chicago
created_at: 2026-08-07T23:53:13Z
updated_at: 2026-08-09T15:51:43Z
---

Implement the fallback-template diff contract in
docs/specs/STYLE_TEMPLATE_EXPRESSIVENESS_AND_PARITY.md. This is Gap B from
docs/specs/CHICAGO_VARIANT_AXES.md.

BibliographySpec.template and CitationSpec.template are currently
Option<Template>, while type-specific templates use TemplateVariant. A child
style therefore has to replace an inherited fallback in full even when it only
needs to remove or modify one component.

Required direction:

- widen the existing template fields to Option<TemplateVariant> rather than
  adding a sibling template-variant field;
- preserve existing YAML sequences as TemplateVariant::Full;
- resolve the parent's effective fallback, including template-ref, before a
  child Diff applies;
- reject a root Diff with no inherited fallback;
- reject TemplateVariantDiff.extends on fallback templates;
- reject template-ref plus Diff in the same section;
- preserve explicit template: null clearing behavior;
- leave only a concrete Full fallback after style resolution;
- cover the same cases for citation and bibliography, including selector
  diagnostics, and regenerate schemas in the implementation commit.

- [x] Draft the docs/specs contract, including compatibility and template-ref
      interaction.
- [ ] Get the Draft specification reviewed and merged.
- [x] Implement the schema and resolver change in a follow-up PR.
- [x] Regenerate schemas and pass citation and bibliography conformance tests.
