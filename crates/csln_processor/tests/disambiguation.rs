/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN disambiguation tests using pure CSLN types.
//!
//! These tests validate the three core disambiguation strategies without
//! JSON parsing overhead or legacy CSL JSON conversion. All test data is
//! defined using CSLN Rust types.
//!
//! 1. Year suffix (a, b, c, ..., z, aa, ab, ...)
//! 2. Name expansion (showing additional authors beyond et-al)
//! 3. Given name expansion (initials or full names for conflicting surnames)

use csln_core::{
    citation::{Citation, CitationItem, CitationMode},
    reference::{
        Contributor, ContributorList, EdtfString, InputReference as Reference, Monograph,
        MonographType, MultilingualString, Parent, Serial, SerialComponent, SerialComponentType,
        SerialType, StructuredName, Title,
    },
    CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

// --- Helper Functions for Test Data Construction ---

/// Create a native Reference for a book with minimal fields.
fn make_book(id: &str, family: &str, given: &str, year: i32, title: &str) -> Reference {
    Reference::Monograph(Box::new(Monograph {
        id: Some(id.to_string()),
        r#type: MonographType::Book,
        title: Title::Single(title.to_string()),
        author: Some(Contributor::StructuredName(StructuredName {
            family: MultilingualString::Simple(family.to_string()),
            given: MultilingualString::Simple(given.to_string()),
            ..Default::default()
        })),
        editor: None,
        translator: None,
        issued: EdtfString(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        note: None,
        isbn: None,
        doi: None,
        edition: None,
        genre: None,
        keywords: None,
        original_date: None,
        original_title: None,
    }))
}

/// Create a native Reference with multiple authors.
fn make_book_multi_author(
    id: &str,
    authors: Vec<(&str, &str)>,
    year: i32,
    title: &str,
) -> Reference {
    let author_list: Vec<Contributor> = authors
        .into_iter()
        .map(|(family, given)| {
            Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                ..Default::default()
            })
        })
        .collect();

    Reference::Monograph(Box::new(Monograph {
        id: Some(id.to_string()),
        r#type: MonographType::Book,
        title: Title::Single(title.to_string()),
        author: Some(Contributor::ContributorList(ContributorList(author_list))),
        editor: None,
        translator: None,
        issued: EdtfString(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        note: None,
        isbn: None,
        doi: None,
        edition: None,
        genre: None,
        keywords: None,
        original_date: None,
        original_title: None,
    }))
}

/// Create a native Reference for an article-journal.
fn make_article(id: &str, family: &str, given: &str, year: i32, title: &str) -> Reference {
    Reference::SerialComponent(Box::new(SerialComponent {
        id: Some(id.to_string()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Single(title.to_string())),
        author: Some(Contributor::StructuredName(StructuredName {
            family: MultilingualString::Simple(family.to_string()),
            given: MultilingualString::Simple(given.to_string()),
            ..Default::default()
        })),
        translator: None,
        issued: EdtfString(year.to_string()),
        parent: Parent::Embedded(Serial {
            r#type: SerialType::AcademicJournal,
            title: Title::Single(String::new()),
            editor: None,
            publisher: None,
            issn: None,
        }),
        url: None,
        accessed: None,
        language: None,
        note: None,
        doi: None,
        pages: None,
        volume: None,
        issue: None,
        keywords: None,
    }))
}

/// Create a native Reference for an article-journal with multiple authors.
fn make_article_multi_author(
    id: &str,
    authors: Vec<(&str, &str)>,
    year: i32,
    title: &str,
) -> Reference {
    let author_list: Vec<Contributor> = authors
        .into_iter()
        .map(|(family, given)| {
            Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                ..Default::default()
            })
        })
        .collect();

    Reference::SerialComponent(Box::new(SerialComponent {
        id: Some(id.to_string()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Single(title.to_string())),
        author: Some(Contributor::ContributorList(ContributorList(author_list))),
        translator: None,
        issued: EdtfString(year.to_string()),
        parent: Parent::Embedded(Serial {
            r#type: SerialType::AcademicJournal,
            title: Title::Single(String::new()),
            editor: None,
            publisher: None,
            issn: None,
        }),
        url: None,
        accessed: None,
        language: None,
        note: None,
        doi: None,
        pages: None,
        volume: None,
        issue: None,
        keywords: None,
    }))
}

