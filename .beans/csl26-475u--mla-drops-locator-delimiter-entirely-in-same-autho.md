---
# csl26-475u
title: MLA drops locator delimiter entirely in same-author collapse
status: todo
type: bug
priority: normal
tags:
    - citation
    - engine
    - rendering
created_at: 2026-08-18T12:49:10Z
updated_at: 2026-08-18T17:49:27Z
parent: csl26-h7oc
---

Found while probing csl26-uctc (locator-aware collapse delimiter). MLA's
same-author collapse concatenates locators onto titles with no delimiter at
all, because group_delimiter derivation (render_group_item_parts_with_format,
crates/citum-engine/src/processor/rendering/grouped/core.rs) scavenges a
leading affix off the first non-author template component, and MLA's
title-based template has none to find.

modern-language-association, [@ITEM-31, p. 100; @ITEM-32]:

    (Garcia, "Methods for Robust Climate Attribution"100, "Methods for
    Probabilistic Climate Attribution")

Wanted something like:

    (Garcia, "Methods for Robust Climate Attribution", 100, "Methods for
    Probabilistic Climate Attribution")

Likely needs a delimiter fallback in group_delimiter derivation (mirroring
the intra_delimiter default used elsewhere) rather than relying solely on a
scavenged leading affix. Separate root cause from csl26-uctc, which is about
choosing between two *existing* delimiters, not synthesizing a missing one.
