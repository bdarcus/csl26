use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A contributor can be a single string, a structured name, or a list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum Contributor {
    SimpleName(SimpleName),
    StructuredName(StructuredName),
    ContributorList(ContributorList),
}

/// A simple name is just a string, with an optional location.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
pub struct SimpleName {
    pub name: String,
    pub location: Option<String>,
}

/// A structured name is a name broken down into its constituent parts.
#[derive(Debug, Deserialize, Serialize, Clone, Default, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct StructuredName {
    pub given: String,
    pub family: String,
    pub suffix: Option<String>,
    pub dropping_particle: Option<String>,
    pub non_dropping_particle: Option<String>,
}

/// A list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, PartialEq)]
pub struct ContributorList(pub Vec<Contributor>);

impl Contributor {
    pub fn to_names_vec(&self) -> Vec<FlatName> {
        match self {
            Contributor::SimpleName(n) => vec![FlatName {
                literal: Some(n.name.clone()),
                ..Default::default()
            }],
            Contributor::StructuredName(n) => vec![FlatName {
                given: Some(n.given.clone()),
                family: Some(n.family.clone()),
                suffix: n.suffix.clone(),
                dropping_particle: n.dropping_particle.clone(),
                non_dropping_particle: n.non_dropping_particle.clone(),
                ..Default::default()
            }],
            Contributor::ContributorList(l) => l.0.iter().flat_map(|c| c.to_names_vec()).collect(),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            Contributor::SimpleName(n) => Some(n.name.clone()),
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
