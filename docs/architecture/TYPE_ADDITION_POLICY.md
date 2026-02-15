# Type Addition Policy

**Status:** Active Policy
**Version:** 1.0
**Date:** 2026-02-14
**Related:** TYPE_SYSTEM_ARCHITECTURE.md

## Purpose

This policy provides clear criteria for deciding when to add a new top-level reference type to CSLN versus using existing structural types (SerialComponent, CollectionComponent, Monograph, Collection).

## Architecture Model: Hybrid

CSLN uses a **hybrid type system**:

- **Structural types** for academic references with meaningful parent-child relationships (journal articles, book chapters)
- **Flat types** for references where the container is a locator rather than a semantic parent (legal cases, treaties, datasets)

This balances:
- **Data efficiency** (parent metadata reused across components)
- **Style clarity** (explicit type-specific overrides)
- **User mental models** (semantic type names)

## Prior Art

**biblatex** (flat model):
- 31 entry types as distinct database types, not subtypes
- Relationships via fields (`crossref`, `xref`) not type hierarchy
- Types chosen based on semantic distinction and field schema differences
- Example: `@mvbook` (multi-volume) separate from `@book` because field requirements differ

**CSL 1.0** (flat model):
- 34 types defined as flat enumeration (article-journal, book, legal_case, treaty)
- Container relationships via variables (`container-title`) not parent types
- Types chosen for citation style discrimination

**CSLN** (hybrid model):
- Structural types where parent-child relationship provides efficiency (SerialComponent → Serial)
- Flat types where semantic distinction and style clarity outweigh data model elegance

## Decision Criteria: The 4-Factor Test

Add a new top-level type when **ALL** of the following are true:

### 1. Semantic Distinction

**Test:** Do users think of this as a fundamentally different thing?

**Threshold:** The reference has a distinct identity in scholarly discourse (legal case ≠ journal article, dataset ≠ report).

**Examples:**
- ✅ LegalCase: Legal scholars think "court decision" not "serial component"
- ✅ Treaty: International law context, distinct from academic articles
- ❌ BlogPost: Variant of article, users think "online article"
- ❌ ReviewBook: Variant of review, genre field suffices

### 2. Style Discrimination

**Test:** Do major citation styles format this type differently?

**Threshold:** At least 20% of major styles (APA, Chicago, MLA, IEEE, Harvard, Nature, AMA, ACS, AIP, Vancouver) require distinct formatting.

**Evaluation:**
- Check style manuals for dedicated sections
- Look for different field ordering, punctuation, emphasis
- Consider legal/domain-specific style guides (Bluebook, AMA)

**Examples:**
- ✅ LegalCase: Legal styles (Bluebook, ALWD) have dedicated case citation rules
- ✅ Statute: Legislative citation formats differ from article citations
- ❌ MagazineArticle: Minor differences from journal articles (often just volume/issue suppression)
- ❓ Dataset: Emerging (DataCite styles exist, APA 7th has data citation guidelines)

### 3. Field Schema Difference

**Test:** Do required/expected fields differ significantly from existing types?

**Threshold:** At least 3 fields that are:
- Required for this type but not others, OR
- Expected in this type but rare/nonsensical in others

**Examples:**
- ✅ LegalCase: `authority` (court), `reporter`, `docket-number` (unique to legal)
- ✅ Dataset: `size`, `format`, `version`, `repository` (unique to datasets)
- ❌ Preprint: Same fields as article, just different publication stage
- ❌ ReviewBook: Same fields as article/review, genre distinguishes

### 4. No Meaningful Parent

**Test:** Is the "container" a locator rather than a semantic parent?

**Threshold:** If the container requires independent citation OR multiple containers are valid (parallel citations), it's a locator not a parent.

**Examples:**
- ✅ LegalCase: Reporter is where to find it, not what it is (parallel citations in multiple reporters)
- ✅ Treaty: Treaty series is publication venue, not semantic container
- ❌ JournalArticle: Journal IS the semantic container (article is part of journal)
- ❌ Chapter: Book IS the semantic container (chapter is part of book)
- ❓ Preprint: arXiv is locator (published version is in journal), but single-container

## Decision Flowchart