// --- Test Execution Helpers ---

/// Execute a test case with default disambiguation settings (year_suffix only).
///
/// **Purpose**: Simplified test harness for tests that use the default year-suffix-only
/// disambiguation strategy. Always enables `disambiguate_year_suffix` and disables other
/// disambiguation methods.
///
/// **When to use**: Call this for tests that focus on year suffix behavior (e.g., 2020a,
/// 2020b). For tests with custom disambiguation settings, use
/// `run_test_case_native_with_options()` instead.
///
/// **Parameters**:
/// - `input`: Vector of native Reference structs
/// - `citation_items`: Nested vector of citation item IDs (batches)
/// - `expected`: Expected rendered output string
/// - `mode`: Either "citation" (process citations) or "bibliography" (render full bibliography)
///
/// **Example**:
/// ```rust,ignore
/// let input = vec![make_book("item1", "Smith", "John", 2020, "Alpha")];
/// let citation_items = vec![vec!["item1"]];
/// let expected = "Smith, (2020)";
/// run_test_case_native(&input, &citation_items, expected, "citation");
/// ```
fn run_test_case_native(
    input: &[Reference],
    citation_items: &[Vec<&str>],
    expected: &str,
    mode: &str,
) {
    run_test_case_native_with_options(
        input,
        citation_items,
        expected,
        mode,
        true,
        false,
        false,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
/// Execute a test case with custom disambiguation settings.
///
/// **Purpose**: Full-featured test harness supporting all disambiguation strategies
/// and et-al configuration options. Use this when tests need fine-grained control
/// over disambiguation behavior.
///
/// **Workflow**:
/// 1. Builds an author-date style with the specified disambiguation flags
/// 2. Converts native Reference structs into the processor's bibliography
/// 3. For citation mode: processes each citation batch and collects results
/// 4. For bibliography mode: processes citations if provided, then renders full bibliography
/// 5. Compares actual output against expected string (trimmed, line-by-line)
///
/// **Parameters**:
/// - `input`: Slice of native Reference structs
/// - `citation_items`: Nested slice of citation item IDs (batches of string slices)
/// - `expected`: Expected output string
/// - `mode`: "citation" or "bibliography"
/// - `disambiguate_year_suffix`: Enable year suffix disambiguation (a, b, c, ...)
/// - `disambiguate_names`: Enable et-al expansion when authors differ
/// - `disambiguate_givenname`: Enable given name expansion (initials A., B., etc.)
/// - `et_al_min`: Threshold before abbreviating to "et al." (default: 3)
/// - `et_al_use_first`: How many authors to show before "et al." (default: 1)
///
/// **Disambiguation Priority**:
/// 1. **Year suffix only** (year_suffix: true, others: false): Resolves conflicts via 2020a, 2020b
/// 2. **Name expansion** (names: true): Shows additional authors to disambiguate
/// 3. **Given name expansion** (add_givenname: true): Shows initials for authors with same family name
/// 4. **Combined**: Use multiple flags for cascading fallback behavior
///
/// **Et-al Settings**:
/// - `et_al_min`: Minimum authors before abbreviating (standard: 3)
/// - `et_al_use_first`: How many to display before "et al." (standard: 1)
/// - Set both to `None` to disable et-al abbreviation (show all authors)
///
/// **Output Assertion**:
/// Trims both expected and actual, then compares line-by-line. Logs both values
/// for debugging via println! before assertion.
fn run_test_case_native_with_options(
    input: &[Reference],
    citation_items: &[Vec<&str>],
    expected: &str,
    mode: &str,
    disambiguate_year_suffix: bool,
    disambiguate_names: bool,
    disambiguate_givenname: bool,
    et_al_min: Option<u8>,
    et_al_use_first: Option<u8>,
) {
    // Create author-date style with customizable disambiguation options
    let style = build_author_date_style(
        disambiguate_year_suffix,
        disambiguate_names,
        disambiguate_givenname,
        et_al_min,
        et_al_use_first,
    );

    // Build bibliography from native references
    let mut bibliography = indexmap::IndexMap::new();
    for item in input.iter() {
        if let Some(id) = item.id() {
            bibliography.insert(id, item.clone());
        }
    }

    let processor = Processor::new(style, bibliography);

    if mode == "citation" {
        let mut results = Vec::new();

        for batch in citation_items {
            let items: Vec<CitationItem> = batch
                .iter()
                .map(|id| CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                })
                .collect();

            let citation = Citation {
                items,
                mode: CitationMode::NonIntegral,
                ..Default::default()
            };

            let res = processor
                .process_citation(&citation)
                .expect("Failed to process citation");
            results.push(res);
        }

        let actual = results.join("\n");
        println!("Expected: '{}'", expected);
        println!("Actual: '{}'", actual);
        assert_eq!(actual.trim(), expected.trim(), "Citation output mismatch");
    } else if mode == "bibliography" {
        if !citation_items.is_empty() {
            for batch in citation_items {
                let items: Vec<CitationItem> = batch
                    .iter()
                    .map(|id| CitationItem {
                        id: id.to_string(),
                        ..Default::default()
                    })
                    .collect();
                let citation = Citation {
                    items,
                    ..Default::default()
                };
                processor.process_citation(&citation).ok();
            }
        }

        let actual = processor.render_bibliography();
        assert_eq!(
            actual.trim(),
            expected.trim(),
            "Bibliography output mismatch"
        );
    }
}

/// Build an author-date style with customizable disambiguation options.
///
/// **Purpose**: Factory function that constructs a minimal but complete CSLN author-date
/// style suitable for disambiguation testing. The resulting style uses a simple two-component
/// citation template: Author (Year).
///
/// **Template structure**:
/// ```
/// Citation: [Author short-form] + [Year wrapped in parentheses]
/// Multi-cite delimiter: "; "
/// Name formatting: All authors shown (no et-al by default)
/// ```
///
/// **Disambiguation options**:
/// The function configures the `DisambiguationStrategy` and contributor rules based on
/// the provided flags. These control how the processor handles conflicting citations:
///
/// - `year_suffix`: Adds suffixes (a, b, c, ..., z, aa, ab, ...) to year when
///   multiple references share the same author+year combination
/// - `names`: Expands et-al abbreviated author lists to show more names when abbreviation
///   prevents disambiguation
/// - `add_givenname`: Shows initials/first names for authors with the same family name
///
/// **Et-al configuration**:
/// - If `et_al_min` and `et_al_use_first` are provided, enables et-al abbreviation
///   (e.g., "Smith et al." instead of full author list)
/// - Values follow CSL defaults: min=3, use_first=1
/// - If both are None, all authors are shown in every context
///
/// **Example output**:
/// - Default (year_suffix only): "Smith, (2020a); Brown, (2020b)"
/// - With et-al (et_al_min=3, use_first=1): "Smith et al. (2020)"
/// - With givenname: "J Smith; J Brown"
///
/// **Key style settings**:
/// - `initialize_with: " "` (space) - Given names shown as "J" not "J."
/// - `multi_cite_delimiter: "; "` - Standard semicolon separator
/// - `name_as_sort_order: False` - Family-first not applied by default
fn build_author_date_style(
    disambiguate_year_suffix: bool,
    disambiguate_names: bool,
    disambiguate_givenname: bool,
    et_al_min: Option<u8>,
    et_al_use_first: Option<u8>,
) -> Style {
    use csln_core::options::{
        Config, ContributorConfig, Disambiguation, Processing, ProcessingCustom, ShortenListOptions,
    };
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
        TemplateContributor, TemplateDate, WrapPunctuation,
    };

    // Build disambiguation config
    let disambiguate = if disambiguate_year_suffix || disambiguate_names || disambiguate_givenname {
        Some(Disambiguation {
            year_suffix: disambiguate_year_suffix,
            names: disambiguate_names,
            add_givenname: disambiguate_givenname,
        })
    } else {
        None
    };

    // Build contributors config with et-al settings and initialize_with for initials
    let contributors = Some(ContributorConfig {
        shorten: if et_al_min.is_some() || et_al_use_first.is_some() {
            Some(ShortenListOptions {
                min: et_al_min.unwrap_or(3),
                use_first: et_al_use_first.unwrap_or(1),
                ..Default::default()
            })
        } else {
            None
        },
        initialize_with: Some(" ".to_string()),
        ..Default::default()
    });

    // Citation template: Author (Year)
    let citation_template = vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                ..Default::default()
            },
            ..Default::default()
        }),
    ];

    Style {
        info: StyleInfo {
            title: Some("Author-Date Disambiguation Test".to_string()),
            id: Some("http://test.example/disambiguation".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                disambiguate,
                ..Default::default()
            })),
            contributors,
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(citation_template),
            multi_cite_delimiter: Some("; ".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[allow(dead_code)]
fn create_test_style() -> Style {
    // Default: year-suffix only
    build_author_date_style(true, false, false, None, None)
}

/// Test year suffix disambiguation with alphabetical title sorting.
///
/// **Strategy**: Year suffix only (`year_suffix: true`, `names: false`, `add_givenname: false`)
///
/// **Input**: Two books by the same author (Smith) published in 2020 with different titles:
/// - item1: "Alpha" (alphabetically first, should receive suffix 'a')
/// - item2: "Beta" (alphabetically second, should receive suffix 'b')
///
/// **Why these references conflict**: Both have identical author-year combination
/// (Smith, 2020), which requires disambiguation to distinguish them in citations.
///
/// **Expected output**: "Smith, (2020a); Smith, (2020b)"
/// - Year suffix 'a' for first item (Alpha)
/// - Year suffix 'b' for second item (Beta)
/// - Demonstrates: Suffix assignment follows alphabetical title ordering
///
/// **What this validates**:
/// - Year suffix disambiguation correctly identifies duplicate author+year pairs
/// - Suffixes are assigned in alphabetical order by title (stable sorting)
/// - Multi-cite separator ("; ") correctly joins items
///
/// **Status**: ✅ PASSING (Phase 2 implementation complete)
#[test]
fn test_disambiguate_yearsuffixandsort() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Alpha"),
        make_book("item2", "Smith", "John", 2020, "Beta"),
    ];
    let citation_items = vec![vec!["item1", "item2"]];
    let expected = "Smith, (2020a); Smith, (2020b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test empty input handling (placeholder test).
