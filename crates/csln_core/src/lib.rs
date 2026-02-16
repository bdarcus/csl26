use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod renderer; // Expose the renderer
pub use renderer::Renderer;

// New CSLN schema modules
pub mod citation;
pub mod grouping;
pub mod legacy;
pub mod locale;
pub mod options;
pub mod presets;
pub mod reference;
pub mod template;

// Embedded templates for priority styles (APA, Chicago, Vancouver, IEEE, Harvard)
pub mod embedded;

pub use citation::{Citation, CitationItem, CitationMode, Citations, ItemVisibility, LocatorType};
pub use grouping::{
    BibliographyGroup, CitedStatus, FieldMatcher, GroupSelector, GroupSort, GroupSortKey,
    NameSortOrder, SortKey, TypeSelector,
};
pub use legacy::{
    AndTerm, ConditionBlock, CslnInfo, CslnLocale, CslnNode, CslnStyle, DateBlock, DateForm,
    DateOptions, DatePartForm, DateParts, DelimiterPrecedes, ElseIfBranch, EtAlOptions,
    EtAlSubsequent, FontStyle, FontVariant, FontWeight, FormattingOptions, GroupBlock, ItemType,
    LabelForm, LabelOptions, NameAsSortOrder, NameMode, NamesBlock, NamesOptions, TermBlock,
    TextDecoration, Variable, VariableBlock, VerticalAlign,
};
pub use locale::Locale;
pub use options::Config;
pub use presets::{ContributorPreset, DatePreset, SubstitutePreset, TitlePreset};
pub use template::TemplateComponent;

/// A collection of bibliographic references with optional metadata.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct InputBibliography {
    /// Bibliography metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<InputBibliographyInfo>,
    /// The list of references.
    pub references: Vec<reference::InputReference>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Metadata for an input bibliography.
#[derive(Debug, Default, Deserialize, Serialize, JsonSchema, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct InputBibliographyInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

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
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
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
    /// Configuration for integral (narrative) citations (e.g., "Smith (2020)").
    /// Overrides fields from the main citation spec when mode is Integral.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integral: Option<Box<CitationSpec>>,
    /// Configuration for non-integral (parenthetical) citations (e.g., "(Smith, 2020)").
    /// Overrides fields from the main citation spec when mode is NonIntegral.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_integral: Option<Box<CitationSpec>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
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

    /// Resolve the effective spec for a given citation mode.
    ///
    /// If a mode-specific spec exists (e.g., `integral`), it merges with and overrides
    /// the base spec.
    pub fn resolve_for_mode(
        &self,
        mode: &crate::citation::CitationMode,
    ) -> std::borrow::Cow<'_, CitationSpec> {
        use crate::citation::CitationMode;
        let mode_spec = match mode {
            CitationMode::Integral => self.integral.as_ref(),
            CitationMode::NonIntegral => self.non_integral.as_ref(),
        };

        match mode_spec {
            Some(spec) => {
                // Merge logic: mode specific > base
                let mut merged = self.clone();
                // We don't want to recurse infinitely or keep the mode specs in the merged result
                merged.integral = None;
                merged.non_integral = None;

                if spec.options.is_some() {
                    merged.options = spec.options.clone();
                }
                if spec.use_preset.is_some() {
                    merged.use_preset = spec.use_preset.clone();
                }
                if spec.template.is_some() {
                    merged.template = spec.template.clone();
                }
                if spec.wrap.is_some() {
                    merged.wrap = spec.wrap.clone();
                }
                if spec.prefix.is_some() {
                    merged.prefix = spec.prefix.clone();
                }
                if spec.suffix.is_some() {
                    merged.suffix = spec.suffix.clone();
                }
                if spec.delimiter.is_some() {
                    merged.delimiter = spec.delimiter.clone();
                }
                if spec.multi_cite_delimiter.is_some() {
                    merged.multi_cite_delimiter = spec.multi_cite_delimiter.clone();
                }

                std::borrow::Cow::Owned(merged)
            }
            None => std::borrow::Cow::Borrowed(self),
        }
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
    /// Optional bibliography grouping specification.
    ///
    /// When present, divides the bibliography into labeled sections with
    /// optional per-group sorting. Items match the first group whose selector
    /// evaluates to true (first-match semantics). Omit for flat bibliography.
    ///
    /// See `BibliographyGroup` for examples.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<grouping::BibliographyGroup>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
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
    fn test_style_custom_fields() {
        let yaml = r#"
version: "1.1"
info:
  title: Custom Fields Test
custom:
  my-extension: true
  author-tool: "StyleAuthor v2.0"
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(style.version, "1.1");
        let custom = style.custom.as_ref().unwrap();
        assert_eq!(
            custom.get("my-extension").unwrap(),
            &serde_json::Value::Bool(true)
        );
        assert_eq!(
            custom.get("author-tool").unwrap(),
            &serde_json::Value::String("StyleAuthor v2.0".to_string())
        );

        // Round-trip test
        let round_tripped = serde_yaml::to_string(&style).unwrap();
        assert!(
            round_tripped.contains("version: 1.1")
                || round_tripped.contains("version: \"1.1\"")
                || round_tripped.contains("version: '1.1'")
        );
        assert!(round_tripped.contains("my-extension: true"));
        assert!(round_tripped.contains("author-tool:"));
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

    #[test]
    fn test_bibliography_with_groups() {
        let yaml = r#"
info:
  title: Grouped Bibliography Test
bibliography:
  template:
    - contributor: author
      form: long
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
    - id: other
      selector:
        not:
          field:
            language: vi
"#;
        let style: Style = serde_yaml::from_str(yaml).unwrap();
        let bib = style.bibliography.unwrap();

        assert!(bib.groups.is_some());
        let groups = bib.groups.unwrap();
        assert_eq!(groups.len(), 2);

        // First group
        assert_eq!(groups[0].id, "vietnamese");
        assert_eq!(groups[0].heading.as_ref().unwrap(), "Tài liệu tiếng Việt");
        assert!(groups[0].sort.is_some());

        // Second group (fallback with negation)
        assert_eq!(groups[1].id, "other");
        assert!(groups[1].heading.is_none());
        assert!(groups[1].selector.not.is_some());
    }
}
