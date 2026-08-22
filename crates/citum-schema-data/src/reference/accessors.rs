/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Typed read/mutation surface for [`InputReference`].
//!
//! All shared bibliographic data lives inside the class-specific payload in
//! `extension`; this file dispatches accessors through every known class.

use serde_json::Value as JsonValue;
use url::Url;

use super::classes::class_dispatch;
use super::contributor::{Contributor, ContributorEntry, ContributorList, ContributorRole};
use super::date::DateValue;
use super::types::common::{
    FieldLanguageMap, HasNumbering, LangID, MultilingualString, NumOrStr, NumberingType, Publisher,
    RefID, RichText, Title,
};
use super::types::legal::{Brief, Hearing, LegalCase, Regulation, Statute, Treaty};
use super::types::specialized::{
    AudioVisualType, AudioVisualWork, Classic, Dataset, Event, Patent, Software, Standard,
};
use super::types::structural::{
    Collection, CollectionComponent, CollectionType, Monograph, MonographComponentType,
    MonographType, Serial, SerialComponent, SerialType,
};
use super::{
    ClassExtension, EMPTY_FIELD_LANGUAGES, IdentifierName, InputReference, ReferenceClass,
    SupplementaryIdentifiers, UnknownClassData, WorkRelation,
};

impl InputReference {
    /// Return all supplementary standardized identifiers.
    #[must_use]
    pub fn identifiers(&self) -> &SupplementaryIdentifiers {
        &self.identifiers
    }

    /// Return one supplementary standardized identifier.
    #[must_use]
    pub fn identifier(&self, name: &str) -> Option<&str> {
        self.identifiers.get(name)
    }

    /// Insert one supplementary standardized identifier.
    pub fn insert_identifier(
        &mut self,
        name: IdentifierName,
        value: impl Into<String>,
    ) -> Option<String> {
        self.identifiers.insert(name, value)
    }

    /// Return the typed class discriminator.
    #[must_use]
    pub fn class(&self) -> ReferenceClass {
        self.extension.reference_class()
    }

    /// Return the active class-specific overlay.
    #[must_use]
    pub fn extension(&self) -> &ClassExtension {
        &self.extension
    }

    /// Return the mutable active class-specific overlay.
    #[must_use]
    pub fn extension_mut(&mut self) -> &mut ClassExtension {
        &mut self.extension
    }

