use crate::reference::types::MultilingualString;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A contributor can be a single string, a structured name, or a list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum Contributor {
    SimpleName(SimpleName),
    StructuredName(StructuredName),
    Multilingual(MultilingualName),
    ContributorList(ContributorList),
}

/// Holistic multilingual name representation.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualName {
    /// The name in its original script.
    pub original: StructuredName,
    /// ISO 639/BCP 47 language code for the original name.
    pub lang: Option<crate::reference::types::LangID>,
    /// Transliterations/Transcriptions of the name.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub transliterations: std::collections::HashMap<String, StructuredName>,
    /// Translations of the name.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub translations: std::collections::HashMap<crate::reference::types::LangID, StructuredName>,
}

/// A simple name is just a string, with an optional location.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SimpleName {
    pub name: MultilingualString,
    pub location: Option<String>,
}

/// A structured name is a name broken down into its constituent parts.
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct StructuredName {
    pub given: MultilingualString,
    pub family: MultilingualString,
    pub suffix: Option<String>,
    pub dropping_particle: Option<String>,
    pub non_dropping_particle: Option<String>,
}

/// A list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ContributorList(pub Vec<Contributor>);

impl Contributor {
    pub fn to_names_vec(&self) -> Vec<FlatName> {
        match self {
            Contributor::SimpleName(n) => vec![FlatName {
                literal: Some(n.name.to_string()),
                ..Default::default()
            }],
            Contributor::StructuredName(n) => vec![FlatName {
                given: Some(n.given.to_string()),
                family: Some(n.family.to_string()),
                suffix: n.suffix.clone(),
                dropping_particle: n.dropping_particle.clone(),
                non_dropping_particle: n.non_dropping_particle.clone(),
                ..Default::default()
            }],
            Contributor::Multilingual(m) => vec![FlatName {
                given: Some(m.original.given.to_string()),
                family: Some(m.original.family.to_string()),
                suffix: m.original.suffix.clone(),
                dropping_particle: m.original.dropping_particle.clone(),
                non_dropping_particle: m.original.non_dropping_particle.clone(),
                ..Default::default()
            }],
            Contributor::ContributorList(l) => l.0.iter().flat_map(|c| c.to_names_vec()).collect(),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            Contributor::SimpleName(n) => Some(n.name.to_string()),
            Contributor::Multilingual(m) => {
                Some(format!("{} {}", m.original.given, m.original.family))
            }
            _ => None,
        }
    }

    pub fn location(&self) -> Option<String> {
        match self {
            Contributor::SimpleName(n) => n.location.clone(),
            _ => None,
        }
    }
}

/// A flattened name for internal processing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlatName {
    pub family: Option<String>,
    pub given: Option<String>,
    pub suffix: Option<String>,
    pub dropping_particle: Option<String>,
    pub non_dropping_particle: Option<String>,
    pub literal: Option<String>,
}

impl FlatName {
    pub fn family_or_literal(&self) -> &str {
        if let Some(ref f) = self.family {
            f
        } else if let Some(ref l) = self.literal {
            l
        } else {
            ""
        }
    }
}

impl fmt::Display for Contributor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Contributor::SimpleName(n) => write!(f, "{}", n.name),
            Contributor::StructuredName(n) => write!(f, "{} {}", n.given, n.family),
            Contributor::Multilingual(m) => write!(f, "{} {}", m.original.given, m.original.family),
            Contributor::ContributorList(l) => write!(f, "{}", l),
        }
    }
}

impl fmt::Display for ContributorList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self.0.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", names.join(", "))
    }
}
