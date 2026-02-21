---
# csl26-reux
title: MLA-specific schema and rendering gaps
status: completed
type: feature
priority: normal
created_at: 2026-02-21T20:58:42Z
updated_at: 2026-02-21T20:58:42Z
blocked_by:
  - csl26-wms5
---

Remaining gaps in modern-language-association.yaml blocking 3/32 bibliography
entries (entries 30, 31, 32 in the oracle fixture). Current fidelity: 29/32
bibliography, 11/13 citations.

## Missing contributor roles

MLA uses contributor roles not yet in CSLN schema:
- director -- needed for film entries (Entry 30: "Directed by Louis Lumiere")
- interviewer -- needed for interview entries (Entry 31: "Interview by Stephen Colbert")

These roles need to be added to the contributor role enum in csln_core and
then the MLA bibliography template can use form: verb with these roles.

## Abbreviated month date format

Entry 31 oracle output: "10 Nov. 2023" -- day + abbreviated month + year.
Current CSLN date options support month: long or month: short but the
MLA format requires a day-month-year order with abbreviated month
(Jan., Feb., Mar., etc.).

## Media type / genre field

Entries 30 and 31 include media type labels ("Short film", "Film",
"Video interview") from the reference genre or medium field.
CSLN needs a variable: genre or variable: medium component to render these.

## Patent number field

Entry 32 oracle: "US 11,043,211 B2, 13 Jul 2021"
Needs a variable: number or dedicated patent-number component rendering
the full patent identifier including jurisdiction prefix and kind code.

## Citation: family-name-only contributor form

MLA in-text citations show family name only (e.g., "Hawking" not "Hawking, S.").
Currently no contributor form renders family name only -- form: long renders
"Hawking, S." and form: short renders "Hawking, T. S.".
A form: family-only contributor form (or equivalent) is needed.

Fix csl26-wms5 (group_length disambiguation) first so disambiguate_only title
suppression works before changing the citation author form.

