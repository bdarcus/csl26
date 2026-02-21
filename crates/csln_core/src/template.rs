/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Template components for CSLN styles.
//!
//! This module defines the declarative template language for CSLN.
//! Unlike CSL 1.0's procedural rendering elements, these components
//! are simple, typed instructions that the processor interprets.
//!
//! ## Design Philosophy
//!
//! **Explicit over magic**: All rendering behavior should be expressible in the
//! style YAML. The processor should not have hidden conditional logic based on
//! reference types. Instead, use `overrides` to declare type-specific behavior.
//!
//! ## Type-Specific Overrides
//!
//! Components support `overrides` to customize rendering per reference type:
//!
//! ```yaml
//! - variable: publisher
//!   overrides:
//!     article-journal:
//!       suppress: true  # Don't show publisher for journals
//! - number: pages
//!   overrides:
//!     chapter:
//!       wrap: parentheses
//!       prefix: "pp. "  # Show as "(pp. 1-10)" for chapters
//! ```
//!
//! This keeps all conditional logic in the style, making it testable and portable.

use crate::locale::{GeneralTerm, TermForm};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rendering instructions applied to template components.
///
/// These fields are flattened into parent structs, so in YAML you write:
/// ```yaml
/// - title: primary
///   emph: true
///   prefix: "In "
/// ```
/// Rather than nesting under a `rendering:` key.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default, deny_unknown_fields)]
pub struct Rendering {
    /// Render in italics/emphasis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    /// Render in quotes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<bool>,
    /// Render in bold/strong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    /// Render in small caps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    /// Text to prepend to the rendered value (outside any wrap).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Text to append to the rendered value (outside any wrap).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Text to prepend inside the wrap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_prefix: Option<String>,
    /// Text to append inside the wrap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_suffix: Option<String>,
    /// Punctuation to wrap the value in (e.g., parentheses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<WrapPunctuation>,
    /// If true, suppress this component entirely (render as empty string).
    /// Useful for type-specific overrides like suppressing publisher for journals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
    /// Override name initialization (e.g., ". " or "").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    /// Strip trailing periods from rendered value.
    #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
    pub strip_periods: Option<bool>,
}

impl Rendering {
    /// Merge another rendering into this one, with the other taking precedence.
    pub fn merge(&mut self, other: &Rendering) {
        crate::merge_options!(
            self,
            other,
            emph,
            quote,
            strong,
            small_caps,
            prefix,
            suffix,
            inner_prefix,
            inner_suffix,
            wrap,
            suppress,
            initialize_with,
            strip_periods,
        );
    }
}

/// Punctuation to wrap a component in.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum WrapPunctuation {
    Parentheses,
    Brackets,
    Quotes,
    #[default]
    None,
}

/// Type-specific rendering overrides for components.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum ComponentOverride {
    /// A full component replacement for specific reference types.
    Component(Box<TemplateComponent>),
    /// Simple rendering options override.
    Rendering(Rendering),
}

/// Selector for reference types in overrides.
/// Can be a single type string or a list of types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeSelector {
    Single(String),
    Multiple(Vec<String>),
}

impl TypeSelector {
    /// Check if this selector matches a reference type.
    pub fn matches(&self, ref_type: &str) -> bool {
        match self {
            TypeSelector::Single(s) => {
                s == ref_type || s == "all" || (s == "default" && ref_type == "default")
            }
            TypeSelector::Multiple(types) => types
                .iter()
                .any(|t| t == ref_type || t == "all" || (t == "default" && ref_type == "default")),
        }
    }
}

/// A template component - the building blocks of citation/bibliography templates.
///
/// Each variant handles a specific data type with appropriate formatting options.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum TemplateComponent {
    Contributor(TemplateContributor),
    Date(TemplateDate),
    Title(TemplateTitle),
    Number(TemplateNumber),
    Variable(TemplateVariable),
    List(TemplateList),
    Term(TemplateTerm),
}

