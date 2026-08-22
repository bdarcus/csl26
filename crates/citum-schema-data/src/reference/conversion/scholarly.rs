/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Legacy CSL → Citum converters for scholarly references (monographs and
//! their components, serials, preprints, datasets, documents, events,
//! standalone edited books).

#[allow(
    clippy::wildcard_imports,
    reason = "submodule shares the parent's helper + type pool by design"
)]
use super::*;
use crate::reference::Classic;

/// Resolve the primary title and nested monograph relation for a legacy book.
fn monograph_title_and_container(
    legacy: &csl_legacy::csl_json::Reference,
    r#type: &MonographType,
    ctx: &RefContext,
    volume: Option<&str>,
    volume_title: Option<&str>,
    part_title: Option<&str>,
    collection_title: Option<String>,
) -> (Option<String>, Option<WorkRelation>) {
    let title = if *r#type == MonographType::Webpage {
        ctx.title.clone().map(|base_title| {
            let mut combined = base_title;
            if let Some(part_num) = legacy_extra_str(legacy, "part-number") {
                combined.push_str(": Pt. ");
                combined.push_str(&part_num);
            }
            if let Some(part) = part_title {
                if legacy_extra_str(legacy, "part-number").is_none() {
                    combined.push(':');
                    combined.push(' ');
                } else {
                    combined.push('.');
                    combined.push(' ');
                }
                combined.push_str(part);
            }
            combined
        })
    } else if let Some(part) = part_title {
        Some(part.to_string())
    } else {
        ctx.title.clone()
    };

    let base_title = legacy
        .container_title
        .clone()
        .map(Title::Single)
        .or_else(|| {
            (volume.is_some() && (part_title.is_some() || volume_title.is_some()))
                .then(|| ctx.title.clone().map(Title::Single))
                .flatten()
        });
    let collection = relation_collection_title(collection_title);
    let container = match base_title {
        Some(title) => {
            let parent = Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
                Box::new(Monograph {
                    title: Some(title),
                    container: collection,
                    ..Default::default()
                }),
            ))));
            if part_title.is_some() && volume_title.is_some() {
                Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
                    Box::new(Monograph {
                        title: volume_title.map(|title| Title::Single(title.to_string())),
                        container: parent,
                        ..Default::default()
                    }),
                ))))
            } else {
                parent
            }
        }
        None if collection.is_some() => {
            // A book in a series with no intermediate container-title still
            // needs a title-less parent so collection_title can walk upward.
            Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
                Box::new(Monograph {
                    title: None,
                    container: collection,
                    ..Default::default()
                }),
            ))))
        }
        None => None,
    };

    (title, container)
}

