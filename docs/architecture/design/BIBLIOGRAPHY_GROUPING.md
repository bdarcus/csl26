# Bibliography Grouping Architecture

**Status:** Design
**Created:** 2026-02-15
**Related Issues:** csl26-group

## Overview

This document defines the architecture for configurable bibliography grouping in CSL Next. The design enables styles to divide bibliographies into labeled sections with distinct sorting rules, addressing use cases from legal citations (type-based hierarchies) to multilingual bibliographies (culturally-appropriate name sorting).

## Problem Statement

Current implementation provides only basic hardcoded support for separating visible vs silent (nocite) citations under an "Additional Reading" heading. Real-world use cases require:

1. **Legal Hierarchy (The Bluebook):** Type-based grouping with rigid ordering (Constitutions → Statutes → Cases) that ignores alphabetical content
2. **Multilingual Grouping (Juris-M):** Language-based grouping with distinct sorting logic per group (Vietnamese by given-name, English by family-name)
3. **Topical Groupings:** Custom field-based grouping (keywords, reference types, custom metadata)

The critical requirement is **per-group sorting** - a simple "group then sort" approach fails when different groups require different collation rules.

## Design Principles

### Three Orthogonal Concerns

1. **Selection** - Which items belong to which group (predicate logic)
2. **Ordering** - How groups appear relative to each other (sequence)
3. **Presentation** - Per-group sorting, headings, styling

### Key Constraints

- **First-match semantics:** Items appear in first matching group only (prevents duplication)
- **Explicit over magic:** All grouping declared in YAML, no hardcoded hierarchies
- **Graceful degradation:** Omitting `groups` field produces flat bibliography (current behavior)
- **Per-group sorting:** Groups override global sort when culturally appropriate

## Schema Design

### BibliographySpec Extension

```yaml
bibliography:
  # Global defaults
  options: ...
  template: ...

  # Optional grouping (omit for flat bibliography)
  groups:
    - id: primary-legal
      heading: "Primary Sources"
      selector:
        type: [legal-case, statute, treaty]
      sort:
        template:
          - key: type
            order: [legal-case, statute, treaty]  # Explicit sequence
          - key: issued

    - id: vietnamese
      heading: "Tài liệu tiếng Việt"
      selector:
        field:
          language: vi
      sort:
        template:
          - key: author
            sort-order: given-family  # Vietnamese convention
          - key: year

    - id: other
      heading: "Other Sources"
      selector:
        not:
          type: [legal-case, statute, treaty]
          field:
            language: vi
      sort:
        template:
          - key: author
            sort-order: family-given
```

### Rust Types

```rust
pub struct BibliographySpec {
    pub options: Option<Config>,
    pub template: Option<Template>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<BibliographyGroup>>,
}

pub struct BibliographyGroup {
    /// Unique identifier for this group
    pub id: String,

    /// Optional heading (omit for no heading)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,

    /// Selector predicate
    pub selector: GroupSelector,

    /// Optional per-group sorting (falls back to global sort if omitted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<GroupSort>,

    /// Optional per-group template override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,

    /// Optional disambiguation scope
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disambiguate: Option<DisambiguationScope>,
}

pub enum DisambiguationScope {
    Globally,  // Default: cross-group suffixes
    Locally,   // Reset suffixes within this group
}

pub struct GroupSelector {
    /// Type-based filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeSelector>,

    /// Citation status filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited: Option<CitedStatus>,

    /// Field-based filtering (language, keywords, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<HashMap<String, FieldMatcher>>,

    /// Negation for fallback groups
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<GroupSelector>>,
}

pub enum TypeSelector {
    Single(String),
    Multiple(Vec<String>),
}

pub enum CitedStatus {
    Visible,  // Cited in document
    Silent,   // Nocite only
    Any,
}

pub enum FieldMatcher {
    Exact(String),
    Multiple(Vec<String>),
    Pattern(FieldPattern),
}

pub struct GroupSort {
    pub template: Vec<GroupSortSpec>,
}

pub struct GroupSortSpec {
    pub key: SortKey,

    #[serde(default = "default_true")]
    pub ascending: bool,

    /// For type-based ordering (e.g., [legal-case, statute, treaty])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,

    /// For name sorting (family-given vs given-family)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<NameSortOrder>,
}

pub enum NameSortOrder {
    FamilyGiven,  // Western convention
    GivenFamily,  // Vietnamese convention
}
```