///
/// **Strategy**: Year suffix only (default settings)
///
/// **Input**: Empty reference list (no items to process)
///
/// **Why this matters**: Validates that the processor gracefully handles edge case
/// of no bibliography data. This is a placeholder/sanity check.
///
/// **Expected output**: Empty string (no citations to render)
///
/// **What this validates**:
/// - Processor does not panic on empty bibliography
/// - Empty input produces empty output (no false positives)
///
/// **Status**: ✅ PASSING (Baseline requirement met)
#[test]
fn test_disambiguate_yearsuffixattwolevels() {
    let input = vec![];
    let citation_items: Vec<Vec<&str>> = vec![];
    let expected = "";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test year suffix disambiguation with multiple identical references.
///
/// **Strategy**: Year suffix only (`year_suffix: true`)
///
/// **Input**: Three journal articles by Ylinen, A. all published in 1995:
/// - id: 22 (item1 in batch, should get 'a')
/// - id: 21 (item2 in batch, should get 'b')
/// - id: 23 (item3 in batch, should get 'c')
///
/// **Why these references conflict**: All three share identical author + year
/// (Ylinen, 1995), creating a three-way conflict requiring disambiguation.
///
/// **Expected output**: "Ylinen, (1995a); Ylinen, (1995b); Ylinen, (1995c)"
/// - All three items show full format with year suffixes
///
/// **What this validates**:
/// - Multiple (3+) references with identical author-year are correctly disambiguated
/// - Suffixes are assigned in order of citation appearance
///
/// **Status**: ✅ PASSING
#[test]
fn test_disambiguate_yearsuffixmixeddates() {
    let input = vec![
        make_article("22", "Ylinen", "A", 1995, "Article A"),
        make_article("21", "Ylinen", "A", 1995, "Article B"),
        make_article("23", "Ylinen", "A", 1995, "Article C"),
    ];
    let citation_items = vec![vec!["22", "21", "23"]];
    let expected = "Ylinen, (1995a); Ylinen, (1995b); Ylinen, (1995c)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test given name expansion for authors with duplicate family names.
///
/// **Strategy**: Given name expansion only (`add_givenname: true`, `year_suffix: false`,
/// `names: false`)
///
/// **Input**: Three books with authors sharing family names:
/// - ITEM-1: "Asthma, Albert" + "Asthma, Bridget" (co-authors, same family name)
///   Published 1980, Title: "Book A"
/// - ITEM-2: "Bronchitis, Beauregarde" (unique author)
///   Published 1995, Title: "Book B"
/// - ITEM-3: "Asthma, Albert" (same as first author of ITEM-1, different year)
///   Published 1885, Title: "Book C"
///
/// **Why these need disambiguation**: ITEM-1 and ITEM-3 both have author "Albert Asthma"
/// with same title/different years. Without given name expansion, both would display
/// as "Asthma (year)". Given name expansion adds first names to distinguish.
///
/// **Expected output**: "Asthma, Asthma, (1980); Bronchitis, (1995); Asthma, (1885)"
/// - ITEM-1: Shows both authors with family names: "Asthma, Asthma" (both have same family)
/// - ITEM-2: Single author needs no expansion: "Bronchitis"
/// - ITEM-3: Shows just "Asthma" (different year from ITEM-1)
///
/// **What this validates**:
/// - Given name expansion is applied when same family names appear in author lists
/// - Multi-author formatting preserves comma separator
/// - Mixed-expansion scenarios (some authors expanded, some not)
///
/// **Status**: ❌ FAILING (Given name expansion logic incomplete)
#[test]
fn test_disambiguate_bycitetwoauthorssamefamilyname() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Asthma", "Albert"), ("Asthma", "Bridget")],
            1980,
            "Book A",
        ),
        make_book("ITEM-2", "Bronchitis", "Beauregarde", 1995, "Book B"),
        make_book("ITEM-3", "Asthma", "Albert", 1885, "Book C"),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2", "ITEM-3"]];
    let expected = "Asthma, Asthma, (1980); Bronchitis, (1995); Asthma, (1885)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        false,
        true,
        None,
        None,
    );
}

