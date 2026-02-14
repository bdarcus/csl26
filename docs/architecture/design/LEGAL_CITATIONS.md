# Legal Citation Support Architecture

**Status:** Design Phase
**Bean:** csl26-rmoi
**Author:** Design discussion with domain expert (social scientist use case)
**Date:** 2026-02-14

## Problem Statement

Legal citations present a unique challenge: they are needed by two distinct user populations with vastly different complexity requirements:

1. **Academics** (social scientists, historians, political scientists) citing legal materials in standard academic styles (APA, Chicago, MLA)
2. **Legal professionals** (lawyers, law students, legal publishers) using specialized legal citation styles (Bluebook, ALWD)

The challenge: How do we support robust legal citations for legal specialists without burdening academics who occasionally cite legal materials?

## Key Insight: Legal Citations are a Spectrum

Legal citations are not binary (lawyer/non-lawyer). They exist on a spectrum of complexity:

1. **Simple Academic Citation** (APA 7th):
   - Brown v. Board of Education, 347 U.S. 483 (1954)
   - Uses: case name, reporter volume, reporter abbreviation, page, year

2. **Complex Legal Citation** (Bluebook Short Form):
   - *Brown*, 347 U.S. at 495
   - Uses: hereinafter short form, pinpoint page reference

3. **Specialist Legal Citation** (Bluebook with Parallel):
   - Obergefell v. Hodges, 135 S. Ct. 2584, 2604, 192 L. Ed. 2d 609 (2015)
   - Uses: parallel reporters, jurisdiction-specific formatting, court class

**Architecture Insight:** These are all the **same reference type** (legal-case) with different **template complexity**.

## Design Solution: Two-Tier Legal Support

### Tier 1: Core Legal Types (Zero Configuration Burden)

**Purpose:** Support academics citing legal materials in standard academic styles.

**Approach:** Legal reference types become first-class CSLN types alongside article, book, chapter, etc.

**Core Legal Types:**
- `legal-case` - Court decisions (Brown v. Board of Education)
- `statute` - Legislative acts (Civil Rights Act of 1964)
- `treaty` - International agreements (Treaty of Versailles)
- `hearing` - Congressional/parliamentary hearings
- `regulation` - Administrative regulations (CFR, Federal Register)
- `brief` - Legal briefs and filings
- `classic` - Commonly cited classic works (Aristotle, Bible)

**Basic Fields** (all optional except title, authority, issued):
```yaml
reference:
  type: legal-case
  title: Brown v. Board of Education        # Required
  authority: U.S. Supreme Court             # Required (court name)
  volume: "347"
  reporter: U.S.
  page: "483"
  issued: "1954"                            # Required
```

**Standard Style Support** (APA 7th example):
```yaml
# styles/apa-7th.yaml
bibliography:
  template:
    # ... standard template for articles, books ...

    # Legal case override (type-specific)
    overrides:
      legal-case:
        template:
          - title: primary                  # Case name (italicized)
          - variable: volume
          - variable: reporter
          - variable: page
          - date: issued
            wrap: parentheses
```

**Output:**
```
Brown v. Board of Education, 347 U.S. 483 (1954).
```

**Burden Assessment:**
- Social scientist enters: case name, court, reporter citation, year
- Same complexity as citing a book (title, publisher, year)
- No specialist legal knowledge required
- Works with existing academic styles (APA, Chicago, MLA)

### Tier 2: Legal Specialist Features (Opt-In)

**Purpose:** Support legal professionals using specialized legal citation styles.

**Approach:** Optional specialist metadata and template components for Bluebook/ALWD styles.

**Specialist Fields** (all optional):
```yaml
reference:
  type: legal-case
  # Tier 1 fields (required)
  title: Brown v. Board of Education
  authority: U.S. Supreme Court
  volume: "347"
  reporter: U.S.
  page: "483"
  issued: "1954"

  # Tier 2 fields (optional, legal specialist only)
  jurisdiction: us:federal:scotus           # Jurisdiction hierarchy
  court-class: supreme                      # Court classification
  parallel-first: true                      # Parallel citation control
  hereinafter: Brown                        # Short form for subsequent cites
```

**Jurisdiction Hierarchies** (CSL-M pattern):
- Colon-delimited strings: `us:federal:scotus`, `us:state:ca:supreme`, `uk:scot:high`
- Used for jurisdiction-specific formatting rules
- Enables filtering and matching in style templates

**Court Classification:**
- Values: `supreme`, `appellate`, `trial`, `administrative`
- Controls citation formatting (e.g., trial court decisions include district)

**Parallel Citation Control:**
- `parallel-first: true` - First cite in parallel sequence (full citation)
- `parallel-first: false` - Subsequent parallel (suppress repeated elements)
- Used for regional reporters (e.g., Pacific Reporter + California Reporter)