impl Default for TemplateComponent {
    fn default() -> Self {
        TemplateComponent::Variable(TemplateVariable::default())
    }
}

impl TemplateComponent {
    /// Get the rendering options for this component.
    pub fn rendering(&self) -> &Rendering {
        crate::dispatch_component!(self, |inner| &inner.rendering)
    }

    /// Get the type-specific rendering overrides for this component.
    pub fn overrides(&self) -> Option<&HashMap<TypeSelector, ComponentOverride>> {
        crate::dispatch_component!(self, |inner| inner.overrides.as_ref())
    }
}

/// Configuration for role labels (e.g., "eds.", "trans.").
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RoleLabel {
    /// Locale term key for the role (e.g., "editor", "translator").
    pub term: String,
    /// Term form: short ("eds.") or long ("editors").
    #[serde(default)]
    pub form: RoleLabelForm,
    /// Where to place the label relative to names.
    #[serde(default)]
    pub placement: LabelPlacement,
}

/// Term form for role labels.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RoleLabelForm {
    #[default]
    Short,
    Long,
}

/// Label placement relative to contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelPlacement {
    Prefix,
    #[default]
    Suffix,
}

/// A contributor component for rendering names.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateContributor {
    /// Which contributor role to render (author, editor, etc.).
    pub contributor: ContributorRole,
    /// How to display the contributor (long names, short, with label, etc.).
    pub form: ContributorForm,
    /// Optional role label configuration (e.g., "eds." for editors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<RoleLabel>,
    /// Override the global name order for this specific component.
    /// Use to show editors as "Given Family" even when global setting is "Family, Given".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<NameOrder>,
    /// Custom delimiter between names (overrides global setting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Delimiter between family and given name when inverted (overrides global setting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    /// Shorten the list of names (et al. configuration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorten: Option<crate::options::ShortenListOptions>,
    /// Override the conjunction between the last two names.
    /// Use `none` for bibliography when citation uses `text` or `symbol`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<crate::options::AndOptions>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Name display order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NameOrder {
    /// Display as "Given Family" (e.g., "John Smith").
    GivenFirst,
    /// Display as "Family, Given" (e.g., "Smith, John").
    #[default]
    FamilyFirst,
}

/// How to render contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ContributorForm {
    #[default]
    Long,
    Short,
    Verb,
    VerbShort,
}

crate::str_enum! {
    /// Contributor roles.
    #[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "schema", derive(JsonSchema))]
    #[serde(rename_all = "kebab-case")]
    pub enum ContributorRole {
        #[default] Author = "author",
        Editor = "editor",
        Translator = "translator",
        Director = "director",
        Publisher = "publisher",
        Recipient = "recipient",
        Interviewer = "interviewer",
        Interviewee = "interviewee",
        Inventor = "inventor",
        Counsel = "counsel",
        Composer = "composer",
        CollectionEditor = "collection-editor",
        ContainerAuthor = "container-author",
        EditorialDirector = "editorial-director",
        Illustrator = "illustrator",
        OriginalAuthor = "original-author",
        ReviewedAuthor = "reviewed-author"
    }
}

/// A date component for rendering dates.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateDate {
    pub date: DateVariable,
    pub form: DateForm,
    /// Fallback components if the primary date is missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Vec<TemplateComponent>>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Date variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateVariable {
    #[default]
    Issued,
    Accessed,
    OriginalPublished,
    Submitted,
    EventDate,
}

/// Date rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    #[default]
    Year,
    YearMonth,
    Full,
    MonthDay,
    YearMonthDay,
}

/// A title component.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateTitle {
    pub title: TitleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TitleForm>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Types of titles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TitleType {
    /// The primary title of the cited work.
    #[default]
    Primary,
    /// Title of a book/monograph containing the cited work.
    ParentMonograph,
    /// Title of a periodical/serial containing the cited work.
    ParentSerial,
}