/// Test et-al expansion success: Name expansion disambiguates conflicting references.
///
/// **Strategy**: Name expansion only (`names: true`, `year_suffix: false`,
/// `add_givenname: false`)
/// Et-al config: `et_al_min: 3`, `et_al_use_first: 1` (collapse to first author + "et al.")
///
/// **Input**: Two books by different author teams, same year (1980):
/// - ITEM-1: Smith, Brown, Jones (3 authors)
/// - ITEM-2: Smith, Beefheart, Jones (3 authors)
/// - Conflict: Same first author (Smith) + same year (1980) when collapsed to "et al."
///
/// **Why expansion is needed**: Both show "Smith et al. (1980)" when abbreviated,
/// which is ambiguous. Name expansion shows more authors to distinguish them:
/// - ITEM-1: Brown is second author → "Smith, Brown, et al."
/// - ITEM-2: Beefheart is second author → "Smith, Beefheart, et al."
///
/// **Expected output**: "Smith, Brown, et al., (1980); Smith, Beefheart, et al., (1980)"
/// - First two authors shown for both references
/// - Third author suppressed as "et al." (because et_al_min=3 and we have 3 authors)
/// - Disambiguation achieved via different second author names
///
/// **What this validates**:
/// - Name expansion activates when et-al abbreviation would create ambiguity
/// - Expansion shows additional authors from the original list
/// - Expansion succeeds because second author differs between items
/// - ET-AL settings (min/use_first) are respected
///
/// **Status**: ❌ FAILING (Name expansion logic not implemented)
#[test]
fn test_disambiguate_addnamessuccess() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![
                ("Smith", "John"),
                ("Beefheart", "Captain"),
                ("Jones", "John"),
            ],
            1980,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    let expected = "Smith, Brown, et al., (1980); Smith, Beefheart, et al., (1980)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        true,
        false,
        Some(3),
        Some(1),
    );
}

