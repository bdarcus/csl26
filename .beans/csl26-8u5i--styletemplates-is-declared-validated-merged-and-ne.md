---
# csl26-8u5i
title: Style.templates is declared, validated, merged -- and never resolved
status: todo
type: task
priority: normal
tags:
    - schema
created_at: 2026-08-05T13:52:51Z
updated_at: 2026-08-05T13:53:02Z
parent: csl26-ccdt
---

`Style.templates: Option<HashMap<String, Template>>` (crates/citum-schema-style/src/style/model.rs:36) is documented as **"Named reusable templates"**. It is:

- budget-validated (style/validation.rs:67)
- merged per-key across style inheritance (style/overlay.rs:150)
- warned on for unknown enums (citum-engine/src/api/warnings.rs:195)
- banned in profile overrides (style/validation.rs:115, docs/specs/CONFIG_ONLY_PROFILE_OVERRIDES.md)

**But nothing resolves it.** `TemplateReference` is either a closed built-in `TemplatePreset` enum or a `Uri` (template/reference.rs:42); there is no name-keyed lookup into `style.templates` anywhere in the workspace, and the three embedded styles using `template-ref` all name built-in presets (`numeric-citation`).

## Why it matters

TEMPLATE_V3 §Scope lists "Named templates or Macros (Forbidden)" and "Cross-section references to named reusable template fragments (Forbidden)". A schema field literally called "Named reusable templates" sits badly next to that, whichever way the field is meant to go.

## Decide, do not assume

`delimiter-suppressing-terminal-marks` looked equally inert this session and turned out to be **deliberately reserved** for csl26-zfqr. Check for the same before concluding this one is vestigial.

## Todo

- [ ] Determine whether `templates` is reserved for planned work or vestigial
- [ ] If reserved: document what will consume it, and add a doc comment at the definition pointing to the bean (the lesson from csl26-zfqr)
- [ ] If vestigial: remove the field, the validation, the overlay merge, and regenerate schemas; note the breaking schema change
- [ ] Either way, reconcile TEMPLATE_V3 §Scope wording with what the schema actually carries
