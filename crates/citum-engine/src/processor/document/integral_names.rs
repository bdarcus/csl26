/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Integral-name state handling for document processing.
//!
//! Document processing annotates integral citations before rendering so styles
//! can render first narrative mentions differently from subsequent ones. Parsed
//! documents are processed in source-aware order: body prose and generated note
//! references participate by note occurrence order, while flat API inputs use
//! their caller-provided order. Name memory can reset by document, chapter, or
//! section scope, and it can either ignore note-only mentions or let note and
//! body mentions share one state machine.

#[rustfmt::skip]
use super::note_support::{NoteOccurrence, build_note_order_indices};
use super::{
    CitationPlacement, CitationStructure, DocumentIntegralNameOverride, DocumentOptionsOverride,
    DocumentOrgAbbreviationOverride, ParsedDocument,
};
use crate::processor::run_state::RunState;
use crate::reference::{CitationItem, Contributor};
use crate::{Citation, Processor};
use citum_schema::citation::{CitationMode, IntegralNameState};
use citum_schema::options::{IntegralNameContexts, IntegralNameScope};
use std::collections::{HashMap, hash_map::Entry};

const DOCUMENT_SCOPE_KEY: &str = "document";

type NameMemoryKey = (String, String);

#[derive(Debug, Clone)]
pub(super) struct IntegralNameContext {
    /// Where the citation appears in the parsed document.
    pub(super) placement: CitationPlacement,
    /// Chapter and section scope metadata for this citation location.
    pub(super) structure: CitationStructure,
}

/// Tracks how a name has already participated in integral-name memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeenIntegralNameState {
    NoteOnlySeen,
    BodySeen,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NameMemoryKind {
    Personal,
    OrgAbbreviation,
}

