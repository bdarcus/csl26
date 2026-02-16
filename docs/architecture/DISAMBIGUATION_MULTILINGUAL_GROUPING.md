# Disambiguation Extensions: Multilingual and Grouping Support

**Status:** Planning phase
**Created:** 2026-02-16
**Related:** docs/DISAMBIGUATION.md, docs/architecture/MULTILINGUAL.md, docs/architecture/design/BIBLIOGRAPHY_GROUPING.md

## Executive Summary

The existing disambiguation system (11/11 tests passing) operates on monolingual surface forms using family names and year collisions. This document outlines the architecture for extending disambiguation to support:

1. **Multilingual disambiguation** - Match on displayed written forms (transliterated/translated/original)
2. **Group-aware sorting** - Respect per-group bibliography sort order
3. **Per-group suffix assignment** - Restart year suffix sequence within each group

## Problem Statement

### Current Limitations

**Monolingual Keys Only:**
```rust
// Current implementation (processor/disambiguation.rs:322)
let author_key = names_vec.iter()
    .map(|n| n.family_or_literal().to_lowercase())
    .join(",");
```

This bypasses multilingual rendering preferences. If a style shows transliterated names, disambiguation must match on transliterated forms, not original forms.

**Example Collision:**
- Reference A: 王小明 (Wang Xiaoming) - Original Chinese
- Reference B: Wang Xiaoming - Romanized input
- **Current behavior:** Treats as different (王 ≠ Wang)
- **Expected behavior:** Treats as same if style shows transliteration

**Global Suffix Assignment:**

Current implementation assigns year suffixes globally across entire bibliography. For grouped bibliographies (e.g., legal citations separated from books), suffix should restart per group:

```
Legal Cases:
  Smith v. Jones (2020a)
  Brown v. Board (2020b)

Books:
  Smith, J. (2020a)  ← Restarts at 'a', not '2020c'
  Brown, T. (2020b)
```

## Architecture Design

### 1. Multilingual-Aware Disambiguation Keys

**Principle:** Disambiguation matching must use the same surface form the style will render.

#### Implementation

```rust
impl<'a> Disambiguator<'a> {
    fn render_name_for_disambiguation(&self, name: &StructuredName) -> String {
        // Use Display trait which respects MultilingualConfig
        match &self.config.multilingual {
            Some(ml) => match ml.name_mode {
                MultilingualMode::Primary =>
                    name.family.to_string().to_lowercase(),
                MultilingualMode::Transliterated => {
                    name.family.transliteration()
                        .unwrap_or(&name.family.original())
                        .to_lowercase()
                }
                MultilingualMode::Translated => {
                    name.family.translation()
                        .unwrap_or(&name.family.original())
                        .to_lowercase()
                }
                MultilingualMode::Combined => {
                    // Combined mode uses transliteration as primary
                    name.family.transliteration()
                        .unwrap_or(&name.family.original())
                        .to_lowercase()
                }
            }
            None => name.family.to_string().to_lowercase(),
        }
    }
}
```

#### Updated `make_group_key()`

```rust
let author_key = if let Some(authors) = reference.author() {
    let names_vec = authors.to_names_vec();
    // Use multilingual-aware rendering
    names_vec.iter()
        .map(|n| self.render_name_for_disambiguation(n))
        .join(",")
} else {
    "".to_string()
};
```

### 2. Group-Aware Disambiguation Sorting

**Problem:** Year suffix assignment currently sorts by title (line 209), but groups may have custom sort orders (type-order, name-order, language-order).

#### Disambiguator with Group Context

```rust
pub struct Disambiguator<'a> {
    bibliography: &'a Bibliography,
    config: &'a Config,
    group_sort: Option<&'a GroupSort>, // NEW: Per-group sort specification
}

impl<'a> Disambiguator<'a> {
    pub fn new(
        bibliography: &'a Bibliography,
        config: &'a Config
    ) -> Self {
        Self { bibliography, config, group_sort: None }
    }

    pub fn with_group_sort(
        bibliography: &'a Bibliography,
        config: &'a Config,
        group_sort: &'a GroupSort,
    ) -> Self {
        Self { bibliography, config, group_sort: Some(group_sort) }
    }
}
```

#### Updated `apply_year_suffix()`

```rust
fn apply_year_suffix(
    &self,
    hints: &mut HashMap<String, ProcHints>,
    group: &[&Reference],
    key: String,
    len: usize,
) {
    let sorted_group = if let Some(sort_spec) = self.group_sort {
        // Use GroupSorter for per-group ordering
        let locale = &Locale::en_us(); // TODO: Get from config
        let sorter = GroupSorter::new(locale);
        sorter.sort_references(group.to_vec(), sort_spec)
    } else {
        // Fallback to title sorting (current behavior)
        let mut sorted = group.to_vec();
        sorted.sort_by(|a, b| {
            a.title().unwrap_or_default().to_lowercase()
                .cmp(&b.title().unwrap_or_default().to_lowercase())
        });
        sorted
    };

    // Assign suffixes based on sorted order
    for (i, reference) in sorted_group.iter().enumerate() {
        hints.insert(reference.id().unwrap_or_default(), ProcHints {
            disamb_condition: true,
            group_index: i + 1,
            group_length: len,
            group_key: key.clone(),
            expand_given_names: false,
            min_names_to_show: None,
            ..Default::default()
        });
    }
}
```