**Position Extensions** (note styles):
- `far-note` - Cited before, but not recently (>5 intervening notes)
- `container-subsequent` - Same container (case/statute) cited before
- Enables legal footnote conventions (id., supra)

**Legal Specialist Style** (Bluebook example):
```yaml
# styles/bluebook-legal.yaml
citation:
  template:
    - legal-cite:
        style: short-form                   # full-citation | short-form | subsequent
        jurisdiction-display: abbreviated   # full | abbreviated | none
        parallel-suppression: true
        hereinafter: true

bibliography:
  template:
    - legal-cite:
        style: full-citation
        jurisdiction-display: full
        parallel-suppression: false
```

**Output Examples:**

*First citation (full):*
```
Brown v. Board of Education, 347 U.S. 483 (1954).
```

*Subsequent citation (short form):*
```
Brown, 347 U.S. at 495.
```

*Parallel citation:*
```
Obergefell v. Hodges, 135 S. Ct. 2584, 2604, 192 L. Ed. 2d 609 (2015).
```

## Implementation Strategy

### Phase 1: Core Legal Types (Tier 1)

**Priority:** Medium (Feature Roadmap)
**Effort:** 2-3 weeks

1. **Add legal reference types to `csln_core/src/reference/types.rs`:**
   ```rust
   pub enum ReferenceType {
       Article(Article),
       Book(Book),
       LegalCase(LegalCase),      // First-class type
       Statute(Statute),
       Treaty(Treaty),
       Hearing(Hearing),
       Regulation(Regulation),
       Brief(Brief),
       Classic(Classic),
       // ... other types
   }

   pub struct LegalCase {
       pub title: Title,                  // Required
       pub authority: String,             // Required (court name)
       pub volume: Option<String>,
       pub reporter: Option<String>,
       pub page: Option<String>,
       pub issued: EdtfString,            // Required

       // Tier 2 fields (all optional, default to None)
       #[serde(skip_serializing_if = "Option::is_none")]
       pub jurisdiction: Option<Jurisdiction>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub court_class: Option<CourtClass>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub parallel_first: Option<bool>,
       #[serde(skip_serializing_if = "Option::is_none")]
       pub hereinafter: Option<String>,
   }
   ```

2. **Extend legacy CSL-JSON converter** (`csl_legacy/src/types.rs`):
   - Map `legal_case` → `LegalCase`
   - Map `bill`, `legislation` → `Statute`
   - Map `treaty` → `Treaty`
   - Map `hearing` → `Hearing`
   - Map `regulation` → `Regulation`

3. **Add legal type overrides to `styles/apa-7th.yaml`:**
   - Test with Brown v. Board of Education fixture
   - Validate against oracle (citeproc-js APA output)

4. **Test fixtures** (`tests/fixtures/references-legal.json`):
   - Supreme Court case (Brown v. Board of Education)
   - Federal statute (Civil Rights Act of 1964)
   - International treaty (Treaty of Versailles)
   - Congressional hearing

5. **Update `/styleauthor` skill:**
   - Add legal type detection in reference material analysis
   - Include legal type templates in authoring phase

### Phase 2: Legal Specialist Features (Tier 2)

**Priority:** Low (after Phase 1 validated)
**Effort:** 3-4 weeks

1. **Create jurisdiction hierarchy system** (`csln_core/src/legal/jurisdiction.rs`):
   ```rust
   pub struct Jurisdiction {
       raw: String,  // "us:federal:scotus"
   }

   impl Jurisdiction {
       pub fn hierarchy(&self) -> Vec<&str> {
           self.raw.split(':').collect()
       }

       pub fn matches(&self, pattern: &str) -> bool {
           self.raw.starts_with(pattern)
       }

       pub fn level(&self) -> usize {
           self.hierarchy().len()
       }
   }
   ```

2. **Add court classification enum** (`csln_core/src/legal/mod.rs`):
   ```rust
   #[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
   #[serde(rename_all = "kebab-case")]
   pub enum CourtClass {
       Supreme,
       Appellate,
       Trial,
       Administrative,
   }
   ```