impl Processor {
    #[allow(
        clippy::indexing_slicing,
        reason = "indices are verified within bounds"
    )]
    pub(super) fn normalize_integral_name_citations(
        &self,
        parsed: &ParsedDocument,
        run: &mut RunState,
    ) -> Vec<Citation> {
        let mut normalized: Vec<Citation> = parsed
            .citations
            .iter()
            .map(|parsed| parsed.citation.clone())
            .collect();
        let ordered_indices = build_integral_name_order_indices(parsed);
        #[allow(
            clippy::indexing_slicing,
            reason = "index derived from citations collection"
        )]
        let mut ordered_citations: Vec<Citation> = ordered_indices
            .iter()
            .map(|index| normalized[*index].clone())
            .collect();
        #[allow(
            clippy::indexing_slicing,
            reason = "index derived from citations collection"
        )]
        let ordered_contexts: Vec<_> = ordered_indices
            .iter()
            .map(|index| IntegralNameContext {
                placement: parsed.citations[*index].placement.clone(),
                structure: parsed.citations[*index].structure.clone(),
            })
            .collect();
        self.annotate_integral_name_states(&mut ordered_citations, &ordered_contexts);
        #[allow(
            clippy::indexing_slicing,
            reason = "index derived from citations collection"
        )]
        for (citation, index) in ordered_citations.into_iter().zip(ordered_indices) {
            normalized[index] = citation;
        }
        self.normalize_note_context(&normalized, run)
    }

    /// Annotate integral-name First/Subsequent state across a flat, ordered
    /// citation list (API paths with no parsed document).
    ///
    /// Citations are processed in list order. A citation with a `note_number`
    /// is treated as a note context; otherwise as body prose. There is no
    /// chapter/section structure, so all citations share the document scope
    /// regardless of the configured [`IntegralNameScope`].
    pub(crate) fn annotate_flat_integral_name_states(&self, citations: &mut [Citation]) {
        let contexts: Vec<IntegralNameContext> = citations
            .iter()
            .map(|citation| IntegralNameContext {
                placement: if citation.note_number.is_some() {
                    CitationPlacement::ManualFootnote {
                        label: String::new(),
                    }
                } else {
                    CitationPlacement::InlineProse
                },
                structure: CitationStructure::default(),
            })
            .collect();
        self.annotate_integral_name_states(citations, &contexts);
    }

    pub(crate) fn processor_with_document_integral_name_override(
        &self,
        override_config: Option<&DocumentIntegralNameOverride>,
    ) -> Option<Self> {
        let override_config = override_config?;
        let mut style = self.style.clone();
        let base = style
            .options
            .as_ref()
            .and_then(|options| options.integral_name_memory.as_ref());
        let applied = override_config.apply_to(base);
        style
            .options
            .get_or_insert_with(Default::default)
            .integral_name_memory = applied;
        // The style cloned from self.style is already resolved — bypass
        // into_resolved() to prevent a second preset application that would
        // restore null-cleared fields (e.g. type-variants: ~).
        Some(Self::build_processor_pre_resolved(
            style,
            self.bibliography.clone(),
            self.locale.clone(),
            self.compound_sets.clone(),
        ))
    }

    pub(super) fn processor_with_document_org_abbreviation_override(
        &self,
        override_config: Option<&DocumentOrgAbbreviationOverride>,
    ) -> Option<Self> {
        let override_config = override_config?;
        let mut style = self.style.clone();
        let base = style
            .options
            .as_ref()
            .and_then(|options| options.org_abbreviation_memory.as_ref());
        let applied = override_config.apply_to(base);
        style
            .options
            .get_or_insert_with(Default::default)
            .org_abbreviation_memory = applied;
        Some(Self::build_processor_pre_resolved(
            style,
            self.bibliography.clone(),
            self.locale.clone(),
            self.compound_sets.clone(),
        ))
    }

    /// Build a new processor with bibliography overrides from a `DocumentOptionsOverride` applied.
    ///
    /// Writes non-`None` bibliography option fields into a cloned style and
    /// calls `apply_scoped_options` for structural scoped options; label
    /// presentation is resolved at render time.
    pub(super) fn processor_with_bibliography_override(
        &self,
        options: &DocumentOptionsOverride,
    ) -> Self {
        let mut style = self.style.clone();
        options.apply_bibliography_to(&mut style);
        Self::build_processor_pre_resolved(
            style,
            self.bibliography.clone(),
            self.locale.clone(),
            self.compound_sets.clone(),
        )
    }

    fn annotate_name_states_for_kind(
        &self,
        citations: &mut [Citation],
        contexts: &[IntegralNameContext],
        kind: NameMemoryKind,
        scope: IntegralNameScope,
        name_contexts: IntegralNameContexts,
    ) {
        let mut seen: HashMap<NameMemoryKey, SeenIntegralNameState> = HashMap::new();
        for (citation, context) in citations.iter_mut().zip(contexts.iter()) {
            if !citation_counts_as_integral(citation) {
                continue;
            }

            let is_body = is_body_context(context);
            if matches!(name_contexts, IntegralNameContexts::BodyOnly) && !is_body {
                continue;
            }

            let scope_key = scope_key(scope, context);

            for item in &mut citation.items {
                if !kind.tracks_item(&self.bibliography, item) || kind.state_is_set(item) {
                    continue;
                }

                let key = (
                    scope_key.to_string(),
                    kind.key_for_item(&self.bibliography, item),
                );
                let derived = mark_seen_name(&mut seen, key, is_body, name_contexts);
                kind.set_state(item, derived);
            }
        }
    }

    pub(super) fn annotate_integral_name_states(
        &self,
        citations: &mut [Citation],
        contexts: &[IntegralNameContext],
    ) {
        let citation_config = self.get_citation_config();

        if let Some(config) = citation_config
            .integral_name_memory
            .as_ref()
            .map(citum_schema::options::IntegralNameMemoryConfig::resolve)
        {
            self.annotate_name_states_for_kind(
                citations,
                contexts,
                NameMemoryKind::Personal,
                config.scope,
                config.contexts,
            );
        }

        if let Some(config) = citation_config
            .org_abbreviation_memory
            .as_ref()
            .map(citum_schema::options::OrgAbbreviationMemoryConfig::resolve)
        {
            self.annotate_name_states_for_kind(
                citations,
                contexts,
                NameMemoryKind::OrgAbbreviation,
                config.scope,
                config.contexts,
            );
        }
    }
}

