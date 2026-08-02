---
# csl26-lhrl
title: 'ieee: raw phd-thesis enum leak and one fabricated ''no. 1'''
status: todo
type: bug
priority: low
tags:
    - style
    - fidelity
created_at: 2026-08-02T00:09:51Z
updated_at: 2026-08-02T13:16:50Z
---

Two small, unrelated ieee bugs, neither investigated yet:

- "phd-thesis" renders literally instead of "PhD thesis". Check whether VocabMap (citum-schema-style/src/locale/types.rs) is the right fix, or something else is needed.
- One entry (Sagan, "The Universe in a Nutshell") gets a fabricated "no. 1" that isn't in the source data at all. Trace which component is populating it.