#[allow(
    clippy::too_many_lines,
    reason = "Legacy CSL mapping requires extensive branching"
)]
pub(super) fn from_monograph_ref(
    legacy: csl_legacy::csl_json::Reference,
    mut ctx: RefContext,
) -> InputReference {
    if (legacy.ref_type == "personal_communication" || legacy.ref_type == "personal-communication")
        && ctx.note.is_none()
    {
        ctx.note = Some("personal communication".to_string());
    }

    let r#type = if legacy.ref_type == "report" {
        MonographType::Report
    } else if legacy.ref_type == "thesis" {
        MonographType::Thesis
    } else if legacy.ref_type == "manual" {
        MonographType::Manual
    } else if legacy.ref_type == "manuscript" {
        MonographType::Manuscript
    } else if legacy.ref_type == "webpage" {
        MonographType::Webpage
    } else if legacy.ref_type.contains("post") {
        MonographType::Post
    } else if legacy.ref_type == "interview" {
        MonographType::Interview
    } else if legacy.ref_type == "personal_communication"
        || legacy.ref_type == "personal-communication"
    {
        MonographType::PersonalCommunication
    } else {
        MonographType::Book
    };

    let archive_info = archive_info_from_legacy_flat(&legacy);
    let archive = match archive_info.as_ref() {
        Some(ai) if ai.collection.is_some() => None,
        _ => legacy.archive.clone(),
    };

    // Use report numbering only for Report type; all standard types use shorthand fields.
    let mut numbering = if r#type == MonographType::Report {
        legacy
            .number
            .as_ref()
            .map(|n| {
                vec![Numbering {
                    r#type: NumberingType::Report,
                    value: n.clone(),
                }]
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let author = legacy.author.clone().map(Contributor::from);
    let editor = legacy.editor.clone().map(Contributor::from);
    let translator = legacy.translator.clone().map(Contributor::from);
    let original_author = legacy_extra_contributor(&legacy, "original-author");
    let original_date = legacy_extra_date(&legacy, "original-date");
    let original_publisher = legacy_extra_str(&legacy, "original-publisher");
    let original_publisher_place = legacy_extra_str(&legacy, "original-publisher-place");
    let number_of_volumes = legacy_number_of_volumes(&legacy);
    let volume_title = legacy_extra_str(&legacy, "volume-title");
    let part_title = legacy_extra_str(&legacy, "part-title");
    let part_number = legacy_extra_str(&legacy, "part-number");
    let status = legacy_extra_str(&legacy, "status");
    let available_date = legacy_extra_date(&legacy, "available-date");
    let references = legacy_extra_str(&legacy, "references");
    let scale = legacy_extra_str(&legacy, "scale");
    let dimensions = legacy_extra_str(&legacy, "dimensions");
    let event = relation_event(
        legacy_extra_str(&legacy, "event-title"),
        legacy_extra_str(&legacy, "event-place"),
        legacy_extra_date(&legacy, "event-date"),
    );
    let collection_title = legacy.collection_title.clone();
    let mut contributors = legacy_named_contributors(&legacy);
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Author,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Editor,
        legacy.editor.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Host,
        legacy_extra_names(&legacy, "host"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Narrator,
        legacy_extra_names(&legacy, "narrator"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Illustrator,
        legacy_extra_names(&legacy, "illustrator"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Unknown("contributor".to_string()),
        legacy_extra_names(&legacy, "contributor"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Compiler,
        legacy_extra_names(&legacy, "compiler"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Unknown("collection-editor".to_string()),
        legacy_extra_names(&legacy, "collection-editor"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Composer,
        legacy_extra_names(&legacy, "composer"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Performer,
        legacy_extra_names(&legacy, "performer"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Producer,
        legacy_extra_names(&legacy, "producer")
            .or_else(|| legacy_extra_names(&legacy, "executive-producer")),
    );
    let edition = ctx.edition.clone();
    let volume = legacy
        .volume
        .as_ref()
        .map(|v| v.to_string())
        .or_else(|| legacy.collection_number.clone().map(|cn| cn.to_string()));
    let number = if r#type == MonographType::Report {
        None
    } else {
        legacy.number.clone()
    };
    let original = relation_monograph(
        legacy.original_title.clone().map(Title::Single),
        original_author,
        original_date,
        None,
        original_publisher,
        original_publisher_place,
    );

    let (title, container) = monograph_title_and_container(
        &legacy,
        &r#type,
        &ctx,
        volume.as_deref(),
        volume_title.as_deref(),
        part_title.as_deref(),
        collection_title,
    );

    // Batch 1: part-number adds a Part numbering entry.
    if let Some(part_num) = part_number {
        numbering.push(Numbering {
            r#type: NumberingType::Part,
            value: part_num,
        });
    }
    if let Some(number_of_volumes) = number_of_volumes {
        numbering.push(number_of_volumes);
    }
    if let Some(chapter_num) = legacy.chapter_number.clone()
        && numbering.iter().all(|n| n.r#type != NumberingType::Chapter)
    {
        numbering.push(Numbering {
            r#type: NumberingType::Chapter,
            value: chapter_num,
        });
    }

    // Map CSL `dimensions` to size (freeform) or duration (ISO 8601 / HH:MM pattern).
    let (size, duration) = match dimensions {
        Some(ref d)
            if d.starts_with('P')
                || d.starts_with("PT")
                || d.chars().next().is_some_and(|c| c.is_ascii_digit()) && d.contains(':') =>
        {
            (None, dimensions)
        }
        Some(_) => (dimensions, None),
        None => (None, None),
    };

    if legacy.ref_type == "classic" {
        return InputReference::Classic(Box::new(Classic {
            id: ctx.id,
            title: build_title(title, ctx.short_title.clone()),
            container,
            original,
            author,
            editor,
            translator,
            volume,
            issue: None,
            edition,
            number,
            part_number: None,
            supplement_number: None,
            printing_number: legacy.printing_number,
            numbering,
            created: ctx.created,
            issued: ctx.issued,
            publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
            url: ctx.url,
            accessed: ctx.accessed,
            language: ctx.language,
            field_languages: HashMap::new(),
            note: ctx.note.map(RichText::Plain),
            keywords: None,
            unknown_fields: Default::default(),
        }));
    }

    // Seed genre from ref_type for CSL types whose round trip through
    // `ref_type()` depends on genre (see `monograph_ref_type`'s Book and
    // Post arms in accessors.rs); never overwrites a user-supplied genre.
    // `post-weblog` needs this too: it shares `MonographType::Post` with
    // plain `post` (both match `ref_type.contains("post")` above), so
    // genre is the only signal that distinguishes them on the way back out.
    let genre = legacy.genre.clone().or_else(|| {
        matches!(
            legacy.ref_type.as_str(),
            "musical_score" | "pamphlet" | "post-weblog"
        )
        .then(|| legacy.ref_type.clone())
    });

    InputReference::Monograph(Box::new(Monograph {
        id: ctx.id,
        r#type,
        title: build_title(title, ctx.short_title.clone()),
        short_title: None,
        volume_title,
        container,
        author,
        editor,
        translator,
        contributors,
        created: ctx.created,
        issued: ctx.issued,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        abstract_text: None,
        isbn: ctx.isbn,
        doi: ctx.doi,
        ads_bibcode: None,
        version: None,
        volume,
        issue: None,
        edition,
        number,
        part_number: None,
        supplement_number: None,
        printing_number: legacy.printing_number,
        numbering,
        genre,
        medium: legacy.medium,
        archive,
        archive_location: legacy.archive_location,
        archive_info,
        eprint: None,
        keywords: None,
        original,
        event,
        status,
        available_date,
        copyright: ctx.copyright,
        printing: ctx.printing,
        size,
        duration,
        references,
        scale,
        pages: legacy.page.map(NumOrStr::Str),
        unknown_fields: Default::default(),
    }))
}

#[allow(
    clippy::too_many_lines,
    reason = "Legacy CSL collection-component mapping requires extensive field wiring"
)]
pub(super) fn from_collection_component_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let (genre, status) = collection_component_metadata(&legacy);
    let part_title = legacy_extra_str(&legacy, "part-title");
    let part_number = legacy_extra_str(&legacy, "part-number");
    let original_author = legacy_extra_contributor(&legacy, "original-author");
    let original_date = legacy_extra_date(&legacy, "original-date");
    let original_publisher = legacy_extra_str(&legacy, "original-publisher");
    let original_publisher_place = legacy_extra_str(&legacy, "original-publisher-place");
    let number_of_volumes = legacy_number_of_volumes(&legacy);
    let parent_title = legacy.container_title.clone().map(Title::Single);
    let collection_title = legacy.collection_title.clone();
    let parent_volume = legacy
        .collection_number
        .clone()
        .or_else(|| legacy.volume.clone())
        .map(|v| v.to_string());
    let parent_edition = ctx.edition.clone();
    let container_author = legacy_extra_names(&legacy, "container-author").map(Contributor::from);
    let has_named_parent = parent_title.is_some() || container_author.is_some();

    let author = legacy.author.clone().map(Contributor::from);
    let translator = legacy.translator.clone().map(Contributor::from);
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Author,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );
    if !has_named_parent {
        push_legacy_contributor(
            &mut contributors,
            ContributorRole::Editor,
            legacy.editor.clone(),
        );
    }
    if let Some(container_author) = container_author.clone() {
        contributors.push(ContributorEntry {
            roles: ContributorRole::Unknown("container-author".to_string()).into(),
            contributor: container_author,
            gender: None,
        });
    }
    let container_editor = has_named_parent
        .then(|| legacy.editor.clone().map(Contributor::from))
        .flatten();
    let mut container_contributors = Vec::new();
    if has_named_parent {
        push_legacy_contributor(
            &mut container_contributors,
            ContributorRole::Editor,
            legacy.editor.clone(),
        );
    }

    let container = if legacy.ref_type == "report" {
        Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
            Box::new(Monograph {
                r#type: MonographType::Report,
                title: parent_title,
                editor: container_editor,
                contributors: container_contributors,
                publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
                edition: parent_edition.clone(),
                numbering: legacy
                    .number
                    .as_ref()
                    .map(|number| {
                        vec![Numbering {
                            r#type: NumberingType::Report,
                            value: number.clone(),
                        }]
                    })
                    .unwrap_or_default(),
                genre: legacy.genre.clone(),
                ..Default::default()
            }),
        ))))
    } else {
        let event_relation = if legacy.ref_type == "paper-conference" {
            let event_title = legacy_extra_str(&legacy, "event-title");
            let event_place = legacy_extra_str(&legacy, "event-place")
                .or_else(|| legacy_extra_str(&legacy, "event-location"));
            let event_date = legacy_extra_date(&legacy, "event-date");
            relation_event(event_title, event_place, event_date)
        } else {
            None
        };

        Some(WorkRelation::Embedded(Box::new(
            InputReference::Collection(Box::new(Collection {
                id: None,
                r#type: if legacy.ref_type == "paper-conference" {
                    CollectionType::Proceedings
                } else {
                    CollectionType::EditedBook
                },
                title: parent_title,
                short_title: ctx.container_title_short,
                container: relation_collection_title(collection_title),
                editor: container_editor,
                translator: None,
                contributors: container_contributors,
                created: DateValue::new(String::new()),
                issued: DateValue::new(String::new()),
                publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
                edition: parent_edition,
                numbering: parent_volume
                    .clone()
                    .map(|volume| {
                        vec![Numbering {
                            r#type: NumberingType::Volume,
                            value: volume,
                        }]
                    })
                    .unwrap_or_default(),
                event: event_relation,
                ..Default::default()
            })),
        )))
    };

    InputReference::CollectionComponent(Box::new(CollectionComponent {
        id: ctx.id,
        r#type: if legacy.ref_type == "paper-conference" {
            MonographComponentType::Document
        } else {
            MonographComponentType::Chapter
        },
        title: build_title(part_title.or(ctx.title), ctx.short_title.clone()),
        author,
        translator,
        contributors,
        issued: ctx.issued,
        container,
        edition: ctx.edition,
        pages: legacy.page.map(NumOrStr::Str),
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        doi: ctx.doi,
        genre,
        medium: legacy.medium,
        status,
        numbering: {
            let mut numbering = Vec::new();
            if let Some(number_of_volumes) = number_of_volumes {
                numbering.push(number_of_volumes);
            }
            if let Some(part_number) = part_number {
                numbering.push(Numbering {
                    r#type: NumberingType::Part,
                    value: part_number,
                });
            }
            if let Some(chapter_number) = legacy.chapter_number.clone() {
                numbering.push(Numbering {
                    r#type: NumberingType::Chapter,
                    value: chapter_number,
                });
            }
            if legacy.ref_type == "report"
                && let Some(report_number) = legacy.number.clone()
            {
                numbering.push(Numbering {
                    r#type: NumberingType::Report,
                    value: report_number,
                });
            }
            numbering
        },
        original: relation_monograph(
            legacy.original_title.clone().map(Title::Single),
            original_author,
            original_date,
            None,
            original_publisher,
            original_publisher_place,
        ),
        ..Default::default()
    }))
}