    /// Return monograph data when this reference is a monograph.
    #[must_use]
    pub fn as_monograph(&self) -> Option<&Monograph> {
        match &self.extension {
            ClassExtension::Monograph(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return collection-component data when this reference is a collection component.
    #[must_use]
    pub fn as_collection_component(&self) -> Option<&CollectionComponent> {
        match &self.extension {
            ClassExtension::CollectionComponent(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return serial-component data when this reference is a serial component.
    #[must_use]
    pub fn as_serial_component(&self) -> Option<&SerialComponent> {
        match &self.extension {
            ClassExtension::SerialComponent(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return collection data when this reference is a collection.
    #[must_use]
    pub fn as_collection(&self) -> Option<&Collection> {
        match &self.extension {
            ClassExtension::Collection(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return serial data when this reference is a serial.
    #[must_use]
    pub fn as_serial(&self) -> Option<&Serial> {
        match &self.extension {
            ClassExtension::Serial(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return legal-case data when this reference is a legal case.
    #[must_use]
    pub fn as_legal_case(&self) -> Option<&LegalCase> {
        match &self.extension {
            ClassExtension::LegalCase(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return statute data when this reference is a statute.
    #[must_use]
    pub fn as_statute(&self) -> Option<&Statute> {
        match &self.extension {
            ClassExtension::Statute(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return treaty data when this reference is a treaty.
    #[must_use]
    pub fn as_treaty(&self) -> Option<&Treaty> {
        match &self.extension {
            ClassExtension::Treaty(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return hearing data when this reference is a hearing.
    #[must_use]
    pub fn as_hearing(&self) -> Option<&Hearing> {
        match &self.extension {
            ClassExtension::Hearing(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return regulation data when this reference is a regulation.
    #[must_use]
    pub fn as_regulation(&self) -> Option<&Regulation> {
        match &self.extension {
            ClassExtension::Regulation(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return brief data when this reference is a brief.
    #[must_use]
    pub fn as_brief(&self) -> Option<&Brief> {
        match &self.extension {
            ClassExtension::Brief(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return classic-work data when this reference is a classic.
    #[must_use]
    pub fn as_classic(&self) -> Option<&Classic> {
        match &self.extension {
            ClassExtension::Classic(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return patent data when this reference is a patent.
    #[must_use]
    pub fn as_patent(&self) -> Option<&Patent> {
        match &self.extension {
            ClassExtension::Patent(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return dataset data when this reference is a dataset.
    #[must_use]
    pub fn as_dataset(&self) -> Option<&Dataset> {
        match &self.extension {
            ClassExtension::Dataset(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return standard data when this reference is a standard.
    #[must_use]
    pub fn as_standard(&self) -> Option<&Standard> {
        match &self.extension {
            ClassExtension::Standard(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return software data when this reference is software.
    #[must_use]
    pub fn as_software(&self) -> Option<&Software> {
        match &self.extension {
            ClassExtension::Software(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return event data when this reference is an event.
    #[must_use]
    pub fn as_event(&self) -> Option<&Event> {
        match &self.extension {
            ClassExtension::Event(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return audio-visual data when this reference is audio-visual.
    #[must_use]
    pub fn as_audio_visual(&self) -> Option<&AudioVisualWork> {
        match &self.extension {
            ClassExtension::AudioVisual(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Return unknown-class data when this reference names an unknown class.
    #[must_use]
    pub fn unknown_class(&self) -> Option<&UnknownClassData> {
        match &self.extension {
            ClassExtension::Unknown(data) => Some(data),
            _ => None,
        }
    }

    fn numbered(&self) -> Option<&dyn HasNumbering> {
        match &self.extension {
            ClassExtension::Monograph(reference) => Some(reference.as_ref()),
            ClassExtension::Collection(reference) => Some(reference.as_ref()),
            ClassExtension::CollectionComponent(reference) => Some(reference.as_ref()),
            ClassExtension::SerialComponent(reference) => Some(reference.as_ref()),
            ClassExtension::Classic(reference) => Some(reference.as_ref()),
            ClassExtension::AudioVisual(reference) => Some(reference.as_ref()),
            _ => None,
        }
    }

    /// Internal helper to find a numbering by type.
    fn find_numbering(&self, numbering_type: NumberingType) -> Option<String> {
        self.numbered()
            .and_then(|reference| reference.find_numbering(numbering_type))
    }

    /// Return the numbering value for an arbitrary numbering kind.
    pub fn numbering_value(&self, numbering_type: &NumberingType) -> Option<String> {
        self.find_numbering(numbering_type.clone())
    }

    /// Return the reference ID.
    pub fn id(&self) -> Option<RefID> {
        class_dispatch!(&self.extension, |r| r.id.clone(), unknown(data) => data
                .fields
                .get("id")
                .and_then(JsonValue::as_str)
                .map(|id| RefID(id.to_string())))
    }

    /// Return the author.
    pub fn author(&self) -> Option<Contributor> {
        use crate::reference::contributor::ContributorRole as DataRole;

        match &self.extension {
            ClassExtension::Monograph(r) => {
                collect_contributors_by_role(&r.contributors, &DataRole::Author)
                    .or_else(|| r.author.clone())
            }
            ClassExtension::CollectionComponent(r) => {
                collect_contributors_by_role(&r.contributors, &DataRole::Author)
                    .or_else(|| r.author.clone())
            }
            ClassExtension::SerialComponent(r) => {
                let explicit_author =
                    collect_contributors_by_role(&r.contributors, &DataRole::Author)
                        .or_else(|| r.author.clone());

                explicit_author.or_else(|| {
                    let av_like = r
                        .medium
                        .as_deref()
                        .map(|value| {
                            let lowered = value.to_ascii_lowercase();
                            lowered.contains("podcast")
                                || lowered.contains("tv")
                                || lowered.contains("film")
                                || lowered.contains("video")
                        })
                        .unwrap_or(false)
                        || r.genre
                            .as_deref()
                            .map(|value| {
                                let lowered = value.to_ascii_lowercase();
                                lowered.contains("broadcast") || lowered.contains("film")
                            })
                            .unwrap_or(false);

                    if av_like {
                        collect_contributors_by_role(&r.contributors, &DataRole::Producer).or_else(
                            || collect_contributors_by_role(&r.contributors, &DataRole::Host),
                        )
                    } else {
                        None
                    }
                })
            }
            ClassExtension::Treaty(r) => r.author.clone(),
            ClassExtension::Brief(r) => r.author.clone(),
            ClassExtension::Classic(r) => r.author.clone(),
            ClassExtension::Patent(r) => r.author.clone(),
            ClassExtension::Dataset(r) => r.author.clone(),
            ClassExtension::Software(r) => r.author.clone(),
            ClassExtension::Event(r) => {
                collect_contributors_by_role(&r.contributors, &DataRole::Performer)
                    .or_else(|| {
                        collect_contributors_by_role(
                            &r.contributors,
                            &DataRole::Unknown("organizer".to_string()),
                        )
                    })
                    .or_else(|| collect_contributors_by_role(&r.contributors, &DataRole::Author))
            }
            ClassExtension::AudioVisual(r) => {
                let explicit_author =
                    collect_contributors_by_role(&r.core.contributors, &DataRole::Author);

                match r.r#type {
                    AudioVisualType::Film | AudioVisualType::Episode => {
                        explicit_author.or_else(|| {
                            collect_contributors_by_role(&r.core.contributors, &DataRole::Director)
                        })
                    }
                    AudioVisualType::Recording => explicit_author.or_else(|| {
                        collect_contributors_by_role(&r.core.contributors, &DataRole::Composer)
                            .or_else(|| {
                                collect_contributors_by_role(
                                    &r.core.contributors,
                                    &DataRole::Performer,
                                )
                            })
                    }),
                    AudioVisualType::Broadcast => explicit_author
                        .or_else(|| {
                            collect_contributors_by_role(&r.core.contributors, &DataRole::Director)
                        })
                        .or_else(|| {
                            collect_contributors_by_role(&r.core.contributors, &DataRole::Producer)
                        }),
                }
            }
            _ => None,
        }
    }

    pub fn editor(&self) -> Option<Contributor> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::Collection(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::CollectionComponent(r) => r
                .container
                .as_ref()
                .and_then(|c| match c {
                    WorkRelation::Embedded(p) => p.editor(),
                    WorkRelation::Id(_) => None,
                })
                .or_else(|| {
                    collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                }),
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.editor(),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::Serial(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::Classic(r) => r.editor.clone(),
            ClassExtension::AudioVisual(_) => None,
            _ => None,
        }
    }

    /// Return an editor attached directly to this work, excluding an enclosing container's editor.
    pub fn local_editor(&self) -> Option<Contributor> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::Collection(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::CollectionComponent(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
            }
            ClassExtension::Serial(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Editor)
                    .or_else(|| r.editor.clone())
            }
            ClassExtension::Classic(r) => r.editor.clone(),
            _ => None,
        }
    }

    /// Return the translator.
    pub fn translator(&self) -> Option<Contributor> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Translator)
                    .or_else(|| r.translator.clone())
            }
            ClassExtension::CollectionComponent(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Translator)
                    .or_else(|| r.translator.clone())
            }
            ClassExtension::SerialComponent(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Translator)
                    .or_else(|| r.translator.clone())
            }
            ClassExtension::Collection(r) => {
                collect_contributors_by_role(&r.contributors, &ContributorRole::Translator)
                    .or_else(|| r.translator.clone())
            }
            ClassExtension::Classic(r) => r.translator.clone(),
            ClassExtension::AudioVisual(r) => {
                collect_contributors_by_role(&r.core.contributors, &ContributorRole::Translator)
            }
            _ => None,
        }
    }

    /// Return the publisher.
    pub fn publisher(&self) -> Option<Publisher> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.publisher.clone(),
            ClassExtension::CollectionComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher(),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher(),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::Collection(r) => r.publisher.clone(),
            ClassExtension::Serial(r) => r.publisher.clone(),
            ClassExtension::Classic(r) => r.publisher.clone(),
            ClassExtension::Dataset(r) => r.publisher.clone(),
            ClassExtension::Standard(r) => r.publisher.clone(),
            ClassExtension::Software(r) => r.publisher.clone(),
            ClassExtension::AudioVisual(r) => r.publisher.clone(),
            _ => None,
        }
    }

    /// Returns contributors matching `role` for any reference class that
    /// carries a contributors list.
    ///
    /// Returns `None` if no contributors with the given role exist.
    /// Returns a single `Contributor` directly, or folds multiple into a `ContributorList`.
    pub fn contributor(&self, role: ContributorRole) -> Option<Contributor> {
        let entries = self.all_contributor_entries();
        collect_contributors_by_role(entries, &role)
    }

    /// Return all contributor entries matching the requested role.
    pub fn contributor_entries(&self, role: &ContributorRole) -> Vec<&ContributorEntry> {
        self.all_contributor_entries()
            .iter()
            .filter(|entry| entry.roles.contains(role))
            .collect()
    }

    /// Return all contributor entries regardless of role.
    pub fn all_contributor_entries(&self) -> &[ContributorEntry] {
        match &self.extension {
            ClassExtension::Monograph(r) => &r.contributors,
            ClassExtension::Collection(r) => &r.contributors,
            ClassExtension::CollectionComponent(r) => &r.contributors,
            ClassExtension::Serial(r) => &r.contributors,
            ClassExtension::SerialComponent(r) => &r.contributors,
            ClassExtension::Event(r) => &r.contributors,
            ClassExtension::AudioVisual(r) => &r.core.contributors,
            _ => &[],
        }
    }

    /// Return the title.
    pub fn title(&self) -> Option<Title> {
        match &self.extension {
            ClassExtension::Monograph(r) => match (&r.title, &r.short_title) {
                (Some(Title::Single(long)), Some(short)) => {
                    Some(Title::Shorthand(short.clone(), long.clone()))
                }
                _ => r.title.clone(),
            },
            ClassExtension::CollectionComponent(r) => r.title.clone(),
            ClassExtension::SerialComponent(r) => r.title.clone(),
            ClassExtension::Collection(r) => match (&r.title, &r.short_title) {
                (Some(Title::Single(long)), Some(short)) => {
                    Some(Title::Shorthand(short.clone(), long.clone()))
                }
                _ => r.title.clone(),
            },
            ClassExtension::Serial(r) => match (&r.title, &r.short_title) {
                (Some(Title::Single(long)), Some(short)) => {
                    Some(Title::Shorthand(short.clone(), long.clone()))
                }
                _ => r.title.clone(),
            },
            ClassExtension::LegalCase(r) => r.title.clone(),
            ClassExtension::Statute(r) => r.title.clone(),
            ClassExtension::Treaty(r) => r.title.clone(),
            ClassExtension::Hearing(r) => r.title.clone(),
            ClassExtension::Regulation(r) => r.title.clone(),
            ClassExtension::Brief(r) => r.title.clone(),
            ClassExtension::Classic(r) => r.title.clone(),
            ClassExtension::Patent(r) => r.title.clone(),
            ClassExtension::Dataset(r) => r.title.clone(),
            ClassExtension::Standard(r) => r.title.clone(),
            ClassExtension::Software(r) => r.title.clone(),
            ClassExtension::Event(r) => r.title.clone(),
            ClassExtension::AudioVisual(r) => match (&r.core.title, &r.core.short_title) {
                (Some(Title::Single(long)), Some(short)) => {
                    Some(Title::Shorthand(short.clone(), long.clone()))
                }
                _ => r.core.title.clone(),
            },
            ClassExtension::Unknown(data) => data
                .fields
                .get("title")
                .and_then(JsonValue::as_str)
                .map(|title| Title::Single(title.to_string())),
        }
    }

    /// Return the title of an individual volume.
    pub fn volume_title(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.volume_title.clone(),
            _ => None,
        }
    }

    fn non_empty_date(date: DateValue) -> Option<DateValue> {
        if date.is_empty() { None } else { Some(date) }
    }

    /// Return the creation or origination date.
    pub fn created(&self) -> Option<DateValue> {
        match &self.extension {
            ClassExtension::Monograph(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::CollectionComponent(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::SerialComponent(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Collection(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Serial(_) => None,
            ClassExtension::LegalCase(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Statute(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Treaty(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Hearing(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Regulation(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Brief(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Classic(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Patent(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Dataset(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Standard(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Software(r) => Self::non_empty_date(r.created.clone()),
            ClassExtension::Event(_) => None,
            ClassExtension::AudioVisual(r) => Self::non_empty_date(r.core.created.clone()),
            ClassExtension::Unknown(_) => None,
        }
    }

    /// Return the explicit publication or release date.
    pub fn issued(&self) -> Option<DateValue> {
        match &self.extension {
            ClassExtension::Monograph(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::CollectionComponent(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::SerialComponent(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Collection(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Serial(_) => None,
            ClassExtension::LegalCase(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Statute(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Treaty(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Hearing(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Regulation(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Brief(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Classic(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Patent(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Dataset(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Standard(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Software(r) => Self::non_empty_date(r.issued.clone()),
            ClassExtension::Event(r) => r.date.clone(),
            ClassExtension::AudioVisual(r) => Self::non_empty_date(r.core.issued.clone()),
            ClassExtension::Unknown(_) => None,
        }
    }

    /// Return the effective issued date, falling back to the created date when
    /// no explicit issued date is present.
    pub fn effective_issued_date(&self) -> Option<DateValue> {
        self.issued().or_else(|| self.created())
    }

    /// Return the copyright year, a publication-year substitute used when
    /// the true issue date is unknown (GB/T 7714 §7.5.4.3's `c1988`).
    pub fn copyright(&self) -> Option<DateValue> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.copyright.clone().and_then(Self::non_empty_date),
            _ => None,
        }
    }

    /// Return the printing/impression year, another publication-year
    /// substitute (GB/T 7714 §7.5.4.3's `1995印刷`).
    pub fn printing(&self) -> Option<DateValue> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.printing.clone().and_then(Self::non_empty_date),
            _ => None,
        }
    }

    /// Return the DOI.
    pub fn doi(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.doi.clone(),
            ClassExtension::CollectionComponent(r) => r.doi.clone(),
            ClassExtension::SerialComponent(r) => r.doi.clone(),
            ClassExtension::LegalCase(r) => r.doi.clone(),
            ClassExtension::Dataset(r) => r.doi.clone(),
            ClassExtension::Standard(r) => r.doi.clone(),
            ClassExtension::Software(r) => r.doi.clone(),
            ClassExtension::AudioVisual(_) => None,
            _ => None,
        }
    }

    /// Return the ADS bibcode.
    pub fn ads_bibcode(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.ads_bibcode.clone(),
            ClassExtension::SerialComponent(r) => r.ads_bibcode.clone(),
            ClassExtension::AudioVisual(_) => None,
            _ => None,
        }
    }

    /// Return the note.
    pub fn note(&self) -> Option<RichText> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.note.clone(),
            ClassExtension::CollectionComponent(r) => r.note.clone(),
            ClassExtension::SerialComponent(r) => r.note.clone(),
            ClassExtension::Collection(r) => r.note.clone(),
            ClassExtension::Serial(r) => r.note.clone(),
            ClassExtension::LegalCase(r) => r.note.clone(),
            ClassExtension::Statute(r) => r.note.clone(),
            ClassExtension::Treaty(r) => r.note.clone(),
            ClassExtension::Standard(r) => r.note.clone(),
            ClassExtension::Event(r) => r.note.clone(),
            ClassExtension::AudioVisual(r) => r.note.clone(),
            _ => None,
        }
    }

    /// Return the URL.
    pub fn url(&self) -> Option<Url> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.url.clone(),
            ClassExtension::CollectionComponent(r) => r.url.clone(),
            ClassExtension::SerialComponent(r) => r.url.clone(),
            ClassExtension::Collection(r) => r.url.clone(),
            ClassExtension::Serial(r) => r.url.clone(),
            ClassExtension::LegalCase(r) => r.url.clone(),
            ClassExtension::Statute(r) => r.url.clone(),
            ClassExtension::Treaty(r) => r.url.clone(),
            ClassExtension::Hearing(r) => r.url.clone(),
            ClassExtension::Regulation(r) => r.url.clone(),
            ClassExtension::Brief(r) => r.url.clone(),
            ClassExtension::Classic(r) => r.url.clone(),
            ClassExtension::Patent(r) => r.url.clone(),
            ClassExtension::Dataset(r) => r.url.clone(),
            ClassExtension::Standard(r) => r.url.clone(),
            ClassExtension::Software(r) => r.url.clone(),
            ClassExtension::Event(r) => r.url.clone(),
            ClassExtension::AudioVisual(r) => r.url.clone(),
            ClassExtension::Unknown(_) => None,
        }
    }

    /// Return the publisher place.
    pub fn publisher_place(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into))
                .or_else(|| {
                    r.container.as_ref().and_then(|c| match c {
                        WorkRelation::Embedded(p) => p.publisher_place(),
                        _ => None,
                    })
                }),
            ClassExtension::CollectionComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher_place(),
                _ => None,
            }),
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher_place(),
                _ => None,
            }),
            ClassExtension::Collection(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into))
                .or_else(|| {
                    r.container.as_ref().and_then(|c| match c {
                        WorkRelation::Embedded(p) => p.publisher_place(),
                        _ => None,
                    })
                }),
            ClassExtension::Serial(_) => None,
            ClassExtension::Classic(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into)),
            ClassExtension::Dataset(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into)),
            ClassExtension::Standard(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into)),
            ClassExtension::Software(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into)),
            ClassExtension::Event(r) => r.location.clone(),
            ClassExtension::AudioVisual(r) => r
                .publisher
                .as_ref()
                .and_then(|p| p.place.clone().map(Into::into)),
            _ => None,
        }
    }

    /// Return the publisher as a string.
    pub fn publisher_str(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .publisher
                .as_ref()
                .map(|p| p.name.to_string())
                .or_else(|| {
                    r.container.as_ref().and_then(|c| match c {
                        WorkRelation::Embedded(p) => p.publisher_str(),
                        _ => None,
                    })
                }),
            ClassExtension::CollectionComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher_str(),
                _ => None,
            }),
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.publisher_str(),
                _ => None,
            }),
            ClassExtension::Collection(r) => r
                .publisher
                .as_ref()
                .map(|p| p.name.to_string())
                .or_else(|| {
                    r.container.as_ref().and_then(|c| match c {
                        WorkRelation::Embedded(p) => p.publisher_str(),
                        _ => None,
                    })
                }),
            ClassExtension::Serial(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            ClassExtension::Classic(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            ClassExtension::Dataset(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            ClassExtension::Standard(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            ClassExtension::Software(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            ClassExtension::Event(r) => r.network.clone(),
            ClassExtension::AudioVisual(r) => r.publisher.as_ref().map(|p| p.name.to_string()),
            _ => None,
        }
    }

    /// Normalize genre/medium values to canonical kebab-case (defensive fallback for legacy producers).
    ///
    /// Converts to ASCII lowercase and replaces whitespace/underscores with dashes.
    pub(super) fn normalize_genre_medium(s: &str) -> String {
        let lower = s.to_ascii_lowercase();
        lower
            .split(|c: char| c.is_whitespace() || c == '_')
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Return the genre/type as string, normalized to canonical kebab-case.
    pub fn genre(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                r.genre.as_ref().map(|g| Self::normalize_genre_medium(g))
            }
            ClassExtension::CollectionComponent(r) => {
                r.genre.as_ref().map(|g| Self::normalize_genre_medium(g))
            }
            ClassExtension::SerialComponent(r) => {
                r.genre.as_ref().map(|g| Self::normalize_genre_medium(g))
            }
            ClassExtension::Event(r) => r.genre.as_ref().map(|g| Self::normalize_genre_medium(g)),
            ClassExtension::AudioVisual(r) => r
                .core
                .genre
                .as_ref()
                .map(|g| Self::normalize_genre_medium(g)),
            ClassExtension::Dataset(r) => r.genre.as_ref().map(|g| Self::normalize_genre_medium(g)),
            _ => None,
        }
    }

    /// Return the archive or repository name.
    pub fn archive(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive.clone(),
            _ => None,
        }
    }

    /// Return the archive shelfmark or repository location.
    pub fn archive_location(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .archive_info
                .as_ref()
                .and_then(|info| info.location.clone())
                .or_else(|| r.archive_location.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.location.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.location.clone())
            }
            _ => None,
        }
    }

    /// Return the archive name from structured ArchiveInfo.
    pub fn archive_name(&self) -> Option<MultilingualString> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.name.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.name.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.name.clone())
            }
            _ => None,
        }
    }

    /// Return the archive geographic place from structured ArchiveInfo.
    pub fn archive_place(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.place.clone().map(Into::into)),
            ClassExtension::CollectionComponent(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.place.clone().map(Into::into)),
            ClassExtension::SerialComponent(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.place.clone().map(Into::into)),
            _ => None,
        }
    }

    /// Return the archive collection name from structured ArchiveInfo.
    pub fn archive_collection(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                r.archive_info.as_ref().and_then(|i| i.collection.clone())
            }
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.collection.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.collection.clone())
            }
            _ => None,
        }
    }

    /// Return the archive collection identifier from structured ArchiveInfo.
    pub fn archive_collection_id(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.collection_id.clone()),
            ClassExtension::CollectionComponent(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.collection_id.clone()),
            ClassExtension::SerialComponent(r) => r
                .archive_info
                .as_ref()
                .and_then(|i| i.collection_id.clone()),
            _ => None,
        }
    }

    /// Return the archive series from structured ArchiveInfo.
    pub fn archive_series(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.series.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.series.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.series.clone())
            }
            _ => None,
        }
    }

    /// Return the archive box number from structured ArchiveInfo.
    pub fn archive_box(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.r#box.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.r#box.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.r#box.clone())
            }
            _ => None,
        }
    }

    /// Return the archive folder from structured ArchiveInfo.
    pub fn archive_folder(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.folder.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.folder.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.folder.clone())
            }
            _ => None,
        }
    }

    /// Return the archive item identifier from structured ArchiveInfo.
    pub fn archive_item(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.item.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.item.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.item.clone())
            }
            _ => None,
        }
    }

    /// Return the archive URL from structured ArchiveInfo.
    pub fn archive_url(&self) -> Option<Url> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.archive_info.as_ref().and_then(|i| i.url.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.url.clone())
            }
            ClassExtension::SerialComponent(r) => {
                r.archive_info.as_ref().and_then(|i| i.url.clone())
            }
            _ => None,
        }
    }

    /// Return the publication status.
    pub fn status(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.status.clone(),
            ClassExtension::CollectionComponent(r) => r.status.clone(),
            ClassExtension::SerialComponent(r) => r.status.clone(),
            ClassExtension::Standard(r) => r.status.clone(),
            _ => None,
        }
    }

    /// Return the eprint identifier.
    pub fn eprint_id(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.eprint.as_ref().map(|e| e.id.clone()),
            ClassExtension::CollectionComponent(r) => r.eprint.as_ref().map(|e| e.id.clone()),
            ClassExtension::SerialComponent(r) => r.eprint.as_ref().map(|e| e.id.clone()),
            _ => None,
        }
    }

    /// Return the eprint server name.
    pub fn eprint_server(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.eprint.as_ref().map(|e| e.server.clone()),
            ClassExtension::CollectionComponent(r) => r.eprint.as_ref().map(|e| e.server.clone()),
            ClassExtension::SerialComponent(r) => r.eprint.as_ref().map(|e| e.server.clone()),
            _ => None,
        }
    }

    /// Return the eprint subject class.
    pub fn eprint_class(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.eprint.as_ref().and_then(|e| e.class.clone()),
            ClassExtension::CollectionComponent(r) => {
                r.eprint.as_ref().and_then(|e| e.class.clone())
            }
            ClassExtension::SerialComponent(r) => r.eprint.as_ref().and_then(|e| e.class.clone()),
            _ => None,
        }
    }

    /// Return the medium, normalized to canonical kebab-case.
    pub fn medium(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => {
                r.medium.as_ref().map(|m| Self::normalize_genre_medium(m))
            }
            ClassExtension::CollectionComponent(r) => {
                r.medium.as_ref().map(|m| Self::normalize_genre_medium(m))
            }
            ClassExtension::SerialComponent(r) => {
                r.medium.as_ref().map(|m| Self::normalize_genre_medium(m))
            }
            ClassExtension::AudioVisual(r) => {
                r.medium.as_ref().map(|m| Self::normalize_genre_medium(m))
            }
            // Software carries the CSL `medium` value under `platform` (see
            // `from_software_ref`); unlike the other classes, GB/T 7714's oracle
            // expects the carrier code verbatim (e.g. "Apparatus"), not
            // kebab-cased.
            ClassExtension::Software(r) => r.platform.clone(),
            _ => None,
        }
    }

    /// Return the version.
    pub fn version(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.version.clone(),
            ClassExtension::Dataset(r) => r.version.clone(),
            ClassExtension::Software(r) => r.version.clone(),
            _ => None,
        }
    }

    /// Return the cartographic scale.
    pub fn scale(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.scale.clone(),
            _ => None,
        }
    }

    /// Return the abstract.
    pub fn abstract_text(&self) -> Option<RichText> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.abstract_text.clone(),
            ClassExtension::CollectionComponent(r) => r.abstract_text.clone(),
            ClassExtension::SerialComponent(r) => r.abstract_text.clone(),
            _ => None,
        }
    }

    /// Return the container-style title for parent works, reporters, or codes.
    pub fn container_title(&self) -> Option<Title> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::CollectionComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::Serial(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::LegalCase(r) => r.reporter.clone().map(Title::Single),
            ClassExtension::Statute(r) => r.code.clone().map(Title::Single),
            ClassExtension::Regulation(r) => r.code.clone().map(Title::Single),
            ClassExtension::Treaty(r) => r.reporter.clone().map(Title::Single),
            ClassExtension::Event(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::AudioVisual(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.title().or_else(|| p.container_title()),
                WorkRelation::Id(_) => None,
            }),
            _ => None,
        }
    }

    /// Return the series or collection title for a parent work.
    pub fn collection_title(&self) -> Option<Title> {
        fn nested_collection_title(relation: &WorkRelation) -> Option<Title> {
            let WorkRelation::Embedded(parent) = relation else {
                return None;
            };
            let container = match parent.extension() {
                ClassExtension::Monograph(r) => r.container.as_ref(),
                ClassExtension::Collection(r) => r.container.as_ref(),
                ClassExtension::Serial(r) => r.container.as_ref(),
                _ => None,
            }?;
            match container {
                WorkRelation::Embedded(collection) => collection.title(),
                WorkRelation::Id(_) => None,
            }
        }

        match &self.extension {
            ClassExtension::Monograph(r) => r.container.as_ref().and_then(nested_collection_title),
            ClassExtension::Collection(r) => {
                r.container.as_ref().and_then(|relation| match relation {
                    WorkRelation::Embedded(collection) => collection.title(),
                    WorkRelation::Id(_) => None,
                })
            }
            ClassExtension::CollectionComponent(r) => {
                r.container.as_ref().and_then(nested_collection_title)
            }
            ClassExtension::SerialComponent(r) => {
                r.container.as_ref().and_then(nested_collection_title)
            }
            _ => None,
        }
    }

    /// Return the volume.
    pub fn volume(&self) -> Option<NumOrStr> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .volume
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Volume))
                .map(NumOrStr::Str),
            ClassExtension::Collection(r) => r
                .volume
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Volume))
                .map(NumOrStr::Str),
            ClassExtension::CollectionComponent(r) => r
                .volume
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Volume))
                .or_else(|| {
                    r.container.as_ref().and_then(|container| match container {
                        WorkRelation::Embedded(parent) => {
                            parent.volume().map(|volume| volume.to_string())
                        }
                        WorkRelation::Id(_) => None,
                    })
                })
                .map(NumOrStr::Str),
            ClassExtension::SerialComponent(r) => r
                .volume
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Volume))
                .map(NumOrStr::Str),
            ClassExtension::Classic(r) => r
                .volume
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Volume))
                .map(NumOrStr::Str),
            ClassExtension::LegalCase(r) => r.volume.clone().map(NumOrStr::Str),
            ClassExtension::Statute(r) => r.volume.clone().map(NumOrStr::Str),
            ClassExtension::Treaty(r) => r.volume.clone().map(NumOrStr::Str),
            ClassExtension::Regulation(r) => r.volume.clone().map(NumOrStr::Str),
            _ => self
                .find_numbering(NumberingType::Volume)
                .map(NumOrStr::Str),
        }
    }

    /// Return the total number of volumes in a multi-volume work.
    pub fn number_of_volumes(&self) -> Option<String> {
        self.find_numbering(NumberingType::Custom("number-of-volumes".to_string()))
    }

    /// Return the collection number (series number).
    pub fn collection_number(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::CollectionComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(p) => p.collection_number(),
                WorkRelation::Id(_) => None,
            }),
            _ => self.find_numbering(NumberingType::Volume),
        }
    }

    /// Return the issue.
    pub fn issue(&self) -> Option<NumOrStr> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .issue
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Issue))
                .map(NumOrStr::Str),
            ClassExtension::Collection(r) => r
                .issue
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Issue))
                .map(NumOrStr::Str),
            ClassExtension::CollectionComponent(r) => r
                .issue
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Issue))
                .map(NumOrStr::Str),
            ClassExtension::SerialComponent(r) => r
                .issue
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Issue))
                .map(NumOrStr::Str),
            ClassExtension::Classic(r) => r
                .issue
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Issue))
                .map(NumOrStr::Str),
            _ => self.find_numbering(NumberingType::Issue).map(NumOrStr::Str),
        }
    }

    /// Return the pages.
    pub fn pages(&self) -> Option<NumOrStr> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.pages.clone(),
            ClassExtension::CollectionComponent(r) => r.pages.clone(),
            ClassExtension::SerialComponent(r) => r.pages.clone().map(NumOrStr::Str),
            ClassExtension::LegalCase(r) => r.page.clone().map(NumOrStr::Str),
            ClassExtension::Statute(r) => r.page.clone().map(NumOrStr::Str),
            ClassExtension::Treaty(r) => r.page.clone().map(NumOrStr::Str),
            ClassExtension::Patent(r) => r.pages.clone().map(NumOrStr::Str),
            _ => None,
        }
    }

    /// Return the authority (court, legislative body, standards org, etc.).
    pub fn authority(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::LegalCase(r) => r.authority.clone(),
            ClassExtension::Statute(r) => r.authority.clone(),
            ClassExtension::Hearing(r) => r.authority.clone(),
            ClassExtension::Regulation(r) => r.authority.clone(),
            ClassExtension::Brief(r) => r.authority.clone(),
            ClassExtension::Patent(r) => r.authority.clone(),
            ClassExtension::Standard(r) => r.authority.clone(),
            _ => None,
        }
    }

    /// Return the reporter (legal reporter series).
    pub fn reporter(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::LegalCase(r) => r.reporter.clone(),
            ClassExtension::Treaty(r) => r.reporter.clone(),
            _ => None,
        }
    }

    /// Return the code (legal code abbreviation).
    pub fn code(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Statute(r) => r.code.clone(),
            ClassExtension::Regulation(r) => r.code.clone(),
            _ => None,
        }
    }

    /// Return the section identifier for legal, classic, or serial-component references.
    pub fn section(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Statute(r) => r.section.clone(),
            ClassExtension::Regulation(r) => r.section.clone(),
            ClassExtension::Classic(_) => self.find_numbering(NumberingType::Section),
            ClassExtension::SerialComponent(r) => r.section.clone(),
            _ => None,
        }
    }

    /// Return the generic document number.
    pub fn number(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .number
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Number)),
            ClassExtension::Statute(r) => r.number.clone(),
            ClassExtension::Hearing(r) => r.session_number.clone(),
            ClassExtension::Collection(r) => r
                .number
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Number)),
            ClassExtension::CollectionComponent(r) => r
                .number
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Number)),
            ClassExtension::SerialComponent(r) => r
                .number
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Number)),
            ClassExtension::Classic(r) => r
                .number
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Number)),
            _ => self.find_numbering(NumberingType::Number),
        }
    }

    /// Return the report identifier.
    pub fn report_number(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(_) => self.find_numbering(NumberingType::Report),
            _ => None,
        }
    }

    /// Return the edition.
    pub fn edition(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r
                .edition
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Edition)),
            ClassExtension::Collection(r) => r
                .edition
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Edition)),
            ClassExtension::CollectionComponent(r) => r
                .edition
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Edition)),
            ClassExtension::SerialComponent(r) => r
                .edition
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Edition)),
            ClassExtension::Classic(r) => r
                .edition
                .clone()
                .or_else(|| self.find_numbering(NumberingType::Edition)),
            _ => self.find_numbering(NumberingType::Edition),
        }
    }

    /// Return the accessed date.
    pub fn accessed(&self) -> Option<DateValue> {
        class_dispatch!(&self.extension, |r| r.accessed.clone(), unknown(_) => None)
    }

    /// Return the forward-compat `unknown_fields` captured for this reference.
    ///
    /// Returns `None` for unknown-class references: their fields are kept
    /// wholesale in [`UnknownClassData::fields`] and reported separately via
    /// the unknown-class warning.
    pub fn unknown_fields(&self) -> Option<&std::collections::BTreeMap<String, JsonValue>> {
        class_dispatch!(&self.extension, |r| Some(&r.unknown_fields), unknown(_) => None)
    }

    /// Return the embedded inline reference behind `original` if any.
    ///
    /// All 16 reference classes that carry an `original` relation expose it via
    /// the same `WorkRelation` shape (only `AudioVisualWork` nests it through
    /// `core`). Centralising the dispatch here lets each `original_*` accessor
    /// stay a one-liner.
    fn original_embedded(&self) -> Option<&InputReference> {
        let relation = match &self.extension {
            ClassExtension::Monograph(r) => r.original.as_ref(),
            ClassExtension::CollectionComponent(r) => r.original.as_ref(),
            ClassExtension::SerialComponent(r) => r.original.as_ref(),
            ClassExtension::LegalCase(r) => r.original.as_ref(),
            ClassExtension::Statute(r) => r.original.as_ref(),
            ClassExtension::Treaty(r) => r.original.as_ref(),
            ClassExtension::Hearing(r) => r.original.as_ref(),
            ClassExtension::Regulation(r) => r.original.as_ref(),
            ClassExtension::Brief(r) => r.original.as_ref(),
            ClassExtension::Classic(r) => r.original.as_ref(),
            ClassExtension::Patent(r) => r.original.as_ref(),
            ClassExtension::Dataset(r) => r.original.as_ref(),
            ClassExtension::Standard(r) => r.original.as_ref(),
            ClassExtension::Software(r) => r.original.as_ref(),
            ClassExtension::Event(r) => r.original.as_ref(),
            ClassExtension::AudioVisual(r) => r.core.original.as_ref(),
            _ => None,
        }?;
        match relation {
            WorkRelation::Embedded(p) => Some(p),
            WorkRelation::Id(_) => None,
        }
    }

    /// Return the original publication date.
    pub fn original_date(&self) -> Option<DateValue> {
        self.original_embedded()?.effective_issued_date()
    }

    /// Return the original title.
    pub fn original_title(&self) -> Option<Title> {
        self.original_embedded()?.title()
    }

    /// Return the original publisher as a string.
    pub fn original_publisher_str(&self) -> Option<String> {
        self.original_embedded()?
            .publisher_str()
            .filter(|value| !value.is_empty())
    }

    /// Return the original publisher place.
    pub fn original_publisher_place(&self) -> Option<String> {
        self.original_embedded()?.publisher_place()
    }

    /// Return the ISBN.
    pub fn isbn(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.isbn.clone(),
            _ => None,
        }
    }

    /// Return the ISSN.
    pub fn issn(&self) -> Option<String> {
        match &self.extension {
            ClassExtension::SerialComponent(r) => r.container.as_ref().and_then(|c| match c {
                WorkRelation::Embedded(s) => s.issn(),
                WorkRelation::Id(_) => None,
            }),
            ClassExtension::Serial(r) => r.issn.clone(),
            _ => None,
        }
    }

    /// Return the Keywords.
    pub fn keywords(&self) -> Option<Vec<String>> {
        match &self.extension {
            ClassExtension::Monograph(r) => r.keywords.clone(),
            ClassExtension::CollectionComponent(r) => r.keywords.clone(),
            ClassExtension::SerialComponent(r) => r.keywords.clone(),
            ClassExtension::Collection(r) => r.keywords.clone(),
            ClassExtension::Serial(_) => None,
            ClassExtension::LegalCase(r) => r.keywords.clone(),
            ClassExtension::Statute(r) => r.keywords.clone(),
            ClassExtension::Treaty(r) => r.keywords.clone(),
            ClassExtension::Hearing(r) => r.keywords.clone(),
            ClassExtension::Regulation(r) => r.keywords.clone(),
            ClassExtension::Brief(r) => r.keywords.clone(),
            ClassExtension::Classic(r) => r.keywords.clone(),
            ClassExtension::Patent(r) => r.keywords.clone(),
            ClassExtension::Dataset(r) => r.keywords.clone(),
            ClassExtension::Standard(r) => r.keywords.clone(),
            ClassExtension::Software(r) => r.keywords.clone(),
            ClassExtension::Event(_) => None,
            ClassExtension::AudioVisual(_) => None,
            ClassExtension::Unknown(_) => None,
        }
    }

    /// Return the language.
    pub fn language(&self) -> Option<LangID> {
        class_dispatch!(
            &self.extension,
            |r| r.language.clone(),
            audio_visual(r) => r.core.language.clone(),
            unknown(_) => None
        )
    }

    /// Return field-level language overrides.
    pub fn field_languages(&self) -> &FieldLanguageMap {
        class_dispatch!(
            &self.extension,
            |r| &r.field_languages,
            unknown(_) => &EMPTY_FIELD_LANGUAGES
        )
    }

    /// Set the reference ID on the class-specific extension.
    ///
    /// For unknown-class references the id is stored as a `JsonValue::String`
    /// inside `UnknownClassData::fields["id"]`. The wire schema requires
    /// `id: string`, so round-trip is lossless for valid inputs.
    pub fn set_id(&mut self, id: impl Into<RefID>) {
        let id = id.into();
        class_dispatch!(&mut self.extension, |r| r.id = Some(id.clone()), unknown(data) => {
            data.fields
                .insert("id".to_string(), JsonValue::String(id.to_string()));
        });
    }

    /// Return the reference type as a string (CSL-compatible).
    pub fn ref_type(&self) -> String {
        match &self.extension {
            ClassExtension::Monograph(r) => self.monograph_ref_type(r),
            ClassExtension::CollectionComponent(r) => collection_component_ref_type(r),
            ClassExtension::SerialComponent(r) => serial_component_ref_type(r),
            ClassExtension::Collection(r) => match r.r#type {
                CollectionType::EditedBook => "book",
                _ => "collection",
            }
            .to_string(),
            ClassExtension::Serial(r) => match r.r#type {
                SerialType::AcademicJournal => "article-journal",
                SerialType::Magazine => "article-magazine",
                SerialType::Newspaper => "article-newspaper",
                SerialType::BroadcastProgram => "broadcast",
                _ => "serial",
            }
            .to_string(),
            ClassExtension::LegalCase(_) => "legal-case".to_string(),
            ClassExtension::Statute(_) => "statute".to_string(),
            ClassExtension::Treaty(_) => "treaty".to_string(),
            ClassExtension::Hearing(_) => "hearing".to_string(),
            ClassExtension::Regulation(_) => "regulation".to_string(),
            ClassExtension::Brief(_) => "brief".to_string(),
            ClassExtension::Classic(_) => "classic".to_string(),
            ClassExtension::Patent(_) => "patent".to_string(),
            ClassExtension::Dataset(_) => "dataset".to_string(),
            ClassExtension::Standard(_) => "standard".to_string(),
            ClassExtension::Software(_) => "software".to_string(),
            ClassExtension::Event(r) => event_ref_type(r).to_string(),
            ClassExtension::AudioVisual(r) => audio_visual_ref_type(&r.r#type).to_string(),
            ClassExtension::Unknown(data) => {
                // Unknown classes round-trip but cannot route to a known CSL
                // ref-type; the engine has no template branch for the raw
                // class string, so rendering will fall through to the default
                // path (typically empty output).
                //
                // TODO(csl26-1bdr): Layer 5 `CompatibilityWarning` plumbing
                // will surface this as a soft-degrade warning rather than
                // silent fall-through. Until then we return the raw class
                // string so debug builds and logs can identify the value.
                debug_assert!(
                    !ReferenceClass::KNOWN.contains(&data.class.as_str()),
                    "ClassExtension::Unknown should never wrap a known class string (got `{}`)",
                    data.class
                );
                data.class.clone()
            }
        }
    }

    fn monograph_ref_type(&self, r: &Monograph) -> String {
        match r.r#type {
            MonographType::Book => {
                if r.genre
                    .as_deref()
                    .is_some_and(|genre| genre.eq_ignore_ascii_case("musical_score"))
                {
                    return "musical-score".to_string();
                }
                if r.genre.as_deref().is_some_and(|genre| genre == "pamphlet") {
                    return "pamphlet".to_string();
                }
                if r.medium
                    .as_deref()
                    .is_some_and(|m| m.to_ascii_lowercase().contains("interview"))
                {
                    "interview"
                } else {
                    "book"
                }
                .to_string()
            }
            MonographType::Manual => "manual".to_string(),
            MonographType::Report => "report".to_string(),
            MonographType::Thesis => "thesis".to_string(),
            MonographType::Webpage => "webpage".to_string(),
            MonographType::Post => if r.genre.as_deref() == Some("post-weblog") {
                "post-weblog"
            } else {
                "post"
            }
            .to_string(),
            MonographType::Interview => "interview".to_string(),
            MonographType::Manuscript => "manuscript".to_string(),
            MonographType::Preprint => "preprint".to_string(),
            MonographType::PersonalCommunication => "personal-communication".to_string(),
            MonographType::Document => {
                // Case-insensitive: real-world exports carry capitalized
                // genre labels (e.g. Zotero's `Map`), and the pre-existing
                // map back-map matched them ignoring case; the canonical
                // lowercase CSL type string is returned either way.
                if let Some(genre) = r.genre.as_deref() {
                    let lowered = genre.to_ascii_lowercase();
                    if matches!(
                        lowered.as_str(),
                        "map" | "figure" | "graphic" | "periodical" | "collection"
                    ) {
                        return lowered;
                    }
                }
                if let Some(genre) = r.genre.as_deref()
                    && matches!(genre, "bill-proceeding" | "bill-record")
                {
                    return genre.to_string();
                }
                if self.genre().as_deref() == Some("conference-paper") {
                    return "paper-conference".to_string();
                }
                if r.medium
                    .as_deref()
                    .is_some_and(|m| m.to_ascii_lowercase().contains("interview"))
                {
                    "interview"
                } else {
                    "document"
                }
                .to_string()
            }
            _ => r.r#type.as_str().to_string(),
        }
    }

    /// Fold legacy contributor shorthands (`author`, `editor`, `translator`)
    /// into the canonical `contributors` vec on this reference and, when it has
    /// one, on its embedded `container`.
    ///
    /// Deserialized references are already normalized as part of their
    /// `#[serde(from = "...Deser")]` conversion; this exists for references
    /// built directly in Rust — such as the biblatex converter — whose
    /// shorthand fields are `skip_serializing` and would otherwise be silently
    /// dropped on write. Idempotent — safe to call on an already-normalized
    /// reference.
    ///
    /// Recurses into `container` only; `original`, `event`, and `reviewed`
    /// relations are not visited.
    pub fn normalize_contributors(&mut self) {
        let container = match &mut self.extension {
            ClassExtension::Monograph(r) => {
                r.normalize_contributors();
                &mut r.container
            }
            ClassExtension::Collection(r) => {
                r.normalize_contributors();
                &mut r.container
            }
            ClassExtension::Serial(r) => {
                r.normalize_contributors();
                &mut r.container
            }
            ClassExtension::CollectionComponent(r) => {
                r.normalize_contributors();
                &mut r.container
            }
            ClassExtension::SerialComponent(r) => {
                r.normalize_contributors();
                &mut r.container
            }
            _ => return,
        };
        if let Some(WorkRelation::Embedded(container)) = container {
            container.normalize_contributors();
        }
    }
}

