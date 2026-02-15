# Current Type Audit Against Type Addition Policy

**Date:** 2026-02-14
**Policy:** TYPE_ADDITION_POLICY.md (4-factor test)

## Evaluation Criteria

For each current type, evaluate:
1. **Semantic Distinction** - Do users think of it differently?
2. **Style Discrimination** - Do 20%+ of major styles format differently?
3. **Field Schema Difference** - Are there 3+ unique required/expected fields?
4. **No Meaningful Parent** - Is container a locator vs semantic parent?

**Pass all 4 → Flat type justified**
**Fail 3-4 → Structural type justified (efficiency > template complexity)**

---

## Structural Types (Academic References)

### SerialComponent (Article, Post, Review)

**Current Implementation:** Structural type with parent `Serial`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ⚠️ | Minor distinction (article vs review vs blog post) |
| 2. Style | ❌ | Formatted identically in most styles |
| 3. Schema | ❌ | Identical fields (title, author, pages, volume, issue) |
| 4. No Parent | ❌ | Journal/Magazine/Blog IS semantic parent (cited independently) |

**4-Factor Result:** ❌❌❌❌ (Fail all)

**Policy Compliance:** ✅ **CORRECT** - Structural type justified
- Parent-child relationship is meaningful (journal metadata reused)
- Data efficiency: one journal record → many articles reference by ID
- Style discrimination achieved via `parent-type` overrides (acceptable complexity)

**biblatex Mapping:**
- `@article` → SerialComponent(parent: AcademicJournal)
- `@review` → SerialComponent(parent: AcademicJournal) + genre field
- `@online` (blog) → SerialComponent(parent: Blog)

---

### CollectionComponent (Chapter, Document)

**Current Implementation:** Structural type with parent `Collection`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ⚠️ | Minor distinction (chapter vs conference paper) |
| 2. Style | ⚠️ | Some differences (conference papers show proceedings differently) |
| 3. Schema | ❌ | Identical fields (title, author, pages, parent) |
| 4. No Parent | ❌ | Book/Proceedings IS semantic parent (cited independently) |

**4-Factor Result:** ⚠️⚠️❌❌ (Fail 3-4)

**Policy Compliance:** ✅ **CORRECT** - Structural type justified
- Parent-child relationship is meaningful (book/proceedings metadata reused)
- Data efficiency outweighs minor style differences

**biblatex Mapping:**
- `@incollection` → CollectionComponent(parent: Collection)
- `@inproceedings` → CollectionComponent(parent: Collection(Proceedings))
- `@inbook` → CollectionComponent(parent: Monograph)

**Note:** biblatex distinguishes `@inbook` (part of single-author book) from `@incollection` (part of edited volume). CSLN uses parent type to distinguish.

---

### Monograph (Book, Report, Thesis, Webpage, Post, Document)

**Current Implementation:** Flat type with subtypes

| Subtype | Semantic | Style | Schema | No Parent | Justification |
|---------|----------|-------|--------|-----------|---------------|
| Book | ✅ | ⚠️ | ❌ | ✅ | Monolithic work |
| Report | ✅ | ⚠️ | ⚠️ (report number) | ✅ | Monolithic work |
| Thesis | ✅ | ✅ (degree, institution) | ⚠️ (degree field) | ✅ | Monolithic work |
| Webpage | ✅ | ⚠️ | ❌ | ✅ | Monolithic work |
| Post | ⚠️ | ❌ | ❌ | ✅ | Variant of webpage |
| Document | ⚠️ | ❌ | ❌ | ✅ | Generic fallback |

**4-Factor Result:** Mixed (subtypes share structure)

**Policy Compliance:** ✅ **CORRECT** - Flat type with subtypes justified
- All are monolithic works (no parent-child)
- Schema differences are minor (optional fields like `degree`, `number`)
- Subtype discrimination sufficient for style differences

**Recommendation:** ✅ Keep as-is

