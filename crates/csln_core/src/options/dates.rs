/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::localization::MonthFormat;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Date formatting configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DateConfig {
    pub month: MonthFormat,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

impl Default for DateConfig {
    fn default() -> Self {
        Self {
            month: MonthFormat::Long,
            _extra: HashMap::new(),
        }
    }
}
