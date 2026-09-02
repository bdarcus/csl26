---
# csl26-x79y
title: render-when field-present:author ignores editor substitution
status: todo
type: bug
priority: normal
tags:
    - engine
    - chicago
    - contributors
    - render-when
    - fidelity
created_at: 2026-09-02T18:19:10Z
updated_at: 2026-09-02T18:19:14Z
parent: csl26-h7oc
---

TemplateConditionField::Author's presence check (crates/citum-engine/src/values/mod.rs::condition_field_present) is `reference.author().is_some()` -- a literal raw-field check with no awareness of contributor substitution (e.g. editor promoted into the author slot when no author exists, per CSL convention).

Discovered while fixing csl26-4if2 (render_author_for_grouping_with_format now correctly honors a leading Group's render-when instead of ignoring it). chicago-shortened-notes-bibliography-core.yaml gates its author+title citation group with `render-when: field-present: author`, but for editor-only references the OLD (buggy, render-when-ignoring) code happened to still render correctly because `contributor: author`'s own value computation applies editor-substitution independently of the gate. Honoring the gate literally now means editor-only references render with no names at all for this template shape (e.g. "Reis and Judd, Handbook of Research Methods..." -> "Handbook of Research Methods...").

Net effect on chicago-shortened-notes-bibliography's exact-parity corpus: the csl26-4if2 fix is a clear net improvement (roughly 21 items gained vs 3 lost in the visible coverage-audit sample; the style's afterExactParity.passed count moved from 87 to 86, tracked in scripts/report-core.test.js's `generateReport exposes the registered coverage audit on its corresponding style` test), but this is one of the 3 lost items.

Needs a decision: should `condition_field_present`'s Author (and likely Editor/Translator/Recipient) variants check the *effective* (substitution-aware) primary contributor rather than the raw field, or is the literal-field reading the correct, documented semantics and templates author-controlled to gate on it should write field-present:author OR field-absent... explicitly accounting for substitution themselves? Either resolution should re-run report-core.js for chicago-shortened-notes-bibliography and confirm no further regressions.
