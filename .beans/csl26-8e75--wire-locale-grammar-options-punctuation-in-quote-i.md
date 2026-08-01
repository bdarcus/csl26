---
# csl26-8e75
title: Wire locale grammar-options punctuation-in-quote into resolve_punctuation_defaults
status: todo
type: task
priority: normal
tags:
    - punctuation
    - engine
created_at: 2026-08-01T11:58:12Z
updated_at: 2026-08-01T11:58:12Z
---

crates/citum-engine/src/processor/setup.rs resolve_punctuation_defaults only resolves strong_terminal_comma_policy and delimiter_suppressing_terminal_marks from locale grammar-options, not punctuation_in_quote, even though en-US.yaml declares punctuation-in-quote: true. Wiring it would flip the default for every style that doesn't set punctuation-in-quote explicitly -- a cross-style parity event, not a bug fix, so it needs its own review pass. Follow-up from csl26-1hya.
