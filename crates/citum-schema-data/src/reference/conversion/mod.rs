/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Legacy CSL-JSON → Citum reference conversion.
//!
//! The top-level [`From<csl_legacy::csl_json::Reference> for InputReference`]
//! impl dispatches by `ref_type` to a per-category converter in one of the
//! submodules:
//!
//! - `legal` — `legal-case`, `statute`, `regulation`, `treaty`, `standard`,
//!   `patent`, `bill`, `hearing`
//! - `scholarly` — `book`, `chapter`, `article-journal`, `article`,
//!   `paper-conference`, `dataset`, `event`, etc.; plus the standalone
//!   `input_reference_from_legacy_edited_book` re-exported below
//! - `media` — `software`, `motion_picture`, `song`
//!
//! Shared helpers (`legacy_*`, `relation_*`, `build_title`, …) and the
//! `RefContext` struct that bundles the fields every converter pre-extracts
//! live here in `mod.rs` so submodules can pull them in with `use super::*;`.

#[cfg(test)]
mod contract_tests;
mod legal;
mod media;
mod scholarly;

pub use scholarly::input_reference_from_legacy_edited_book;

use crate::reference::citeproc_markup::html_markup_to_djot;
use crate::reference::contributor::{
    Contributor, ContributorEntry, ContributorList, ContributorRole, SimpleName, StructuredName,
};
use crate::reference::date::DateValue;
use crate::reference::types::{
    ArchiveInfo, Collection, CollectionComponent, CollectionType, Dataset, Hearing, LegalCase,
    Monograph, MonographComponentType, MonographType, NumOrStr, Patent, Publisher, Regulation,
    RichText, Serial, SerialComponent, SerialComponentType, SerialType, Software, Standard,
    Statute, StructuredTitle, Subtitle, Title, Treaty,
};
use crate::reference::{
    AudioVisualType, AudioVisualWork, Event, IdentifierName, InputReference, LangID, Numbering,
    NumberingType, RefID, WorkCore, WorkRelation,
};
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;
use url::Url;

/// One row of the CSL 1.0.2 → Citum `ref_type()` conversion contract.
///
/// Source of truth for the generated CSL-JSON mapping doc
/// (`docs/reference/generated/CSL_JSON_MAPPING.md`, via
/// `docs/schemas/type-map.json`). See
/// `docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md` for the full canonicalization
/// rationale.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct CslTypeMapping {
    /// The CSL 1.0.2 item type (or accepted extension spelling).
    pub csl_type: &'static str,
    /// The `ref_type()` this type round-trips to, given the minimal shape
    /// `contract_tests::minimal_reference` builds (an id, the type, a title,
    /// and an issued year).
    pub citum_ref_type: &'static str,
    /// Rationale for divergences from the identity mapping; `None` when the
    /// mapping is unremarkable.
    pub note: Option<&'static str>,
}

/// Expected `ref_type()` output for every CSL 1.0.2 type, given the minimal
/// shape `contract_tests::minimal_reference` builds. Most entries are the
/// identity mapping; the ones that are not are intentional and documented in
/// `note` (also recorded in `docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md`'s
/// canonicalization table). Asserted against
/// `csl_legacy::csl_json::CSL_TYPES` by
/// `contract_tests::every_csl_1_0_2_type_has_an_expectation_table_entry`.
pub const CSL_TYPE_MAP: &[CslTypeMapping] = &[
    CslTypeMapping {
        csl_type: "article",
        citum_ref_type: "preprint",
        note: Some(
            "Bare `article` carries no container-title, so the converter treats it as a standalone preprint rather than a truncated journal article — this mirrors real-world CSL-JSON exports where a container-less `article` is an arXiv/SSRN-style preprint. See `from_preprint_ref`.",
        ),
    },
    CslTypeMapping {
        csl_type: "article-journal",
        citum_ref_type: "article-journal",
        note: None,
    },
    CslTypeMapping {
        csl_type: "article-magazine",
        citum_ref_type: "article-magazine",
        note: None,
    },
    CslTypeMapping {
        csl_type: "article-newspaper",
        citum_ref_type: "article-newspaper",
        note: None,
    },
    CslTypeMapping {
        csl_type: "bill",
        citum_ref_type: "document",
        note: Some(
            "A minimal `bill` (no `authority`, `chapter-number`, or `container-title`/`volume`/`page` combination) carries none of the shape signals `from_bill_ref` uses to distinguish a hearing, bill-proceeding, or bill-record from a generic government document. See `reference/tests.rs` for the shapes that do round-trip distinctly (`test_parse_csl_bill_*`).",
        ),
    },
    CslTypeMapping {
        csl_type: "book",
        citum_ref_type: "book",
        note: None,
    },
    CslTypeMapping {
        csl_type: "broadcast",
        citum_ref_type: "broadcast",
        note: None,
    },
    CslTypeMapping {
        csl_type: "chapter",
        citum_ref_type: "chapter",
        note: None,
    },
    CslTypeMapping {
        csl_type: "classic",
        citum_ref_type: "classic",
        note: None,
    },
    CslTypeMapping {
        csl_type: "collection",
        citum_ref_type: "collection",
        note: None,
    },
    CslTypeMapping {
        csl_type: "dataset",
        citum_ref_type: "dataset",
        note: None,
    },
    CslTypeMapping {
        csl_type: "document",
        citum_ref_type: "document",
        note: None,
    },
    CslTypeMapping {
        csl_type: "entry",
        citum_ref_type: "entry",
        note: None,
    },
    CslTypeMapping {
        csl_type: "entry-dictionary",
        citum_ref_type: "entry-dictionary",
        note: None,
    },
    CslTypeMapping {
        csl_type: "entry-encyclopedia",
        citum_ref_type: "entry-encyclopedia",
        note: None,
    },
    CslTypeMapping {
        csl_type: "event",
        citum_ref_type: "event",
        note: None,
    },
    CslTypeMapping {
        csl_type: "figure",
        citum_ref_type: "figure",
        note: None,
    },
    CslTypeMapping {
        csl_type: "graphic",
        citum_ref_type: "graphic",
        note: None,
    },
    CslTypeMapping {
        csl_type: "hearing",
        citum_ref_type: "hearing",
        note: None,
    },
    CslTypeMapping {
        csl_type: "interview",
        citum_ref_type: "interview",
        note: None,
    },
    CslTypeMapping {
        csl_type: "legal_case",
        citum_ref_type: "legal-case",
        note: Some(
            "The CSL 1.0.2 spelling uses an underscore; it canonicalizes to the hyphenated `legal-case` on output, matching this codebase's convention of canonicalizing underscore CSL spellings to hyphens (see also `motion_picture`, `musical_score`, `personal_communication`).",
        ),
    },
    CslTypeMapping {
        csl_type: "legislation",
        citum_ref_type: "statute",
        note: Some(
            "`legislation` is the CSL 1.0.2 closed-vocabulary type; it routes to the same converter as the `statute` extension spelling and shares its canonical output.",
        ),
    },
    CslTypeMapping {
        csl_type: "manuscript",
        citum_ref_type: "manuscript",
        note: None,
    },
    CslTypeMapping {
        csl_type: "map",
        citum_ref_type: "map",
        note: None,
    },
    CslTypeMapping {
        csl_type: "motion_picture",
        citum_ref_type: "motion-picture",
        note: Some("Underscore CSL spelling canonicalizes to hyphenated output; see `legal_case`."),
    },
    CslTypeMapping {
        csl_type: "musical_score",
        citum_ref_type: "musical-score",
        note: Some("Underscore CSL spelling canonicalizes to hyphenated output; see `legal_case`."),
    },
    CslTypeMapping {
        csl_type: "pamphlet",
        citum_ref_type: "pamphlet",
        note: None,
    },
    CslTypeMapping {
        csl_type: "paper-conference",
        citum_ref_type: "paper-conference",
        note: None,
    },
    CslTypeMapping {
        csl_type: "patent",
        citum_ref_type: "patent",
        note: None,
    },
    CslTypeMapping {
        csl_type: "performance",
        citum_ref_type: "performance",
        note: None,
    },
    CslTypeMapping {
        csl_type: "periodical",
        citum_ref_type: "periodical",
        note: None,
    },
    CslTypeMapping {
        csl_type: "personal_communication",
        citum_ref_type: "personal-communication",
        note: Some("Underscore CSL spelling canonicalizes to hyphenated output; see `legal_case`."),
    },
    CslTypeMapping {
        csl_type: "post",
        citum_ref_type: "post",
        note: None,
    },
    CslTypeMapping {
        csl_type: "post-weblog",
        citum_ref_type: "post-weblog",
        note: None,
    },
    CslTypeMapping {
        csl_type: "regulation",
        citum_ref_type: "regulation",
        note: None,
    },
    CslTypeMapping {
        csl_type: "report",
        citum_ref_type: "report",
        note: None,
    },
    CslTypeMapping {
        csl_type: "review",
        citum_ref_type: "review",
        note: None,
    },
    CslTypeMapping {
        csl_type: "review-book",
        citum_ref_type: "review-book",
        note: None,
    },
    CslTypeMapping {
        csl_type: "software",
        citum_ref_type: "software",
        note: None,
    },
    CslTypeMapping {
        csl_type: "song",
        citum_ref_type: "song",
        note: None,
    },
    CslTypeMapping {
        csl_type: "speech",
        citum_ref_type: "speech",
        note: None,
    },
    CslTypeMapping {
        csl_type: "standard",
        citum_ref_type: "standard",
        note: None,
    },
    CslTypeMapping {
        csl_type: "thesis",
        citum_ref_type: "thesis",
        note: None,
    },
    CslTypeMapping {
        csl_type: "treaty",
        citum_ref_type: "treaty",
        note: None,
    },
    CslTypeMapping {
        csl_type: "webpage",
        citum_ref_type: "webpage",
        note: None,
    },
];