fn collection_component_metadata(
    legacy: &csl_legacy::csl_json::Reference,
) -> (Option<String>, Option<String>) {
    let mut genre = legacy.genre.clone();
    if genre.is_none() {
        genre = match legacy.ref_type.as_str() {
            "entry-dictionary" => Some("entry-dictionary".to_string()),
            "entry-encyclopedia" => Some("entry-encyclopedia".to_string()),
            "entry" => Some("entry".to_string()),
            _ => None,
        };
    }
    let status = legacy_extra_str(legacy, "status");
    (genre, status)
}

/// Convert a legacy CSL edited book into the standalone Citum collection shape.
#[must_use]
pub fn input_reference_from_legacy_edited_book(
    legacy: csl_legacy::csl_json::Reference,
) -> InputReference {
    let mut numbering = Vec::new();
    if let Some(number_of_volumes) = legacy_number_of_volumes(&legacy) {
        numbering.push(number_of_volumes);
    }
    if let Some(cn) = legacy.collection_number.clone() {
        numbering.push(Numbering {
            r#type: NumberingType::Volume,
            value: cn.to_string(),
        });
    }

    let csl_legacy::csl_json::Reference {
        id,
        title,
        editor,
        translator,
        issued,
        publisher,
        publisher_place,
        url,
        accessed,
        language,
        note: raw_note,
        isbn,
        extra,
        ..
    } = legacy;
    let note = raw_note.map(RichText::Plain);

    let editor_value = editor.clone().map(Contributor::from);
    let translator_value = translator.clone().map(Contributor::from);
    let mut contributors = Vec::new();
    push_legacy_contributor(&mut contributors, ContributorRole::Editor, editor);
    push_legacy_contributor(&mut contributors, ContributorRole::Translator, translator);

    InputReference::Collection(Box::new(Collection {
        id: Some(id.into()),
        r#type: CollectionType::EditedBook,
        title: title.map(Title::Single),
        short_title: extra
            .get("shortTitle")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        container: None,
        editor: editor_value,
        translator: translator_value,
        contributors,
        created: DateValue::new(String::new()),
        issued: issued
            .map(DateValue::from)
            .unwrap_or(DateValue::new(String::new())),
        publisher: publisher_from_parts(publisher, publisher_place),
        numbering,
        url: url.as_deref().and_then(|value| Url::parse(value).ok()),
        accessed: accessed.map(DateValue::from),
        language: language.map(Into::into),
        field_languages: HashMap::new(),
        note,
        isbn,
        event: None,
        volume: None,
        issue: None,
        edition: None,
        number: None,
        part_number: None,
        supplement_number: None,
        printing_number: None,
        keywords: None,
        unknown_fields: Default::default(),
    }))
}