3. **Extend position tracking** (`csln_core/src/citation.rs`):
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub enum Position {
       First,
       Subsequent,
       Ibid,
       IbidWithLocator,
       FarNote,              // New: cited before, not recently
       ContainerSubsequent,  // New: same container cited before
   }
   ```

4. **Add `LegalCite` template component** (`csln_core/src/template.rs`):
   ```rust
   pub struct LegalCite {
       pub style: LegalCiteStyle,
       pub jurisdiction_display: JurisdictionDisplay,
       pub parallel_suppression: bool,
       pub hereinafter: bool,
   }

   pub enum LegalCiteStyle {
       FullCitation,
       ShortForm,
       Subsequent,
   }

   pub enum JurisdictionDisplay {
       Full,
       Abbreviated,
       None,
   }
   ```

5. **Create `styles/bluebook-legal.yaml`:**
   - Full Bluebook legal citation style
   - Uses all Tier 2 specialist features
   - Test with complex legal fixtures (parallel citations, hereinafter)

6. **Documentation:**
   - `docs/LEGAL_CITATIONS.md` - User guide for legal citations
   - `docs/examples/legal-citations.md` - Academic vs legal specialist examples

## Compliance with Project Principles

### 1. Explicit Over Magic
- Legal behavior declared in style templates, not hardcoded in processor
- Type-specific overrides make legal formatting explicit
- No hidden rules for legal types

### 2. Declarative Templates
- Legal templates use same override mechanism as other reference types
- Specialist legal features (parallel-suppression, hereinafter) are flat options
- No procedural logic required

### 3. Code-as-Schema
- Rust `LegalCase` struct is source of truth
- All legal types are proper enum variants
- Serde-driven serialization/deserialization

### 4. Graceful Degradation
- Tier 1 fields work without Tier 2 specialist fields
- Academic styles ignore specialist fields (no errors)
- Legal styles use specialist fields if present, fall back to basic if absent

### 5. No Configuration Burden for Non-Lawyers
- Academics never see specialist fields (not in academic style templates)
- Legal types render in APA/Chicago/MLA out-of-the-box
- Same effort as citing a book

## Persona Evaluation

### Style Author (Academic)
- Adds legal-case override to APA template
- Uses basic fields only (title, authority, volume, reporter, page, issued)
- No knowledge of jurisdiction hierarchies required

### Style Author (Legal)
- Adds Bluebook template with specialist features
- Configures jurisdiction display, parallel citation, hereinafter
- Full control over legal citation formatting

### Web Developer
- Legal types enumerable for dropdown menus
- Specialist fields clearly marked as optional
- JSON Schema documents both tiers

### Systems Architect
- Type-safe Rust enums for all legal types
- Well-commented legal module
- Clear separation of Tier 1 (core) and Tier 2 (specialist)

### Domain Expert (Social Scientist)
- Enters case name, court, reporter, year
- Same workflow as current Zotero/EndNote
- APA output matches expected format

### Domain Expert (Lawyer)
- Can add jurisdiction hierarchies, parallel citation control
- Bluebook style uses specialist features
- Matches law review citation requirements

## References

- **CSL-M Legal Extensions** - PRIOR_ART.md Section 4
- **CSL 1.0 Legal Types** - legal_case, bill, legislation, treaty, hearing, regulation (csl_legacy parsing)
- **Feature Roadmap** - CLAUDE.md (Medium priority)
- **Domain Expert Personas** - PERSONAS.md (Legal citation checklist)
- **Chicago Manual 16th** - Legal citation macros in styles-legacy/

## Open Questions

1. **Parallel Citation Algorithm:** How do we detect parallel sequences without manual tagging? Can we infer from reporter patterns?
2. **Jurisdiction Hierarchy Ontology:** Do we provide a standard jurisdiction list, or rely on style authors to define hierarchies?
3. **Hereinafter Scope:** Should hereinafter be reference-level metadata or citation-level override?
4. **Court Abbreviation Database:** Do we need a built-in court abbreviation lookup, or rely on reference metadata?

## Success Metrics

**Phase 1 (Tier 1) Success:**
- [ ] Social scientist can cite Brown v. Board of Education in APA style
- [ ] APA legal-case output matches oracle (citeproc-js)
- [ ] Chicago author-date legal-case output matches oracle
- [ ] MLA legal-case output matches oracle
- [ ] Zero configuration burden for academic users

**Phase 2 (Tier 2) Success:**
- [ ] Law student can cite case with parallel reporters in Bluebook style
- [ ] Hereinafter short form works in subsequent citations
- [ ] Jurisdiction hierarchies control formatting
- [ ] Position extensions (far-note, container-subsequent) work in note styles
- [ ] Bluebook output matches law review citation manual

## Conclusion

By treating legal citations as a **spectrum** rather than a binary (lawyer/non-lawyer), we can support both academic and legal specialist use cases without burdening either population. Legal types become first-class CSLN reference types (Tier 1), with optional specialist features (Tier 2) for legal professionals.

This architecture aligns with CSLN design principles (explicit over magic, declarative templates, code-as-schema) and satisfies all personas (academic style authors, legal style authors, web developers, systems architects, domain experts).