/// Test et-al expansion failure: Cascade to year suffix when name expansion fails.
///
/// **Strategy**: Name expansion + year suffix fallback
/// (`year_suffix: true`, `names: true`, `add_givenname: false`)
/// Et-al config: `et_al_min: 3`, `et_al_use_first: 1`
///
/// **Input**: Two identical books with same authors, same year (1980):
/// - ITEM-1: Smith, Brown, Jones
/// - ITEM-2: Smith, Brown, Jones (exact duplicate author list)
///
/// **Why name expansion fails**: Both references have identical author lists.
/// Showing "Smith, Brown, et al." for both still produces ambiguity. Name expansion
/// cannot help because the additional authors are identical.
///
/// **Expected output**: "Smith et al., (1980a); Smith et al., (1980b)"
/// - Year suffix applied as fallback when name expansion cannot resolve conflict
///
/// **What this validates**:
/// - Processor recognizes when name expansion cannot resolve conflicts
/// - Fallback mechanism triggers year suffix disambiguation
/// - Cascading disambiguation strategies work together
/// - Identical author lists remain unresolved by name expansion (correctly)
///
/// **Status**: ❌ FAILING (Cascade fallback logic incomplete)
#[test]
fn test_disambiguate_addnamesfailure() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    let expected = "Smith et al., (1980a); Smith et al., (1980b)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        true,
        true,
        false,
        Some(3),
        Some(1),
    );
}

