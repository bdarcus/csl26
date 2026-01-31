use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod renderer; // Expose the renderer
pub use renderer::{CitationItem, Renderer};

// New CSLN schema modules
pub mod locale;
pub mod options;
pub mod presets;
pub mod reference;
pub mod template;

// Embedded templates for priority styles (APA, Chicago, Vancouver, IEEE, Harvard)
pub mod embedded;

pub use locale::Locale;
pub use options::Config;
pub use presets::{ContributorPreset, DatePreset, TitlePreset};
pub use template::TemplateComponent;

/// A named template (reusable sequence of components).
pub type Template = Vec<TemplateComponent>;

/// The new CSLN Style model.
///
/// This is the target schema for CSLN, featuring declarative options
/// and simple template components instead of procedural conditionals.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Style {
    /// Style schema version.
    #[serde(default = "default_version")]
    pub version: String,
    /// Style metadata.
    pub info: StyleInfo,
    /// Named reusable templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<HashMap<String, Template>>,
    /// Global style options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Config>,
    /// Citation specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<CitationSpec>,
    /// Bibliography specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<BibliographySpec>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// Available embedded template presets.
///
/// These reference battle-tested templates for common citation styles.
/// See `csln_core::embedded` for the actual template implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TemplatePreset {
    /// APA 7th edition (author-date)
    Apa,
    /// Chicago Manual of Style (author-date)
    ChicagoAuthorDate,
    /// Vancouver (numeric)
    Vancouver,
    /// IEEE (numeric)
    Ieee,
    /// Harvard/Elsevier (author-date)
    Harvard,
}

impl TemplatePreset {
    /// Resolve this preset to a citation template.
    pub fn citation_template(&self) -> Template {
        match self {
            TemplatePreset::Apa => embedded::apa_citation(),
            TemplatePreset::ChicagoAuthorDate => embedded::chicago_author_date_citation(),
            TemplatePreset::Vancouver => embedded::vancouver_citation(),
            TemplatePreset::Ieee => embedded::ieee_citation(),
            TemplatePreset::Harvard => embedded::harvard_citation(),
        }
    }

    /// Resolve this preset to a bibliography template.
    pub fn bibliography_template(&self) -> Template {
        match self {
            TemplatePreset::Apa => embedded::apa_bibliography(),
            TemplatePreset::ChicagoAuthorDate => embedded::chicago_author_date_bibliography(),
            TemplatePreset::Vancouver => embedded::vancouver_bibliography(),
            TemplatePreset::Ieee => embedded::ieee_bibliography(),
            TemplatePreset::Harvard => embedded::harvard_bibliography(),
        }
    }
}

/// Citation specification.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct CitationSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Config>,
    /// Reference to an embedded template preset.
    /// If both `use_preset` and `template` are present, `template` takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_preset: Option<TemplatePreset>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub template: Option<Template>,
    /// Wrap the entire citation in punctuation. Preferred over prefix/suffix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<template::WrapPunctuation>,
    /// Prefix for the citation (use only when `wrap` doesn't suffice, e.g., " (" or "[Ref ").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Suffix for the citation (use only when `wrap` doesn't suffice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Delimiter between components within a single citation item (e.g., ", " or " ").
    /// Defaults to ", ".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Delimiter between multiple citation items (e.g., "; ").
    /// Defaults to "; ".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "multi-cite-delimiter")]
    pub multi_cite_delimiter: Option<String>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

impl CitationSpec {
    /// Resolve the effective template for this citation.
    ///
    /// Returns the explicit `template` if present, otherwise resolves `use_preset`.
    /// Returns `None` if neither is specified.
    pub fn resolve_template(&self) -> Option<Template> {
        self.template
            .clone()
            .or_else(|| self.use_preset.as_ref().map(|p| p.citation_template()))
    }
}

/// Bibliography specification.
#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone, Default)]
#[serde(rename_all = "kebab-case")]
pub struct BibliographySpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Config>,
    /// Reference to an embedded template preset.
    /// If both `use_preset` and `template` are present, `template` takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_preset: Option<TemplatePreset>,
    /// The default template for bibliography entries.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub template: Option<Template>,
    /// Type-specific template overrides. When present, replaces the default
    /// template for entries of the specified types. Keys are reference type
    /// names (e.g., "chapter", "article-journal").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_templates: Option<HashMap<String, Template>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

impl BibliographySpec {
    /// Resolve the effective template for this bibliography.
    ///
    /// Returns the explicit `template` if present, otherwise resolves `use_preset`.
    /// Returns `None` if neither is specified.
    pub fn resolve_template(&self) -> Option<Template> {
        self.template
            .clone()
            .or_else(|| self.use_preset.as_ref().map(|p| p.bibliography_template()))
    }
}

/// Style metadata.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct StyleInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Default locale for the style (e.g., "en-US", "de-DE").
    /// Used for locale-aware term resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_locale: Option<String>,
}

