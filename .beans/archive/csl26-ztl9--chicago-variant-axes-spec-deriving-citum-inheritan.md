---
# csl26-ztl9
title: 'Chicago variant axes: spec deriving Citum inheritance equivalent of style-variant-builder diffs'
status: completed
type: task
priority: normal
created_at: 2026-08-07T18:11:28Z
updated_at: 2026-08-09T13:37:13Z
---

Andrew Dunning maintains the CSL Chicago/APA families via citation-style-language/style-variant-builder: one monolithic template plus 74 textual .diff patches materializing variants (subsequent-note form, no-url, access-dates, archive-place, publisher-place, annotation, etc). Citum's extends: + TemplateVariantDiff (modify/remove/add on type-variants) already does this natively at load time, cross-style, with no engine work needed for most axes (verified: style/resolution.rs:180 captures inherited_variant_context pre-overlay; template/resolution.rs:236-268 falls back to it; covered by bdd_inheritance.rs:53-130).

Deliverable: docs/specs/CHICAGO_VARIANT_AXES.md (Draft), deriving the axis map from the 74 diffs, written for CSL-literate human readers (Andrew Dunning audience) as well as agents. Cross-referenced from CHICAGO_FAMILY_STRATEGY.md. Records two gaps as evidence, each with a proposed fix direction (not implemented): Gap A - LinksConfig controls hyperlinking not render suppression (csl26-cfgw); Gap B - the style-wide fallback template is Option<Template> (bare Vec), whole-replace only under extends, not patchable like reference-type templates (csl26-4xg9). Also records a real defect (chicago-notes-18th's unconditional citation.ibid block vs. its CSL source and CMOS18 13.37) as bean csl26-adka.

citum-styles pilot wrapper styles and oracle baseline generation via style-variant-builder's make final are a separate, later piece of work (separate repo/PR) — not part of this spec.

- [x] Write docs/specs/CHICAGO_VARIANT_AXES.md (Draft status, doc-standards template)
- [x] Add Related: cross-reference + changelog entry in CHICAGO_FAMILY_STRATEGY.md
- [x] Branch + PR #1154 with the Rust gate required by its test updates
- [x] Confirm CI green
- [x] Address PR review comments: fixed citation-system count (3 of 6, not 2), cut undefined jargon ("head"/"wrapper"), rewrote csl26-adka to lead with the fix action, added the missing Citum-side YAML to the ibid worked example, reassessed Gap A/Gap B as likely accidental with proposed fixes (csl26-cfgw, csl26-4xg9), added a build-next recommendation
- [x] Offered the citum-styles pilot as separate future work; intentionally left unscheduled by maintainer decision

## Summary of Changes

Wrote docs/specs/CHICAGO_VARIANT_AXES.md, mapping citation-style-language/style-variant-builder's template+74-diff model onto Citum's extends: and type-variant patching. Recorded two schema gaps (csl26-cfgw and csl26-4xg9) plus the csl26-adka citation defect, and fixed the independently verified shortened-notes bibliography separator with its Rust expectations.

The separate citum-styles pilot was considered and intentionally left unscheduled; it is not part of PR #1154.