### 3. Per-Group Disambiguation

**Design Decision:** Year suffix sequence restarts per group.

**Rationale:**
- Grouped bibliographies are conceptually separate sections
- Users expect "2020a" in each section to be the first item, not a global counter
- Legal citation conventions require per-section numbering

#### BibliographyRenderer Integration

```rust
// In BibliographyRenderer::render_grouped()
for group in &grouped_refs {
    // Run disambiguation WITHIN each group
    let disambiguator = Disambiguator::with_group_sort(
        group.references,
        self.config,
        &group.sort_spec,
    );
    let hints = disambiguator.calculate_hints();

    // Render items with per-group hints
    for reference in group.references {
        let hint = hints.get(reference.id()).unwrap_or_default();
        self.render_bibliography_entry(reference, hint);
    }
}
```

## Implementation Plan

### Phase 1: Multilingual Disambiguation Keys

**Files:**
- `crates/csln_processor/src/processor/disambiguation.rs`

**Tasks:**
1. Add `render_name_for_disambiguation()` method
2. Update `make_group_key()` to use multilingual rendering
3. Add unit tests for multilingual key generation

**Acceptance Criteria:**
- Vietnamese names (Nguyen Van A) match when style shows given-family order
- Japanese romanization (Satō vs Sato) matches when style shows Hepburn transliteration
- Chinese transliteration (王 → Wang) matches original characters

### Phase 2: Group-Aware Sorting

**Files:**
- `crates/csln_processor/src/processor/disambiguation.rs`

**Tasks:**
1. Add `group_sort` field to Disambiguator struct
2. Add `with_group_sort()` constructor
3. Update `apply_year_suffix()` to use GroupSorter
4. Add unit tests for group-aware suffix assignment

**Acceptance Criteria:**
- Legal citations sorted by case name, not title
- Type-order grouping sorts by reference type first
- Year suffix respects group sort order

### Phase 3: Per-Group Disambiguation

**Files:**
- `crates/csln_processor/src/render/bibliography.rs`

**Tasks:**
1. Modify `render_grouped()` to run disambiguation per group
2. Ensure suffix sequence restarts per group
3. Add integration tests for grouped bibliographies

**Acceptance Criteria:**
- Legal citations group shows (2020a), books group shows (2020a) independently
- No suffix collisions within a group
- Suffix assignment stable across re-renders

### Phase 4: Test Coverage

**Test Files:**
- `crates/csln_processor/tests/disambiguation.rs`
- `crates/csln_processor/tests/multilingual_disambiguation.rs` (new)
- `crates/csln_processor/tests/grouped_disambiguation.rs` (new)

**Test Scenarios:**

**Multilingual Disambiguation:**
1. Vietnamese given-family order with disambiguation
2. Japanese romanization collision detection
3. Chinese transliteration matching
4. Mixed-script bibliography with year suffix
5. Multilingual given name expansion

**Grouped Disambiguation:**
1. Legal citations with type-order sorting
2. Language-based grouping with per-group suffix restart
3. Combined grouping + multilingual + disambiguation
4. Suffix stability across group boundaries

**Acceptance Criteria:**
- All existing 11 tests pass (no regressions)
- 5 new multilingual disambiguation tests pass
- 3 new grouped disambiguation tests pass

### Phase 5: Documentation

**Files:**
- `docs/DISAMBIGUATION.md`
- `docs/architecture/MULTILINGUAL.md`
- `docs/architecture/design/BIBLIOGRAPHY_GROUPING.md`

**Updates:**
1. Document multilingual disambiguation behavior with examples
2. Add section on grouped bibliography disambiguation
3. Update test coverage section with new test count
4. Add cross-references between multilingual, grouping, and disambiguation docs

## Edge Cases and Open Questions

### Open Questions

1. **Suffix restart policy:** Should suffix sequence restart per group or continue globally?
   - **Decision:** Restart per group (see rationale above)

2. **Multilingual mode mismatch:** What if reference A has transliteration but B doesn't?
   - **Decision:** Fall back to original via Display trait chain

3. **Group sort conflicts:** What if style specifies global sort but also uses grouping?
   - **Decision:** Group sort takes precedence within each group

### Edge Cases

1. **Empty multilingual fields:**
   - Use Display trait fallback chain (transliteration → original)
   - Never panic on missing transliteration

2. **Mixed monolingual + multilingual references:**
   - Monolingual references render as Simple strings
   - Disambiguation key generation handles both cases

3. **No author (organizational/institutional):**
   - Use `Contributor::Literal` value
   - Fallback to title if no contributor

## Success Metrics

- [ ] All existing 11 disambiguation tests pass
- [ ] 5 new multilingual disambiguation tests implemented and passing
- [ ] 3 new grouped disambiguation tests implemented and passing
- [ ] Zero regressions in existing test suite
- [ ] Documentation updated with examples
- [ ] Code reviewed and approved

## References

- [DISAMBIGUATION.md](../DISAMBIGUATION.md) - Current disambiguation implementation
- [MULTILINGUAL.md](MULTILINGUAL.md) - Multilingual data model
- [BIBLIOGRAPHY_GROUPING.md](design/BIBLIOGRAPHY_GROUPING.md) - Grouping architecture
- CSL 1.0 Specification - Disambiguation section
