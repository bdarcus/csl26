# BibLaTeX to CSLN Reference Mapping

This document describes the mapping between BibLaTeX fields and the CSLN `InputReference` model.

## Entry Type Mapping

| BibLaTeX Type | CSLN Type | Notes |
|---------------|-----------|-------|
| `@article` | `SerialComponent` | Journal/magazine articles |
| `@book` | `Monograph` | Single-volume books |
| `@mvbook` | `Monograph` | Multi-volume books |
| `@inbook` | `CollectionComponent` | Part of a book (by same author) |
| `@incollection` | `CollectionComponent` | Chapter in edited volume |
| `@inproceedings` | `CollectionComponent` | Conference paper |
| `@proceedings` | `Collection` | Conference proceedings |
| `@collection` | `Collection` | Edited anthology |
| `@mvcollection` | `Collection` | Multi-volume collection |
| `@reference` | `Collection` | Reference work (encyclopedia) |
| `@inreference` | `CollectionComponent` | Entry in reference work |
| `@periodical` | `Serial` | Journal/magazine (as container) |
| `@report` | `Monograph` | Technical report |
| `@thesis` | `Monograph` | Thesis/dissertation |
| `@online` | `Monograph` | Web resource |
| `@software` | `Monograph` | Software |
| `@dataset` | `Monograph` | Dataset |
| `@patent` | `Monograph` | Patent |
| `@booklet` | `Monograph` | Printed work without publisher |
| `@manual` | `Monograph` | Technical manual |
| `@misc` | `Monograph` | Fallback type |
| `@unpublished` | `Monograph` | Unpublished work |

## Field Mapping: Currently Supported

### Core Fields

| BibLaTeX Field | CSLN Field | Type | Notes |
|----------------|------------|------|-------|
| `author` | `author` | `Contributor` | Parsed as `ContributorList` |
| `editor` | `editor` (via parent) | `Contributor` | In `Collection.editor` |
| `translator` | `translator` | `Contributor` | |
| `title` | `title` | `Title` | Maps to `Title::Single` or structured |
| `subtitle` | `title.sub` | `Subtitle` | Combined with main title |
| `booktitle` | `parent.title` | `Title` | Container title for components |
| `journaltitle` | `parent.title` | `Title` | Serial title for articles |
| `date` | `issued` | `EdtfString` | Native EDTF parsing |
| `year` | `issued` | `EdtfString` | Fallback if no `date` |
| `publisher` | `publisher` | `Contributor` | |
| `doi` | `doi` | `String` | |
| `url` | `url` | `Url` | Validated URL |
| `urldate` | `accessed` | `EdtfString` | |
| `isbn` | `isbn` | `String` | |
| `pages` | `pages` | `String` | Page range |
| `volume` | `volume` | `NumOrStr` | |
| `issue` / `number` | `issue` | `NumOrStr` | |
| `edition` | `edition` | `String` | |
| `note` | `note` | `String` | |

### Name Handling

| BibLaTeX | CSLN `StructuredName` | Notes |
|----------|----------------------|-------|
| `family` | `family` | Family/surname |
| `given` | `given` | Given names |
| `prefix` | `non_dropping_particle` | "von", "van", "de" |
| `suffix` | `suffix` | "Jr.", "III" |

Literal/corporate names map to `SimpleName`:

```yaml
# BibLaTeX: author = {{World Health Organization}}
author:
  name: "World Health Organization"
```

## Field Mapping: Suggested Additions

The following fields are commonly used in BibLaTeX but not yet in the CSLN model.

### High Priority (Common Fields)

| BibLaTeX Field | Suggested CSLN Field | Type | Use Case |
|----------------|---------------------|------|----------|
| `keywords` | `keywords` | `Vec<String>` | Search/filtering |
| `language` / `langid` | `language` | `String` | Item language (BCP-47) |
| `abstract` | `abstract` | `String` | Abstract text |
| `annotation` / `annote` | `annotation` | `String` | User annotations |
| `series` | `series` | `String` | Book/report series name |
| `number` | `number` | `NumOrStr` | Report/thesis number |

### Medium Priority (Specialized Fields)

