/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Rendering utilities for CSLN templates.
//!
//! This module provides the logic to transform processed template components
//! into formatted strings. It supports multiple output formats through a
//! pluggable architecture defined by the [`OutputFormat`] trait.
//!
//! ## Modules
//! - [`format`]: Defines the core [`OutputFormat`] trait.
//! - [`plain`], [`html`], [`djot`], [`latex`]: Concrete renderer implementations.
//! - [`component`]: Logic for rendering individual template components.
//! - [`citation`]: Logic for joining components into full citations.
//! - [`bibliography`]: Logic for rendering bibliographies.

pub mod bibliography;
pub mod citation;
pub mod component;
pub mod djot;
pub mod format;
pub mod html;
pub mod latex;
pub mod plain;

#[cfg(test)]
mod test_formats;

pub use bibliography::{refs_to_string, refs_to_string_with_format};
pub use citation::{citation_to_string, citation_to_string_with_format};
pub use component::{
    render_component, render_component_with_format_and_renderer, ProcEntry, ProcTemplate,
    ProcTemplateComponent,
};
