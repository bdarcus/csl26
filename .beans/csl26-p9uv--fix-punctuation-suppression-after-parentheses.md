---
# csl26-p9uv
title: Fix punctuation suppression after parentheses in bibliography
status: todo
type: bug
priority: medium
created_at: 2026-02-08T12:00:00Z
updated_at: 2026-02-08T12:00:00Z
blocking:
    - csl26-o3ek
labels:
    - category: rendering
---

The `refs_to_string` function in `csln_processor` suppresses the bibliography separator
if the previous component ends with a closing parenthesis `)`.

While intended to prevent double punctuation (e.g. after a period), this logic fails
for styles like Elsevier numeric where `(Eds.)` should be followed by a comma
separator.

Example issue:
- Current: `H.T. Reis, C.M. Judd (Eds.) Handbook...`
- Expected: `H.T. Reis, C.M. Judd (Eds.), Handbook...`

The logic in `crates/csln_processor/src/render/mod.rs` (around line 58) needs to be
more nuanced than `matches!(last_char, ... | ')')`.