/// Fold legacy named contributor fields (recipient and interviewer) into a contributors vec.
fn legacy_named_contributors(legacy: &csl_legacy::csl_json::Reference) -> Vec<ContributorEntry> {
    let mut entries = Vec::new();
    push_legacy_contributor(
        &mut entries,
        ContributorRole::Recipient,
        legacy.recipient.clone(),
    );
    push_legacy_contributor(
        &mut entries,
        ContributorRole::Interviewer,
        legacy.interviewer.clone(),
    );
    entries
}

fn push_legacy_contributor(
    entries: &mut Vec<ContributorEntry>,
    role: ContributorRole,
    src: Option<Vec<csl_legacy::csl_json::Name>>,
) {
    if let Some(names) = src {
        let Contributor::ContributorList(list) = Contributor::from(names) else {
            return;
        };
        let mut unmatched = Vec::new();
        for contributor in list.0 {
            let identity = legacy_contributor_identity(&contributor);
            if let Some((entry_index, member_index)) = identity
                .as_ref()
                .and_then(|identity| find_legacy_contributor(entries, &role, identity))
                && merge_legacy_contributor_role(entries, entry_index, member_index, role.clone())
            {
                continue;
            }
            unmatched.push(contributor);
        }
        if !unmatched.is_empty() {
            let contributor = if unmatched.len() == 1 {
                let Some(contributor) = unmatched.pop() else {
                    return;
                };
                contributor
            } else {
                Contributor::ContributorList(ContributorList(unmatched))
            };
            entries.push(ContributorEntry {
                roles: role.into(),
                contributor,
                gender: None,
            });
        }
    }
}

fn find_legacy_contributor(
    entries: &[ContributorEntry],
    role: &ContributorRole,
    identity: &LegacyContributorIdentity,
) -> Option<(usize, Option<usize>)> {
    entries.iter().enumerate().find_map(|(entry_index, entry)| {
        if entry.roles.contains(role) {
            return None;
        }
        match &entry.contributor {
            Contributor::ContributorList(list) => list
                .0
                .iter()
                .position(|contributor| {
                    legacy_contributor_identity(contributor).as_ref() == Some(identity)
                })
                .map(|member_index| (entry_index, Some(member_index))),
            contributor => (legacy_contributor_identity(contributor).as_ref() == Some(identity))
                .then_some((entry_index, None)),
        }
    })
}

fn merge_legacy_contributor_role(
    entries: &mut Vec<ContributorEntry>,
    entry_index: usize,
    member_index: Option<usize>,
    role: ContributorRole,
) -> bool {
    let Some(member_index) = member_index else {
        if let Some(entry) = entries.get_mut(entry_index) {
            entry.roles.insert(role);
            return true;
        }
        return false;
    };
    if entry_index >= entries.len() {
        return false;
    }

    let ContributorEntry {
        roles,
        contributor,
        gender,
    } = entries.remove(entry_index);
    let Contributor::ContributorList(mut list) = contributor else {
        entries.insert(
            entry_index,
            ContributorEntry {
                roles,
                contributor,
                gender,
            },
        );
        return false;
    };
    if member_index >= list.0.len() {
        entries.insert(
            entry_index,
            ContributorEntry {
                roles,
                contributor: Contributor::ContributorList(list),
                gender,
            },
        );
        return false;
    }

    let mut after = list.0.split_off(member_index + 1);
    let Some(matched) = list.0.pop() else {
        return false;
    };
    let mut matched_roles = roles.clone();
    matched_roles.insert(role);
    let mut replacement = Vec::with_capacity(3);
    if !list.0.is_empty() {
        replacement.push(ContributorEntry {
            roles: roles.clone(),
            contributor: Contributor::ContributorList(list),
            gender,
        });
    }
    replacement.push(ContributorEntry {
        roles: matched_roles,
        contributor: matched,
        gender,
    });
    if !after.is_empty() {
        replacement.push(ContributorEntry {
            roles,
            contributor: Contributor::ContributorList(ContributorList(std::mem::take(&mut after))),
            gender,
        });
    }
    entries.splice(entry_index..entry_index, replacement);
    true
}

