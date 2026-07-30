---
# csl26-w0hf
title: Embedded-tier oracle parity to 100%
status: in-progress
type: milestone
priority: high
created_at: 2026-07-30T19:09:35Z
updated_at: 2026-07-30T19:09:41Z
---

Reach 100% oracle text parity (byte-for-byte match with citeproc-js for
CSL-derived styles) across the embedded style tier. Currently 38.7%
(baseline docs/compat.html at fb0a01af) -- see docs/architecture/DESIGN_PRINCIPLES.md
for the parity commitment this target derives from.

Canonical live source: docs/compat.html (regenerate via
`node scripts/report-core.js --write-html`).

This milestone exists because the work was split across two disconnected
epic threads with no shared parent, which made it look like the project
was "done" when csl26-arly's children completed even though the headline
number barely moved:

- csl26-arly (completed): harness-integrity fixes, non-Chicago numeric-label
  gap (class A2), and other narrow-scope defects. Its own scope is fully
  closed, but it explicitly excludes the Chicago family.
- csl26-40n4 (in-progress): the Chicago family pivot -- 55.7% of all 2501
  tracked oracle-parity mismatches (chicago-author-date-18th,
  chicago-notes-18th, chicago-shortened-notes-bibliography,
  taylor-and-francis-chicago-author-date). This is where most of the
  remaining gap to 100% lives.

## Next actions (as of 2026-07-30)

Verified via direct beans GraphQL queries -- these are genuinely unblocked
right now, even though some don't surface via `beans list --ready` (its
readiness computation appears to suppress grandchildren of a not-yet-started
parent):

- [ ] csl26-t6dg (todo, unblocked) -- "Support paired EDTF uncertainty
      markers for Chicago guessed dates". Unblocks csl26-giun, the Chicago
      variant with the most tuning history/momentum (chicago-author-date-18th,
      344/400 bibliography as of 2026-07-02).
- [ ] csl26-7jht (todo, unblocked) -- tune chicago-shortened-notes-bibliography
      to full fidelity. Independently startable, no dependency on t6dg.
- [ ] csl26-gzwj (todo, unblocked) -- tune taylor-and-francis-chicago-author-date
      to full fidelity. Independently startable.
- [ ] csl26-lxy3 (todo, unblocked) -- tune chicago-notes-18th to full
      fidelity. Independently startable.

Re-derive this list with:
`beans query '{ bean(id: "csl26-40n4") { children { id title status blockedBy { id status } } } }'`
and recurse into any in-progress children.
