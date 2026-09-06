/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style schema version and resource-limit constants.

#[cfg(doc)]
use crate::Style;
#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// Canonical Citum style schema version used when `Style.version` is omitted.
pub const STYLE_SCHEMA_VERSION: &str = "0.77.0";

/// Maximum accepted nesting depth for authored template groups and fallbacks.
pub const MAX_TEMPLATE_NESTING_DEPTH: usize = 64;

/// Maximum accepted authored template components in one style.
pub const MAX_TEMPLATE_COMPONENTS: usize = 16_384;

/// A schema version (major.minor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema), schemars(with = "String"))]
pub struct SchemaVersion {
    /// Major version number.
    pub major: u32,
    /// Minor version number (None if not provided in string).
    pub minor: Option<u32>,
    /// Patch version number (None if not provided in string).
    pub patch: Option<u32>,
}

impl SchemaVersion {
    /// Parse a version string into a `SchemaVersion`.
    ///
    /// Requires at least "X.Y". Supports "X.Y.Z".
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid version format
    /// or lacks the required minor version.
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(format!(
                "invalid version format (expected X.Y or X.Y.Z): \"{}\"",
                s
            ));
        }

        let major_str = parts
            .first()
            .ok_or_else(|| "missing major version".to_string())?;
        let major = major_str
            .parse::<u32>()
            .map_err(|_| format!("invalid major version: \"{}\"", major_str))?;

        let minor_str = parts
            .get(1)
            .ok_or_else(|| "missing minor version".to_string())?;
        let minor = Some(
            minor_str
                .parse::<u32>()
                .map_err(|_| format!("invalid minor version: \"{}\"", minor_str))?,
        );

        let patch = if let Some(patch_str) = parts.get(2) {
            Some(
                patch_str
                    .parse::<u32>()
                    .map_err(|_| format!("invalid patch version: \"{}\"", patch_str))?,
            )
        } else {
            None
        };

        Ok(SchemaVersion {
            major,
            minor,
            patch,
        })
    }
}

impl PartialOrd for SchemaVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchemaVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {
                let self_minor = self.minor.unwrap_or(0);
                let other_minor = other.minor.unwrap_or(0);
                match self_minor.cmp(&other_minor) {
                    std::cmp::Ordering::Equal => {
                        let self_patch = self.patch.unwrap_or(0);
                        let other_patch = other.patch.unwrap_or(0);
                        self_patch.cmp(&other_patch)
                    }
                    ord => ord,
                }
            }
            ord => ord,
        }
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(minor) = self.minor {
            write!(f, ".{}", minor)?;
            if let Some(patch) = self.patch {
                write!(f, ".{}", patch)?;
            }
        }
        Ok(())
    }
}

impl Default for SchemaVersion {
    #[allow(
        clippy::expect_used,
        reason = "STYLE_SCHEMA_VERSION is a canonical constant"
    )]
    fn default() -> Self {
        SchemaVersion::parse(STYLE_SCHEMA_VERSION).expect("STYLE_SCHEMA_VERSION is valid")
    }
}

impl<'de> serde::Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        SchemaVersion::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
