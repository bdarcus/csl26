---
# csl26-uy29
title: 'Same-year issued-date sort: month/day precision beats title tiebreak'
status: todo
type: bug
priority: normal
created_at: 2026-08-30T14:08:20Z
updated_at: 2026-08-30T14:08:20Z
---

Surfaced while root-causing csl26-rrsb's letter-ordering defect (root 2:
title-sort article-stripping). Fogel's two 2004 works
(6188419/ESET6WVE "Technophysio evolution..." article-journal,
6188419/QB9KGZ82 "The escape from hunger..." book) still get swapped
year-suffix letters after both the article-stripping fix AND a
Title::Shorthand sort-text fix landed -- neither explains it. Root-caused
via crates/citum-engine/src/processor/disambiguation.rs unit-level testing
with an explicit Author->Issued->Title GroupSort (matching
SortPreset::AuthorDateTitle, what chicago-author-date-18th resolves):

`ReferenceSorter::cache_sort_value`'s `CompiledSortKey::Issued` branch
(crates/citum-engine/src/sorting.rs) compares full `(year, month, day)`
tuples via `issued_date_parts`, with a missing month/day defaulting to 0
(per the archived bean csl26-xm1n, which widened this from year-only to
fix a genuine same-year month-ordering case -- Gourmet May vs. September,
both WITH month precision). But when one same-year entry has real
month/day precision (Technophysio: issued 2004-06-01, month=6) and another
has none (Escape: issued 2004, defaults to month=0), the Issued key -- the
SECOND key in Author->Issued->Title, ahead of Title -- resolves month 0 <
month 6 and decides the sort before Title is ever consulted. Escape sorts
first (gets 'a'), even though citeproc-js's oracle order (confirmed via
the Fogel pair, letter a=Technophysio) implies citeproc does NOT do this --
either its bibliography sort compares year only for this style/macro, or
it treats a missing month/day differently (e.g. not defaulting to the
lowest possible value).

Confirmed reproduction (temporary unit test, not committed -- see PR
history for csl26-rrsb): two same-author, same-year (2004) references,
"Alpha Title" (issued 2004-06-01) and "Zulu Title" (issued 2004, no
month/day), under an explicit Author/Issued/Title GroupSort. Title order
alone would put Alpha first ('A' < 'Z'); actual result puts Zulu first
(month 0 < month 6).

## Scope

Needs research into what citeproc-js actually does for a same-year
sort comparison when date precision differs between two entries --
year-only comparison, a different missing-precision convention (treating
absent month/day as "last" rather than "first"), or per-style macro
differences -- before any engine change. `compare_by_issued`/
`CachedSortValue::Issued` (sorting.rs) is shared by every style with an
Issued sort key, so any fix needs a full embedded-parity-baseline sweep,
not a Chicago-scoped one -- same discipline as csl26-rrsb's own gate.

Not fixed by csl26-rrsb's two landed commits (year-suffix range-key,
article-stripping removal) or its Title::Shorthand sort-text correctness
fix -- confirmed via isolated reproduction, distinct root cause.