/// Title rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TitleForm {
    Short,
    #[default]
    Long,
}

/// A number component (volume, issue, pages, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateNumber {
    pub number: NumberVariable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<NumberForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_form: Option<LabelForm>,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Number variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NumberVariable {
    #[default]
    Volume,
    Issue,
    Pages,
    Edition,
    ChapterNumber,
    CollectionNumber,
    NumberOfPages,
    NumberOfVolumes,
    CitationNumber,
    CitationLabel,
    Number,
    DocketNumber,
    PatentNumber,
    StandardNumber,
    ReportNumber,
}

/// Number rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum NumberForm {
    #[default]
    Numeric,
    Ordinal,
    Roman,
}

/// Label rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    Long,
    #[default]
    Short,
    Symbol,
}

/// A simple variable component (DOI, ISBN, URL, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateVariable {
    pub variable: SimpleVariable,
    /// Whether locator labels (e.g., "p.", "sec.") should be rendered when
    /// `variable: locator` is used. If omitted, processor defaults apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_label: Option<bool>,
    /// Strip trailing periods from locator labels (e.g., "p." -> "p").
    /// Only applies to `variable: locator`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_label_periods: Option<bool>,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides. Use `suppress: true` to hide for certain types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Simple string variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SimpleVariable {
    #[default]
    Doi,
    Isbn,
    Issn,
    Url,
    Pmid,
    Pmcid,
    Abstract,
    Note,
    Annote,
    Keyword,
    Genre,
    Medium,
    Source,
    Status,
    Archive,
    ArchiveLocation,
    Publisher,
    PublisherPlace,
    EventPlace,
    Dimensions,
    Scale,
    Version,
    Locator,
    Authority,
    Reporter,
    Page,
    Volume,
    Number,
    DocketNumber,
    PatentNumber,
    StandardNumber,
    ReportNumber,
}

/// A term component for rendering locale-specific text.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateTerm {
    /// Which term to render.
    pub term: GeneralTerm,
    /// Form: long (default), short, or symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// A list component for grouping multiple items with a delimiter.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateList {
    pub items: Vec<TemplateComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<TypeSelector, ComponentOverride>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Delimiter punctuation options.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPunctuation {
    #[default]
    Comma,
    Semicolon,
    Period,
    Colon,
    Ampersand,
    VerticalLine,
    Slash,
    Hyphen,
    Space,
    None,
    /// Custom delimiter string (e.g., ": ").
    #[serde(untagged)]
    Custom(String),
}

#[cfg(feature = "schema")]
impl JsonSchema for DelimiterPunctuation {
    fn schema_name() -> String {
        "DelimiterPunctuation".into()
    }

    fn json_schema(_gen: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{InstanceType, SchemaObject};
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        }
        .into()
    }
}

