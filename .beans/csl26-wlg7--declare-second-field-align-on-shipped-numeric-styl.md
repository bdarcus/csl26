---
# csl26-wlg7
title: Declare second-field-align on shipped numeric styles
status: todo
type: task
priority: low
tags:
    - style
    - fidelity
    - csl
created_at: 2026-08-20T23:55:42Z
updated_at: 2026-08-20T23:55:48Z
---

Follow-up to csl26-qdff (Implement CSL second-field-align rendering, docs/specs/SECOND_FIELD_ALIGN.md). The mechanism landed mechanism-only: csl-legacy parses the attribute, citum-schema-style declares it, citum-engine renders sibling citum-entry-marker/citum-entry-body HTML slots when declared, citum-migrate extracts it — but no shipped style declares second-field-align, so today's HTML output is unaffected everywhere. This bean is the corpus-adoption pass: declare bibliography.options.second-field-align: flush (or margin) on the ~12 shipped numeric styles whose CSL 1.0 source actually carries the attribute (ieee, american-medical-association, royal-society-of-chemistry, numeric-comp, and the other REFERENCE_MARKERS.md-listed marker styles are good starting candidates — verify against each style's original CSL source, not assumed). This is a visible HTML markup change (new sibling divs replacing flush text concatenation) for every style touched, so it needs its own parity review: node scripts/report-core.js before/after per style, plus a direct render diff to confirm plain-text output is unchanged (the OutputFormat::entry_slots seam guarantees this by construction, but verify empirically). Not urgent.