**biblatex Mapping:**
- `@book` → Monograph(Book)
- `@report` → Monograph(Report)
- `@thesis`, `@mastersthesis`, `@phdthesis` → Monograph(Thesis)
- `@online` → Monograph(Webpage)
- `@unpublished` → Monograph(Document)
- `@booklet` → Monograph(Book) with optional publisher
- `@manual` → Monograph(Report) with `genre: "manual"`

---

## Flat Types (Legal References)

### LegalCase

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Legal scholars think "case" not "article" |
| 2. Style | ✅ | Bluebook, ALWD, legal styles have dedicated case rules |
| 3. Schema | ✅ | `authority` (court), `reporter`, `docket-number` unique |
| 4. No Parent | ✅ | Reporter is locator (parallel citations common) |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Policy Compliance:** ✅ **CORRECT** - Flat type justified

**biblatex Mapping:** `@jurisdiction` → LegalCase

---

### Statute

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Distinct legislative context |
| 2. Style | ✅ | Legal styles format statutes differently |
| 3. Schema | ✅ | `code`, `section`, legislative fields unique |
| 4. No Parent | ✅ | Code is locator (U.S.C., Stat.) |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Policy Compliance:** ✅ **CORRECT** - Flat type justified

**biblatex Mapping:** `@legislation` → Statute

---

