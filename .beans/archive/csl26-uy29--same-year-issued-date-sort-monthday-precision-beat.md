---
# csl26-uy29
title: 'Same-year issued-date sort: month/day precision beats title tiebreak'
status: completed
type: bug
priority: normal
created_at: 2026-08-30T14:08:20Z
updated_at: 2026-08-30T23:27:50Z
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

## Summary of Changes

Root-caused via citeproc-js's actual `<sort>` key list in `styles-legacy/chicago-author-date.csl` (not guessed): `date-sort-year` renders **year only** ("only the year is to be taken into account", CMOS18 13.114, quoted directly in the CSL macro comment) — full-precision date only enters much later, via a raw `<key variable="issued"/>` positioned *after* the title/source/volume keys. Citum's single `Issued` sort key was comparing the full `(year, month, day)` tuple in one step, both deciding the primary Author→Issued→Title order (wrongly overriding Title) and serving as the only date-based tiebreak.

Fix (`crates/citum-engine/src/sorting.rs`, `compare_cached_value`/`compare_cached_values`): the `Issued` key's own comparison is now year-only (`compare_none_last` on just the year). `compare_cached_values` gained an implicit post-loop tiebreak: if the resolved template used an `Issued` key, and every key tied (including Title), the full year/month/day comparison decides — mirroring CSL's own two-stage design without a new schema field or a second, separately-configured sort key.

Both real-world cases verified against citeproc-js:
- **Fogel pair** (`csl26-uy29`'s trigger, `6188419/ESET6WVE`/`QB9KGZ82`, same author/year, one with month precision): previously mismatched (Citum gave Escape 'a', oracle gives Technophysio 'a'). Now matches exactly — confirmed via `node scripts/oracle.js styles-legacy/chicago-author-date.csl --refs-fixture tests/fixtures/test-items-library/chicago-18th.json --scope bibliography`.
- **Gourmet pair** (`csl26-xm1n`'s original case, `6188419/9TVQWT5I`/`UMQEEZLJ`, same author/year/title, different months): unchanged, still correct (May before September) — the implicit tiebreak preserves it.

Two new regression tests in `crates/citum-engine/src/sorting.rs`: `test_issued_year_tie_falls_through_to_title_when_month_differs` (Fogel-shaped), `test_issued_year_and_title_tie_falls_through_to_full_date` (Gourmet-shaped, xm1n regression guard).

**Full embedded-parity sweep** (clean-`main`-worktree baseline vs. this branch, `node scripts/report-core.js --all-features`, both under `systemd-run --user --scope -p MemoryMax=6G`, 35 styles / ~500-1000+ entries each): **exactly 2 entries changed across the entire corpus**, both `6188419/ESET6WVE` (the Fogel Technophysio entry) in `chicago-author-date-18th` and `taylor-and-francis-chicago-author-date` (the two styles sharing this fixture), both flipping `exactMatch: false → true`. Zero regressions anywhere else; `chicago-author-date-18th`'s exact-parity rate moved 211/542 → 212/542.

Verification: `just pre-commit` (fmt + clippy -D warnings + full `cargo nextest run`, 2721/2721 pass, workspace-wide). `cargo nextest run -p citum-engine`: 1420/1420 pass.

## Follow-up (Codex adversarial review of the stack)

Review flagged (medium): "Chicago-specific year-only sorting is applied to every Issued sort key" -- a fair question about whether the year-only convention generalizes past Chicago. Checked primary sources: `apa.csl`'s `date-sort` macro and `elsevier-harvard.csl`'s `issued` sort macro both also extract year-only (`<date date-parts="year">` / `<date-part name="year"/>`), confirming this is the norm across major author-date CSL styles, not a Chicago-only quirk -- and elsevier-harvard has no `title` sort key at all, meaning the fix's trailing full-date tiebreak is the *only* mechanism that can break a same-year tie for that style. Strengthened `compare_cached_values`'s doc comment with this cross-style evidence and added `test_author_date_title_preset_matches_the_year_only_fix_for_non_chicago_styles`, exercising `SortPreset::AuthorDateTitle::group_sort()` directly (the actual resolved template APA/Elsevier-Harvard/etc. get via `Processing::AuthorDate`/`AuthorDateFull`), not a hand-rolled stand-in.
