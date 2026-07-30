---
# csl26-j7uc
title: 'Engine gap: bibliography numeric-label rendering for second-field-align-derived styles'
status: completed
type: task
priority: normal
created_at: 2026-07-30T14:28:57Z
updated_at: 2026-07-30T15:50:31Z
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

## Progress: nature, american-chemical-society, american-medical-association-alphabetical fixed (exemplar tier)

Mechanized the same transform via a small text-based Python script (finds the type-variant/default-template anchor, wraps its first list item in `delimiter: ""` + `group: [number: citation-number <wrap>, <original first item>]`), since the per-file repetition is genuinely mechanical once the wrap format is known. The wrap format itself is NOT mechanical -- had to read oracle output per style: nature and american-medical-association-alphabetical use plain `suffix: "."`; american-chemical-society uses `wrap: punctuation: parentheses` (already partially present on its own `patent` type-variant, but broken by the same delimiter-leak bug -- fixed as a one-off, not via the generic script).

**Script bug caught before landing:** the generic script matched the FIRST literal `  template:` line in american-medical-association-alphabetical.yaml, which was the citation template (also called `template:`), not bibliography's -- would have double-wrapped the citation-number/locator group. Fixed by anchoring searches to start after `bibliography:`, reverted and reran clean.

**Verified (node scripts/report-core.js --styles nature,american-chemical-society,american-medical-association-alphabetical):**
- american-medical-association-alphabetical: fidelity 1.0 (unchanged), citations/bibliography unchanged, exactParity 12/67 (17.9%) -> 33/67 (49.3%).
- american-chemical-society: citations 28/28 (unchanged), exactParity 24/82 (29.3%) -> 30/82 (36.6%). fidelityScore ticked down 0.951 -> 0.939 and lenient bibliography match 50/54 -> 49/54 -- investigated, NOT a regression: bibliography pairing is similarity-based, and adding numbers changed the pairing solution. One 'hearing'-type entry that was previously mis-paired against a different, coincidentally-similar oracle line (falsely showing match:true) now correctly pairs with its real oracle counterpart and correctly shows match:false on a PRE-EXISTING title-case/trailing-period defect unrelated to numbering (confirmed: the diff never touches title-casing; direct citum render output for that item is unchanged aside from the added number). Net effect is more accurate measurement, not worse rendering. Flagging the exposed hearing-type title-case defect for a future style-maintain pass, not filing a new bean for one entry.
- nature: fidelity 0.966 (unchanged), citations/bibliography unchanged, **exactParity unchanged at 40/149 (26.8%)** despite the fix working correctly (direct render confirms every entry now carries its number, byte-identical to oracle on that boundary). Nearly all of nature's 101 originally-attributed 'class A' mismatches are actually COMPOUND defects -- number missing AND a second, independent defect (name-list '&' vs ',' conjunction, container-title trailing punctuation) on the same entry. Fixing only the number doesn't flip these to exact match. This means the original taxonomy's per-class counts overstate what a single-class fix buys when defects co-occur -- worth remembering for future wave planning.

All three: citum check OK, node --test scripts/oracle.test.js 53/53 passing, no .rs touched.