### Treaty

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | International law context |
| 2. Style | ✅ | International relations styles differ |
| 3. Schema | ✅ | Treaty-specific fields (parties, ratification) |
| 4. No Parent | ✅ | Treaty series is locator (U.N.T.S., Parry's) |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Policy Compliance:** ✅ **CORRECT** - Flat type justified

**biblatex Mapping:** No direct equivalent (CSL-M extension)

---

### Hearing, Regulation, Brief, Classic

All pass 4-factor test. See TYPE_ADDITION_POLICY.md for detailed evaluation.

**Policy Compliance:** ✅ **CORRECT** - All flat types justified

---

## Missing Types (biblatex Compatibility)

### Patent

**biblatex:** `@patent`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Patents are distinct from publications |
| 2. Style | ✅ | APA, IEEE, Nature have patent citation rules |
| 3. Schema | ✅ | `patent-number`, `application-number`, `filing-date`, `jurisdiction`, `assignee` |
| 4. No Parent | ✅ | Patent office is issuing authority, not parent |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Recommendation:** ✅ **ADD** Patent type

**Example Citation (APA):**
- Pavlovic, N. (2008). *Bicycle with adjustable suspension* (U.S. Patent No. 7,347,809). U.S. Patent and Trademark Office.

---

### Standard

**biblatex:** `@standard`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Technical standards (ISO, ANSI, IEEE) |
| 2. Style | ✅ | IEEE, engineering styles format standards distinctly |
| 3. Schema | ✅ | `standard-number`, `organization`, `status` unique |
| 4. No Parent | ✅ | Standards body is issuing authority |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Recommendation:** ✅ **ADD** Standard type

**Example Citation (IEEE):**
- IEEE Standard for Floating-Point Arithmetic, IEEE Standard 754-2008, Aug. 2008.

---

### Software

**biblatex:** `@software`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Software distinct from publications |
| 2. Style | ⚠️ | Emerging (FORCE11, APA 7th, DataCite have guidelines) |
| 3. Schema | ✅ | `version`, `repository`, `license`, `platform` unique |
| 4. No Parent | ✅ | Repository (GitHub, Zenodo) is locator |

**4-Factor Result:** ✅⚠️✅✅ (Pass 3/4, Factor 2 emerging)

**Recommendation:** ⚠️ **CONSIDER** - Monitor style evolution, strong case for addition

**Example Citation (APA 7th):**
- R Core Team. (2021). *R: A language and environment for statistical computing* (Version 4.1.0) [Computer software]. R Foundation for Statistical Computing. https://www.R-project.org/

---

### Dataset

**biblatex:** `@dataset`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Research data distinct from publications |
| 2. Style | ✅ | DataCite, APA 7th, Nature have data citation guidelines |
| 3. Schema | ✅ | `size`, `format`, `version`, `repository` unique |
| 4. No Parent | ✅ | Repository (Zenodo, Dryad, figshare) is locator |

**4-Factor Result:** ✅✅✅✅ (Pass all)

**Recommendation:** ✅ **ADD** Dataset type (high priority)

**Example Citation (DataCite):**
- Irino, T., & Tada, R. (2009). *Chemical and mineral compositions of sediments from ODP Site 127-797* [Dataset]. Geological Institute, University of Tokyo. https://doi.org/10.1594/PANGAEA.726855

---

### Performance

**biblatex:** `@performance`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Music, theater performances |
| 2. Style | ⚠️ | Niche (music/theater disciplines) |
| 3. Schema | ✅ | `venue`, `performers`, `conductor` unique |
| 4. No Parent | ✅ | Venue is location, not parent |

**4-Factor Result:** ✅⚠️✅✅ (Pass 3/4, niche domain)

**Recommendation:** ⏸️ **DEFER** - Niche domain, <5% of styles support

---

### Artwork

**biblatex:** `@artwork`

| Factor | Score | Evidence |
|--------|-------|----------|
| 1. Semantic | ✅ | Visual art (paintings, sculptures) |
| 2. Style | ⚠️ | Niche (art history disciplines) |
| 3. Schema | ✅ | `medium`, `dimensions`, `location` unique |
| 4. No Parent | ✅ | Museum/gallery is location, not parent |

**4-Factor Result:** ✅⚠️✅✅ (Pass 3/4, niche domain)

**Recommendation:** ⏸️ **DEFER** - Niche domain, <5% of styles support

---

## Summary

### Current Types: Fully Compliant ✅

All current CSLN types conform to the 4-factor policy:
- **Structural types** (SerialComponent, CollectionComponent) justified by parent-child efficiency
- **Flat types** (legal references) justified by passing all 4 factors
- **Monograph subtypes** justified by shared schema with minor differences

**No revisions needed to current types.**

### Recommended Additions (High Priority)

| Type | Priority | Factors | biblatex | Use Cases |
|------|----------|---------|----------|-----------|
| **Patent** | High | ✅✅✅✅ | @patent | Engineering, CS, chemistry citations |
| **Dataset** | High | ✅✅✅✅ | @dataset | Data-driven research (all disciplines) |
| **Standard** | Medium | ✅✅✅✅ | @standard | Engineering, CS, technical fields |
| **Software** | Medium | ✅⚠️✅✅ | @software | CS, digital humanities (emerging styles) |

### Deferred (Niche Domains)

| Type | Factors | Reason |
|------|---------|--------|
| Performance | ✅⚠️✅✅ | Music/theater niche (<5% styles) |
| Artwork | ✅⚠️✅✅ | Art history niche (<5% styles) |

### No Addition Required

| biblatex Type | CSLN Mapping | Reason |
|---------------|--------------|--------|
| @online | Monograph(Webpage) | Same schema |
| @manual | Monograph(Report) + genre | Same schema, use genre field |
| @booklet | Monograph(Book) | Same schema, optional publisher |
| @mvbook | Monograph(Book) + volume | Volume field handles multi-volume |
| @unpublished | Monograph(Document) | Generic document type |

---

## Implementation Plan

1. ✅ Audit complete - all current types validated
2. ⏳ Add Patent type (high priority, clear use case)
3. ⏳ Add Dataset type (high priority, emerging standard)
4. ⏳ Add Standard type (medium priority, engineering/CS)
5. ⏳ Add Software type (medium priority, monitor style evolution)
6. ⏳ Update styles/apa-7th.yaml with overrides for new types
7. ⏳ Create test fixtures for new types
8. ⏳ Document biblatex mapping in migration guide

**Estimated effort:** 1 week for high-priority types (Patent, Dataset)