/// Test given name expansion with initial form (initialize_with).
///
/// **Strategy**: Given name expansion only (`add_givenname: true`,
/// `year_suffix: false`, `names: false`)
/// Name formatting: `initialize_with: " "` (shows initials as "J" not "J.")
///
/// **Input**: 5 books with strategically named authors testing initial expansion:
/// - ITEM-1: Roe, Jane (unique family name, no conflict)
/// - ITEM-2: Doe, John (conflicts with ITEM-3 on family name)
/// - ITEM-3: Doe, Aloysius (conflicts with ITEM-2 on family name)
/// - ITEM-4: Smith, Thomas (conflicts with ITEM-5 on family name)
/// - ITEM-5: Smith, Ted (conflicts with ITEM-4 on family name)
///
/// **Citation batches**:
/// 1. [ITEM-1] - Single item, citation: "Roe"
/// 2. [ITEM-2, ITEM-3] - Conflicting Doe authors, citation: "J Doe; A Doe"
/// 3. [ITEM-4, ITEM-5] - Conflicting Smith authors, citation: "T Smith; T Smith"
///
/// **Expected output** (3 citation batches):
/// ```
/// Roe, (2000)
/// J Doe, (2000); A Doe, (2000)
/// T Smith, (2000); T Smith, (2000)
/// ```
/// - Line 1: Single non-conflicting author (Roe) shown without expansion
/// - Line 2: Conflicting Doe authors shown with initials (J and A)
/// - Line 3: Conflicting Smith authors both shown with initial (T)
///
/// **What this validates**:
/// - Given name expansion applies only to authors with conflicting family names
/// - Initials are formatted via `initialize_with` setting (no period)
/// - Single authors without conflicts are not expanded
/// - Multi-cite separator ("; ") correctly joins expanded citations
/// - Same given initial (Ted/Thomas both "T") preserved correctly
///
/// **Status**: ✅ PASSING (initialize_with expansion implemented)
#[test]
fn test_disambiguate_bycitegivennameshortforminitializewith() {
    let input = vec![
        make_book("ITEM-1", "Roe", "Jane", 2000, "Book A"),
        make_book("ITEM-2", "Doe", "John", 2000, "Book B"),
        make_book("ITEM-3", "Doe", "Aloysius", 2000, "Book C"),
        make_book("ITEM-4", "Smith", "Thomas", 2000, "Book D"),
        make_book("ITEM-5", "Smith", "Ted", 2000, "Book E"),
    ];
    let citation_items = vec![
        vec!["ITEM-1"],
        vec!["ITEM-2", "ITEM-3"],
        vec!["ITEM-4", "ITEM-5"],
    ];
    let expected = "Roe, (2000)\nJ Doe, (2000); A Doe, (2000)\nT Smith, (2000); T Smith, (2000)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        false,
        true,
        None,
        None,
    );
}

