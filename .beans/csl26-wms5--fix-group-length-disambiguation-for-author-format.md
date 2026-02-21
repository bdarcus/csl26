---
# csl26-wms5
title: Fix group_length disambiguation for author-format styles
status: completed
type: bug
priority: high
created_at: 2026-02-21T20:58:13Z
updated_at: 2026-02-21T20:58:13Z
---

In author-format styles like MLA, the citation title should only appear when
disambiguating between multiple works by the same author in the document.
The TemplateTitle schema field `disambiguate_only: true` was added for this
purpose, and the processor guard exists in
`crates/csln_processor/src/values/title.rs`:

    if self.disambiguate_only == Some(true) && hints.group_length <= 1 {
        return None;
    }

However, `hints.group_length` is always <= 1 for all items, so all titles are
suppressed when `disambiguate_only: true` is set, causing citation fidelity to
drop from 11/13 to 6/13 for MLA.

The fix requires `ProcHints.group_length` to correctly reflect the count of
distinct works by the same author in the reference list being processed.
This likely needs to be populated during bibliography/citation pre-processing,
grouping references by their primary author key.

Discovered during MLA style authoring (oracle: 11/13 citations, failing items:
locator-section-with-suffix and single-with-prefix-and-suffix).
Both show title in CSLN output but not in citeproc-js oracle output, confirming
the items have only one work per author in the fixture.