fn collection_component_ref_type(r: &CollectionComponent) -> String {
    match r.r#type {
        MonographComponentType::Chapter => match r.genre.as_deref() {
            Some("entry-dictionary") => "entry-dictionary",
            Some("entry-encyclopedia") => "entry-encyclopedia",
            Some("entry") => "entry",
            _ => "chapter",
        }
        .to_string(),
        MonographComponentType::Document => "paper-conference".to_string(),
        _ => r.r#type.as_str().to_string(),
    }
}

fn serial_component_ref_type(r: &SerialComponent) -> String {
    if r.genre.as_deref() == Some("entry-encyclopedia") {
        return "entry-encyclopedia".to_string();
    }
    if let Some(genre) = r.genre.as_deref()
        && matches!(genre, "review" | "review-book")
    {
        return genre.to_string();
    }
    let container_type = r.container.as_ref().and_then(|c| match c {
        WorkRelation::Embedded(p) => Some(p.ref_type()),
        WorkRelation::Id(_) => None,
    });
    match container_type.as_deref() {
        Some("article-magazine") => "article-magazine".to_string(),
        Some("article-newspaper") => "article-newspaper".to_string(),
        Some("broadcast") => if r
            .genre
            .as_deref()
            .is_some_and(|g| g.to_ascii_lowercase().contains("film"))
        {
            "motion-picture"
        } else {
            "broadcast"
        }
        .to_string(),
        _ => "article-journal".to_string(),
    }
}