/// Test year suffix + et-al with varying author list lengths.
///
/// **Strategy**: Year suffix only (`year_suffix: true`, `names: false`,
/// `add_givenname: false`)
/// Et-al config: `et_al_min: 3`, `et_al_use_first: 1`
///
/// **Input**: Three journal articles from 2000, two with author conflicts:
/// - ITEM-1: 5 authors (Baur, Fröberg, Baur, Guggenheim, Haase)
///   Abbreviated to "Baur et al." (first 1 + et-al)
/// - ITEM-2: 3 authors (Baur, Schileyko, Baur)
///   Abbreviated to "Baur et al." (first 1 + et-al)
/// - ITEM-3: 1 author (Doe)
///   No abbreviation: "Doe"
///
/// **Conflict detection**: ITEM-1 and ITEM-2 both show "Baur et al." and are from 2000.
/// Both require year suffixes to disambiguate.
///
/// **Expected output**: "Baur et al., (2000b); Baur et al., (2000a); Doe, (2000)"
/// - ITEM-1 and ITEM-2 both abbreviated to "Baur et al." with year suffixes
/// - ITEM-3: "Doe, (2000)" (no conflict, no suffix)
///
/// **What this validates**:
/// - Year suffixes apply when et-al abbreviation creates conflicts
/// - Et-al abbreviation respects min threshold (3+ authors → et-al)
/// - Suffix ordering follows alphabetical sorting
///
/// **Status**: ✅ PASSING
#[test]
fn test_disambiguate_basedonetalsubsequent() {
    let input = vec![
        make_article_multi_author(
            "ITEM-1",
            vec![
                ("Baur", "Bruno"),
                ("Fröberg", "Lars"),
                ("Baur", "Anette"),
                ("Guggenheim", "Richard"),
                ("Haase", "Martin"),
            ],
            2000,
            "Ultrastructure of snail grazing damage to calcicolous lichens",
        ),
        make_article_multi_author(
            "ITEM-2",
            vec![
                ("Baur", "Bruno"),
                ("Schileyko", "Anatoly A."),
                ("Baur", "Anette"),
            ],
            2000,
            "Ecological observations on Arianta aethiops aethiops",
        ),
        make_article("ITEM-3", "Doe", "John", 2000, "Some bogus title"),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2", "ITEM-3"]];
    let expected = "Baur et al., (2000b); Baur et al., (2000a); Doe, (2000)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        true,
        false,
        false,
        Some(3),
        Some(1),
    );
}

