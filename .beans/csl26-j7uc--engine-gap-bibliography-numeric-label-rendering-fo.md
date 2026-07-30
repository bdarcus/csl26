---
# csl26-j7uc
title: 'Engine gap: bibliography numeric-label rendering for second-field-align-derived styles'
status: todo
type: task
priority: normal
created_at: 2026-07-30T14:28:57Z
updated_at: 2026-07-30T14:28:57Z
parent: csl26-arly
---

Class A2 (318 of 2501 parity mismatches, the largest single class). citeproc-js's second-field-align is a processor-level feature: for numeric CSL styles that declare it, the processor generates the bibliography list number itself (rendered into a separate csl-left-margin DOM node) independent of whether the CSL template also renders a number inline. Citum has no equivalent -- a style only gets a bibliography number if its template explicitly includes a 'number: citation-number' component (confirmed: ieee.yaml has this, american-medical-association.yaml does not). Affects at minimum: nature (101 mismatches), american-chemical-society (53), american-medical-association + american-medical-association-alphabetical (47 each), and unblocks up to 8 exemplar styles currently reading exactly 0.0% oracle text parity. Root cause and disposition options (processor-level auto-numbering vs. requiring each affected style to add an explicit number component) documented in docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md. Not started.