fn event_ref_type(r: &Event) -> &'static str {
    let lowered = r.genre.as_deref().map(str::to_ascii_lowercase);
    match lowered.as_deref() {
        Some(g) if g.contains("conference") || g.contains("paper") => "paper-conference",
        Some(g) if g.contains("broadcast") => "broadcast",
        Some(g) if g.contains("performance") => "performance",
        Some(g) if g.contains("talk") || g.contains("speech") => "speech",
        _ => "event",
    }
}

fn audio_visual_ref_type(kind: &AudioVisualType) -> &'static str {
    match kind {
        AudioVisualType::Film => "motion-picture",
        AudioVisualType::Episode | AudioVisualType::Broadcast => "broadcast",
        AudioVisualType::Recording => "song",
    }
}

/// Collects contributors with a given role from a slice of entries.
///
/// Returns `None` if no entries match. A single match returns the contributor
/// unwrapped; two or more fold into a [`ContributorList`].
fn collect_contributors_by_role(
    entries: &[ContributorEntry],
    role: &ContributorRole,
) -> Option<Contributor> {
    let mut matches = entries
        .iter()
        .filter(|entry| entry.roles.contains(role))
        .map(|e| &e.contributor);
    let first = matches.next()?;
    let Some(second) = matches.next() else {
        return Some(first.clone());
    };
    let list = std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(matches)
        .cloned()
        .collect();
    Some(Contributor::ContributorList(ContributorList(list)))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::reference::contributor::SimpleName;

    #[test]
    fn given_monograph_with_embedded_container_when_normalized_then_container_contributors_fold_too()
     {
        // Regression test: `normalize_contributors` must recurse into `container`
        // for every structural type that has one (Monograph/Collection/Serial),
        // not just the component types -- a nested container built directly in
        // Rust (bypassing deserialization) has the same skip_serializing shorthand
        // problem as the top-level reference.
        let container_monograph = Monograph {
            r#type: MonographType::Book,
            editor: Some(Contributor::SimpleName(SimpleName {
                name: "Series Editor".into(),
                location: None,
                short_name: None,
            })),
            ..Default::default()
        };
        let outer_monograph = Monograph {
            r#type: MonographType::Book,
            container: Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
                Box::new(container_monograph),
            )))),
            ..Default::default()
        };
        let mut reference = InputReference::Monograph(Box::new(outer_monograph));

        reference.normalize_contributors();

        let outer = reference.as_monograph().expect("expected a Monograph");
        let container = match outer.container.as_ref().expect("expected a container") {
            WorkRelation::Embedded(inner) => inner.as_monograph().expect("expected a Monograph"),
            WorkRelation::Id(_) => panic!("expected an embedded container, not an id reference"),
        };
        assert_eq!(container.contributors.len(), 1);
        assert!(
            container.contributors[0]
                .roles
                .contains(&ContributorRole::Editor)
        );
    }
}