| BibLaTeX Field | Suggested CSLN Field | Type | Use Case |
|----------------|---------------------|------|----------|
| `institution` / `school` | `institution` | `Contributor` | Thesis/report affiliation |
| `pubstate` | `pub_state` | `PubState` enum | "inpress", "forthcoming" |
| `origtitle` | `original_title` | `Title` | For translations |
| `origdate` | `original_date` | `EdtfString` | Original publication date |
| `origlanguage` | `original_language` | `String` | Original language |
| `origlocation` | `original_location` | `String` | Original publisher location |
| `eventdate` | `event_date` | `EdtfString` | Conference date |
| `eventtitle` | `event_title` | `String` | Conference name |
| `venue` | `venue` | `String` | Conference location |
| `location` / `address` | `location` | `String` | Publisher location |
| `pagetotal` | `page_count` | `NumOrStr` | Total pages |

### Low Priority (Specialized/Rare)

| BibLaTeX Field | Suggested CSLN Field | Type | Use Case |
|----------------|---------------------|------|----------|
| `addendum` | `addendum` | `String` | Appended notes |
| `library` | `library` | `String` | Library holdings |
| `shorthand` | `shorthand` | `String` | Custom citation key |
| `shorttitle` | `short_title` | `String` | Abbreviated title |
| `shortauthor` | `short_author` | `Contributor` | Abbreviated author |
| `shortjournal` | `short_journal` | `String` | Abbreviated journal |
| `issn` | `issn` | `String` | Serial ISSN |
| `isrn` | `isrn` | `String` | Report number |
| `isan` / `ismn` / `iswc` | identifiers | `String` | AV/music identifiers |
| `eprint` | `eprint` | `String` | arXiv/preprint ID |
| `eprinttype` | `eprint_type` | `String` | "arxiv", "hdl", etc. |
| `eprintclass` | `eprint_class` | `String` | arXiv subject class |
| `version` | `version` | `String` | Software/dataset version |
| `type` | `genre` | `String` | Thesis type, report type |

## Suggested Enum Additions

### PubState

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PubState {
    InPress,
    Submitted,
    Forthcoming,
    InPreparation,
    Preprint,
}
```

### EditorType (for multi-role editing)

BibLaTeX supports `editor`, `editora`, `editorb`, `editorc` with type annotations:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EditorType {
    Editor,
    Compiler,
    Founder,
    Continuator,
    Redactor,
    Reviser,
    Collaborator,
    Organizer,
}
```

## Struct Enhancements

### Add to `Monograph`

```rust
pub struct Monograph {
    // ... existing fields ...
    pub series: Option<String>,
    pub number: Option<NumOrStr>,
    pub institution: Option<Contributor>,  // For theses/reports
    pub language: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub abstract_text: Option<String>,
    pub pub_state: Option<PubState>,
}
```

### Add to `SerialComponent`

```rust
pub struct SerialComponent {
    // ... existing fields ...
    pub language: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub abstract_text: Option<String>,
}
```

### Add to `CollectionComponent`

```rust
pub struct CollectionComponent {
    // ... existing fields ...
    pub language: Option<String>,
    pub keywords: Option<Vec<String>>,
}
```

### Add to `Collection` (for proceedings)

```rust
pub struct Collection {
    // ... existing fields ...
    pub event_title: Option<String>,
    pub event_date: Option<EdtfString>,
    pub venue: Option<String>,
    pub series: Option<String>,
}
```

## Implementation Notes

1. **EDTF Dates**: The `edtf` crate handles BibLaTeX date formats natively, including:
   - Ranges: `2020/2021`
   - Uncertainty: `2020?`, `2020~`
   - Seasons: `2020-21` (spring)

2. **Name Parsing**: The `biblatex` crate parses names into `Person` structs with family/given/prefix/suffix parts.

3. **Location Field**: BibLaTeX's `location` (publisher location) is distinct from `SimpleName.location` (organization location). Consider renaming one.

4. **Crossref Resolution**: BibLaTeX `crossref` and `xref` fields reference parent entries. The converter should resolve these to inline parent structures.

## References

- [BibLaTeX Manual](https://ctan.org/pkg/biblatex) - Authoritative field documentation
- [biblatex crate](https://docs.rs/biblatex) - Rust parsing library
- [EDTF Specification](https://www.loc.gov/standards/datetime/) - Date format standard
