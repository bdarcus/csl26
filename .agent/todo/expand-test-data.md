# TODO: Expand Test Data Coverage

## Current Limitation

The oracle comparison tests currently use only 5 reference items:

1. **ITEM-1**: Journal article (Kuhn) - with publisher-place and DOI
2. **ITEM-2**: Book (Hawking) - with publisher-place
3. **ITEM-3**: Journal article (LeCun) - multi-author, with DOI
4. **ITEM-4**: Chapter (Ericsson) - with editors and pages
5. **ITEM-5**: Report (World Bank) - institutional author, with place

## Coverage Gaps

### Missing Reference Types (CSL 1.0 has 34 types)

**High Priority** (common in academic citations):
- `article-magazine` - magazine articles
- `article-newspaper` - newspaper articles
- `paper-conference` - conference papers/proceedings
- `thesis` - dissertations and theses
- `webpage` - web documents
- `post` / `post-weblog` - blog posts
- `software` - software citations (increasingly important)
- `dataset` - data citations (increasingly important)

**Medium Priority** (legal/specialized):
- `legal_case` - court cases
- `legislation` - laws and statutes
- `bill` - legislative bills
- `treaty` - international treaties
- `patent` - patents

**Lower Priority** (less common but should work):
- `broadcast` - TV/radio programs
- `motion_picture` - films
- `musical_score` - sheet music
- `graphic` - artwork, charts
- `interview` - interviews
- `manuscript` - unpublished manuscripts
- `map` - cartographic materials
- `pamphlet` - pamphlets
- `personal_communication` - letters, emails
- `review` / `review-book` - book reviews
- `song` - audio recordings
- `speech` - speeches

### Missing Field Variations

**Date formats:**
- Ranges: `"issued": {"date-parts": [[2020, 1], [2020, 12]]}`
- Circa dates: `"circa": true`
- Seasons: `"season": "Spring"`
- Open-ended ranges: `"date-parts": [[2020], []]`
- EDTF dates (when implemented)

**Name variations:**
- Single author
- Two authors (tests "and" conjunction)
- Three authors (edge case for et-al thresholds)
- 10+ authors (tests et-al truncation)
- Institutional authors: `{"literal": "..."}`
- Mixed individual/institutional: `[{"family": "Smith"}, {"literal": "WHO"}]`
- Non-dropping particles: `{"family": "Gogh", "non-dropping-particle": "van"}`
- Dropping particles: `{"family": "Winter", "dropping-particle": "de"}`
- Suffixes: `{"family": "King", "suffix": "Jr."}`
- No given name: `{"family": "Plato"}`

**Title variations:**
- Titles with subtitles containing colons
- Titles with math/special characters
- Titles in multiple languages
- Titles with quotation marks
- Very long titles (>200 chars)

**Locator variations:**
- Different types: page, chapter, verse, volume, line, section, paragraph
- Ranges: `"page": "100-150"`
- Multiple: `"page": "10, 25, 30-35"`
- Roman numerals: `"page": "xii-xv"`

**Container variations:**
- Journal with volume and issue
- Journal with volume only
- Journal with issue only (rare but exists)
- Series information
- Edition numbers
- Multi-volume works

**Access information:**
- URL only
- DOI only
- Both URL and DOI (should prefer DOI)
- Accessed dates
- Archive information

### Edge Cases

**Ambiguous citations** (for disambiguation testing):
- Multiple works by same author in same year (suffix: a, b, c)
- Authors with same family name (need given names)
- Same author, different years
- Institutional authors with similar names

**Missing data:**
- No author (should use title or editor)
- No date (should use "n.d." or similar)
- No publisher
- No place
- Minimal data (just title)

**Multilingual:**
- Non-Latin scripts (when implementing csln#66)
- RTL languages
- Transliteration variants

## Implementation Strategy

### Phase 1: Expand to 20 Core Items

Add 15 more items covering common types:
- 2 more journal articles (different configurations)
- 1 magazine article
- 1 newspaper article
- 1 thesis
- 1 webpage
- 1 conference paper
- 1 dataset
- 1 software
- 2 edge cases (no author, no date)
- 4 name variation tests

### Phase 2: Type-Specific Test Suites

Create separate test files for each major category:
- `tests/fixtures/academic.json` - journal, book, chapter, thesis
- `tests/fixtures/legal.json` - legal_case, legislation, bill
- `tests/fixtures/media.json` - webpage, post, broadcast, film
- `tests/fixtures/data.json` - dataset, software
- `tests/fixtures/edge-cases.json` - missing data, disambiguation

### Phase 3: Style-Specific Tests

Different styles handle types differently:
- APA has specific rules for datasets, software
- Chicago has detailed legal citation formats
- Note styles need repeated citation tests (ibid, subsequent)

### Phase 4: CSL Test Suite Integration

The official CSL project has a comprehensive test suite:
- Repository: https://github.com/citation-style-language/test-suite
- 1000+ tests covering edge cases
- Could import and run subset relevant to CSLN

## Data Sources

**Realistic citations from:**
- Zotero sample library
- CSL test suite
- Real academic papers (permissions permitting)
- CrossRef/DataCite sample data

## Test Infrastructure

**Enhancements needed:**
- Parameterized tests (run each style against all items)
- Test result matrix (style × item type)
- Regression detection (alert on new failures)
- Performance benchmarking (time per 100 citations)
- Coverage reporting (which types/fields tested)

## Priority

**Medium-High** - Current 5-item test set is sufficient for basic development but insufficient for:
- Comprehensive style testing
- Edge case discovery
- Regression prevention
- Production readiness

Should be addressed after tier 1 styles are stable and before declaring v1.0.

## Related Issues

- csln#64 (Math in variables - need test data with equations in titles)
- csln#66 (Multilingual - need non-Latin script test data)
- Future: Note styles need position-based test data (ibid, subsequent)

## Notes

When expanding test data, maintain the oracle comparison approach:
1. Add items to both CSL and CSLN test fixtures
2. Verify citeproc-js renders correctly
3. Compare CSLN output against citeproc-js
4. Document any intentional divergences