#[allow(
    clippy::too_many_lines,
    reason = "Legacy CSL mapping requires extensive field wiring"
)]
pub(super) fn from_serial_component_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let mut genre = legacy.genre.clone();
    if legacy.ref_type == "entry-encyclopedia" && genre.is_none() {
        genre = Some("entry-encyclopedia".to_string());
    }
    if matches!(legacy.ref_type.as_str(), "review" | "review-book") && genre.is_none() {
        genre = Some(legacy.ref_type.clone());
    }
    let host_names = legacy_extra_names(&legacy, "host");
    let narrator_names = legacy_extra_names(&legacy, "narrator");
    let producer_names = legacy_extra_names(&legacy, "producer")
        .or_else(|| legacy_extra_names(&legacy, "executive-producer"));
    let container_author = legacy_extra_names(&legacy, "container-author");
    let reviewed_author = legacy_extra_names(&legacy, "reviewed-author");
    let reviewed_title = legacy_extra_str(&legacy, "reviewed-title").map(Title::Single);
    let reviewed_genre = legacy_extra_str(&legacy, "reviewed-genre");
    if genre.is_none() {
        genre = reviewed_genre.clone();
    }
    let event_title = legacy_extra_str(&legacy, "event-title");
    let event_place = legacy_extra_str(&legacy, "event-place")
        .or_else(|| legacy_extra_str(&legacy, "event-location"));
    let event_date = legacy_extra_date(&legacy, "event-date");
    let supplement_number = legacy_extra_str(&legacy, "supplement-number");
    let status = legacy_extra_str(&legacy, "status");
    let available_date = legacy_extra_date(&legacy, "available-date");
    let dimensions = legacy_extra_str(&legacy, "dimensions").or(legacy.dimensions.clone());
    let original_author = legacy_extra_contributor(&legacy, "original-author");
    let original_date = legacy_extra_date(&legacy, "original-date");
    let original_publisher = legacy_extra_str(&legacy, "original-publisher");
    let original_publisher_place = legacy_extra_str(&legacy, "original-publisher-place");
    let serial_type = match legacy.ref_type.as_str() {
        "article-journal" => SerialType::AcademicJournal,
        "article-magazine" => SerialType::Magazine,
        "article-newspaper" => SerialType::Newspaper,
        "broadcast" | "motion_picture" => SerialType::BroadcastProgram,
        _ => SerialType::AcademicJournal,
    };
    let parent_title = legacy.container_title.clone().map(Title::Single);
    let collection_title = legacy.collection_title.clone();

    let volume = legacy.volume.as_ref().map(ToString::to_string);
    let issue = legacy
        .issue
        .clone()
        .or_else(|| {
            if legacy.ref_type == "broadcast" || legacy.ref_type == "motion_picture" {
                legacy.number.as_ref().map(|n| {
                    normalize_broadcast_issue(&legacy.ref_type, legacy.medium.as_deref(), n)
                })
            } else {
                None
            }
        })
        .map(|i| i.to_string());

    let author = legacy.author.clone().map(Contributor::from);
    let translator = legacy.translator.clone().map(Contributor::from);
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Author,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );
    push_legacy_contributor(&mut contributors, ContributorRole::Host, host_names);
    push_legacy_contributor(&mut contributors, ContributorRole::Narrator, narrator_names);
    push_legacy_contributor(&mut contributors, ContributorRole::Producer, producer_names);
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Writer,
        legacy_extra_names(&legacy, "script-writer"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Director,
        legacy
            .director
            .clone()
            .or_else(|| legacy_extra_names(&legacy, "director")),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Performer,
        legacy.contributor.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Unknown("reviewed-author".to_string()),
        reviewed_author.clone(),
    );
    let serial_editor = legacy.editor.clone().map(Contributor::from);
    let mut serial_contributors = Vec::new();
    push_legacy_contributor(
        &mut serial_contributors,
        ContributorRole::Editor,
        legacy.editor.clone(),
    );
    let reviewed = relation_monograph(
        reviewed_title,
        reviewed_author
            .clone()
            .map(Contributor::from)
            .or_else(|| container_author.clone().map(Contributor::from)),
        None,
        reviewed_genre,
        None,
        None,
    );
    if reviewed.is_none() {
        push_legacy_contributor(
            &mut serial_contributors,
            ContributorRole::Author,
            container_author,
        );
    }

    InputReference::SerialComponent(Box::new(SerialComponent {
        id: ctx.id,
        r#type: SerialComponentType::Article,
        title: build_title(ctx.title, ctx.short_title.clone()),
        author,
        translator,
        contributors,
        created: ctx.created,
        issued: ctx.issued,
        container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
            Box::new(Serial {
                id: None,
                r#type: serial_type,
                title: parent_title,
                short_title: ctx.container_title_short.or(ctx.journal_abbreviation),
                container: relation_collection_title(collection_title),
                editor: serial_editor,
                contributors: serial_contributors,
                publisher: publisher_from_parts(
                    legacy.publisher.clone(),
                    legacy.publisher_place.clone(),
                ),
                url: None,
                accessed: None,
                language: None,
                field_languages: HashMap::new(),
                note: None,
                issn: legacy.issn.clone(),
                unknown_fields: Default::default(),
            }),
        )))),
        volume,
        issue,
        number: legacy.number.clone(),
        numbering: {
            let mut numbering = Vec::new();
            if let Some(number_of_volumes) = legacy_number_of_volumes(&legacy) {
                numbering.push(number_of_volumes);
            }
            if let Some(supplement_number) = supplement_number {
                numbering.push(Numbering {
                    r#type: NumberingType::Supplement,
                    value: supplement_number,
                });
            }
            numbering
        },
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        doi: ctx.doi,
        ads_bibcode: None,
        pages: legacy.page,
        genre,
        medium: legacy.medium,
        archive_info: None,
        eprint: None,
        keywords: None,
        unknown_fields: Default::default(),
        section: legacy.section,
        status,
        available_date,
        reviewed,
        original: relation_monograph(
            legacy.original_title.clone().map(Title::Single),
            original_author,
            original_date,
            None,
            original_publisher,
            original_publisher_place,
        ),
        event: relation_event(event_title, event_place, event_date),
        duration: dimensions,
        ..Default::default()
    }))
}