#[derive(Debug, PartialEq, Eq)]
enum LegacyContributorIdentity {
    Structured(Vec<String>),
    Literal(String),
}

fn legacy_contributor_identity(contributor: &Contributor) -> Option<LegacyContributorIdentity> {
    let name = contributor.to_names_vec().into_iter().next()?;
    if let Some(literal) = normalized_identity_part(name.literal.as_deref()) {
        return Some(LegacyContributorIdentity::Literal(literal));
    }
    let parts = [
        name.given.as_deref(),
        name.family.as_deref(),
        name.suffix.as_deref(),
        name.dropping_particle.as_deref(),
        name.non_dropping_particle.as_deref(),
    ]
    .into_iter()
    .map(|part| normalized_identity_part(part).unwrap_or_default())
    .collect::<Vec<_>>();
    parts
        .iter()
        .any(|part| !part.is_empty())
        .then_some(LegacyContributorIdentity::Structured(parts))
}

fn normalized_identity_part(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.nfc().collect())
}

fn legacy_extra_names(
    legacy: &csl_legacy::csl_json::Reference,
    key: &str,
) -> Option<Vec<csl_legacy::csl_json::Name>> {
    legacy
        .extra
        .get(key)
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn legacy_extra_str(legacy: &csl_legacy::csl_json::Reference, key: &str) -> Option<String> {
    legacy
        .extra
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn legacy_extra_date(legacy: &csl_legacy::csl_json::Reference, key: &str) -> Option<DateValue> {
    legacy
        .extra
        .get(key)
        .and_then(|value| {
            serde_json::from_value::<csl_legacy::csl_json::DateVariable>(value.clone()).ok()
        })
        .map(DateValue::from)
        .or_else(|| {
            legacy
                .extra
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(|value| DateValue::new(value.to_string()))
        })
}

fn legacy_extra_contributor(
    legacy: &csl_legacy::csl_json::Reference,
    key: &str,
) -> Option<Contributor> {
    legacy_extra_names(legacy, key).map(Contributor::from)
}

/// Build a publisher from legacy name/place, preserving a place-only imprint.
///
/// `Publisher::name` is non-optional, so when only a place is present (no
/// publisher name) this returns a `Publisher` with an empty name rather than
/// dropping the place entirely — GB/T 7714 and other styles render
/// place-only imprints, so silently discarding `publisher-place` when
/// `publisher` is absent would lose data.
fn publisher_from_parts(name: Option<String>, place: Option<String>) -> Option<Publisher> {
    match (name, place) {
        (Some(name), place) => Some(Publisher {
            name: name.into(),
            place: place.map(Into::into),
        }),
        (None, Some(place)) => Some(Publisher {
            name: String::new().into(),
            place: Some(place.into()),
        }),
        (None, None) => None,
    }
}

fn relation_monograph(
    title: Option<Title>,
    author: Option<Contributor>,
    issued: Option<DateValue>,
    genre: Option<String>,
    publisher: Option<String>,
    publisher_place: Option<String>,
) -> Option<WorkRelation> {
    if title.is_none()
        && author.is_none()
        && issued.is_none()
        && genre.is_none()
        && publisher.is_none()
        && publisher_place.is_none()
    {
        return None;
    }

    let publisher = publisher_from_parts(publisher, publisher_place);

    Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
        Box::new(Monograph {
            title,
            author,
            issued: issued.unwrap_or_default(),
            genre,
            publisher,
            ..Default::default()
        }),
    ))))
}

fn legacy_original_relation(legacy: &csl_legacy::csl_json::Reference) -> Option<WorkRelation> {
    relation_monograph(
        legacy.original_title.clone().map(Title::Single),
        legacy_extra_contributor(legacy, "original-author"),
        legacy_extra_date(legacy, "original-date"),
        None,
        legacy_extra_str(legacy, "original-publisher"),
        legacy_extra_str(legacy, "original-publisher-place"),
    )
}

fn relation_event(
    title: Option<String>,
    location: Option<String>,
    date: Option<DateValue>,
) -> Option<WorkRelation> {
    if title.is_none() && location.is_none() && date.is_none() {
        return None;
    }
    Some(WorkRelation::Embedded(Box::new(InputReference::Event(
        Box::new(Event {
            title: title.map(Title::Single),
            location,
            date,
            ..Default::default()
        }),
    ))))
}

fn relation_collection_title(title: Option<String>) -> Option<WorkRelation> {
    title.map(|title| {
        WorkRelation::Embedded(Box::new(InputReference::Collection(Box::new(Collection {
            title: Some(Title::Single(title)),
            ..Default::default()
        }))))
    })
}

fn short_title_from_legacy(legacy: &csl_legacy::csl_json::Reference, key: &str) -> Option<String> {
    legacy_extra_str(legacy, key)
}

fn normalize_broadcast_issue(
    ref_type: &str,
    medium: Option<&str>,
    number: &str,
) -> csl_legacy::csl_json::StringOrNumber {
    let normalized = if matches!(ref_type, "broadcast" | "motion_picture")
        && medium
            .map(|value| value.to_ascii_lowercase().contains("podcast"))
            .unwrap_or(false)
        && number.chars().all(|ch| ch.is_ascii_digit())
    {
        format!("No. {number}")
    } else {
        number.to_string()
    };

    csl_legacy::csl_json::StringOrNumber::String(normalized)
}

/// Build a title, optionally structured if short_title is present and title contains a colon.
///
/// `html_markup_to_djot` runs again here even though `normalize_rich_text_markup`
/// (called earlier, in `From<csl_legacy::csl_json::Reference>`) already converts
/// `legacy.title`/`legacy.title_short` -- this function is also reached with
/// `part_title`, sourced from `legacy_extra_str(&legacy, "part-title")`
/// (`scholarly.rs`), which lives in the `extra` map the central pass
/// deliberately skips (see `normalize_rich_text_markup`'s doc comment). The
/// call is idempotent on already-converted text -- recognized tags leave no
/// `<` behind, and any text already in Djot has none to begin with -- so the
/// redundancy for the already-normalized case is free.
fn build_title(title: Option<String>, short_title: Option<String>) -> Option<Title> {
    let title = title.map(|t| html_markup_to_djot(&t));
    let short_title = short_title.map(|t| html_markup_to_djot(&t));
    match (title, short_title) {
        (Some(full_title), Some(short)) => {
            if let Some(colon_pos) = full_title.find(':') {
                #[allow(
                    clippy::string_slice,
                    reason = "colon_pos is found via find(':'), which is a 1-byte ASCII boundary"
                )]
                let potential_main = full_title[..colon_pos].trim();
                // Check if short title matches pre-colon portion
                if potential_main.eq_ignore_ascii_case(short.as_str())
                    || potential_main.contains(&short)
                {
                    #[allow(
                        clippy::string_slice,
                        reason = "colon_pos + 1 is a valid boundary after ':' (1-byte ASCII)"
                    )]
                    let post_colon = full_title[colon_pos + 1..].trim();
                    return Some(Title::Structured(StructuredTitle {
                        full: None,
                        main: short,
                        sub: Subtitle::String(post_colon.to_string()),
                    }));
                }
            }
            // Fallback: just use the full title
            Some(Title::Single(full_title))
        }
        (Some(title), None) => Some(Title::Single(title)),
        _ => None,
    }
}