```
┌─────────────────────────────────────────────────┐
│ Does this reference require different          │
│ citation formatting across major styles?        │
│ (Factor 2: Style Discrimination)                │
└─────────┬───────────────────────────────────────┘
          │
    ┌─────┴─────┐
    │    NO     │──► Use existing type + optional field
    └───────────┘    (e.g., genre, medium)
          │
          │ YES
          ▼
┌─────────────────────────────────────────────────┐
│ Do users think of this as fundamentally         │
│ different from existing types?                  │
│ (Factor 1: Semantic Distinction)                │
└─────────┬───────────────────────────────────────┘
          │
    ┌─────┴─────┐
    │    NO     │──► Use subtype or genre field
    └───────────┘
          │
          │ YES
          ▼
┌─────────────────────────────────────────────────┐
│ Does it have unique required/expected fields?   │
│ (Factor 3: Field Schema Difference)             │
└─────────┬───────────────────────────────────────┘
          │
    ┌─────┴─────┐
    │    NO     │──► Use existing type + optional fields
    └───────────┘
          │
          │ YES
          ▼
┌─────────────────────────────────────────────────┐
│ Is the "container" a locator rather than        │
│ a semantic parent?                              │
│ (Factor 4: No Meaningful Parent)                │
└─────────┬───────────────────────────────────────┘
          │
    ┌─────┴─────┐
    │    NO     │──► Use structural type with parent
    └───────────┘    (SerialComponent, CollectionComponent)
          │
          │ YES
          ▼
┌─────────────────────────────────────────────────┐
│ ADD NEW TOP-LEVEL TYPE                          │
│                                                  │
│ Example: LegalCase, Treaty, Dataset             │
└──────────────────────────────────────────────────┘
```

## Examples: Applying the 4-Factor Test

### LegalCase (Court Decision)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ✅ | Legal scholars think "case" not "article" |
| 2. Style | ✅ | Bluebook, ALWD have dedicated case citation rules |
| 3. Schema | ✅ | `authority` (court), `reporter`, `docket-number` unique |
| 4. No Parent | ✅ | Reporter is locator (parallel citations common) |

**Decision:** Add flat type `LegalCase`

### Treaty (International Agreement)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ✅ | Distinct identity in international law |
| 2. Style | ✅ | Legal and international relations styles differ |
| 3. Schema | ✅ | Treaty-specific fields (parties, ratification date) |
| 4. No Parent | ✅ | Treaty series is publication venue, not semantic parent |

**Decision:** Add flat type `Treaty`

### BlogPost (Blog Article)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ❌ | Users think "online article" |
| 2. Style | ❌ | Cited same as magazine/newspaper articles |
| 3. Schema | ❌ | Same fields as article (title, author, date, URL) |
| 4. No Parent | ❌ | Blog IS the parent (like magazine/journal) |

**Decision:** Use `SerialComponent` with parent `Serial(type: Blog)`

### Dataset (Research Data)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ✅ | Distinct scholarly artifact (not a publication) |
| 2. Style | ✅? | DataCite, APA 7th have data citation guidelines |
| 3. Schema | ✅ | `size`, `format`, `version`, `repository` unique |
| 4. No Parent | ✅ | Repository is locator (Zenodo, figshare, Dryad) |

**Decision:** Add flat type `Dataset` (emerging, validate against APA/Chicago/Nature)

### MagazineArticle (Popular Press)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ❌ | Variant of article |
| 2. Style | ⚠️ | Minor differences (volume/issue suppression) |
| 3. Schema | ❌ | Same fields as journal article |
| 4. No Parent | ❌ | Magazine IS the parent |

**Decision:** Use `SerialComponent` with parent `Serial(type: Magazine)`

**Rationale for structural type:** Style discrimination (Factor 2) is minor and achievable via parent-type overrides. Data efficiency (journal metadata reused) outweighs template complexity.

### Preprint (Pre-publication Article)

| Factor | Score | Rationale |
|--------|-------|-----------|
| 1. Semantic | ⚠️ | Variant of article (publication stage, not type) |
| 2. Style | ⚠️ | Minor differences (archive identifier instead of DOI) |
| 3. Schema | ❌ | Same fields as article + archive identifier |
| 4. No Parent | ⚠️ | arXiv is locator, but published version is in journal |

**Decision:** **Ambiguous** - Use `SerialComponent` with `archive` field for now. Monitor style evolution.

**Rationale:** Preprints are temporally-qualified articles (pre-peer-review). Most citation styles treat them as articles with archive metadata. However, if dedicated preprint citation formats emerge, re-evaluate.

## Migration Compatibility Factor (Optional 5th Factor)

**Test:** If CSL 1.0 has a distinct type for this, does style discrimination justify a CSLN flat type or is a structural subtype sufficient?

**Purpose:** Guide CSL 1.0 → CSLN migration decisions.

**Examples:**

**CSL 1.0 Types → CSLN Decision:**

