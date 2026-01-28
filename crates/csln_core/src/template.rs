/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Template components for CSLN styles.
//!
//! This module defines the declarative template language for CSLN.
//! Unlike CSL 1.0's procedural rendering elements, these components
//! are simple, typed instructions that the processor interprets.

use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "kebab-case", default)]
pub struct Rendering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<WrapPunctuation>,
}

/// Punctuation to wrap a component in.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WrapPunctuation {
    Parentheses,
    Brackets,
    #[default]
    None,
}

/// A template component - the building blocks of citation/bibliography templates.
///
/// Each variant handles a specific data type with appropriate formatting options.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
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
}

/// A contributor component for rendering names.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateContributor {
    pub contributor: ContributorRole,
    pub form: ContributorForm,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// How to render contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContributorForm {
    #[default]
    Long,
    Short,
    Verb,
    VerbShort,
}

/// Contributor roles.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ContributorRole {
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

/// A date component for rendering dates.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateDate {
    pub date: DateVariable,
    pub form: DateForm,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Date variables.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DateVariable {
    Issued,
    Accessed,
    OriginalPublished,
    Submitted,
    EventDate,
}

/// Date rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    #[default]
    Year,
    YearMonth,
    Full,
    MonthDay,
}

/// A title component.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateTitle {
    pub title: TitleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TitleForm>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Types of titles.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TitleType {
    /// The primary title of the cited work.
    Primary,
    /// Title of a book/monograph containing the cited work.
    ParentMonograph,
    /// Title of a periodical/serial containing the cited work.
    ParentSerial,
}

/// Title rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TitleForm {
    Short,
    #[default]
    Long,
}

/// A number component (volume, issue, pages, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateNumber {
    pub number: NumberVariable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<NumberForm>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Number variables.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum NumberVariable {
    Volume,
    Issue,
    Pages,
    Edition,
    ChapterNumber,
    CollectionNumber,
    NumberOfPages,
    NumberOfVolumes,
}

/// Number rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NumberForm {
    #[default]
    Numeric,
    Ordinal,
    Roman,
}

/// A simple variable component (DOI, ISBN, URL, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateVariable {
    pub variable: SimpleVariable,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Simple string variables.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SimpleVariable {
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
}

/// A list component for grouping multiple items with a delimiter.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateList {
    pub items: Vec<TemplateComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Delimiter punctuation options.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
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
}
