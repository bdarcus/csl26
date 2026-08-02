---
# csl26-ww77
title: Check the Z-unclassified residual bucket for more label_only-shaped engine bugs
status: todo
type: task
priority: normal
tags:
    - engine
    - fidelity
    - research
created_at: 2026-08-02T12:34:31Z
updated_at: 2026-08-02T13:15:49Z
parent: csl26-ccdt
---

Check the 37% "unclassified" bucket from the 2026-07-30 parity audit (925 of 2,501 mismatches) for more bugs shaped like the one this wave just fixed in ieee: a bibliography number wrongly treated as real content, causing a stray separator before the next field.

- That bug was tagged "unclassified" because it didn't match any known defect pattern -- there may be more like it hiding the same way.
- Run the oracle-parity report, filter to unclassified mismatches, sample a chunk by hand for the same signature (stray punctuation right after a bibliography number).
- Investigation only -- file a new bean for anything found, don't fix inline.