impl DelimiterPunctuation {
    /// Convert to string with trailing space (for most delimiters).
    /// Returns the punctuation followed by a space, except for Space and None.
    pub fn to_string_with_space(&self) -> String {
        match self {
            Self::Comma => ", ".to_string(),
            Self::Semicolon => "; ".to_string(),
            Self::Period => ". ".to_string(),
            Self::Colon => ": ".to_string(),
            Self::Ampersand => " & ".to_string(),
            Self::VerticalLine => " | ".to_string(),
            Self::Slash => "/".to_string(),
            Self::Hyphen => "-".to_string(),
            Self::Space => " ".to_string(),
            Self::None => "".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Parse from a CSL delimiter string.
    /// Handles common patterns like ", ", ": ", etc.
    /// Returns Custom variant for unrecognized delimiters.
    pub fn from_csl_string(s: &str) -> Self {
        match s {
            "," | ", " => Self::Comma,
            ";" | "; " => Self::Semicolon,
            "." | ". " => Self::Period,
            ":" | ": " => Self::Colon,
            "&" | " & " => Self::Ampersand,
            "|" | " | " => Self::VerticalLine,
            "/" => Self::Slash,
            "-" => Self::Hyphen,
            " " => Self::Space,
            "" => Self::None,
            "none" => Self::None,
            _ => {
                let trimmed = s.trim();
                match trimmed {
                    "," => Self::Comma,
                    ";" => Self::Semicolon,
                    "." => Self::Period,
                    ":" => Self::Colon,
                    "&" => Self::Ampersand,
                    "|" => Self::VerticalLine,
                    "/" => Self::Slash,
                    "-" => Self::Hyphen,
                    "" => Self::None,
                    _ => Self::Custom(s.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_deserialization() {
        let yaml = r#"
contributor: author
form: long
"#;
        let comp: TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comp.contributor, ContributorRole::Author);
        assert_eq!(comp.form, ContributorForm::Long);
    }

    #[test]
    fn test_template_component_untagged() {
        let yaml = r#"
- contributor: author
  form: short
- date: issued
  form: year
- title: primary
"#;
        let components: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(components.len(), 3);

        match &components[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, ContributorRole::Author);
            }
            _ => panic!("Expected Contributor"),
        }

        match &components[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.date, DateVariable::Issued);
            }
            _ => panic!("Expected Date"),
        }
    }

    #[test]
    fn test_flattened_rendering() {
        // Test that rendering options can be specified directly on the component
        let yaml = r#"
- title: parent-monograph
  prefix: "In "
  emph: true
- date: issued
  form: year
  wrap: parentheses
"#;
        let components: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(components.len(), 2);

        match &components[0] {
            TemplateComponent::Title(t) => {
                assert_eq!(t.rendering.prefix, Some("In ".to_string()));
                assert_eq!(t.rendering.emph, Some(true));
            }
            _ => panic!("Expected Title"),
        }

        match &components[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.rendering.wrap, Some(WrapPunctuation::Parentheses));
            }
            _ => panic!("Expected Date"),
        }
    }

    #[test]
    fn test_contributor_with_wrap() {
        let yaml = r#"
contributor: publisher
form: short
wrap: parentheses
"#;
        let comp: TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comp.contributor, ContributorRole::Publisher);
        assert_eq!(comp.rendering.wrap, Some(WrapPunctuation::Parentheses));
    }

    #[test]
    fn test_variable_deserialization() {
        // Test that `variable: publisher` parses as Variable, not Number
        let yaml = "variable: publisher\n";
        let comp: TemplateComponent = serde_yaml::from_str(yaml).unwrap();
        match comp {
            TemplateComponent::Variable(v) => {
                assert_eq!(v.variable, SimpleVariable::Publisher);
            }
            _ => panic!("Expected Variable(Publisher), got {:?}", comp),
        }
    }

    #[test]
    fn test_variable_array_parsing() {
        let yaml = r#"
- variable: doi
  prefix: "https://doi.org/"
- variable: publisher
"#;
        let comps: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comps.len(), 2);
        match &comps[0] {
            TemplateComponent::Variable(v) => assert_eq!(v.variable, SimpleVariable::Doi),
            _ => panic!("Expected Variable for doi, got {:?}", comps[0]),
        }
        match &comps[1] {
            TemplateComponent::Variable(v) => assert_eq!(v.variable, SimpleVariable::Publisher),
            _ => panic!("Expected Variable for publisher, got {:?}", comps[1]),
        }
    }

    #[test]
    fn test_type_selector_default_only_matches_default_context() {
        let selector = TypeSelector::Single("default".to_string());
        assert!(selector.matches("default"));
        assert!(!selector.matches("article-journal"));

        let mixed = TypeSelector::Multiple(vec!["default".to_string(), "chapter".to_string()]);
        assert!(mixed.matches("default"));
        assert!(mixed.matches("chapter"));
        assert!(!mixed.matches("book"));
    }
}
