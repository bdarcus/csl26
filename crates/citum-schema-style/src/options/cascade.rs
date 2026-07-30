/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Field-level merge support for the runtime scoped-options cascade.
//!
//! The `extends` overlay deep-merges nested option blocks using authored
//! raw-document presence (`docs/specs/STYLE_INHERITANCE.md` rule 1). The
//! runtime scope cascade (global → citation/bibliography options) has the
//! same authored-vs-default ambiguity: a scope block that sets one field of
//! `dates` deserializes with serde defaults for every other field, so a typed
//! struct merge cannot tell which fields the author wrote and must replace
//! the whole block. This module carries the authored scope-level `options`
//! mappings through resolution — chain-merged across `extends` — so the
//! cascade can merge nested blocks field-by-field, and falls back to the
//! typed whole-block merge whenever no trustworthy raw mapping is available.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Style;
use crate::style::overlay::{deep_merge_yaml_value, effective_overlay_options};

use super::Config;

/// Chain-merged authored `options` mappings for the citation and bibliography
/// scopes of a style.
///
/// Populated when a style is parsed from a document (preset names expanded to
/// their resolved mappings) and maintained through `extends` resolution by
/// deep-merging each child's authored scope mapping over the parent's. `None`
/// for a scope means no trustworthy authored mapping is available
/// (programmatic construction, explicit clearing, or a non-mapping value);
/// the runtime cascade then falls back to the typed whole-block merge.
#[derive(Debug, Default, Clone)]
pub struct ScopedRawOptions {
    /// Authored `citation.options` mapping, presets expanded, chain-merged.
    pub citation: Option<serde_yaml::Value>,
    /// Authored `bibliography.options` mapping, presets expanded, chain-merged.
    pub bibliography: Option<serde_yaml::Value>,
}

impl ScopedRawOptions {
    /// Capture the authored scope mappings of a freshly parsed style document.
    pub(crate) fn capture(style: &Style) -> Self {
        Self::default().merged_with_child(style)
    }

    /// Overlay a child's authored scope mappings onto this (parent) capture.
    ///
    /// A child without `raw_yaml` (programmatic construction) has an unknown
    /// authored key-set, so both scopes reset to `None` and the runtime
    /// cascade falls back to the typed merge — mirroring the overlay's own
    /// raw-path fallback.
    pub(crate) fn merged_with_child(self, child: &Style) -> Self {
        let Some(raw) = child.raw_yaml.as_ref() else {
            return Self::default();
        };
        Self {
            citation: merge_scope(
                self.citation,
                authored_scope_options(
                    raw,
                    "citation",
                    child.citation.as_ref().and_then(|c| c.options.as_ref()),
                ),
            ),
            bibliography: merge_scope(
                self.bibliography,
                authored_scope_options(
                    raw,
                    "bibliography",
                    child.bibliography.as_ref().and_then(|b| b.options.as_ref()),
                ),
            ),
        }
    }
}

/// Authored state of one scope's `options` mapping in a raw style document.
enum AuthoredScope {
    /// The scope section or its `options` key is absent: inherit the parent's.
    Inherit,
    /// The `options` key is explicitly null, non-mapping, or inconsistent with
    /// the typed value: no presence information survives.
    Clear,
    /// The authored `options` mapping, preset-name strings expanded to their
    /// resolved mappings via the typed scope options.
    Authored(serde_yaml::Value),
}

/// Extract one scope's authored `options` mapping from a raw style document,
/// expanding preset-name strings through the typed scope options so stored
/// mappings layer like authored blocks (STYLE_INHERITANCE.md rule 3).
fn authored_scope_options<T: Serialize>(
    raw_doc: &serde_yaml::Value,
    section: &str,
    typed: Option<&T>,
) -> AuthoredScope {
    let Some(section_value) = raw_doc.get(section) else {
        return AuthoredScope::Inherit;
    };
    if section_value.is_null() {
        return AuthoredScope::Clear;
    }
    let Some(options_value) = section_value.get("options") else {
        return AuthoredScope::Inherit;
    };
    if !options_value.is_mapping() {
        return AuthoredScope::Clear;
    }
    let Some(typed) = typed else {
        return AuthoredScope::Clear;
    };
    match serde_yaml::to_value(typed) {
        Ok(typed_value) => {
            AuthoredScope::Authored(effective_overlay_options(options_value, &typed_value))
        }
        Err(_) => AuthoredScope::Clear,
    }
}