pub(super) fn from_dataset_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let original = legacy_original_relation(&legacy);
    let version = legacy_extra_str(&legacy, "version");

    InputReference::Dataset(Box::new(Dataset {
        id: ctx.id,
        title: build_title(ctx.title.clone(), ctx.short_title.clone()),
        author: legacy.author.map(Contributor::from),
        original,
        created: ctx.created,
        issued: ctx.issued,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        version,
        genre: legacy.genre,
        format: legacy.medium,
        size: None,
        repository: None,
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

pub(super) fn from_document_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let archive_info = archive_info_from_legacy_flat(&legacy);
    let archive = match archive_info.as_ref() {
        Some(ai) if ai.collection.is_some() => None,
        _ => legacy.archive.clone(),
    };
    let original = legacy_original_relation(&legacy);

    let volume = legacy.volume.as_ref().map(ToString::to_string);
    let number = legacy.number.clone();
    let numbering = legacy_number_of_volumes(&legacy)
        .into_iter()
        .collect::<Vec<_>>();
    let scale = legacy_extra_str(&legacy, "scale");
    let dimensions = legacy_extra_str(&legacy, "dimensions");
    // Seed genre from ref_type for CSL types whose round trip through
    // `ref_type()` depends on genre (see `monograph_ref_type`'s Document
    // arm in accessors.rs); never overwrites a user-supplied genre.
    let genre = legacy.genre.clone().or_else(|| {
        matches!(
            legacy.ref_type.as_str(),
            "map" | "figure" | "graphic" | "periodical" | "collection"
        )
        .then(|| legacy.ref_type.clone())
    });
    let author = legacy.author.clone().map(Contributor::from);
    let editor = legacy.editor.clone().map(Contributor::from);
    let translator = legacy.translator.clone().map(Contributor::from);
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Author,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Editor,
        legacy.editor.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );

    InputReference::Monograph(Box::new(Monograph {
        id: ctx.id,
        r#type: MonographType::Document,
        title: build_title(ctx.title, ctx.short_title.clone()),
        short_title: None,
        container: legacy.container_title.map(|t| {
            WorkRelation::Embedded(Box::new(InputReference::Monograph(Box::new(Monograph {
                title: Some(Title::Single(t)),
                ..Default::default()
            }))))
        }),
        author,
        editor,
        translator,
        contributors,
        created: ctx.created,
        issued: ctx.issued,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        isbn: ctx.isbn,
        doi: ctx.doi,
        ads_bibcode: None,
        scale,
        size: dimensions,
        volume,
        number,
        numbering,
        genre,
        medium: legacy.medium,
        edition: legacy.edition.as_ref().map(ToString::to_string),
        pages: legacy.page.map(NumOrStr::Str),
        archive,
        archive_location: legacy.archive_location,
        archive_info,
        eprint: None,
        keywords: None,
        unknown_fields: Default::default(),
        original,
        ..Default::default()
    }))
}

pub(super) fn from_preprint_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let archive_info = archive_info_from_legacy_flat(&legacy);
    let archive = match archive_info.as_ref() {
        Some(ai) if ai.collection.is_some() => None,
        _ => legacy.archive.clone(),
    };

    let author = legacy.author.clone().map(Contributor::from);
    let editor = legacy.editor.clone().map(Contributor::from);
    let translator = legacy.translator.clone().map(Contributor::from);
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Author,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Editor,
        legacy.editor.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Translator,
        legacy.translator.clone(),
    );

    let mut numbering = legacy
        .number
        .as_ref()
        .map(|number| {
            vec![Numbering {
                r#type: NumberingType::Report,
                value: number.clone(),
            }]
        })
        .unwrap_or_default();
    if let Some(number_of_volumes) = legacy_number_of_volumes(&legacy) {
        numbering.push(number_of_volumes);
    }
    let version = legacy_extra_str(&legacy, "version");

    InputReference::Monograph(Box::new(Monograph {
        id: ctx.id,
        r#type: MonographType::Preprint,
        title: build_title(ctx.title, ctx.short_title.clone()),
        short_title: None,
        container: None,
        author,
        editor,
        translator,
        contributors,
        created: ctx.created,
        issued: ctx.issued,
        publisher: publisher_from_parts(legacy.publisher, legacy.publisher_place),
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        doi: ctx.doi,
        isbn: ctx.isbn,
        version,
        numbering,
        genre: legacy.genre,
        medium: legacy.medium,
        archive,
        archive_location: legacy.archive_location,
        archive_info,
        eprint: None,
        keywords: None,
        unknown_fields: Default::default(),
        original: None,
        ..Default::default()
    }))
}