impl NameMemoryKind {
    fn tracks_item(&self, bibliography: &crate::Bibliography, item: &CitationItem) -> bool {
        match self {
            Self::Personal => !first_author_is_org(bibliography, &item.id),
            Self::OrgAbbreviation => first_author_is_org(bibliography, &item.id),
        }
    }

    fn state_is_set(&self, item: &CitationItem) -> bool {
        match self {
            Self::Personal => item.integral_name_state.is_some(),
            Self::OrgAbbreviation => item.org_abbreviation_state.is_some(),
        }
    }

    fn key_for_item(&self, bibliography: &crate::Bibliography, item: &CitationItem) -> String {
        match self {
            Self::Personal => personal_author_key_for_item(bibliography, &item.id),
            Self::OrgAbbreviation => first_author_key_for_item(bibliography, &item.id),
        }
    }

    fn set_state(&self, item: &mut CitationItem, state: IntegralNameState) {
        match self {
            Self::Personal => item.integral_name_state = Some(state),
            Self::OrgAbbreviation => item.org_abbreviation_state = Some(state),
        }
    }
}

fn citation_counts_as_integral(citation: &Citation) -> bool {
    matches!(citation.mode, CitationMode::Integral) || citation.suppress_author
}

fn is_body_context(context: &IntegralNameContext) -> bool {
    matches!(context.placement, CitationPlacement::InlineProse)
}

fn scope_key(scope: IntegralNameScope, context: &IntegralNameContext) -> &str {
    match scope {
        IntegralNameScope::Document => DOCUMENT_SCOPE_KEY,
        IntegralNameScope::Chapter => context.structure.chapter_scope.as_str(),
        IntegralNameScope::Section => context.structure.section_scope.as_str(),
    }
}

fn mark_seen_name(
    seen: &mut HashMap<NameMemoryKey, SeenIntegralNameState>,
    key: NameMemoryKey,
    is_body: bool,
    name_contexts: IntegralNameContexts,
) -> IntegralNameState {
    match name_contexts {
        IntegralNameContexts::BodyOnly => mark_body_only_name(seen, key),
        IntegralNameContexts::BodyAndNotes if is_body => mark_body_name(seen, key),
        IntegralNameContexts::BodyAndNotes => mark_note_name(seen, key),
    }
}

fn mark_body_only_name(
    seen: &mut HashMap<NameMemoryKey, SeenIntegralNameState>,
    key: NameMemoryKey,
) -> IntegralNameState {
    match seen.entry(key) {
        Entry::Occupied(mut entry) if *entry.get() != SeenIntegralNameState::BodySeen => {
            entry.insert(SeenIntegralNameState::BodySeen);
            IntegralNameState::First
        }
        Entry::Occupied(_) => IntegralNameState::Subsequent,
        Entry::Vacant(entry) => {
            entry.insert(SeenIntegralNameState::BodySeen);
            IntegralNameState::First
        }
    }
}

fn mark_body_name(
    seen: &mut HashMap<NameMemoryKey, SeenIntegralNameState>,
    key: NameMemoryKey,
) -> IntegralNameState {
    match seen.entry(key) {
        Entry::Occupied(entry) if *entry.get() == SeenIntegralNameState::BodySeen => {
            IntegralNameState::Subsequent
        }
        Entry::Occupied(mut entry) => {
            // Note-only sightings do not consume the first body mention.
            entry.insert(SeenIntegralNameState::BodySeen);
            IntegralNameState::First
        }
        Entry::Vacant(entry) => {
            entry.insert(SeenIntegralNameState::BodySeen);
            IntegralNameState::First
        }
    }
}

