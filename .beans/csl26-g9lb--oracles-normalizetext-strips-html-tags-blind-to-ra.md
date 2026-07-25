---
# csl26-g9lb
title: Oracle's normalizeText strips HTML tags, blind to raw-markup-leak regressions
status: todo
type: bug
priority: normal
tags:
    - oracle
    - tooling
    - fidelity
created_at: 2026-07-25T12:18:16Z
updated_at: 2026-07-25T12:18:34Z
---

Discovered while verifying csl26-6eoi (raw citeproc HTML leaking into rendered output).
scripts/oracle-utils.js's normalizeText() strips ALL HTML tags before comparing citum's
output to real citeproc-js:

  .replace(/<[^>]+>/g, '')  // Strip HTML tags

This means the oracle/report-core fidelity pipeline is structurally blind to raw-HTML
leaking into citum's rendered output -- it strips the exact defect from both sides
before comparing, so a regression of csl26-6eoi's bug (or any similar raw-markup leak)
would never move fidelityScore or the pass/fail counts.

Confirmed empirically: gb-t-7714-2025-numeric's fidelityScore (0.996) and GB/T upstream
corpus pass rate (193/203) were IDENTICAL before and after fixing csl26-6eoi, despite
that fix changing citum's actual rendered text for the affected entries (raw
`<span class="nocase">...</span>` -> clean interpreted text).

## Suggested fix
Add a raw-HTML-tag assertion to the fidelity pipeline (e.g. in report-core.js or a
dedicated check) that fails if citum's *un-normalized* bibliography/citation output
contains `<[a-z]+[^>]*>` -- run before normalizeText() strips it, so a future
regression of this class of bug is actually caught.