fn archive_info_from_legacy_flat(legacy: &csl_legacy::csl_json::Reference) -> Option<ArchiveInfo> {
    if legacy.archive.is_none() && legacy.archive_location.is_none() {
        return None;
    }

    let collection = legacy
        .extra
        .get("archive-collection")
        .or_else(|| legacy.extra.get("archive_collection"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let place = legacy
        .extra
        .get("archive-place")
        .or_else(|| legacy.extra.get("archive_place"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Some(ArchiveInfo {
        name: legacy.archive.clone().map(Into::into),
        location: legacy.archive_location.clone(),
        place: place.map(Into::into),
        collection,
        ..Default::default()
    })
}

/// Pre-extracted common fields shared by all reference conversion functions.
struct RefContext {
    id: Option<RefID>,
    title: Option<String>,
    short_title: Option<String>,
    created: DateValue,
    issued: DateValue,
    url: Option<Url>,
    accessed: Option<DateValue>,
    language: Option<LangID>,
    note: Option<String>,
    doi: Option<String>,
    isbn: Option<String>,
    edition: Option<String>,
    container_title_short: Option<String>,
    journal_abbreviation: Option<String>,
    /// Copyright year, a publication-year substitute used when the true
    /// issue date is unknown. See `copyright_year_from_legacy`.
    copyright: Option<DateValue>,
    /// Printing/impression year, another publication-year substitute. See
    /// `docs/specs/DATE_MODEL.md`.
    printing: Option<DateValue>,
}

/// Extract a GB/T-style copyright year from a CSL `c<year>` literal issued
/// date (e.g. `"c1988"`). GB/T 7714 §7.5.4.3 uses this as a
/// publication-year substitute when the true issue date is unknown; an
/// earlier revision misread the `c` as EDTF circa (`~`) instead of
/// copyright — see `docs/specs/DATE_MODEL.md`.
fn copyright_year_from_legacy(legacy: &csl_legacy::csl_json::Reference) -> Option<DateValue> {
    let literal = legacy.issued.as_ref()?.literal.as_deref()?;
    let year = literal.strip_prefix('c')?.trim();
    (year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| DateValue::new(year.to_string()))
}

/// Interpret a raw `issued:` note-field override that didn't reduce to a
/// structured date (see csl-legacy's `issued-note-literal` extra, set when
/// the override coexists with an already-structured `issued`). GB/T 7714
/// §7.5.4.3 defines two more publication-year substitutes alongside
/// copyright: a printing/impression year (Chinese suffix `印刷`) and an
/// estimated year (EDTF approximate, trailing `~`).
fn printing_year_from_legacy(legacy: &csl_legacy::csl_json::Reference) -> Option<DateValue> {
    let literal = legacy_extra_str(legacy, "issued-note-literal")?;
    let year = literal.strip_suffix("印刷")?.trim();
    (year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| DateValue::new(year.to_string()))
}

/// Interpret an estimated-year note-field override (trailing `~`) as an
/// EDTF approximate `issued` date. See `printing_year_from_legacy`.
fn estimated_issued_from_legacy(legacy: &csl_legacy::csl_json::Reference) -> Option<DateValue> {
    let literal = legacy_extra_str(legacy, "issued-note-literal")?;
    let year = literal.strip_suffix('~')?.trim();
    (year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| DateValue::new(format!("{year}~")))
}

/// Interpret a source-calendar note-field override — the bounded shape
/// `<year>（<note>）` using **full-width** parentheses only (not half-width
/// `( )`) — as an annotated `issued` date. Full-width-only keeps this
/// disjoint from the `copyright`/`printing`/`estimated` substitutes above
/// and from any half-width-parenthesized Latin-script literal; it does not
/// perform calendar conversion or general-purpose date-prose parsing. See
/// `printing_year_from_legacy`, `docs/specs/CALENDAR_DATE_ANNOTATIONS.md`.
fn annotated_issued_from_legacy(legacy: &csl_legacy::csl_json::Reference) -> Option<DateValue> {
    let literal = legacy_extra_str(legacy, "issued-note-literal")?;
    let (year, rest) = literal.split_once('（')?;
    let note = rest.strip_suffix('）')?.trim();
    let year = year.trim();
    (year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit()) && !note.is_empty()).then(
        || DateValue {
            value: year.to_string(),
            note: Some(note.to_string()),
        },
    )
}

fn legacy_type_uses_created(ref_type: &str) -> bool {
    matches!(
        ref_type,
        "manuscript"
            | "interview"
            | "personal_communication"
            | "personal-communication"
            | "speech"
            | "presentation"
    )
}

/// Convert citeproc-js's literal HTML rich-text convention to Djot across
/// every free-text CSL-JSON field that legitimately carries it, in place.
///
/// citeproc-js flip-flops HTML rich-text markup (`<span class="nocase">`,
/// `<i>`, `<b>`, `<sc>`, `<sup>`, `<sub>`) into text variables generally, not
/// just `title` -- this benchmark's own `container-title` carries `nocase`
/// spans (`csl26-6eoi`). Verified directly against the `citeproc` npm
/// package (the reference implementation) for every field below: each
/// strips `<span class="nocase">` the same way `title` does. Bean
/// `csl26-zaqk` converted `title` and `short_title` at the `build_title`
/// call site; this is the remaining scope that bean explicitly deferred.
/// Runs once, immediately after `parse_note_field_hacks()` and before any
/// per-type converter reads the reference, so no future conversion path
/// can bypass it.
///
/// Deliberately does **not** touch:
/// - `note` -- `parse_note_field_hacks()` parses it for extra-field
///   key/value overrides; converting first would corrupt that parse.
/// - `doi`, `url`, `isbn`, `issn`, `language` -- identifiers, not rich text.
/// - `number` -- despite the CSL test suite's `flipflop_NumericField.json`
///   fixture name, real citeproc-js does *not* flip-flop this field:
///   `<number variable="number"/>` on `"number": "1<sup>er</sup>"` renders
///   the tag literally, HTML-escaped (`1&#60;sup&#62;er&#60;/sup&#62;`),
///   confirmed by running the fixture through the real `citeproc` engine.
///   `number` is a CSL number-type variable, not a plain text one, and is
///   exempt from flip-flop -- converting it here would be a regression, not
///   a fix.
/// - `page`, `volume`, `issue`, `edition` (the latter three `StringOrNumber`)
///   -- real citeproc-js *does* flip-flop these (verified the same way as
///   above), but no fixture in this repo's corpus carries rich text there;
///   recorded as a follow-up in `csl26-6eoi` rather than guessed at here.
/// - The `extra` map (`issued-note-literal`, `available-date`,
///   `original-author`, etc.) -- not verified; same follow-up.
fn normalize_rich_text_markup(legacy: &mut csl_legacy::csl_json::Reference) {
    for value in [
        &mut legacy.title,
        &mut legacy.container_title,
        &mut legacy.collection_title,
        &mut legacy.original_title,
        &mut legacy.publisher,
        &mut legacy.publisher_place,
        &mut legacy.event,
        &mut legacy.genre,
        &mut legacy.medium,
        &mut legacy.section,
        &mut legacy.authority,
        &mut legacy.abstract_text,
        &mut legacy.archive,
        &mut legacy.archive_location,
        &mut legacy.dimensions,
    ]
    .into_iter()
    .flatten()
    {
        *value = html_markup_to_djot(value);
    }
}

impl From<csl_legacy::csl_json::Reference> for InputReference {
    #[allow(
        clippy::too_many_lines,
        reason = "Legacy CSL mapping requires extensive branching"
    )]
    fn from(mut legacy: csl_legacy::csl_json::Reference) -> Self {
        legacy.parse_note_field_hacks();
        normalize_rich_text_markup(&mut legacy);
        let cstr = legacy_extra_str(&legacy, "CSTR");
        // GB/T 7714 §7.5.4.3 publication-year substitutes, used when the
        // true issue date is unknown. Copyright is a top-level `c<year>`
        // issued literal; printing and estimated are note-field overrides
        // that didn't reduce to a structured date (see
        // `copyright_year_from_legacy` and friends). Copyright and printing
        // route to their own fields, leaving `issued` empty so a style's
        // fallback chain renders them; estimated and annotated fold into
        // `issued` itself, as an EDTF approximate date and an annotated
        // `DateValue` respectively.
        let copyright = copyright_year_from_legacy(&legacy);
        let printing = printing_year_from_legacy(&legacy);
        let estimated_issued = estimated_issued_from_legacy(&legacy);
        let annotated_issued = annotated_issued_from_legacy(&legacy);
        let ctx = RefContext {
            id: Some(legacy.id.clone().into()),
            title: legacy.title.clone(),
            short_title: short_title_from_legacy(&legacy, "shortTitle")
                .or_else(|| short_title_from_legacy(&legacy, "title-short")),
            created: if legacy_type_uses_created(&legacy.ref_type) {
                legacy
                    .issued
                    .clone()
                    .map(DateValue::from)
                    .unwrap_or(DateValue::new(String::new()))
            } else {
                DateValue::new(String::new())
            },
            issued: if copyright.is_some() || printing.is_some() {
                DateValue::new(String::new())
            } else if let Some(estimated) = estimated_issued.clone() {
                estimated
            } else if let Some(annotated) = annotated_issued.clone() {
                annotated
            } else {
                legacy
                    .issued
                    .clone()
                    .map(DateValue::from)
                    .unwrap_or(DateValue::new(String::new()))
            },
            url: legacy.url.as_ref().and_then(|u| Url::parse(u).ok()),
            accessed: legacy.accessed.clone().map(DateValue::from),
            language: legacy.language.clone().map(Into::into),
            note: legacy.note.clone(),
            doi: legacy.doi.clone(),
            isbn: legacy.isbn.clone(),
            edition: legacy.edition.as_ref().map(|e| e.to_string()),
            container_title_short: short_title_from_legacy(&legacy, "container-title-short"),
            journal_abbreviation: short_title_from_legacy(&legacy, "journalAbbreviation"),
            copyright,
            printing,
        };

        let mut reference = match legacy.ref_type.as_str() {
            "software" => media::from_software_ref(legacy, ctx),
            "book"
            | "thesis"
            | "manual"
            | "manuscript"
            | "classic"
            | "webpage"
            | "post"
            | "post-weblog"
            | "interview"
            | "personal_communication"
            | "personal-communication"
            | "musical_score"
            | "pamphlet" => scholarly::from_monograph_ref(legacy, ctx),
            "report"
                if legacy.page.is_some()
                    && (legacy.editor.is_some() || legacy.container_title.is_some()) =>
            {
                scholarly::from_collection_component_ref(legacy, ctx)
            }
            "report" => scholarly::from_monograph_ref(legacy, ctx),
            "chapter" | "paper-conference" | "entry" | "entry-dictionary"
            | "entry-encyclopedia" => scholarly::from_collection_component_ref(legacy, ctx),
            "article-journal" | "article-magazine" | "article-newspaper" | "review"
            | "review-book" => scholarly::from_serial_component_ref(legacy, ctx),
            "article" => {
                if legacy.container_title.is_none() {
                    scholarly::from_preprint_ref(legacy, ctx)
                } else {
                    scholarly::from_serial_component_ref(legacy, ctx)
                }
            }
            "motion_picture" | "song" => media::from_audio_visual_ref(legacy, ctx),
            "broadcast" => scholarly::from_serial_component_ref(legacy, ctx),
            "speech" | "presentation" | "performance" | "event" => {
                scholarly::from_event_ref(legacy, ctx)
            }
            "bill" => legal::from_bill_ref(legacy, ctx),
            "hearing" => legal::from_hearing_ref(legacy, ctx),
            "legal-case" | "legal_case" => legal::from_legal_case_ref(legacy, ctx),
            "statute" | "legislation" => legal::from_statute_ref(legacy, ctx),
            "regulation" => legal::from_regulation_ref(legacy, ctx),
            "treaty" => legal::from_treaty_ref(legacy, ctx),
            "standard" => legal::from_standard_ref(legacy, ctx),
            "patent" => legal::from_patent_ref(legacy, ctx),
            "dataset" => scholarly::from_dataset_ref(legacy, ctx),
            // `collection` is CSL 1.0.2's *archival* collection (a body of
            // manuscripts/papers held by an archive: author, archive,
            // archive-place). It routes with the other archival/document
            // shapes so those fields survive; Citum's editorial
            // `ClassExtension::Collection` (anthology/proceedings) has no
            // author or archive fields and would silently drop them. See
            // docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md.
            "document" | "map" | "figure" | "graphic" | "periodical" | "collection" => {
                scholarly::from_document_ref(legacy, ctx)
            }
            _ => {
                // Every CSL 1.0.2 type string has an explicit arm above; a
                // known type reaching this fallback means a routing arm was
                // dropped, not that the type is genuinely unmapped.
                //
                // TODO(csl26-1bdr): Layer 5 `CompatibilityWarning` plumbing
                // will surface unrecognized types as a soft-degrade warning
                // rather than silent fall-through. Until then this
                // fallback mirrors the ClassExtension::Unknown loud-fail
                // pattern in accessors.rs.
                debug_assert!(
                    !csl_legacy::csl_json::CSL_TYPES.contains(&legacy.ref_type.as_str()),
                    "unmapped CSL 1.0.2 type `{}` fell through to the document fallback; \
                     add a routing arm in conversion/mod.rs",
                    legacy.ref_type
                );
                scholarly::from_document_ref(legacy, ctx)
            }
        };
        if let Some(cstr) = cstr
            && let Ok(name) = IdentifierName::new("cstr")
        {
            reference.insert_identifier(name, cstr);
        }
        reference
    }
}

impl From<csl_legacy::csl_json::DateVariable> for DateValue {
    fn from(date: csl_legacy::csl_json::DateVariable) -> Self {
        if let Some(literal) = date.literal {
            return DateValue::new(literal);
        }
        if let Some(parts) = date.date_parts {
            let mut rendered = parts.iter().map(|part| render_date_part(part));
            if let Some(first) = rendered.next() {
                if let Some(second) = rendered.next() {
                    return DateValue::new(apply_csl_circa_marker(
                        format!("{first}/{second}"),
                        date.circa,
                    ));
                }
                return DateValue::new(apply_csl_circa_marker(first, date.circa));
            }
        }
        if let Some(raw) = date.raw {
            return DateValue::new(raw);
        }
        DateValue::new(String::new())
    }
}

/// Apply CSL's structured circa flag to an EDTF date that lacks a qualifier.
fn apply_csl_circa_marker(value: String, circa: Option<bool>) -> String {
    if circa == Some(true) && !value.ends_with('~') && !value.ends_with('%') {
        format!("{value}~")
    } else {
        value
    }
}

fn render_date_part(part: &[i32]) -> String {
    let year = part
        .first()
        .map(|y| {
            if *y < 0 {
                format!("-{:04}", y.abs())
            } else {
                format!("{:04}", y)
            }
        })
        .unwrap_or_default();
    let month = part
        .get(1)
        .map(|m| format!("-{:02}", m))
        .unwrap_or_default();
    let day = part
        .get(2)
        .map(|d| format!("-{:02}", d))
        .unwrap_or_default();
    format!("{year}{month}{day}")
}

impl From<Vec<csl_legacy::csl_json::Name>> for Contributor {
    fn from(names: Vec<csl_legacy::csl_json::Name>) -> Self {
        let contributors: Vec<Contributor> = names
            .into_iter()
            .map(|n| {
                if let Some(literal) = n.literal {
                    Contributor::SimpleName(SimpleName {
                        name: literal.into(),
                        location: None,
                        short_name: None,
                    })
                } else {
                    let given_str = n.given.as_deref().map(str::trim).unwrap_or("");
                    if given_str.is_empty()
                        && n.dropping_particle.is_none()
                        && n.non_dropping_particle.is_none()
                    {
                        // No given name and no particles: treat family as a literal name
                        Contributor::SimpleName(SimpleName {
                            name: n.family.unwrap_or_default().into(),
                            location: None,
                            short_name: None,
                        })
                    } else {
                        Contributor::StructuredName(StructuredName {
                            given: given_str.to_string().into(),
                            family: n.family.unwrap_or_default().into(),
                            suffix: n.suffix,
                            dropping_particle: n.dropping_particle,
                            non_dropping_particle: n.non_dropping_particle,
                        })
                    }
                }
            })
            .collect();
        Contributor::ContributorList(ContributorList(contributors))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use crate::reference::ClassExtension;

    use super::*;
    use serde_json::json;

    fn legacy_year(year: i32) -> csl_legacy::csl_json::DateVariable {
        csl_legacy::csl_json::DateVariable {
            date_parts: Some(vec![vec![year]]),
            ..Default::default()
        }
    }

    #[test]
    fn legacy_report_number_maps_to_report_numbering() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "report-1".to_string(),
            ref_type: "report".to_string(),
            title: Some("Report".to_string()),
            issued: Some(legacy_year(2024)),
            number: Some("TR-7".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(converted.number(), None);
        assert_eq!(converted.report_number(), Some("TR-7".to_string()));
    }

    #[test]
    fn legacy_book_number_maps_to_generic_numbering() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-1".to_string(),
            ref_type: "book".to_string(),
            title: Some("Book".to_string()),
            issued: Some(legacy_year(2024)),
            number: Some("2".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(converted.number(), Some("2".to_string()));
        assert_eq!(converted.report_number(), None);
    }

    #[test]
    fn legacy_note_type_classic_maps_to_classic_reference() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "classic-1".to_string(),
            ref_type: "book".to_string(),
            title: Some("De civitate Dei".to_string()),
            issued: Some(legacy_year(1931)),
            note: Some("type: classic".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(converted.ref_type(), "classic");
        assert!(matches!(converted.extension(), ClassExtension::Classic(_)));
    }

    #[test]
    fn legacy_monograph_original_relation_uses_original_author_and_date() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-2".to_string(),
            ref_type: "book".to_string(),
            title: Some("Translated Book".to_string()),
            issued: Some(legacy_year(2024)),
            original_title: Some("Original Book".to_string()),
            extra: HashMap::from([
                (
                    "original-author".to_string(),
                    json!([{"family":"Woolf","given":"Virginia"}]),
                ),
                ("original-date".to_string(), json!({"date-parts":[[1925]]})),
            ]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        let Some(WorkRelation::Embedded(original)) = monograph.original.as_ref() else {
            panic!("expected embedded original relation");
        };
        let ClassExtension::Monograph(original_monograph) = original.extension() else {
            panic!("expected original monograph relation");
        };

        assert_eq!(
            original_monograph.title,
            Some(Title::Single("Original Book".to_string()))
        );
        assert_eq!(
            original_monograph.issued,
            DateValue::new("1925".to_string())
        );
        assert!(original_monograph.author.is_some());
    }

    #[test]
    fn legacy_copyright_year_literal_routes_to_copyright_not_issued() {
        // GB/T 7714 §7.5.4.3: a `c<year>` issued literal is a copyright
        // year, a publication-year substitute — not EDTF circa. It must
        // land on `copyright`, leaving `issued` empty so a style's
        // issued->copyright fallback chain renders it.
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-3".to_string(),
            ref_type: "book".to_string(),
            title: Some("A Brief History of Time".to_string()),
            issued: Some(csl_legacy::csl_json::DateVariable {
                literal: Some("c1988".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(
            monograph.copyright,
            Some(DateValue::new("1988".to_string()))
        );
        assert_eq!(monograph.issued, DateValue::new(String::new()));
    }

    #[test]
    fn legacy_printing_year_note_override_routes_to_printing_not_issued() {
        // GB/T 7714 §7.5.4.3: a note-field "issued: <year>印刷" override is
        // a printing/impression year, another publication-year substitute.
        // It must land on `printing`, leaving `issued` empty.
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-4".to_string(),
            ref_type: "book".to_string(),
            title: Some("Printed Book".to_string()),
            issued: Some(legacy_year(1995)),
            note: Some("issued: 1995印刷".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(monograph.printing, Some(DateValue::new("1995".to_string())));
        assert_eq!(monograph.issued, DateValue::new(String::new()));
    }

    #[test]
    fn legacy_estimated_year_note_override_folds_into_approximate_issued() {
        // GB/T 7714 §7.5.4.3: a note-field "issued: <year>~" override is an
        // estimated year — the same publication date, marked inferred. It
        // folds into `issued` itself as an EDTF approximate date, not a
        // separate field.
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-5".to_string(),
            ref_type: "book".to_string(),
            title: Some("Estimated-Date Book".to_string()),
            issued: Some(legacy_year(1936)),
            note: Some("issued: 1936~".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(monograph.issued, DateValue::new("1936~".to_string()));
        assert_eq!(monograph.copyright, None);
        assert_eq!(monograph.printing, None);
    }

    #[test]
    fn legacy_annotated_year_note_override_folds_into_annotated_issued() {
        // Calendar Date Annotations: a note-field "issued: <year>（<note>）"
        // override (full-width parens only) captures source-calendar
        // wording alongside the Gregorian year. It folds into `issued`
        // itself as an annotated `DateValue`, not a separate field.
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-6".to_string(),
            ref_type: "book".to_string(),
            title: Some("Annotated-Date Book".to_string()),
            issued: Some(legacy_year(1947)),
            note: Some("issued: 1947（民国三十六年）".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(
            monograph.issued,
            DateValue {
                value: "1947".to_string(),
                note: Some("民国三十六年".to_string()),
            }
        );
        assert_eq!(monograph.copyright, None);
        assert_eq!(monograph.printing, None);
    }

    #[test]
    fn legacy_half_width_parens_note_override_does_not_annotate_issued() {
        // Full-width parens only: a half-width-parenthesized note-field
        // override must not misfire the calendar-annotation path (it stays
        // disjoint from ordinary Latin-script parenthetical text). It falls
        // through to the plain structured `issued` date.
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-7".to_string(),
            ref_type: "book".to_string(),
            title: Some("Parenthetical Note Book".to_string()),
            issued: Some(legacy_year(1947)),
            note: Some("issued: 1947 (reprint)".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(monograph.issued, DateValue::new("1947".to_string()));
    }

    #[test]
    fn legacy_serial_component_maps_reviewed_relation_and_supplement_number() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "article-1".to_string(),
            ref_type: "article-journal".to_string(),
            title: Some("Review Essay".to_string()),
            container_title: Some("Journal".to_string()),
            issued: Some(legacy_year(2024)),
            extra: HashMap::from([
                ("reviewed-title".to_string(), json!("Reviewed Book")),
                (
                    "reviewed-author".to_string(),
                    json!([{"family":"Morrison","given":"Toni"}]),
                ),
                ("supplement-number".to_string(), json!("S1")),
            ]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::SerialComponent(component) = converted.extension() else {
            panic!("expected serial component");
        };
        assert!(
            component
                .numbering
                .iter()
                .any(|entry| entry.r#type == NumberingType::Supplement && entry.value == "S1")
        );
        let Some(WorkRelation::Embedded(reviewed)) = component.reviewed.as_ref() else {
            panic!("expected reviewed relation");
        };
        let ClassExtension::Monograph(reviewed_work) = reviewed.extension() else {
            panic!("expected reviewed monograph relation");
        };
        assert_eq!(
            reviewed_work.title,
            Some(Title::Single("Reviewed Book".to_string()))
        );
        assert!(reviewed_work.author.is_some());
    }

    #[test]
    fn legacy_serial_component_exposes_section_and_review_event_metadata() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "newspaper-review".to_string(),
            ref_type: "article-newspaper".to_string(),
            title: Some("Young Concert Artists Is Back".to_string()),
            container_title: Some("New York Times".to_string()),
            section: Some("Arts".to_string()),
            issued: Some(csl_legacy::csl_json::DateVariable::full(2021, 11, 12)),
            extra: HashMap::from([
                ("reviewed-genre".to_string(), json!("recital")),
                (
                    "reviewed-author".to_string(),
                    json!([{"family":"Zhu Wang (piano)","given":""}]),
                ),
                ("event-title".to_string(), json!("Zankel Hall")),
                ("event-place".to_string(), json!("New York")),
            ]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(converted.section(), Some("Arts".to_string()));
        assert!(
            converted
                .contributor(ContributorRole::Unknown("reviewed-author".to_string()))
                .is_some(),
            "reviewed author should be renderable from the top-level article component"
        );

        let ClassExtension::SerialComponent(component) = converted.extension() else {
            panic!("expected serial component");
        };
        let Some(WorkRelation::Embedded(event)) = component.event.as_ref() else {
            panic!("expected review event relation");
        };
        let ClassExtension::Event(event) = event.extension() else {
            panic!("expected event relation");
        };
        assert_eq!(event.title, Some(Title::Single("Zankel Hall".to_string())));
        assert_eq!(event.location, Some("New York".to_string()));
    }

    #[test]
    fn legacy_broadcast_preserves_writer_cast_network_and_duration() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "broadcast-rich".to_string(),
            ref_type: "broadcast".to_string(),
            title: Some("Her Sister's Shadow".to_string()),
            container_title: Some("The Brady Bunch".to_string()),
            number: Some("season 3, episode 10".to_string()),
            publisher: Some("ABC".to_string()),
            dimensions: Some("26m".to_string()),
            issued: Some(csl_legacy::csl_json::DateVariable::full(1971, 11, 19)),
            extra: HashMap::from([(
                "script-writer".to_string(),
                json!([{"family":"Schwartz","given":"Sherwood"}]),
            )]),
            contributor: Some(vec![csl_legacy::csl_json::Name {
                family: Some("Reed".to_string()),
                given: Some("Robert".to_string()),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(converted.ref_type(), "broadcast");
        let ClassExtension::SerialComponent(component) = converted.extension() else {
            panic!("expected serial component");
        };
        assert_eq!(component.issue.as_deref(), Some("season 3, episode 10"));
        assert_eq!(component.duration.as_deref(), Some("26m"));
        assert!(
            component
                .contributors
                .iter()
                .any(|entry| entry.roles.contains(&ContributorRole::Writer))
        );
        assert!(
            component
                .contributors
                .iter()
                .any(|entry| entry.roles.contains(&ContributorRole::Performer))
        );
    }

    #[test]
    fn legacy_event_prefers_extra_event_fields() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "event-1".to_string(),
            ref_type: "speech".to_string(),
            title: Some("Fallback Title".to_string()),
            issued: Some(legacy_year(2024)),
            extra: HashMap::from([
                ("event-title".to_string(), json!("Actual Event")),
                ("event-place".to_string(), json!("Chicago")),
                (
                    "event-date".to_string(),
                    json!({"date-parts":[[2023, 5, 6]]}),
                ),
            ]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Event(event) = converted.extension() else {
            panic!("expected event");
        };
        assert_eq!(
            event.title,
            Some(Title::Single("Fallback Title".to_string()))
        );
        assert_eq!(
            event.series.as_ref().and_then(|relation| match relation {
                WorkRelation::Embedded(parent) => parent.title(),
                WorkRelation::Id(_) => None,
            }),
            Some(Title::Single("Actual Event".to_string()))
        );
        assert_eq!(event.location, Some("Chicago".to_string()));
        assert_eq!(event.date, Some(DateValue::new("2023-05-06".to_string())));
    }

    #[test]
    fn legacy_event_omits_empty_fallback_date() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "event-2".to_string(),
            ref_type: "speech".to_string(),
            title: Some("Fallback Title".to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Event(event) = converted.extension() else {
            panic!("expected event");
        };
        assert_eq!(event.date, None);
    }

    #[test]
    fn legacy_broadcast_maps_executive_producer_to_producer() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "broadcast-1".to_string(),
            ref_type: "broadcast".to_string(),
            title: Some("Episode".to_string()),
            issued: Some(legacy_year(2024)),
            extra: HashMap::from([(
                "executive-producer".to_string(),
                json!([{"family":"Rhimes","given":"Shonda"}]),
            )]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::SerialComponent(work) = converted.extension() else {
            panic!("expected serial component");
        };
        assert!(
            work.contributors
                .iter()
                .any(|entry| entry.roles.contains(&ContributorRole::Producer))
        );
    }

    #[test]
    fn legacy_monograph_dedupes_extra_role_pushes() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-3".to_string(),
            ref_type: "book".to_string(),
            title: Some("Role Dedup".to_string()),
            issued: Some(legacy_year(2024)),
            extra: HashMap::from([
                (
                    "composer".to_string(),
                    json!([{"family":"Glass","given":"Philip"}]),
                ),
                (
                    "producer".to_string(),
                    json!([{"family":"Jones","given":"Quincy"}]),
                ),
                (
                    "executive-producer".to_string(),
                    json!([{"family":"Jones","given":"Quincy"}]),
                ),
            ]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };

        let composer_count = monograph
            .contributors
            .iter()
            .filter(|entry| entry.roles.contains(&ContributorRole::Composer))
            .count();
        let producer_count = monograph
            .contributors
            .iter()
            .filter(|entry| entry.roles.contains(&ContributorRole::Producer))
            .count();

        assert_eq!(
            composer_count, 1,
            "duplicate composer entry after conversion"
        );
        assert_eq!(
            producer_count, 1,
            "duplicate producer entry after conversion"
        );
    }

    #[test]
    fn legacy_monograph_prefers_part_title_over_title() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-4".to_string(),
            ref_type: "book".to_string(),
            title: Some("Container Work".to_string()),
            extra: HashMap::from([("part-title".to_string(), json!("Actual Part"))]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::Monograph(monograph) = converted.extension() else {
            panic!("expected monograph");
        };
        assert_eq!(
            monograph.title,
            Some(Title::Single("Actual Part".to_string()))
        );
    }

    #[test]
    fn legacy_collection_component_prefers_part_title_over_title() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "chapter-1".to_string(),
            ref_type: "chapter".to_string(),
            title: Some("Collected Volume".to_string()),
            extra: HashMap::from([("part-title".to_string(), json!("Actual Chapter"))]),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        let ClassExtension::CollectionComponent(component) = converted.extension() else {
            panic!("expected collection component");
        };
        assert_eq!(
            component.title,
            Some(Title::Single("Actual Chapter".to_string()))
        );
    }

    #[test]
    fn legacy_publisher_place_without_name_preserves_place() {
        let legacy = csl_legacy::csl_json::Reference {
            id: "book-5".to_string(),
            ref_type: "book".to_string(),
            title: Some("A history of Chinese mathematics".to_string()),
            publisher_place: Some("Cambridge, Eng".to_string()),
            issued: Some(legacy_year(1959)),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(
            converted.publisher(),
            Some(Publisher {
                name: String::new().into(),
                place: Some("Cambridge, Eng".to_string().into()),
            })
        );
        assert_eq!(
            converted.publisher_place(),
            Some("Cambridge, Eng".to_string())
        );
    }

    #[test]
    fn legacy_title_with_nocase_html_span_converts_to_djot_case_protection() {
        // Confirms the bean csl26-zaqk regression at the conversion boundary:
        // CSL-JSON titles carry citeproc-js's literal HTML rich-text
        // convention, which must become Djot on ingestion rather than
        // leaking verbatim into rendered output.
        let legacy = csl_legacy::csl_json::Reference {
            id: "loc-record".to_string(),
            ref_type: "webpage".to_string(),
            title: Some(r#"<span class="nocase">Library of Congress</span>"#.to_string()),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(
            converted.title(),
            Some(Title::Single("[Library of Congress]{.nocase}".to_string()))
        );
    }

    #[test]
    fn legacy_container_title_with_nocase_html_spans_converts_to_djot_case_protection() {
        // The gb7714-bench regression (bean csl26-6eoi): entry gbt7714.8.6.1:5's
        // `container-title` carries citeproc-js's literal HTML rich-text
        // convention, same as `title` in the csl26-zaqk case above, but through
        // a field `build_title` never sees. Must become Djot on ingestion
        // rather than leaking verbatim into rendered output.
        let legacy = csl_legacy::csl_json::Reference {
            id: "gbt7714.8.6.1:5".to_string(),
            ref_type: "book".to_string(),
            title: Some("Advances in holographic photoelasticity".to_string()),
            container_title: Some(
                r#"<span class="nocase">Symposium on Applications of Holography in Mechanics</span>, August 23-25, 1971, <span class="nocase">University of Southern California</span>, <span class="nocase">Los Angeles, California</span>"#
                    .to_string(),
            ),
            ..Default::default()
        };

        let converted = InputReference::from(legacy);

        assert_eq!(
            converted.container_title(),
            Some(Title::Single(
                "[Symposium on Applications of Holography in Mechanics]{.nocase}, \
                 August 23-25, 1971, [University of Southern California]{.nocase}, \
                 [Los Angeles, California]{.nocase}"
                    .to_string()
            ))
        );
    }

    #[test]
    fn normalize_rich_text_markup_converts_collection_title_and_leaves_note_untouched() {
        // Direct test of the new central pass: confirms it reaches a field
        // beyond `title`/`container-title` (collection-title, i.e. a book
        // series) and confirms the deliberate exclusion of `note`, which
        // `parse_note_field_hacks()` depends on parsing unconverted.
        let mut legacy = csl_legacy::csl_json::Reference {
            collection_title: Some(r#"<i>Springer Monographs</i>"#.to_string()),
            note: Some(r#"CSTR: <span class="nocase">unrelated</span>"#.to_string()),
            ..Default::default()
        };

        normalize_rich_text_markup(&mut legacy);

        assert_eq!(
            legacy.collection_title,
            Some("_Springer Monographs_".to_string())
        );
        assert_eq!(
            legacy.note,
            Some(r#"CSTR: <span class="nocase">unrelated</span>"#.to_string())
        );
    }
}
