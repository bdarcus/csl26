# Sorting Specification

**Status:** Active
**Date:** 2026-05-31
**Related:** [`EXPLICIT_DEFAULT_SORTING.md`](./EXPLICIT_DEFAULT_SORTING.md),
  [`UNICODE_BIBLIOGRAPHY_SORTING.md`](./UNICODE_BIBLIOGRAPHY_SORTING.md),
  [`MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md),
  [`MULTILINGUAL_SORTING.md`](./MULTILINGUAL_SORTING.md),
  sorting sections of [`MULTILINGUAL.md`](./MULTILINGUAL.md)

## Purpose

Canonical end-to-end specification for bibliography and citation sorting in Citum.
Sorting predates the spec-driven-development policy, so this document captures shipped
behavior and design intent, and references narrower specs that extend or constrain it.

## Scope

**In scope:** bibliography sort resolution, citation-cluster sort policy, sort keys and
presets, collation, secondary/tiebreak rules, grouping interplay.

**Out of scope:** script/language partitioning (see
[`MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md)),
transliteration-aware sort keys and multilingual sort modes (see
[`MULTILINGUAL_SORTING.md`](./MULTILINGUAL_SORTING.md)), per-entry sort overrides.

## Core Separation of Concerns

Following biblatex's design, Citum separates sorting into two independent concerns:

| Concern | Schema location | Who may set it |
|---|---|---|
| Bibliography ordering | `bibliography.sort` | Style author; falls back to processing-family default |
| Citation-cluster ordering | `citation.sort` | Style author only; no family default |

These two sort specifications are fully independent. A style may have a `citation.sort`
without any `bibliography.sort`, or vice versa.

## Bibliography Sort Resolution

Resolution is applied at bibliography-render time, in priority order:

1. Explicit `bibliography.sort` in the style YAML.
2. Processing-family default (`Processing::default_bibliography_sort()`).
3. No sort — preserve insertion order.

### Processing-Family Bibliography Defaults

| Processing class | Default bibliography sort preset |
|---|---|
| `author-date` | `author-date-title` |
| `note` | `author-title-date` |
| `label` | `author-date-title` |
| `numeric` | None (insertion order) |

These defaults exist because author-date and note families have strong conventional
ordering, while numeric styles depend on their own numbering logic.

**Implementation:** `crates/citum-schema-style/src/options/processing.rs` →
`Processing::default_bibliography_sort()` and `Processing::config()`.

## Citation-Cluster Sort Policy

Citation-cluster ordering is explicit-only in the current implementation:

- If `citation.sort` is present, apply it.
- Otherwise preserve citation input order.

No processing family provides an implicit citation-list sort. This mirrors
biblatex's `sortcites` opt-in philosophy.

## Sort Keys

Sort keys are defined by `SortKey` (non-exhaustive) in
`crates/citum-schema-style/src/options/processing.rs`:

| Key | Semantics |
|---|---|
| `Author` | Primary author name (family-first); falls back to editor, then title if no contributor. When the Author key falls back to title (no contributor present), the title value is normalized through the same `title_sort_key_with_options` pass as `SortKey::Title`. |
| `Year` | Issued date year; year-bearing entries precede year-less entries |
| `Title` | Literal title text (markup-stripped, multilingual-resolved, `Title::Shorthand` sorts by its full form) — no leading-article stripping, matching citeproc-js's `variable: title` sort semantics (`title_sort_key_with_options`; `Locale::strip_sort_articles` exists as public API but is unused by this call site, csl26-rrsb) |
| `CitationNumber` | Reserved for citation-cluster sort templates. In a bibliography sort template it produces `Equal` for all comparisons (effectively a no-op there) — numeric ordering of bibliography entries is assigned by the citation-processing pass, not by sorting. |

Each key has an `ascending` flag (default `true`).

## Sort Presets

Named `SortPreset` values resolve to fixed `SortKey` chains:

| Preset | Key chain |
|---|---|
| `author-date-title` | `Author → Year → Title` |
| `author-title-date` | `Author → Title → Year` |

