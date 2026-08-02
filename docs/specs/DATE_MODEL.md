# Date Model Specification

## Status
- **Date**: 2026-04-08
- **Status**: Active

## Objective
Refine the Citum date model to distinguish between content creation/origination (`created`) and formal publication/issue events (`issued`). This addresses the publication-centric bias of existing citation schemas (like CSL) which poorly represent all major resource types, including archival, unpublished, and born-digital material.

## Background
In many descriptive standards (including archival standards like DACS), the primary date of a record is its creation date. Using `issued` for an unpublished letter, personal manuscript, or dataset is semantically incorrect and forces non-traditional data into a publication-oriented mental model.

## Proposed Model

### Core Date Fields
- `created`: (EDTF) The primary creation or origination date of the content (e.g., when the text was written, the dataset compiled, the software release developed, the recording performed). Applicable to both published and unpublished works.
- `issued`: (EDTF) When the work was formally published or released. For unpublished materials, this field remains empty.
- `available`: (EDTF) When a work first became accessible to its intended or actual audience (e.g., online release preceding formal publication, preprint deposits, or internal reports).
- `accessed`: (EDTF) The date a specific digital instance of a resource (e.g., a URL) was last viewed.

*Naming Note: The field name `issued` is chosen for compatibility with existing citation ecosystems (primarily CSL and Zotero). It denotes the formal release/publication date of a work. Future revisions MAY revisit this name (e.g., `published`) if compatibility with those ecosystems becomes less important.*

### Role-Specific Dates
- `recorded`: For audio-visual works, oral histories, or performances.
- `filed`: For patents, legal filings, or bureaucratic records.
- `revised`: For updated editions or versioned documents.
- `copyright`: A copyright year, used as a publication-year substitute when
  the true issue date is unknown (e.g. a CSL `c1988` literal). This is a
  distinct date event, not an approximate/circa qualifier on `issued`.
- `printing`: A printing/impression year, another publication-year
  substitute distinct from `copyright` (e.g. a Chinese-suffixed literal like
  `1995印刷`).

These role-specific dates supplement `created` and `issued`; they do not replace `created` as the canonical creation date.

### Date Annotations

Any date field may need to retain source-calendar wording alongside its
canonical EDTF value. The
[Date Annotations specification](./CALENDAR_DATE_ANNOTATIONS.md) defines a
backward-compatible structured form:

```yaml
issued:
  value: "1947"
  note: "民国三十六年"
```

The EDTF `value` remains the sole input for sorting, disambiguation, and
other date computation. `note` is opaque display metadata — it identifies no
calendar system and appears only when a style opts in — so it applies
uniformly to any date field, not just a calendar-note special case. Existing
scalar input such as `issued: "1947"` remains canonical when no annotation is
present.

### Publication-Year Substitutes (GB/T 7714 §7.5.4.3)

When the true publication year is unknown, GB/T 7714 §7.5.4.3 defines three
substitute forms, which Citum models two different ways:

| Substitute | Citum model | Example |
|---|---|---|
| Copyright year | `copyright` field | `c1988` |
| Printing/impression year | `printing` field | `1995印刷` |
| Estimated year | `issued` marked EDTF approximate (`~`) | `1936~` → `[1936]` |

Copyright and printing are distinct *date events* standing in for an
unknown publication year, so each gets its own field; a style can chain them
into an `issued` fallback (see `gb-t-7714-2025-base.yaml`). An estimated
year is the *same* publication date, only marked inferred — it stays on
`issued` as an EDTF approximate date, and a style renders that qualifier
however it chooses (GB/T wraps it in brackets via
`approximation-marker`/`approximation-marker-suffix`).

An earlier revision (PR #1064) misread a copyright-year literal like
`c1988` as EDTF circa and normalized it to `1988~`, conflating copyright
with genuine approximation. The two are unrelated: circa marks uncertainty
about an otherwise-known date, while copyright substitutes an entirely
different date for an unknown one.

## Mapping & Downstream Compatibility

### Citum to CSL
- If `type` is a publication type (book, article, etc.), use `issued` when present; if `issued` is missing, fall back to `created` when mapping to CSL `issued`.
- If `type` is an inherently unpublished type (e.g., personal manuscripts, letters, many archival records, personal communications), map `created` to CSL `issued`.
- This ensures Citum remains the semantically "pure" source while maintaining compatibility with legacy formatters.

## Implementation Plan
1. **Schema Update**: Add `created` and other specific date fields to `WorkCore` and relevant specialized structs.
2. **Logic Refinement**: Update rendering and sorting logic to prioritize `created` when `issued` is missing.
3. **Migration**: Provide a script or automated path to migrate `issued` to `created` for unpublished document types in existing datasets.

## References
- [DACS Chapter 2.4: Date](https://saa-ts-dacs.github.io/dacs/06_part_I/03_chapter_02/04_date.html)
- [CSL Date Specification](https://docs.citationstyles.org/en/stable/specification.html#dates)

## Related Specs
- [`LOCALE_DATE_NAME_KEYING.md`](./LOCALE_DATE_NAME_KEYING.md): how locale
  month/season names are keyed and overridden, and the `day-zero-pad`
  rendering option — orthogonal to this document's EDTF *value* model, which
  covers date *fields* rather than their rendered names.
