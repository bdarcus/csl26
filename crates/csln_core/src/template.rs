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
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case", default)]
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
    /// Text to prepend to the rendered value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Text to append to the rendered value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Punctuation to wrap the value in (e.g., parentheses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<WrapPunctuation>,
    /// If true, suppress this component entirely (render as empty string).
    /// Useful for type-specific overrides like suppressing publisher for journals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
}

impl Rendering {
    /// Merge another rendering into this one, with the other taking precedence.
    pub fn merge(&mut self, other: &Rendering) {
        if other.emph.is_some() {
            self.emph = other.emph;
        }
        if other.quote.is_some() {
            self.quote = other.quote;
        }
        if other.strong.is_some() {
            self.strong = other.strong;
        }
        if other.small_caps.is_some() {
            self.small_caps = other.small_caps;
        }
        if other.prefix.is_some() {
            self.prefix = other.prefix.clone();
        }
        if other.suffix.is_some() {
            self.suffix = other.suffix.clone();
        }
        if other.wrap.is_some() {
            self.wrap = other.wrap.clone();
        }
        if other.suppress.is_some() {
            self.suppress = other.suppress;
        }
    }
}

/// Punctuation to wrap a component in.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WrapPunctuation {
    Parentheses,
    Brackets,
    Quotes,
    #[default]
    None,
}

/// A template component - the building blocks of citation/bibliography templates.
///
/// Each variant handles a specific data type with appropriate formatting options.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum TemplateComponent {
    Contributor(TemplateContributor),
    Date(TemplateDate),
    Title(TemplateTitle),
    Number(TemplateNumber),
    Variable(TemplateVariable),
    List(TemplateList),
}

impl Default for TemplateComponent {
    fn default() -> Self {
        TemplateComponent::Variable(TemplateVariable::default())
    }
}

impl TemplateComponent {
    /// Get the rendering options for this component.
    pub fn rendering(&self) -> &Rendering {
        match self {
            TemplateComponent::Contributor(c) => &c.rendering,
            TemplateComponent::Date(d) => &d.rendering,
            TemplateComponent::Title(t) => &t.rendering,
            TemplateComponent::Number(n) => &n.rendering,
            TemplateComponent::Variable(v) => &v.rendering,
            TemplateComponent::List(l) => &l.rendering,
        }
    }

    /// Get the type-specific rendering overrides for this component.
    pub fn overrides(&self) -> Option<&HashMap<String, Rendering>> {
        match self {
            TemplateComponent::Contributor(c) => c.overrides.as_ref(),
            TemplateComponent::Date(d) => d.overrides.as_ref(),
            TemplateComponent::Title(t) => t.overrides.as_ref(),
            TemplateComponent::Number(n) => n.overrides.as_ref(),
            TemplateComponent::Variable(v) => v.overrides.as_ref(),
            TemplateComponent::List(l) => l.overrides.as_ref(),
        }
    }
}

/// A contributor component for rendering names.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateContributor {
    /// Which contributor role to render (author, editor, etc.).
    pub contributor: ContributorRole,
    /// How to display the contributor (long names, short, with label, etc.).
    pub form: ContributorForm,
    /// Override the global name order for this specific component.
    /// Use to show editors as "Given Family" even when global setting is "Family, Given".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<NameOrder>,
    /// Custom delimiter between names (overrides global setting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Override the conjunction between the last two names.
    /// Use `none` for bibliography when citation uses `text` or `symbol`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<crate::options::AndOptions>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Name display order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NameOrder {
    /// Display as "Given Family" (e.g., "John Smith").
    GivenFirst,
    /// Display as "Family, Given" (e.g., "Smith, John").
    #[default]
    FamilyFirst,
}

/// How to render contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContributorForm {
    #[default]
    Long,
    Short,
    Verb,
    VerbShort,
}

/// Contributor roles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ContributorRole {
    #[default]
    Author,
    Editor,
    Translator,
    Director,
    Publisher,
    Recipient,
    Interviewer,
    Interviewee,
    Inventor,
    Counsel,
    Composer,
    CollectionEditor,
    ContainerAuthor,
    EditorialDirector,
    Illustrator,
    OriginalAuthor,
    ReviewedAuthor,
}

impl ContributorRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContributorRole::Author => "author",
            ContributorRole::Editor => "editor",
            ContributorRole::Translator => "translator",
            ContributorRole::Director => "director",
            ContributorRole::Publisher => "publisher",
            ContributorRole::Recipient => "recipient",
            ContributorRole::Interviewer => "interviewer",
            ContributorRole::Interviewee => "interviewee",
            ContributorRole::Inventor => "inventor",
            ContributorRole::Counsel => "counsel",
            ContributorRole::Composer => "composer",
            ContributorRole::CollectionEditor => "collection-editor",
            ContributorRole::ContainerAuthor => "container-author",
            ContributorRole::EditorialDirector => "editorial-director",
            ContributorRole::Illustrator => "illustrator",
            ContributorRole::OriginalAuthor => "original-author",
            ContributorRole::ReviewedAuthor => "reviewed-author",
        }
    }
}

/// A date component for rendering dates.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateDate {
    pub date: DateVariable,
    pub form: DateForm,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Date variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
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
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    #[default]
    Year,
    YearMonth,
    Full,
    MonthDay,
}

/// A title component.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
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
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Types of titles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
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
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TitleForm {
    Short,
    #[default]
    Long,
}

/// A number component (volume, issue, pages, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateNumber {
    pub number: NumberVariable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<NumberForm>,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Number variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
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
}

/// Number rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NumberForm {
    #[default]
    Numeric,
    Ordinal,
    Roman,
}

/// A simple variable component (DOI, ISBN, URL, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateVariable {
    pub variable: SimpleVariable,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Type-specific rendering overrides. Use `suppress: true` to hide for certain types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Simple string variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
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
}

/// A list component for grouping multiple items with a delimiter.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateList {
    pub items: Vec<TemplateComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Type-specific rendering overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, Rendering>>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Delimiter punctuation options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, JsonSchema)]
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
}

impl DelimiterPunctuation {
    /// Convert to string with trailing space (for most delimiters).
    /// Returns the punctuation followed by a space, except for Space and None.
    pub fn to_string_with_space(&self) -> &'static str {
        match self {
            Self::Comma => ", ",
            Self::Semicolon => "; ",
            Self::Period => ". ",
            Self::Colon => ": ",
            Self::Ampersand => " & ",
            Self::VerticalLine => " | ",
            Self::Slash => "/",
            Self::Hyphen => "-",
            Self::Space => " ",
            Self::None => "",
        }
    }

    /// Parse from a CSL delimiter string.
    /// Handles common patterns like ", ", ": ", etc.
    pub fn from_csl_string(s: &str) -> Self {
        let trimmed = s.trim();
        match trimmed {
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
            _ => Self::Comma, // Default fallback
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
}