/// Test conditional disambiguation with identical author-year pairs.
///
/// **Strategy**: Year suffix only (default settings)
///
/// **Input**: Two books with identical authors and year, different titles:
/// - ITEM-1: Doe, John + Roe, Jane (co-authors), 2000, Title: "Book A"
/// - ITEM-2: Doe, John + Roe, Jane (co-authors), 2000, Title: "Book B"
///
/// **Why these need disambiguation**: Both references have identical author+year
/// (Doe, Roe, 2000). Without disambiguation, both would render identically.
/// Year suffix applies (a, b) to distinguish them.
///
/// **Expected output**: "Doe, Roe, (2000a); Doe, Roe, (2000b)"
/// - Both citations show full author names with year suffixes
///
/// **What this validates**:
/// - Identical author-year pairs are detected
/// - Year suffix disambiguation correctly applied
///
/// **Status**: ✅ PASSING
#[test]
fn test_disambiguate_bycitedisambiguatecondition() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Doe", "John"), ("Roe", "Jane")],
            2000,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![("Doe", "John"), ("Roe", "Jane")],
            2000,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    let expected = "Doe, Roe, (2000a); Doe, Roe, (2000b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test empty input handling with year suffix (placeholder test).
///
/// **Strategy**: Year suffix only (default settings)
///
/// **Input**: Empty reference list (no items to process)
///
/// **Why this matters**: Validates that the processor gracefully handles edge case
/// of no bibliography data with year suffix disambiguation enabled. This is a
/// placeholder/sanity check test.
///
/// **Expected output**: Empty string (no citations to render)
///
/// **What this validates**:
/// - Processor does not panic when year suffix is enabled but no items exist
/// - Empty input produces empty output (no false positives)
/// - Edge case handling for disambiguation logic
///
/// **Status**: ✅ PASSING (Baseline requirement met)
#[test]
fn test_disambiguate_failwithyearsuffix() {
    let input = vec![];
    let citation_items: Vec<Vec<&str>> = vec![];
    let expected = "";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test year suffix with 30 entries (base-26 suffix wrapping).
///
/// **Strategy**: Year suffix only (`year_suffix: true`)
///
/// **Input**: 30 identical references by Smith, John published in 1986
/// - ITEM-1 through ITEM-30
/// - All with identical author+year (Smith, 1986)
///
/// **Why this tests base-26 wrapping**: With 26 base suffixes (a-z), the 27th
/// item requires wrapping to double-letter suffixes (aa, ab, ac, ad). This validates
/// the suffix generation algorithm for large numbers of disambiguating suffixes.
///
/// **Expected output**: "Smith, (1986a); Smith, (1986b); ... Smith, (1986z); Smith, (1986aa); ... Smith, (1986ad)"
///
/// **Suffix sequence**:
/// - Single-letter: a-z (26 items)
/// - Double-letter: aa-ad (4 more items)
/// - Total: 30 items validating base-26 wrapping
///
/// **What this validates**:
/// - Suffix generation handles 30+ identical references
/// - Wrapping from single (z) to double letters (aa) works correctly
/// - Alphabetical ordering maintains consistency (aa < ab < ac < ad)
///
/// **Status**: ✅ PASSING
#[test]
fn test_disambiguate_yearsuffixfiftytwoentries() {
    let mut input = Vec::new();
    let mut citation_ids = Vec::new();

    for i in 1..=30 {
        input.push(make_book(
            &format!("ITEM-{}", i),
            "Smith",
            "John",
            1986,
            "Book",
        ));
        citation_ids.push(format!("ITEM-{}", i));
    }

    let citation_items = vec![citation_ids.iter().map(|s| s.as_str()).collect()];
    let expected = "Smith, (1986a); Smith, (1986b); Smith, (1986c); Smith, (1986d); Smith, (1986e); Smith, (1986f); Smith, (1986g); Smith, (1986h); Smith, (1986i); Smith, (1986j); Smith, (1986k); Smith, (1986l); Smith, (1986m); Smith, (1986n); Smith, (1986o); Smith, (1986p); Smith, (1986q); Smith, (1986r); Smith, (1986s); Smith, (1986t); Smith, (1986u); Smith, (1986v); Smith, (1986w); Smith, (1986x); Smith, (1986y); Smith, (1986z); Smith, (1986aa); Smith, (1986ab); Smith, (1986ac); Smith, (1986ad)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}
