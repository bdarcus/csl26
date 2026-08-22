/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Legacy CSL → Citum converters for media references (software, audio-visual).

#[allow(
    clippy::wildcard_imports,
    reason = "submodule shares the parent's helper + type pool by design"
)]
use super::*;

pub(super) fn from_software_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let original = legacy_original_relation(&legacy);
    let version = legacy_extra_str(&legacy, "version");
    let platform = legacy.medium.clone();
    InputReference::Software(Box::new(Software {
        id: ctx.id,
        title: build_title(ctx.title, ctx.short_title),
        original,
        author: legacy.author.map(Contributor::from),
        created: ctx.created,
        issued: ctx.issued,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        version,
        repository: None,
        license: None,
        platform,
        doi: ctx.doi,
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        keywords: None,
        unknown_fields: Default::default(),
    }))
}

pub(super) fn from_audio_visual_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let original = legacy_original_relation(&legacy);
    let r#type = audio_visual_type(&legacy.ref_type);
    let contributors = audio_visual_contributors(&legacy);
    let event = relation_event(
        legacy_extra_str(&legacy, "event-title"),
        legacy_extra_str(&legacy, "event-place"),
        legacy_extra_date(&legacy, "event-date"),
    );
    let dimensions = legacy_extra_str(&legacy, "dimensions");
    let numbering = audio_visual_numbering(&legacy);

    InputReference::AudioVisual(Box::new(AudioVisualWork {
        id: ctx.id,
        r#type,
        core: WorkCore {
            title: build_title(ctx.title, ctx.short_title.clone()),
            short_title: None,
            contributors,
            created: ctx.created,
            issued: ctx.issued,
            original,
            language: ctx.language,
            genre: legacy.genre,
        },
        container: legacy.container_title.map(|title| {
            WorkRelation::Embedded(Box::new(InputReference::Monograph(Box::new(Monograph {
                title: Some(Title::Single(title)),
                ..Default::default()
            }))))
        }),
        event,
        numbering,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        medium: legacy.medium,
        dimensions,
        platform: None,
        url: ctx.url,
        accessed: ctx.accessed,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        unknown_fields: Default::default(),
    }))
}

fn audio_visual_type(ref_type: &str) -> AudioVisualType {
    match ref_type {
        "motion_picture" => AudioVisualType::Film,
        "song" => AudioVisualType::Recording,
        "broadcast" => AudioVisualType::Broadcast,
        _ => AudioVisualType::Broadcast,
    }
}

fn audio_visual_contributors(legacy: &csl_legacy::csl_json::Reference) -> Vec<ContributorEntry> {
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Director,
        legacy
            .director
            .clone()
            .or_else(|| legacy_extra_names(legacy, "director")),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Composer,
        legacy_extra_names(legacy, "composer"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Performer,
        legacy_extra_names(legacy, "performer"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Narrator,
        legacy_extra_names(legacy, "narrator"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Unknown("contributor".to_string()),
        legacy_extra_names(legacy, "contributor"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Producer,
        legacy_extra_names(legacy, "producer")
            .or_else(|| legacy_extra_names(legacy, "executive-producer")),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Writer,
        legacy_extra_names(legacy, "script-writer"),
    );
    push_legacy_contributor(
        &mut contributors,
        audio_visual_author_role(legacy),
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );
    contributors
}

fn audio_visual_author_role(legacy: &csl_legacy::csl_json::Reference) -> ContributorRole {
    // For recordings, CSL/Zotero `author` is the performer; store it that way
    // so that composer can remain the primary rendered author where applicable.
    if legacy.ref_type == "song" {
        ContributorRole::Performer
    } else {
        ContributorRole::Author
    }
}

fn audio_visual_numbering(legacy: &csl_legacy::csl_json::Reference) -> Vec<Numbering> {
    let mut numbering: Vec<Numbering> = legacy
        .number
        .iter()
        .cloned()
        .map(|number| Numbering {
            r#type: NumberingType::Number,
            value: number,
        })
        .collect();
    if let Some(vol) = legacy.volume.as_ref().map(ToString::to_string) {
        numbering.push(Numbering {
            r#type: NumberingType::Volume,
            value: vol,
        });
    }
    if let Some(chapter) = legacy.chapter_number.clone() {
        numbering.push(Numbering {
            r#type: NumberingType::Chapter,
            value: chapter,
        });
    }
    if let Some(number_of_volumes) = legacy_number_of_volumes(legacy) {
        numbering.push(number_of_volumes);
    }
    numbering
}
