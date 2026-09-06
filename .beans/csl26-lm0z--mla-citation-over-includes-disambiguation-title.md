---
# csl26-lm0z
title: 'MLA: citation over-includes disambiguation title'
status: todo
type: bug
priority: normal
tags:
    - engine
    - fidelity
    - disambiguation
created_at: 2026-09-05T20:49:37Z
updated_at: 2026-09-06T15:27:08Z
parent: csl26-ccdt
---

MLA's citation template always renders the disambiguate-only primary title
(quote-wrapped) even when author-only (or author+et-al) already
disambiguates -- e.g. Citum renders (Chen, "Neural Networks for Natural
Language Understanding") where the oracle has (Chen, Neural Networks for
Natural Language Understanding) [title present but NOT quoted -- matches
the title's own bibliography wrap form, italic for a thesis] and
(Smith, Lee, Kumar, et al., "Adaptive Climate Risk Modeling in Coastal
Cities") where the oracle has just (Smith, Lee, Kumar, et al.) with no
title at all, since the et-al author list is already unambiguous.

Two distinct sub-issues found tuning csl26-on47 (modern-language-association,
"B title quote boundary" label, 10 residual rows):
1. `disambiguate-only: true` on the citation title component doesn't appear
   to gate rendering on whether disambiguation is actually needed -- title
   renders unconditionally.
2. When the title IS needed, it should carry the SAME wrap form the
   bibliography gives that reference's type (quotes for an article,
   italic/plain for a monograph/thesis) rather than being hardcoded to
   `wrap: punctuation: quotes` regardless of type.

Likely needs engine investigation (disambiguation gating), not just a YAML
tweak -- deferred out of csl26-on47's style-YAML-only scope.

Repro: node scripts/analyze-parity-residuals.js <report.json> --list "B title quote boundary"
against modern-language-association.
