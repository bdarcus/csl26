/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::reference::types::{MultilingualString, Place};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "bindings")]
use specta::Type;
use std::fmt;

/// Grammatical gender carried on contributor records for role-label agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
pub enum ContributorGender {
    /// Masculine grammatical gender.
    Masculine,
    /// Feminine grammatical gender.
    Feminine,
    /// Neuter grammatical gender.
    Neuter,
    /// Common or shared grammatical gender.
    Common,
}

/// A contributor can be a single string, a structured name, or a list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
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
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualName {
    /// The name in its original script.
    pub original: StructuredName,
    /// ISO 639/BCP 47 language code for the original name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<crate::reference::types::LangID>,
    /// Hidden whole-name key used only for bibliography sorting.
    #[serde(rename = "sort-as", skip_serializing_if = "Option::is_none")]
    pub sort_as: Option<String>,
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
#[cfg_attr(feature = "bindings", derive(Type))]
pub struct SimpleName {
    /// Institutional or organization name.
    pub name: MultilingualString,
    /// Geographic place associated with the name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Place>,
    /// Short form of the name (e.g., abbreviation or shortened form).
    #[serde(rename = "short-name", skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
}

/// A structured name is a name broken down into its constituent parts.
#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
pub struct StructuredName {
    pub given: MultilingualString,
    pub family: MultilingualString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropping_particle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_dropping_particle: Option<String>,
}

/// A list of contributors.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
pub struct ContributorList(pub Vec<Contributor>);

impl Contributor {
    pub fn to_names_vec(&self) -> Vec<FlatName> {
        match self {
            Contributor::SimpleName(n) => vec![FlatName {
                literal: Some(n.name.to_string()),
                short_name: n.short_name.clone(),
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
            Contributor::SimpleName(n) => n.location.clone().map(Into::into),
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
    pub short_name: Option<String>,
    /// Original-script display form of a multilingual name (e.g. `华林甫`),
    /// carried alongside the selected transliteration so rendering can apply
    /// native ordering and append the original script after the romanized
    /// name when a name pattern requests both views.
    pub original_script: Option<String>,
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

crate::tolerant_enum! {
    /// A contributor role for use in the unified contributors list.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum ContributorRole {
        Author = "author",
        Editor = "editor",
        Translator = "translator",
        Director = "director",
        Performer = "performer",
        Composer = "composer",
        Illustrator = "illustrator",
        Narrator = "narrator",
        Host = "host",
        Guest = "guest",
        Interviewer = "interviewer",
        Recipient = "recipient",
        Compiler = "compiler",
        Producer = "producer",
        Writer = "writer",
        /// Author of annotations accompanying the work (BibLaTeX `annotator`).
        Annotator = "annotator",
        /// Author of a commentary on the work (BibLaTeX `commentator`).
        Commentator = "commentator",
        /// Author of a foreword (BibLaTeX `foreword`).
        ForewordAuthor = "foreword-author",
        /// Author of an introduction (BibLaTeX `introduction`).
        IntroductionAuthor = "introduction-author",
        /// Author of an afterword (BibLaTeX `afterword`).
        AfterwordAuthor = "afterword-author"
    }
}

/// One or more distinct roles explicitly assigned to a contributor entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(transparent)]
pub struct ContributorRoles(
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))] Vec<ContributorRole>,
);

impl ContributorRoles {
    /// Return the roles in their authored order.
    #[must_use]
    pub fn as_slice(&self) -> &[ContributorRole] {
        &self.0
    }

    /// Return whether this entry carries `role`.
    #[must_use]
    pub fn contains(&self, role: &ContributorRole) -> bool {
        self.0.contains(role)
    }

    /// Add a role when it is not already present.
    #[cfg(feature = "legacy-convert")]
    pub(crate) fn insert(&mut self, role: ContributorRole) {
        if !self.contains(&role) {
            self.0.push(role);
        }
    }
}

impl From<ContributorRole> for ContributorRoles {
    fn from(role: ContributorRole) -> Self {
        Self(vec![role])
    }
}

impl PartialEq<ContributorRole> for ContributorRoles {
    fn eq(&self, other: &ContributorRole) -> bool {
        self.as_slice() == std::slice::from_ref(other)
    }
}

impl TryFrom<Vec<ContributorRole>> for ContributorRoles {
    type Error = &'static str;

    fn try_from(roles: Vec<ContributorRole>) -> Result<Self, Self::Error> {
        if roles.is_empty() {
            return Err("contributor roles must not be empty");
        }
        let mut distinct = Vec::with_capacity(roles.len());
        for role in &roles {
            if distinct.contains(role) {
                return Err("contributor roles must be distinct");
            }
            distinct.push(role.clone());
        }
        Ok(Self(roles))
    }
}

impl<'de> Deserialize<'de> for ContributorRoles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum AuthoredRoles {
            Single(ContributorRole),
            Multiple(Vec<ContributorRole>),
        }

        let roles = match AuthoredRoles::deserialize(deserializer)? {
            AuthoredRoles::Single(role) => vec![role],
            AuthoredRoles::Multiple(roles) => roles,
        };
        Self::try_from(roles).map_err(serde::de::Error::custom)
    }
}

/// A single entry in a reference's contributors list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(rename_all = "kebab-case")]
pub struct ContributorEntry {
    /// The explicit roles this contributor plays in relation to the work.
    #[serde(rename = "roles", alias = "role")]
    pub roles: ContributorRoles,
    /// The contributor (name, organization, or list).
    pub contributor: Contributor,
    /// The grammatical gender used for role-label agreement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<ContributorGender>,
}