| CSL 1.0 Type | CSLN Type | Rationale |
|--------------|-----------|-----------|
| `article-journal` | SerialComponent(parent: AcademicJournal) | Parent reuse efficiency |
| `article-magazine` | SerialComponent(parent: Magazine) | Same structure, parent differs |
| `article-newspaper` | SerialComponent(parent: Newspaper) | Same structure, parent differs |
| `legal_case` | LegalCase | Passes 4-factor test |
| `treaty` | Treaty | Passes 4-factor test |
| `book` | Monograph(type: Book) | Monolithic work |
| `chapter` | CollectionComponent | Parent-child relationship |

**Trade-off:** Accept template complexity (parent-type discrimination) for data model efficiency when factors 3-4 fail.

## Policy Enforcement

**For new type proposals:**

1. **Create GitHub issue** using "New Reference Type" template
2. **Complete 4-factor test** with evidence for each factor
3. **Provide examples** from at least 3 major citation styles
4. **List unique fields** required/expected for this type
5. **Discuss parent relationship** - locator vs semantic container?

**Review criteria:**

- All 4 factors must be ✅ for flat type
- If any factor is ❌ or ⚠️, justify why exception is warranted
- Consider CSL 1.0 compatibility (optional 5th factor)
- Prefer structural types when factors 3-4 fail (efficiency > template simplicity)

## Rationale for Hybrid Model

**Why not pure flat (Option C)?**

1. **Data bloat:** Repeating journal metadata (title, ISSN, publisher) across 50 articles from the same journal wastes space
2. **Update complexity:** If journal name changes, must update 50 records vs 1 parent record
3. **Query inefficiency:** "All articles from APSR" requires full scan vs parent ID lookup

**Why not pure structural (Option B)?**

1. **Violates "explicit over magic":** Parent type discrimination (`serial-component.parent-type.treaty-series`) is procedural logic in declarative templates
2. **User confusion:** Legal experts think "legal case" not "serial component of reporter series"
3. **Parallel citations:** Same treaty in multiple reporters breaks parent-child model

**Hybrid model achieves:**

- ✅ Data efficiency where parent-child is meaningful (academic references)
- ✅ Style clarity where semantic distinction matters (legal, datasets)
- ✅ Alignment with user mental models (legal case, treaty, dataset)
- ✅ CSL 1.0 compatibility (maps flat types 1:1, structural types many:1)

## Audit of Current Types

**Structural types (parent-child relationship):**

| CSLN Type | Rationale |
|-----------|-----------|
| SerialComponent(Article) | ✅ Journal is semantic parent, metadata reused |
| SerialComponent(Post) | ✅ Blog/Magazine is parent |
| SerialComponent(Review) | ✅ Journal is parent |
| CollectionComponent(Chapter) | ✅ Book is semantic parent |
| CollectionComponent(Document) | ✅ Conference proceedings is parent |

**Flat types (no parent or locator parent):**

| CSLN Type | Rationale |
|-----------|-----------|
| LegalCase | ✅ Passes 4-factor test |
| Statute | ✅ Passes 4-factor test |
| Treaty | ✅ Passes 4-factor test |
| Hearing | ✅ Legislative context, unique fields |
| Regulation | ✅ Regulatory context, unique fields |
| Brief | ✅ Legal filing context |
| Classic | ✅ Standard citation forms (Aristotle, Bible) |
| Monograph(Book) | ✅ Monolithic work, no parent |
| Monograph(Report) | ✅ Monolithic work |
| Monograph(Thesis) | ✅ Academic work, institution is not parent |
| Monograph(Webpage) | ✅ Web content, site is not semantic parent |

**Policy compliance:** All current types conform to the 4-factor test or structural efficiency rationale.

## Future Type Candidates

Monitor these emerging reference types for potential addition:

| Type | Status | Factors | Action |
|------|--------|---------|--------|
| Dataset | High priority | ✅✅✅✅ | Validate against APA/Nature/DataCite |
| Software | Medium | ✅⚠️✅✅ | Style guidelines emerging (FORCE11) |
| Preprint | Low | ⚠️⚠️❌⚠️ | Monitor, use SerialComponent for now |
| Standard | Medium | ✅✅✅✅ | ISO, ANSI standards (unique fields) |
| Performance | Low | ✅⚠️✅✅ | Music/theater domain, niche |
| Artwork | Low | ✅⚠️✅✅ | Art history domain, niche |

## References

- TYPE_SYSTEM_ARCHITECTURE.md - Full analysis of structural vs flat options
- biblatex manual (CTAN) - 31 entry types, flat model
- CSL 1.0 specification - 34 types, flat enumeration
- CLAUDE.md - CSLN design principles

## Changelog

**v1.0 (2026-02-14):**
- Initial policy based on legal citations architectural analysis
- 4-factor test established
- Decision flowchart added
- Example evaluations provided