pub(super) fn from_event_ref(
    legacy: csl_legacy::csl_json::Reference,
    ctx: RefContext,
) -> InputReference {
    let original = legacy_original_relation(&legacy);
    let producer_names = legacy_extra_names(&legacy, "producer")
        .or_else(|| legacy_extra_names(&legacy, "executive-producer"));
    let host_names = legacy_extra_names(&legacy, "host");
    let chair_names = legacy_extra_names(&legacy, "chair");
    let available_date = legacy_extra_date(&legacy, "available-date");
    let event_title = legacy_extra_str(&legacy, "event-title");
    let event_place = legacy_extra_str(&legacy, "event-place");
    let event_date = legacy_extra_date(&legacy, "event-date");
    let mut contributors = Vec::new();
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Performer,
        legacy.author.clone(),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Narrator,
        legacy_extra_names(&legacy, "narrator"),
    );
    push_legacy_contributor(
        &mut contributors,
        ContributorRole::Unknown("chair".to_string()),
        chair_names,
    );
    push_legacy_contributor(&mut contributors, ContributorRole::Producer, producer_names);
    push_legacy_contributor(&mut contributors, ContributorRole::Host, host_names);
    if let Some(organizer_name) = legacy.publisher.clone() {
        contributors.push(ContributorEntry {
            roles: ContributorRole::Unknown("organizer".to_string()).into(),
            contributor: Contributor::SimpleName(SimpleName {
                name: organizer_name.into(),
                location: None,
                short_name: None,
            }),
            gender: None,
        });
    }
    InputReference::Event(Box::new(Event {
        id: ctx.id,
        title: build_title(ctx.title, ctx.short_title.clone()),
        original,
        container: legacy.container_title.clone().map(|t| {
            WorkRelation::Embedded(Box::new(InputReference::Monograph(Box::new(Monograph {
                title: Some(Title::Single(t)),
                ..Default::default()
            }))))
        }),
        series: event_title.map(|title| {
            WorkRelation::Embedded(Box::new(InputReference::Collection(Box::new(Collection {
                title: Some(Title::Single(title)),
                ..Default::default()
            }))))
        }),
        location: event_place.or(legacy.publisher_place.clone()),
        date: event_date.or_else(|| (!ctx.issued.is_empty()).then_some(ctx.issued)),
        available_date,
        // Seed genre from ref_type for CSL types whose round trip through
        // `ref_type()` depends on genre (see `event_ref_type` in
        // accessors.rs); never overwrites a user-supplied genre.
        genre: legacy.genre.clone().or_else(|| {
            matches!(legacy.ref_type.as_str(), "speech" | "performance")
                .then(|| legacy.ref_type.clone())
        }),
        network: None,
        contributors,
        url: ctx.url,
        accessed: ctx.accessed,
        language: ctx.language,
        field_languages: HashMap::new(),
        note: ctx.note.map(RichText::Plain),
        unknown_fields: Default::default(),
    }))
}
