/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Bibliography grouping support.
//!
//! This module provides functionality for dividing bibliographies into
//! labeled groups with distinct sorting rules.

pub mod selector;

pub use selector::SelectorEvaluator;