## Processor Logic

### High-Level Flow

```rust
pub fn render_grouped_bibliography(
    &self,
    refs: &[Reference],
    style: &BibliographySpec,
) -> Result<Vec<GroupedSection>, ProcessorError> {
    let groups = style.groups.as_ref().ok_or_else(|| flat_bibliography)?;

    let mut sections = Vec::new();
    let mut assigned = HashSet::new();

    for group in groups {
        // Filter items matching this group's selector
        let items: Vec<&Reference> = refs
            .iter()
            .filter(|r| !assigned.contains(&r.id))
            .filter(|r| self.matches_selector(r, &group.selector))
            .collect();

        // Mark as assigned (first-match semantics)
        for item in &items {
            assigned.insert(&item.id);
        }

        // Sort using per-group or global sort
        let sorted = self.sort_items(items, group.sort.as_ref().unwrap_or(&style.global_sort));

        sections.push(GroupedSection {
            heading: group.heading.clone(),
            items: self.render_items(sorted, &group.template.unwrap_or(&style.template)),
        });
    }

    // Fallback for ungrouped items
    let unassigned: Vec<&Reference> = refs
        .iter()
        .filter(|r| !assigned.contains(&r.id))
        .collect();

    if !unassigned.is_empty() {
        let sorted = self.sort_items(unassigned, &style.global_sort);
        sections.push(GroupedSection {
            heading: None,
            items: self.render_items(sorted, &style.template),
        });
    }

    Ok(sections)
}

fn matches_selector(&self, reference: &Reference, selector: &GroupSelector) -> bool {
    let mut matches = true;

    // Type filtering
    if let Some(type_sel) = &selector.r#type {
        matches &= match type_sel {
            TypeSelector::Single(t) => reference.ref_type == *t,
            TypeSelector::Multiple(types) => types.contains(&reference.ref_type),
        };
    }

    // Citation status filtering
    if let Some(cited) = &selector.cited {
        matches &= match cited {
            CitedStatus::Visible => self.cited_ids.contains(&reference.id),
            CitedStatus::Silent => !self.cited_ids.contains(&reference.id),
            CitedStatus::Any => true,
        };
    }

    // Field filtering
    if let Some(fields) = &selector.field {
        for (field_name, matcher) in fields {
            matches &= self.matches_field(reference, field_name, matcher);
        }
    }

    // Negation
    if let Some(not_sel) = &selector.not {
        matches &= !self.matches_selector(reference, not_sel);
    }

    matches
}
```

### Performance Considerations

- **Complexity:** O(n × groups) for selector evaluation
- **Optimization:** Pre-index by type/field for O(1) lookup if needed
- **Typical case:** 3-5 groups, <1000 items per bibliography = negligible overhead

## Use Case Examples

### Legal Hierarchy (Bluebook)

```yaml
bibliography:
  groups:
    - id: cases
      heading: "Cases"
      selector:
        type: legal-case
      sort:
        template:
          - key: field
            field: court-class
            order: [supreme, appellate, trial]
          - key: issued
            ascending: false

    - id: statutes
      heading: "Statutes"
      selector:
        type: statute
      sort:
        template:
          - key: title
```

**Input:** Brown (1954, supreme), District case (2020, trial), Roe (1973, supreme)

**Output:**
```
Cases
Roe v. Wade, 410 U.S. 113 (1973).
Brown v. Board of Education, 347 U.S. 483 (1954).
District case (2020).

Statutes
[...]
```

### Multilingual Sorting (Juris-M)

```yaml
bibliography:
  groups:
    - id: vietnamese
      heading: "Tài liệu tiếng Việt"
      selector:
        field:
          language: vi
      sort:
        template:
          - key: author
            sort-order: given-family

    - id: western
      selector:
        not:
          field:
            language: vi
      sort:
        template:
          - key: author
            sort-order: family-given
```

