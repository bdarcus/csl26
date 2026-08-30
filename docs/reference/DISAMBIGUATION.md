# Disambiguation in Citum

> **Normative design:** [`docs/specs/DISAMBIGUATION.md`](../specs/DISAMBIGUATION.md)

Disambiguation is the process of modifying citation output when
multiple references produce identical rendered strings. CSL 1.0 provides
several strategies to resolve these ambiguities.

## Overview

When citations are identical (e.g., multiple works by "Smith, 2000"),
Citum applies disambiguation strategies in cascade order, from least to
most disruptive, stopping at the first that resolves the collision:

1. **Name Expansion** (`disambiguate-add-names`) — reveal names hidden by et-al.
2. **Given Name Addition** (`disambiguate-add-givenname`) — add initials/given
   names; scope set by `givenname-disambiguation-rule`.
3. **Year Suffix** (`disambiguate-add-year-suffix`) — append `a`, `b`, `c`… to
   the issued year.

Once a strategy resolves an ambiguity, later (more disruptive) strategies are
not applied. The normative cascade and per-guide application live in the
[spec](../specs/DISAMBIGUATION.md) (§2, §3.1).

> **Same surname ≠ year suffix.** Different authors who share a surname (e.g.
> "A. Johnson" vs "B. Johnson") are disambiguated by *given names/initials*, never
> by a year suffix — year suffixes apply only to the *same author, same year*. APA
> needs `primary-name-with-initials` for this — not because `by-cite` compares fewer
> references (it doesn't; see below), but because APA §8.20 shows initials on the
> *first* author only, which `primary-name` restricts to.
>
> **MLA disambiguates by short title, not suffix.** MLA is author-*page*: it sets
> `year-suffix: false` and resolves same-author works with a `disambiguate-only`
> short title in the citation template.

## Year Suffix

Year suffix appends letters (a, b, c, ..., z, aa, ab, ...) to the year
when multiple references share the same year and other identifying
information.

```yaml
citation:
  options:
    disambiguate-add-year-suffix: true
```

### Suffix Generation

Suffixes are assigned based on a deterministic sort order:

- Primary: By first appearance in citation
- Secondary: By reference identifier or title (varies by style)

### Example

Three references all by "Smith, 2000":

```
Smith, 2000a
Smith, 2000b
Smith, 2000c
```

## Name Expansion

When author names are abbreviated (e.g., "et al."), expanding the name
list can disambiguate:

```yaml
citation:
  options:
    disambiguate-add-names: true
    et-al-min: 3
    et-al-use-first: 1
```

### Behavior

- If et-al is triggered (e.g., "Smith et al."), expand to full author
  list
- If full list is already shown, name expansion cannot help
- Can be combined with given name expansion for maximum differentiation

### Example

Two works with same first author and year:

```
Smith, Brown, et al. (2000)
Smith, Beefheart, et al. (2000)
```

Becomes:

```
Smith, Brown, Jones (2000)
Smith, Beefheart, Williams (2000)
```

## Given Name Expansion

Adding initials or full given names to the author list:

```yaml
citation:
  options:
    disambiguate-add-givenname: true
    givenname-disambiguation-rule: "by-cite"
```

### Rules

All rules detect collisions the same way: against every reference in the document,
never a subset. What differs is *which name positions* are eligible to expand:

- **by-cite**: caps how far a name is forced to escalate — a name position
  identical across every colliding reference (e.g. a shared first author)
  stays at its default depth instead of gratuitously expanding to the full
  given name
- **all-names**: apply to all uses of the name (ensures consistency
  across document)
- **primary-name**: apply given names only to the first author position

### Example

Multiple "Smith, J." authors, disambiguated document-wide:

```
Smith, John (1980)
Smith, Jane (1985)
```

See [the spec](../specs/DISAMBIGUATION.md), §2.1.1, for the per-position
escalation model (csl26-5753) and how it interacts with the document-wide
collision pool. A reference is never left unexpanded merely because it was
cited alone — that was a defect (csl26-8nrt), not `by-cite` semantics.

## Group-Aware Disambiguation

Citum supports advanced disambiguation controls within bibliography groups. This is essential for legal bibliographies or multilingual works where local sorting rules must drive year suffix assignment.

### Group-Aware Sorting

When a `BibliographyGroup` defines a custom `sort`, the `Disambiguator` respects that sort order for year suffix assignment (e.g., 2020a, 2020b). This ensures suffixes follow the logic of the group (like Case Name) rather than the default global sort.

### Localized Suffixes

The `disambiguate: locally` option allows a group to perform disambiguation independently of the rest of the bibliography.

- **Scenario:** A legal style may want suffixes in the "Cases" group to start from "a" even if those same years were used in the "Books" group.
- **Behavior:** Suffix sequences reset at the start of the group.

```yaml
groups:
  - id: cases
    heading: "Cases"
    disambiguate: locally
    sort:
      template:
        - key: title  # Sort cases by name
```

### Locale-Aware Collation

Disambiguation is culturally aware. By passing a `Locale` to the disambiguator, Citum ensures that name matching and sorting follow locale-specific rules (e.g., handling of particles, diacritics, and transliterations in keys).

## Combined Strategies

Multiple strategies can be active simultaneously. The processor applies
them in order, stopping at the first successful disambiguation.

### Example: APA 7th Edition

APA uses all three strategies with the `primary-name` given-name rule so
same-surname authors get initials on the *first* author only in every in-text
citation (APA §8.20) — `by-cite` would expand every colliding position instead
of just the first. This is the major author-date guide profile, bundled by the
**`author-date-full` preset** (names + add-givenname +
`primary-name` + year-suffix), which APA and Chicago AD share
(see [`apa-7th.yaml`](../../crates/citum-schema-style/embedded/styles/apa-7th.yaml)):

```yaml
options:
  processing: author-date-full
```

The initials form comes from APA's contributor config (`initialize-with`); Chicago,
with the same preset, renders the full first given name. The preset also supplies the
`author-date-title` bibliography sort, so no explicit `bibliography.sort` is needed.

## Test Coverage

Disambiguation behavior is verified through functional integration tests in the `citum_engine` crate:

### Functional Tests (`citations` target)

Tests verify disambiguation logic including:
- Year suffix collation and sorting
- Name expansion interactions with et-al
- Given name disambiguation by-cite and all-names rules
- Fallback behaviors and edge cases

**Run:**

```bash
cargo nextest run --test citations
```

The disambiguation system is fully integrated:
- Year suffix rendering (a-z, aa-az wrapping for 26+ items)
- Et-al expansion based on disambiguation needs
- Given name/initial expansion for conflicting surnames
- Cascading fallback strategies
- Full test coverage for common CSL 1.0 scenarios

Test file: `../crates/citum-engine/tests/citations.rs`

## Performance Characteristics

Disambiguation runs once per citation during processing:

1. **Single-pass calculation**: Hints computed once per `Processor::process_citation()` call
2. **Reference grouping**: References grouped by author-year key for collision detection
3. **Hint propagation**: Pre-calculated hints passed through rendering pipeline
4. **No runtime overhead**: Disambiguation logic doesn't slow down component rendering

For large bibliographies (1000+ items), disambiguation adds <5% overhead vs non-disambiguated rendering.

## Implementation Details

### Processor

Citation processor applies disambiguation after rendering:

1. Render all citations with initial style settings
2. Identify duplicates by rendered string
3. For each duplicate group, apply strategies incrementally
4. Re-render affected citations

### Data Flow

```
Reference → [Render] → String
              ↓
          [Deduplicate]
              ↓
      [Apply Year Suffix] (if enabled + ambiguous)
              ↓
      [Apply Name Expansion] (if enabled + ambiguous)
              ↓
      [Apply Given Names] (if enabled + ambiguous)
              ↓
          Output String
```

## Known Limitations

- **Fallback on exhaustion**: If all strategies fail (52+ identical
  entries), year suffix wraps (a→z→aa, etc.)
- **No cross-document**: Disambiguation is per-document; different
  documents may use inconsistent suffixes for the same reference
- **Suffix order tracks the bibliography**: year-suffix letters follow the
  effective bibliography sort (literal-text title as the same-author/
  same-year tiebreaker — no leading-article stripping, matching
  citeproc-js), not reference input order. Suffixes are recomputed when the
  bibliography context changes.

## Test Case Reference

### Current Test Cases (15 total)

1. `disambiguate_YearSuffixAndSort` - Year suffix with bibliography
   sort
2. `disambiguate_YearSuffixAtTwoLevels` - Nested year suffix
   collapsing
3. `disambiguate_YearSuffixMixedDates` - Partial date handling
4. `disambiguate_ByCiteTwoAuthorsSameFamilyName` - Givenname by-cite
   rule
5. `disambiguate_AddNamesSuccess` - Name expansion resolves ambiguity
6. `disambiguate_AddNamesFailure` - Name expansion insufficient
7. `disambiguate_ByCiteGivennameShortFormInitializeWith` - Initials
   in by-cite mode
8. `disambiguate_ByCiteMinimalGivennameExpandMinimalNames` -
   Citation-local by-cite expansion
9. `disambiguate_ByCiteGivennameExpandCrossNestedNames` - Nested
   by-cite name expansion
10. `disambiguate_ByCiteBaseNameCountOnFailureIfYearSuffixAvailable` -
    By-cite base-name counting with year-suffix fallback
11. `disambiguate_AllNamesBaseNameCountOnFailureIfYearSuffixAvailable` -
    All-names base-name counting with year-suffix fallback
12. `disambiguate_BasedOnEtAlSubsequent` - Et-al with subsequent names
13. `disambiguate_ByCiteDisambiguateCondition` - Conditional
   rendering when disambiguate=true
14. `disambiguate_FailWithYearSuffix` - Fallback behavior
15. `disambiguate_YearSuffixFiftyTwoEntries` - Large-scale year
    suffix wrapping

## Related Reading

- [CSL 1.0 Specification](https://citeproc-js.readthedocs.io/en/latest/csl-json/markup.html#disambiguation)
- [Citum Architecture](../architecture/MIGRATION_STRATEGY_ANALYSIS.md)
