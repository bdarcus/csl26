---
# csl26-j7uc
title: 'Engine gap: bibliography numeric-label rendering for second-field-align-derived styles'
status: in-progress
type: task
priority: normal
created_at: 2026-07-30T14:28:57Z
updated_at: 2026-07-30T15:02:35Z
parent: csl26-arly
---

Class A2 (318 of 2501 parity mismatches, the largest single class). citeproc-js's second-field-align is a processor-level feature: for numeric CSL styles that declare it, the processor generates the bibliography list number itself (rendered into a separate csl-left-margin DOM node) independent of whether the CSL template also renders a number inline. Citum has no equivalent -- a style only gets a bibliography number if its template explicitly includes a 'number: citation-number' component (confirmed: ieee.yaml has this, american-medical-association.yaml does not). Affects at minimum: nature (101 mismatches), american-chemical-society (53), american-medical-association + american-medical-association-alphabetical (47 each), and unblocks up to 8 exemplar styles currently reading exactly 0.0% oracle text parity. Root cause and disposition options (processor-level auto-numbering vs. requiring each affected style to add an explicit number component) documented in docs/architecture/audits/2026-07-30_EMBEDDED_PARITY_CLASS_A.md. Not started.

## Progress: american-medical-association fixed (embedded tier)

Fix: explicitly authored a `number: citation-number, suffix: '.'` component as the first element of every bibliography type-variant (18 variants) and the default template, wrapped together with the variant's original first component in a `delimiter: ""` group so the number sits flush against the following text (matches oracle exactly -- confirmed no join-space bug per the class-A audit). ieee.yaml already uses this exact pattern; american-medical-association.yaml had never had it authored.

americnn-medical-association-alphabetical inherits via 'extends: american-medical-association' for its citation/options blocks, but **redefines its own separate bibliography.type-variants** -- so it does NOT automatically inherit this fix. Needs the same 18-variant treatment applied to its own file.

**Verified (american-medical-association only):**
- node scripts/report-core.js --styles american-medical-association: fidelity 1.0 (unchanged), citations 20/20 (unchanged), bibliography 47/47 (unchanged, was already lenient-passing), **exactParity 16/67 (23.9%) -> 48/67 (71.6%)** -- 32 additional exact matches, all from the number-prefix fix.
- Direct render diff: all 47 entries now carry the number prefix, byte-identical to oracle on that boundary.
- citum check: OK (schema-valid).
- Remaining 15/67 mismatches are unrelated small defects (case: 'In:' vs 'in:'; abbreviation: 'phd-thesis' vs 'PhD thesis', 'CFR' vs 'C.F.R.'; a couple of stray leading-space quirks) -- out of scope for this bean, not investigated further.

**Remaining scope (not done):** nature (101 mismatches), american-chemical-society (53), american-medical-association-alphabetical (47) -- all exemplar tier, same mechanical pattern, each needs its own type-variant-by-type-variant pass (verified per-style, not just copy-pasted, since each style's component structure differs). royal-society-of-chemistry is explicitly NOT part of this fix -- its report evidence doesn't match its actual rendered output (separate bug, csl26-waly).
