---
# csl26-jxco
title: 'Chicago: title quote boundary, all source types at once'
status: todo
type: task
priority: high
tags:
    - style
    - chicago
    - fidelity
    - title
    - punctuation
created_at: 2026-08-23T20:40:45Z
updated_at: 2026-08-23T22:20:39Z
parent: csl26-h7oc
---

Leverage class from the 2026-08-23 audit. 300 entries -- the single largest class family-wide. Supersedes/extends completed cluster csl26-87yl, which fixed article-newspaper + thesis quoting one type at a time (+1 entry) and explicitly deferred map/dataset/report/webpage. This bean's scope is every source type at once, verified per-type against citeproc-js render sites per docs/specs/CHICAGO_FAMILY_STRATEGY.md's authority rule. Touches all four Chicago variants.

## Update from wave 1 (2026-08-23)

Wave 1 (csl26-4xr6, title case) discovered this class's scope is larger
than originally estimated. Adding `manuscript`/`motion-picture`/
`broadcast`/`collection`/`song`/`webpage` to `chicago-notes-18th`'s
and `chicago-shortened-notes-bibliography-core`'s `titles.type-mapping`
(mirroring chicago-author-date-18th's existing list) regressed 3
previously-passing archival/manuscript entries: those types picked up
this family's `titles.component.quote: true`, which is correct for
genuine article titles but wrong for bare collection titles like
'Revere Family Papers' and 'Landscapes of Zambia, Central Africa'. Wave
1 landed narrower (map/dataset only) and reverted that list; this bean
now also owns:

- Extending `chicago-notes-18th`/`chicago-shortened-notes-bibliography-core`'s
  type-mapping to manuscript/motion-picture/broadcast/collection/song/
  webpage, per-type, with the same regression verification wave 1 used
  (per-entry exactMatch diff, not just aggregate count).
- `map`/`dataset` in `chicago-author-date-18th`/
  `taylor-and-francis-chicago-author-date` are now correctly title-cased
  (wave 1) but still fail exact match because of this class's quote
  boundary -- these are concrete, ready-to-verify entries to start from
  (previously: 'The Racial Dot Map' rendered quoted when the oracle does
  not quote map titles at all).

See docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md's
postscript for the full wave-1 writeup.