Styles may also supply a custom `SortSpec` template instead of a named preset.

## Collation

All text comparisons (author, title keys) use a locale-aware `TextCollator`
(`crates/citum-engine/src/sort_support.rs`), backed by ICU4X when the `icu`
feature is enabled.

Configuration:
- **Strength:** Secondary — base letters and diacritics distinguished; case is not.
- **Case level:** Off — case-insensitive via collator configuration, not lowercasing.
- **Alternate handling:** Shifted — punctuation and whitespace ignorable at primary/secondary
  levels (leading "al-", "O'", etc. do not break alphabetical ordering).
- **Locale fallback:** Progressively strips subtags (`de-DE-x` → `de-DE` → `de`) until a
  recognized locale is found; falls back to `en-US`.

Full collation semantics are specified in
[`UNICODE_BIBLIOGRAPHY_SORTING.md`](./UNICODE_BIBLIOGRAPHY_SORTING.md).
Optional multilingual sort modes (`options.sorting` — romanized sort keys,
per-script shorthand) layer on top of this collator and are specified in
[`MULTILINGUAL_SORTING.md`](./MULTILINGUAL_SORTING.md).

## Deterministic Tiebreaking

When all sort-key comparisons produce `Equal`, entries are ordered by citation-key
string comparison (`id.0.as_str()`). Entries without an ID sort last. The underlying
sort is stable, so entries that are collator-equal through all steps retain their original
input order if their IDs are also equal.

## Grouping Interplay

- Numeric citation-number initialization and year-suffix/disambiguation ordering both
  consume the resolved bibliography sort; they must be applied after sort resolution.
- Grouped bibliographies (`bibliography.groups`) apply their own per-group sort independently.
  Partition-aware sorting (`sort-partitioning`) runs as a pre-pass before the normal key
  chain; see [`MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`](./MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md).

## Key Implementation Files

| File | Role |
|---|---|
| `crates/citum-engine/src/processor/sorting.rs` | `Sorter` struct; multi-key sort dispatch |
| `crates/citum-engine/src/sort_support.rs` | `TextCollator`, `author_sort_key_opt`, `title_sort_key` |
| `crates/citum-engine/src/grouping/sorting.rs` | Grouped bibliography sort integration |
| `crates/citum-engine/src/sort_partitioning.rs` | Script/language partition pre-pass |
| `crates/citum-schema-style/src/options/processing.rs` | `SortKey`, `SortSpec`, `SortEntry`, `SortPreset`, `Processing::default_bibliography_sort()` |

## Test Anchor

`crates/citum-engine/tests/sort_oracle.rs` — end-to-end bibliography and citation sort
behavior. Bibliography-specific sort tests: `mod sorting` in
`crates/citum-engine/tests/bibliography.rs`.

## Open Work

- `EXPLICIT_DEFAULT_SORTING.md` tracks any remaining cleanup around
  `Processing::default_citation_sort_policy()` and `CitationSortPolicy::ExplicitOnly`.
  Both are already public (`processing.rs:162`, `processing.rs:207`) and re-exported
  from `options/mod.rs`; the spec's implementation steps are effectively complete.
- Per-script partitioning (`sort-partitioning`) acceptance criteria are tracked in
  `MULTILINGUAL_BIBLIOGRAPHY_PARTITIONING.md`.

## Changelog

- 2026-05-31: Initial version — documents shipped behavior; references narrow sub-specs.
- 2026-07-08: Reference `MULTILINGUAL_SORTING.md` for multilingual sort modes and transliteration-aware sort keys.
- 2026-08-30: Removed unconditional leading-article stripping from
  `SortKey::Title` (and the `Author`-falls-back-to-title case). CSL has no
  automatic article-stripping for `variable: title` sorting; citeproc-js
  sorts the literal title text. Citum's stripping was never independently
  oracle-validated and caused year-suffix letters to diverge from
  citeproc-js on same-year collisions where exactly one title carried a
  leading article. See `DISAMBIGUATION.md` §3 for the disambiguation-side
  detail and csl26-rrsb.
