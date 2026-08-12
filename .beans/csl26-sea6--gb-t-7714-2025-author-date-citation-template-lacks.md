---
# csl26-sea6
title: gb-t-7714-2025-author-date citation template lacks type-conditional date logic
status: todo
type: bug
priority: normal
tags:
    - engine
    - fidelity
    - gb-t-7714
created_at: 2026-08-12T13:27:17Z
updated_at: 2026-08-12T13:27:17Z
parent: csl26-ccdt
---

citum's `gb-t-7714-2025-author-date.yaml` `citation:` section has one flat `date: issued` component with no `type-variants:`, while its `bibliography:` section fully expresses upstream's type-conditional `date-intext` macro (article-journal/magazine branches never reach the no-date term; webpage falls back to an access year; etc). Upstream citeproc-js reuses the exact same macro for both citation and bibliography rendering, so its citation output is type-differentiated too — citum's citation output is not, a pre-existing migration gap. Sibling of csl26-6eak (driving this style to full fidelity).

Surfaced during csl26-huuz's Codex review: because collision-group letters are computed once and shared between citation and bibliography rendering (correctly — a reference must carry one letter everywhere), and the shared discriminant is bibliography-preferred (the more complete template), an undated reference's *citation* text can render generically (e.g. "n.d.") while its letter reflects a bibliography-side distinction the citation's own template can't express. Not a defect in the disambiguation mechanism itself — see docs/specs/DISAMBIGUATION.md §1's bibliography-preferred rationale — but a fidelity gap in the citation template's completeness.

Fix: port the type-conditional structure from bibliography.type-variants into citation.type-variants for this style (or a shared subset), so citation output matches upstream's date-intext branching per type.
