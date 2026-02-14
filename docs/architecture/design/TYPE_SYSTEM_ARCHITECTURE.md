# Type System Architecture: Structural vs Flat Types

**Status:** Open Question
**Bean:** csl26-[to be created]
**Triggered By:** Legal citations PR (csl26-rmoi, PR #164)
**Date:** 2026-02-14

## Problem Statement

The legal citations implementation (PR #164) uses **flat types** (LegalCase, Statute, Treaty as separate `InputReference` variants) rather than fitting them into the existing **structural type system** (SerialComponent, Monograph, Collection).

This raises a fundamental architectural question: **Should CSLN use structural types, flat types, or a hybrid approach?**

## Current State

### Structural Types (Current CSLN Model)

CSLN organizes references into structural categories based on publication model:

**Monograph** (monolithic works):
- Book, Report, Thesis, Webpage, Post, Document

**Collection** (edited collections):
- Anthology, Proceedings, EditedBook, EditedVolume

**CollectionComponent** (parts of collections):
- Chapter, Document (conference paper)

**SerialComponent** (parts of serials):
- Article, Post, Review
- Parent: Serial (AcademicJournal, Blog, Magazine, Newspaper, etc.)

**Parent-Child Relationships:**
```yaml
# Article references parent serial
reference:
  type: serial-component
  title: "Article Title"
  author: Smith
  parent:
    type: academic-journal
    title: "American Political Science Review"
    issn: "0003-0554"
  volume: 25
  issue: 3
  pages: "188-195"
```

### Flat Types (Legal Citations Model)

Legal citations PR adds 7 new top-level types:

**Legal Types:**
- LegalCase, Statute, Treaty, Hearing, Regulation, Brief, Classic

**No Parent-Child Relationships:**
```yaml
# Treaty is monolithic
reference:
  type: treaty
  title: "Treaty of Versailles"
  volume: 225
  reporter: "U.N.T.S."  # Just a locator, not a parent
  page: 188
  issued: "1919-06-28"
```

## The Tension

If legal materials (which ARE published in serial reporters like U.N.T.S., C.F.R.) get **flat types** instead of using **SerialComponent**, why don't we do the same for:

- Magazine article vs Journal article vs Newspaper article (currently all SerialComponent with parent type discrimination)
- Book vs Report vs Thesis (currently all Monograph with subtype discrimination)
- Chapter vs Conference paper (currently both CollectionComponent)

**The inconsistency:** Legal materials use flat types for style clarity, but academic materials use structural types for code efficiency.

## Three Architectural Options

### Option A: Hybrid Approach (Current State + Legal Types)

**Description:** Keep structural types for academic materials, use flat types for legal materials.

**Rationale:**
- Structural types work for references with meaningful parent-child relationships (article ↔ journal)
- Flat types work for monolithic references where publication venue is just a locator (treaty ↔ reporter)

**Type Count:**
- Current structural types: 4 (Monograph, Collection, CollectionComponent, SerialComponent)
- Legal flat types: 7 (LegalCase, Statute, Treaty, Hearing, Regulation, Brief, Classic)
- **Total: 11 top-level `InputReference` variants**

**Style Template Complexity:**
- Academic references: Moderate (parent type discrimination)
- Legal references: Low (explicit type overrides)

**Example Override:**
```yaml
overrides:
  # Structural (discriminate by parent type)
  serial-component:
    parent-type:
      academic-journal: [...]
      magazine: [...]

  # Flat (explicit type override)
  legal-case: [...]
  treaty: [...]
```

**Pros:**
- ✅ Pragmatic: uses the right model for each domain
- ✅ Preserves parent-child relationships where meaningful
- ✅ Avoids duplicate journal metadata (efficiency)
- ✅ Legal citations get explicit, simple overrides

**Cons:**
- ❌ Inconsistent architecture (structural vs flat)
- ❌ Unclear decision criteria for future types
- ❌ Mixed mental model for developers and style authors

### Option B: Full Structural Consolidation

**Description:** Force all references (including legal) into structural categories.

**Implementation:**
- Legal materials become **SerialComponent** with legal parent serial types
- Treaty → SerialComponent with parent TreatySeries (U.N.T.S.)
- Regulation → SerialComponent with parent RegulatoryCode (C.F.R.)
- Legal case → SerialComponent with parent LegalReporter (U.S. Reports)

**Type Count:**
- Structural types: 4 (unchanged)
- Serial subtypes: Expand from ~8 to ~15 (add TreatySeries, RegulatoryCode, LegalReporter, etc.)
- **Total: 4 top-level `InputReference` variants**

**Style Template Complexity:**
- High: all references require parent type discrimination

**Example Override:**
```yaml
overrides:
  serial-component:
    parent-type:
      academic-journal:
        template: [...]
      treaty-series:
        template: [...]
      regulatory-code:
        template: [...]
```

**Pros:**
- ✅ Consistent data model (all publications are structural)
- ✅ Fewer top-level enum variants
- ✅ Reuses SerialComponent infrastructure

**Cons:**
- ❌ Violates "explicit over magic" principle (nested discrimination logic)
- ❌ Complex style templates (style authors must understand parent types)
- ❌ Misleading model (treaty ≠ component of treaty series in the same way article = component of journal)
- ❌ Parallel citations problem (same treaty in different reporters)

### Option C: Full Flat Type Expansion

**Description:** Break up all structural types into explicit semantic types.

**Implementation:**
- SerialComponent → JournalArticle, MagazineArticle, NewspaperArticle, BlogPost, PodcastEpisode, etc.
- Monograph → Book, Report, Thesis, Webpage, Post, Document
- CollectionComponent → Chapter, ConferencePaper
- Legal → LegalCase, Statute, Treaty, Hearing, Regulation, Brief, Classic

**Type Count:**
- Academic types: ~15-20 (expand from current 4 structural)
- Legal types: 7
- **Total: ~25-30 top-level `InputReference` variants**

**Style Template Complexity:**
- Low: all overrides are explicit, flat

**Example Override:**
```yaml
overrides:
  journal-article:
    template: [...]
  magazine-article:
    template: [...]
  legal-case:
    template: [...]
  treaty:
    template: [...]
```

**Pros:**
- ✅ Fully aligned with "explicit over magic" principle
- ✅ Simple, flat style overrides (no discrimination logic)
- ✅ Clear semantic types (journal-article ≠ magazine-article)
- ✅ Matches user mental model (people think "journal article" not "serial component")

**Cons:**
- ❌ Many enum variants (25-30+)
- ❌ Code duplication across similar types (JournalArticle vs MagazineArticle have nearly identical fields)
- ❌ Loss of parent-child relationship efficiency (duplicate journal metadata)
- ❌ Larger enum size (minor performance impact)

## Impact Analysis

### Code Maintenance

| Option | Enum Variants | Struct Definitions | Accessor Methods | Complexity |
|--------|---------------|--------------------|--------------------|------------|
| A: Hybrid | 11 | ~8 | ~15 | Medium |
| B: Structural | 4 | ~4 | ~15 | Low (code), High (logic) |
| C: Flat | 25-30 | ~25 | ~30 | High (volume), Low (simplicity) |

### Style Authoring Complexity

| Option | Override Depth | Discrimination Logic | Mental Model |
|--------|----------------|----------------------|--------------|
| A: Hybrid | Mixed (1-2 levels) | Some parent type discrimination | Mixed |
| B: Structural | 2+ levels | Heavy parent type discrimination | Abstract |
| C: Flat | 1 level | None | Concrete |

### Data Entry (User Experience)

| Option | Type Selection | Field Complexity | Error Clarity |
|--------|----------------|------------------|---------------|
| A: Hybrid | Moderate (11 types) | Low-Medium | Good |
| B: Structural | Low (4 types) | Medium (parent nesting) | Poor (nested errors) |
| C: Flat | High (25-30 types) | Low (flat fields) | Excellent (clear type) |

### Migration from CSL 1.0

CSL 1.0 has ~30 reference types (article-journal, book, legal_case, treaty, etc.).

| Option | Mapping Complexity | Semantic Accuracy |
|--------|--------------------|--------------------|
| A: Hybrid | Medium | Good |
| B: Structural | High (many-to-one, lossy) | Poor |
| C: Flat | Low (nearly 1:1) | Excellent |

### Performance Impact

| Option | Enum Size | Match Arm Count | Runtime Impact |
|--------|-----------|-----------------|----------------|
| A: Hybrid | 11 variants | ~11 per match | Negligible |
| B: Structural | 4 variants | ~4 per match, nested | Negligible |
| C: Flat | 25-30 variants | ~30 per match | Negligible (compiled) |

**Note:** Rust compiles match statements efficiently; 30 variants vs 4 variants has no measurable runtime cost.

### JSON Schema Size

| Option | Top-Level Variants | Schema Complexity |
|--------|--------------------|--------------------|
| A: Hybrid | 11 | Medium |
| B: Structural | 4 | High (nested discriminators) |
| C: Flat | 25-30 | Low (flat types) |

## Alignment with CSLN Principles

### 1. Explicit Over Magic

| Option | Score | Rationale |
|--------|-------|-----------|
| A: Hybrid | 7/10 | Legal types explicit, academic types implicit |
| B: Structural | 3/10 | Heavy discrimination logic, nested conditionals |
| C: Flat | 10/10 | All types explicit, no discrimination needed |

### 2. Declarative Templates

| Option | Score | Rationale |
|--------|-------|-----------|
| A: Hybrid | 7/10 | Mixed declarative (legal) and discriminatory (academic) |
| B: Structural | 4/10 | Parent type discrimination is procedural |
| C: Flat | 10/10 | Pure declarative, flat overrides |

### 3. Code-as-Schema

| Option | Score | Rationale |
|--------|-------|-----------|
| A: Hybrid | 8/10 | Types are clear, some discrimination |
| B: Structural | 6/10 | Nested types obscure semantics |
| C: Flat | 10/10 | One type = one Rust enum variant |

### 4. User-Centered Design

From [PERSONAS.md](../PERSONAS.md):

**Style Author:**
- Prefers: Explicit type overrides (Option C)
- Tolerates: Hybrid (Option A)
- Dislikes: Nested discrimination (Option B)

**Web Developer:**
- Prefers: Clear type dropdowns (Option C)
- Tolerates: Hybrid (Option A)
- Dislikes: Abstract structural types (Option B)

**Systems Architect:**
- Prefers: Efficient data model (Option B)
- Tolerates: Hybrid (Option A)
- Neutral: Type proliferation (Option C)

**Domain Expert:**
- Prefers: Semantic clarity (Option C: "legal-case" not "serial-component")
- Tolerates: Hybrid (Option A)
- Confused by: Structural abstraction (Option B: "why is a treaty a serial component?")

## Decision Criteria

**Choose Option A (Hybrid) if:**
- Pragmatism over consistency
- Parent-child relationships are valuable for academic materials
- Style template complexity is acceptable
- Code efficiency matters

**Choose Option B (Structural) if:**
- Code consolidation is highest priority
- Willing to accept style template complexity
- Parent-child model is philosophically correct
- Enum size minimization matters

**Choose Option C (Flat) if:**
- "Explicit over magic" is non-negotiable
- Style authoring simplicity is highest priority
- Semantic clarity matters more than code efficiency
- Alignment with CSL 1.0 types is valuable
- CSLN should match user mental models

## Recommendation Framework

**Questions to resolve:**

1. **Is the parent-child relationship genuinely valuable for academic materials?**
   - If YES: Lean toward Option A or B
   - If NO: Lean toward Option C

2. **How important is style template simplicity?**
   - Critical: Option C
   - Important: Option A
   - Acceptable trade-off: Option B

3. **How do we prioritize code efficiency vs style authoring clarity?**
   - Code efficiency: Option B
   - Balanced: Option A
   - Style clarity: Option C

4. **Should CSLN match CSL 1.0 type vocabulary or innovate?**
   - Match CSL 1.0: Option C (nearly 1:1 type mapping)
   - Innovate: Option B (new structural model)
   - Hybrid: Option A

5. **Can we tolerate architectural inconsistency?**
   - Yes: Option A is viable
   - No: Must choose B or C

## Migration Path

If we decide to change from current (Option A) to Option C:

**Breaking Change:** Yes (reference type names change)

**Migration Steps:**
1. Add new flat types alongside structural types (deprecate old)
2. Create converter: SerialComponent → JournalArticle/MagazineArticle/etc.
3. Update all example styles to use new types
4. Remove structural types in next major version

**Effort Estimate:** 4-6 weeks
- Type definitions: 1 week
- Converter logic: 1 week
- Style updates: 2 weeks
- Testing and documentation: 1-2 weeks

## Open Questions

1. **Parent-child efficiency:** If we use flat types, how do we avoid duplicating journal metadata across multiple articles?
   - Possible solution: Separate journal registry, articles reference by ID
   - Trade-off: More complex data model

2. **Parallel citations:** Legal materials can be cited from multiple reporters (parallel citations). Does this prove that reporters are NOT semantic parents?
   - If yes: supports flat types for legal
   - Does the same apply to preprints (arXiv + journal version)?

3. **CSL 1.0 compatibility:** CSL 1.0 has 30+ types. Should CSLN match this vocabulary or simplify?
   - Match: Option C
   - Simplify: Option B

4. **Future extensibility:** As we add more domains (music, art, datasets), will we add more flat types or fit them into structural categories?
   - Flat types: bounded by domain diversity (~50 types max?)
   - Structural: bounded by publication models (~10 types)

5. **Serialization size:** Do flat types with duplicate metadata (journal name repeated in every article) create unacceptable JSON bloat?
   - Mitigation: compression, ID references

## Success Metrics

Whichever option is chosen should be evaluated on:

1. **Style author feedback:** Can they write type overrides intuitively?
2. **Code maintainability:** Is the type system easy to extend?
3. **Error clarity:** Do users understand type validation errors?
4. **Migration ease:** Can CSL 1.0 styles be converted accurately?
5. **Performance:** No measurable impact on processing speed

## References

- Legal Citations PR: #164 (csl26-rmoi)
- CSLN Design Principles: CLAUDE.md
- User Personas: PERSONAS.md
- CSL 1.0 Specification: https://docs.citationstyles.org/en/stable/specification.html

## Next Steps

1. **User decision:** Choose Option A, B, or C based on priority trade-offs
2. **If Option C chosen:** Create migration plan bean
3. **If Option A chosen:** Document decision criteria for future type additions
4. **If Option B chosen:** Re-implement legal citations as SerialComponents

---

**Status:** Awaiting decision from project lead.
