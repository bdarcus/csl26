---
# csl26-873v
title: 'GB/T: legal-type refs skip author-substitute otherwise'
status: todo
type: bug
priority: normal
tags:
    - engine
    - fidelity
    - gb-t
created_at: 2026-09-06T15:56:09Z
updated_at: 2026-09-06T15:56:09Z
parent: csl26-ccdt
---

gb-t-7714-2025-author-date's bibliography.options.substitute is correctly
configured (candidates: [editor, translator], otherwise: message:
term.anonymous), and the merge/resolve chain (Substitute::merge,
Config::merge, effective_substitute) is correctly wired end to end --
verified by static trace of crates/citum-engine/src/values/contributor/mod.rs
(TemplateContributor::values -> resolve_author_substitute ->
resolve_author_fallback -> otherwise_message).

But for legal-type references (bill, legislation, regulation, hearing --
confirmed via direct `citum render refs` against isolated fixtures) the
author contributor slot renders as literally nothing: no author, no
editor/translator substitute, no otherwise-message fallback either. The
whole author+date pair that should lead the entry ("Anon，2013. ") is
missing; everything after it (title, genre marker, date-in-parens, URL)
renders correctly and matches oracle.

## Repro
    echo '{"references":[{"id":"t","type":"bill","title":"Mental Health on Campus Improvement Act","number":"1100","issued":{"date-parts":[[2013]]}}]}' > /tmp/t.json
    target/release/citum render refs -b /tmp/t.json -s styles/embedded/gb-t-7714-2025-author-date.yaml -m bib
    # got:  Mental Health on Campus Improvement Act：1100[Z]. （2013）.
    # want: 佚名，2013. Mental Health on Campus Improvement Act：1100[Z]. （2013）.

Also reproduces for a monograph-shaped synthetic ref that additionally sets
`authority: "113th Cong."` (matches the real TLIB-SEL-BILL-1/HEARING-1
fixtures), so `authority` alone is not the trigger -- the common factor
across all four failing types is that they're legal reference types.

Likely cause: something upstream of TemplateContributor::values --
probably `merged::is_role_suppressed` -- unconditionally suppresses the
author role for legal-type references (a reasonable default for styles
that never show an author position for legal citations, e.g. Chicago
notes/Bluebook-style), with no per-style override to let GB/T's
substitute.otherwise still fire when there's genuinely no author,
editor, or translator.

## Scope
This is 27/28 of gb-t-7714-2025-author-date's exact-parity residual (the
style's lowest fidelity among non-Chicago embedded styles, 0.881) and
likely affects any other author-date style whose corpus includes legal
references with no substitute promotion. Needs an engine-side decision:
either (a) let substitute.otherwise fire even when the role is
suppressed for "no author position at all" reasons, or (b) add a
per-style override to opt legal types back into the substitute chain.
Not a style-YAML fix -- do not attempt a targeted patch without full
corpus verification (see csl26-q67h's history: an earlier bibliography.sort
attempt for this same style regressed american-medical-association-alphabetical
from 21/67 to 1/67 exact-parity when combined with a sorting.rs change).