// ============================================================================
// Legacy types below - kept for migration bridge from CSL 1.0
// These will be deprecated once migration is complete
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ItemType {
    Article,
    ArticleJournal,
    ArticleMagazine,
    ArticleNewspaper,
    Bill,
    Book,
    Broadcast,
    Chapter,
    Dataset,
    Entry,
    EntryDictionary,
    EntryEncyclopedia,
    Figure,
    Graphic,
    Interview,
    LegalCase,
    Legislation,
    Manuscript,
    Map,
    MotionPicture,
    MusicalScore,
    Pamphlet,
    PaperConference,
    Patent,
    PersonalCommunication,
    Post,
    PostWeblog,
    Report,
    Review,
    ReviewBook,
    Song,
    Speech,
    Thesis,
    Treaty,
    Webpage,
    Software,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Variable {
    Author,
    CollectionEditor,
    Composer,
    ContainerAuthor,
    Director,
    Editor,
    EditorialDirector,
    Illustrator,
    Interviewer,
    OriginalAuthor,
    Recipient,
    ReviewedAuthor,
    Translator,
    Accessed,
    AvailableDate,
    EventDate,
    Issued,
    OriginalDate,
    Submitted,
    ChapterNumber,
    CollectionNumber,
    Edition,
    Issue,
    Number,
    NumberOfPages,
    NumberOfVolumes,
    Volume,
    Abstract,
    Annote,
    Archive,
    ArchiveLocation,
    ArchivePlace,
    Authority,
    CallNumber,
    CitationLabel,
    CitationNumber,
    CollectionTitle,
    ContainerTitle,
    ContainerTitleShort,
    Dimensions,
    DOI,
    Event,
    EventPlace,
    FirstReferenceNoteNumber,
    Genre,
    ISBN,
    ISSN,
    Jurisdiction,
    Keyword,
    Locator,
    Medium,
    Note,
    OriginalPublisher,
    OriginalPublisherPlace,
    OriginalTitle,
    Page,
    PageFirst,
    PMCID,
    PMID,
    Publisher,
    PublisherPlace,
    References,
    ReviewedTitle,
    Scale,
    Section,
    Source,
    Status,
    Title,
    TitleShort,
    URL,
    Version,
    YearSuffix,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CslnStyle {
    pub info: CslnInfo,
    pub locale: CslnLocale,
    pub citation: Vec<CslnNode>,
    pub bibliography: Vec<CslnNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CslnLocale {
    pub terms: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CslnInfo {
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CslnNode {
    Text { value: String },
    Variable(VariableBlock),
    Date(DateBlock),
    Names(NamesBlock),
    Group(GroupBlock),
    Condition(ConditionBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableBlock {
    pub variable: Variable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelOptions>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub overrides: HashMap<ItemType, FormattingOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupBlock {
    pub children: Vec<CslnNode>,
    pub delimiter: Option<String>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionBlock {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_item_type: Vec<ItemType>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_variables: Vec<Variable>,
    pub then_branch: Vec<CslnNode>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub else_if_branches: Vec<ElseIfBranch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub else_branch: Option<Vec<CslnNode>>,
}

/// An else-if branch in a condition block, capturing type or variable conditions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElseIfBranch {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_item_type: Vec<ItemType>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_variables: Vec<Variable>,
    pub children: Vec<CslnNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LabelOptions {
    pub form: LabelForm,
    pub pluralize: bool,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    Long,
    Short,
    Symbol,
    Verb,
    VerbShort,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DateBlock {
    pub variable: Variable,
    #[serde(flatten)]
    pub options: DateOptions,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NamesBlock {
    pub variable: Variable,
    #[serde(flatten)]
    pub options: NamesOptions,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NamesOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<NameMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<AndTerm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_last: Option<DelimiterPrecedes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_as_sort_order: Option<NameAsSortOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al: Option<EtAlOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelOptions>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub substitute: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NameMode {
    Long,
    Short,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AndTerm {
    Text,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPrecedes {
    Contextual,
    AfterInvertedName,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NameAsSortOrder {
    First,
    All,
}

/// Configuration for et-al abbreviation in names.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EtAlOptions {
    /// Minimum number of names to trigger abbreviation.
    pub min: u8,
    /// Number of names to show when triggered.
    pub use_first: u8,
    /// Optional separate configuration for subsequent citations (CSL 1.0 legacy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent: Option<Box<EtAlSubsequent>>,
    /// The term to use (e.g., "et al.", "and others").
    pub term: String,
    /// Formatting for the term (italic, bold).
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EtAlSubsequent {
    pub min: u8,
    pub use_first: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DateOptions {
    pub form: Option<DateForm>,
    pub parts: Option<DateParts>,
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_form: Option<DatePartForm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    Text,
    Numeric,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DateParts {
    Year,
    YearMonth,
    YearMonthDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DatePartForm {
    Numeric,
    NumericLeadingZeros,
    Ordinal,
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FormattingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<TextDecoration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecoration {
    None,
    Underline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VerticalAlign {
    Baseline,
    Superscript,
    Subscript,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_minimal_deserialization() {
        let yaml = r#"
info:
  title: Test Style
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(style.info.title.as_ref().unwrap(), "Test Style");
    }

    #[test]
    fn test_style_with_citation() {
        let yaml = r#"
info:
  title: Test
citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        let citation = style.citation.unwrap();
        assert_eq!(citation.resolve_template().unwrap().len(), 2);
    }

    #[test]
    fn test_style_with_options() {
        let yaml = r#"
info:
  title: APA
options:
  processing: author-date
  contributors:
    display-as-sort: first
    and: symbol
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        let options = style.options.unwrap();
        assert_eq!(options.processing, Some(options::Processing::AuthorDate));
    }

    #[test]
    fn test_csln_first_yaml() {
        // Test parsing the actual csln-first.yaml file structure
        let yaml = r#"
info:
  title: APA
options:
  substitute:
    contributor-role-form: short
    template:
      - editor
      - title
  processing: author-date
  contributors:
    display-as-sort: first
    and: symbol
citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
    - title: parent-monograph
      prefix: "In "
      emph: true
    - number: volume
    - variable: doi
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();

        // Verify info
        assert_eq!(style.info.title.as_ref().unwrap(), "APA");

        // Verify options
        let options = style.options.unwrap();
        assert_eq!(options.processing, Some(options::Processing::AuthorDate));
        assert!(options.substitute.is_some());

        // Verify citation
        let citation = style.citation.unwrap();
        let citation_template = citation.resolve_template().unwrap();
        assert_eq!(citation_template.len(), 2);

        // Verify bibliography
        let bib = style.bibliography.unwrap();
        let bib_template = bib.resolve_template().unwrap();
        assert_eq!(bib_template.len(), 6);

        // Verify flattened rendering worked
        match &bib_template[1] {
            template::TemplateComponent::Date(d) => {
                assert_eq!(
                    d.rendering.wrap,
                    Some(template::WrapPunctuation::Parentheses)
                );
            }
            _ => panic!("Expected Date"),
        }

        match &bib_template[3] {
            template::TemplateComponent::Title(t) => {
                assert_eq!(t.rendering.prefix, Some("In ".to_string()));
                assert_eq!(t.rendering.emph, Some(true));
            }
            _ => panic!("Expected Title"),
        }
    }

    #[test]
    fn test_style_forward_compatibility() {
        let yaml = r#"
version: "1.1"
info:
  title: Future Style
future-option: true
citation:
  template:
    - contributor: author
      form: long
      future-modifier: bold
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(style.version, "1.1");
        assert_eq!(
            style._extra.get("future-option").unwrap(),
            &serde_json::Value::Bool(true)
        );

        let citation = style.citation.as_ref().unwrap();
        let citation_template = citation.resolve_template().unwrap();
        match &citation_template[0] {
            template::TemplateComponent::Contributor(c) => {
                assert_eq!(
                    c._extra.get("future-modifier").unwrap(),
                    &serde_json::Value::String("bold".to_string())
                );
            }
            _ => panic!("Expected Contributor"),
        }

        // Round-trip test
        let round_tripped = serde_yaml::to_string(&style).unwrap();
        assert!(
            round_tripped.contains("version: 1.1")
                || round_tripped.contains("version: \"1.1\"")
                || round_tripped.contains("version: '1.1'")
        );
        assert!(round_tripped.contains("future-option: true"));
        assert!(round_tripped.contains("future-modifier: bold"));
    }

    #[test]
    fn test_style_with_preset() {
        let yaml = r#"
info:
  title: Preset Test
citation:
  use-preset: apa
bibliography:
  use-preset: vancouver
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();

        // Test Citation Preset (APA)
        let citation = style.citation.unwrap();
        assert!(citation.use_preset.is_some());
        assert!(citation.template.is_none());

        let citation_template = citation.resolve_template().unwrap();
        assert_eq!(citation_template.len(), 2); // APA citation is (Author, Year)

        // precise check for APA structure
        match &citation_template[0] {
            template::TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, template::ContributorRole::Author)
            }
            _ => panic!("Expected Contributor"),
        }

        // Test Bibliography Preset (Vancouver)
        let bib = style.bibliography.unwrap();
        let bib_template = bib.resolve_template().unwrap();
        // Vancouver bib has roughly 8 components
        assert!(bib_template.len() >= 5);

        // Verify first component is citation number
        match &bib_template[0] {
            template::TemplateComponent::Number(n) => {
                assert_eq!(n.number, template::NumberVariable::CitationNumber)
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_preset_override_precedence() {
        let yaml = r#"
info:
  title: Override Test
citation:
  use-preset: apa
  template:
    - variable: doi
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        let citation = style.citation.unwrap();

        // Should have both
        assert!(citation.use_preset.is_some());
        assert!(citation.template.is_some());

        // Template should win
        let resolved = citation.resolve_template().unwrap();
        assert_eq!(resolved.len(), 1);
        match &resolved[0] {
            template::TemplateComponent::Variable(v) => {
                assert_eq!(v.variable, template::SimpleVariable::Doi)
            }
            _ => panic!("Expected Variable"),
        }
    }
}
