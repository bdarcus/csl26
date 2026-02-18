/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Reference not found: {0}")]
    ReferenceNotFound(String),

    #[error("Date parse error: {0}")]
    DateParseError(String),

    #[error("Locale error: {0}")]
    LocaleError(String),

    #[error("Substitution error: {0}")]
    SubstitutionError(String),

    #[error("File I/O error: {0}")]
    FileIO(#[from] std::io::Error),

    #[error("Parse error ({0}): {1}")]
    ParseError(String, String),
}