**Input:** Nguyễn Văn A (vi), Trần Thị B (vi), Smith John (en)

**Output:**
```
Tài liệu tiếng Việt
Nguyễn Văn A. (2019).
Trần Thị B. (2020).

Smith, J. (2018).
```

### Topical Grouping

```yaml
bibliography:
  groups:
    - id: primary-sources
      heading: "Primary Sources"
      selector:
        type: [manuscript, interview, archival-document]

    - id: datasets
      heading: "Datasets"
      selector:
        type: dataset

    - id: secondary
      heading: "Secondary Sources"
      selector:
        not:
          type: [manuscript, interview, archival-document, dataset]
```

## User-Defined Grouping

Users can override style-defined grouping by providing a custom `groups` field:

```yaml
# User's bibliography.yaml
groups:
  - id: must-read
    heading: "Essential Reading"
    selector:
      field:
        keywords: essential

  - id: other
    selector:
      not:
        field:
          keywords: essential
```

**Processor precedence:** User-defined groups > Style-defined groups > Flat bibliography

## Implementation Roadmap

1. **Schema Extension** (2 days)
   - Add types to `csln_core/src/lib.rs`
   - Generate JSON schema
   - Add tests for YAML parsing

2. **Selector Logic** (3 days)
   - Create `csln_processor/src/grouping/selector.rs`
   - Implement predicate evaluation
   - Add unit tests for selector matching

3. **Group Sorting** (4 days)
   - Extend sorting with type-order and name-order variants
   - Refactor `sort_references` to accept sort specification
   - Add integration tests

4. **Processor Integration** (3 days)
   - Refactor `render_grouped_bibliography_with_format`
   - Add fallback logic for ungrouped items
   - Preserve backward compatibility

5. **Legal Use Case** (2 days)
   - Create `styles/bluebook-legal.yaml`
   - Test type hierarchy with legal fixtures
   - Validate against Bluebook manual

6. **Multilingual Use Case** (2 days)
   - Create `styles/multilingual-academic.yaml`
   - Test per-group sorting with Vietnamese/English fixtures
   - Validate collation rules

7. **Documentation** (2 days)
   - Style authoring guide for grouping
   - Migration guide from hardcoded logic
   - Update CLAUDE.md with grouping patterns

## Prior Art

### biblatex

Uses predicate-based filtering with explicit section commands:

```latex
\printbibliography[type=article,title=Articles]
\printbibliography[nottype=article,title=Other Sources]
```

**Key insight:** Flat, declarative syntax with negation for fallback groups.

### CSL 1.0

No bibliography grouping constructs. All grouping is hardcoded in processors.

**CSLN opportunity:** First-class grouping support in style schema.

### CSL-M

Locale-specific `<layout>` elements for presentation, but no grouping.

**CSLN improvement:** Unified grouping + sorting + presentation model.

## Open Questions

**Q: Should we support nested groups?**
**A:** No. Flat groups with selectors are sufficient for all known use cases. Nesting adds complexity without clear benefits.

**Q: Should items appear in multiple groups?**
**A:** No. First-match semantics prevent duplication, which is critical for legal citations.

**Q: Performance with 10,000+ items?**
**A:** Pre-index by type/field if benchmarks show O(n × groups) is problematic. Defer optimization until measured.

## Compliance with CSLN Principles

- **Explicit Over Magic:** All grouping declared in YAML
- **Declarative Templates:** Flat selector syntax, no procedural conditionals
- **Code-as-Schema:** Rust structs drive JSON schema validation
- **Graceful Degradation:** Omitting `groups` uses flat bibliography
- **Multilingual Support:** Per-group sorting enables culturally appropriate collation

## References

- The Bluebook: A Uniform System of Citation, 21st Edition
- biblatex Manual, Section 3.6.5 (Bibliography Filters)
- Juris-M Documentation (Multilingual Sorting)
- CSL-M Specification (Legal Citations)
