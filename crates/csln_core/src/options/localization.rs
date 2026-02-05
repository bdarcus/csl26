/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Localization scope settings.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Localize {
    pub scope: Scope,
}

impl Default for Localize {
    fn default() -> Self {
        Self {
            scope: Scope::Global,
        }
    }
}

/// Localization scope.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    #[default]
    Global,
    PerItem,
}

/// Month display format.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MonthFormat {
    #[default]
    Long,
    Short,
    Numeric,
}
