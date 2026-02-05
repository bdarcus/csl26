use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod renderer; // Expose the renderer
pub use renderer::Renderer;

// New CSLN schema modules
pub mod citation;
pub mod legacy;
pub mod locale;
pub mod options;
pub mod presets;
pub mod reference;
pub mod template;

// Embedded templates for priority styles (APA, Chicago, Vancouver, IEEE, Harvard)
pub mod embedded;

pub use citation::{Citation, CitationItem, CitationMode, Citations, LocatorType};
pub use legacy::{
    AndTerm, ConditionBlock, CslnInfo, CslnLocale, CslnNode, CslnStyle, DateBlock, DateForm,
    DateOptions, DatePartForm, DateParts, DelimiterPrecedes, ElseIfBranch, EtAlOptions,
    EtAlSubsequent, FontStyle, FontVariant, FontWeight, FormattingOptions, GroupBlock, ItemType,
    LabelForm, LabelOptions, NameAsSortOrder, NameMode, NamesBlock, NamesOptions, TextDecoration,
    Variable, VariableBlock, VerticalAlign,
};
pub use locale::Locale;
pub use options::Config;
pub use presets::{ContributorPreset, DatePreset, SubstitutePreset, TitlePreset};
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