/// Merge a child's authored scope mapping over the parent's chain capture.
fn merge_scope(
    parent: Option<serde_yaml::Value>,
    child: AuthoredScope,
) -> Option<serde_yaml::Value> {
    match child {
        AuthoredScope::Inherit => parent,
        AuthoredScope::Clear => None,
        AuthoredScope::Authored(child_value) => match parent {
            Some(mut merged) if merged.is_mapping() => {
                deep_merge_yaml_value(&mut merged, &child_value);
                Some(merged)
            }
            _ => Some(child_value),
        },
    }
}

/// True when `raw` is a mapping that faithfully round-trips to `typed` — the
/// post-parse mutation guard of STYLE_INHERITANCE.md applied at cascade time.
/// Styles mutated programmatically after parse carry stale captures; the
/// guard makes the cascade fall back to the typed merge for them.
pub(crate) fn authored_matches<T>(raw: &serde_yaml::Value, typed: &T) -> bool
where
    T: DeserializeOwned + PartialEq,
{
    raw.is_mapping()
        && serde_yaml::from_value::<T>(raw.clone()).is_ok_and(|authored| authored == *typed)
}

/// Re-merge the nested option blocks a citation scope can override,
/// field-by-field from the authored raw scope mapping, with the resolved
/// global `base` supplying every field the scope author did not write.
pub(crate) fn merge_citation_config_blocks_from_raw(
    merged: &mut Config,
    base: &Config,
    raw: &serde_yaml::Value,
) {
    merge_block(
        &mut merged.substitute,
        base.substitute.as_ref(),
        raw.get("substitute"),
    );
    merge_block(
        &mut merged.multilingual,
        base.multilingual.as_ref(),
        raw.get("multilingual"),
    );
    merge_block(
        &mut merged.contributors,
        base.contributors.as_ref(),
        raw.get("contributors"),
    );
    merge_block(&mut merged.dates, base.dates.as_ref(), raw.get("dates"));
    merge_block(&mut merged.titles, base.titles.as_ref(), raw.get("titles"));
    merge_block(
        &mut merged.locators,
        base.locators.as_ref(),
        raw.get("locators"),
    );
    merge_block(&mut merged.links, base.links.as_ref(), raw.get("links"));
    merge_block(&mut merged.notes, base.notes.as_ref(), raw.get("notes"));
    merge_block(
        &mut merged.integral_name_memory,
        base.integral_name_memory.as_ref(),
        raw.get("integral-name-memory"),
    );
    merge_block(
        &mut merged.org_abbreviation_memory,
        base.org_abbreviation_memory.as_ref(),
        raw.get("org-abbreviation-memory"),
    );
}

/// Re-merge the nested option blocks a bibliography scope can override,
/// field-by-field from the authored raw scope mapping, with the resolved
/// global `base` supplying every field the scope author did not write.
pub(crate) fn merge_bibliography_config_blocks_from_raw(
    merged: &mut Config,
    base: &Config,
    raw: &serde_yaml::Value,
) {
    merge_block(
        &mut merged.substitute,
        base.substitute.as_ref(),
        raw.get("substitute"),
    );
    merge_block(
        &mut merged.multilingual,
        base.multilingual.as_ref(),
        raw.get("multilingual"),
    );
    merge_block(
        &mut merged.sorting,
        base.sorting.as_ref(),
        raw.get("sorting"),
    );
    merge_block(
        &mut merged.contributors,
        base.contributors.as_ref(),
        raw.get("contributors"),
    );
    merge_block(&mut merged.dates, base.dates.as_ref(), raw.get("dates"));
    merge_block(&mut merged.titles, base.titles.as_ref(), raw.get("titles"));
    merge_block(&mut merged.links, base.links.as_ref(), raw.get("links"));
}

/// Merge one nested block: serialize the inherited base block, deep-merge the
/// authored raw sub-mapping over it, and deserialize the result. Skips (keeping
/// the typed whole-block merge already in `slot`) when either side is absent,
/// the raw value is not a mapping, or the round-trip fails.
fn merge_block<T>(
    slot: &mut Option<T>,
    base_block: Option<&T>,
    raw_block: Option<&serde_yaml::Value>,
) where
    T: Serialize + DeserializeOwned,
{
    let (Some(base_block), Some(raw_block)) = (base_block, raw_block) else {
        return;
    };
    if !raw_block.is_mapping() {
        return;
    }
    let Ok(mut merged_value) = serde_yaml::to_value(base_block) else {
        return;
    };
    deep_merge_yaml_value(&mut merged_value, raw_block);
    if let Ok(block) = serde_yaml::from_value(merged_value) {
        *slot = Some(block);
    }
}