fn mark_note_name(
    seen: &mut HashMap<NameMemoryKey, SeenIntegralNameState>,
    key: NameMemoryKey,
) -> IntegralNameState {
    match seen.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(SeenIntegralNameState::NoteOnlySeen);
            IntegralNameState::First
        }
        Entry::Occupied(_) => IntegralNameState::Subsequent,
    }
}

fn personal_author_key_for_item(bibliography: &crate::Bibliography, item_id: &str) -> String {
    bibliography
        .get(item_id)
        .and_then(|r| r.author())
        .and_then(|c| contributor_personal_key(&c))
        .unwrap_or_else(|| item_id.to_string())
}

fn contributor_personal_key(contributor: &Contributor) -> Option<String> {
    match contributor {
        Contributor::StructuredName(n) => {
            let given = n.given.to_string();
            let family = n.family.to_string();
            let suffix = n.suffix.as_deref().unwrap_or("");
            Some(format!("{given}|{family}|{suffix}"))
        }
        Contributor::Multilingual(m) => {
            let given = m.original.given.to_string();
            let family = m.original.family.to_string();
            let suffix = m.original.suffix.as_deref().unwrap_or("");
            Some(format!("{given}|{family}|{suffix}"))
        }
        Contributor::SimpleName(n) => Some(n.name.to_string()),
        Contributor::ContributorList(l) => l.0.first().and_then(contributor_personal_key),
    }
}

/// Returns a name-based tracking key for integral name-memory state.
///
/// Uses the first author's family name so that two works by the same author
/// share a single First/Subsequent slot, regardless of citation key.
/// Falls back to `item_id` when no author or family name is available.
fn first_author_key_for_item(bibliography: &crate::Bibliography, item_id: &str) -> String {
    bibliography
        .get(item_id)
        .and_then(|r| r.author())
        .and_then(|c| contributor_first_family(&c))
        .unwrap_or_else(|| item_id.to_string())
}

fn contributor_first_family(contributor: &Contributor) -> Option<String> {
    match contributor {
        Contributor::StructuredName(n) => Some(n.family.to_string()),
        Contributor::Multilingual(m) => Some(m.original.family.to_string()),
        Contributor::SimpleName(n) => Some(n.name.to_string()),
        Contributor::ContributorList(l) => l.0.first().and_then(contributor_first_family),
    }
}

fn first_author_is_org(bibliography: &crate::Bibliography, item_id: &str) -> bool {
    bibliography
        .get(item_id)
        .and_then(|r| r.author())
        .is_some_and(|c| first_contributor_is_org(&c))
}

fn first_contributor_is_org(contributor: &Contributor) -> bool {
    match contributor {
        Contributor::SimpleName(_) => true,
        Contributor::StructuredName(_) | Contributor::Multilingual(_) => false,
        Contributor::ContributorList(l) => l.0.first().is_some_and(first_contributor_is_org),
    }
}

pub(super) fn build_integral_name_order_indices(parsed: &ParsedDocument) -> Vec<usize> {
    let mut manual_citations: HashMap<String, Vec<usize>> = HashMap::new();
    let mut note_occurrences: Vec<NoteOccurrence> = parsed
        .manual_note_references
        .iter()
        .map(|note| NoteOccurrence::Manual {
            label: note.label.clone(),
            start: note.start,
        })
        .collect();

    for (index, parsed_citation) in parsed.citations.iter().enumerate() {
        match &parsed_citation.placement {
            CitationPlacement::InlineProse => note_occurrences.push(NoteOccurrence::Generated {
                citation_index: index,
                start: parsed_citation.start,
            }),
            CitationPlacement::ManualFootnote { label } => {
                manual_citations
                    .entry(label.clone())
                    .or_default()
                    .push(index);
            }
        }
    }

    for indices in manual_citations.values_mut() {
        #[allow(
            clippy::indexing_slicing,
            reason = "index derived from citations collection"
        )]
        indices.sort_by_key(|index| parsed.citations[*index].start);
    }

    note_occurrences.sort_by_key(NoteOccurrence::start);
    build_note_order_indices(&note_occurrences, &manual_citations)
}
