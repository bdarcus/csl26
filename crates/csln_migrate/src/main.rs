use csl_legacy::parser::parse_style;
use csln_core::{template::TemplateComponent, BibliographySpec, CitationSpec, Style, StyleInfo};
use csln_migrate::{
    analysis, debug_output::DebugOutputFormatter, passes, provenance::ProvenanceTracker,
    Compressor, MacroInliner, OptionsExtractor, TemplateCompiler, Upsampler,
};
use roxmltree::Document;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Parse command-line arguments
    let mut path = "styles/apa.csl";
    let mut debug_variable: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--debug-variable" => {
                if i + 1 < args.len() {
                    debug_variable = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --debug-variable requires an argument");
                    std::process::exit(1);
                }
            }
            arg if !arg.starts_with("--") => {
                path = &args[i];
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Initialize provenance tracking if debug variable is specified
    let enable_provenance = debug_variable.is_some();
    let tracker = ProvenanceTracker::new(enable_provenance);

    eprintln!("Migrating {} to CSLN...", path);

    let text = fs::read_to_string(path)?;
    let doc = Document::parse(&text)?;
    let legacy_style = parse_style(doc.root_element())?;

    // 0. Extract global options (new CSLN Config)
    let mut options = OptionsExtractor::extract(&legacy_style);

    // If it's APA, add the title config
    if legacy_style.info.title.contains("APA") {
        options.titles = Some(csln_core::options::TitlesConfig {
            periodical: Some(csln_core::options::TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            serial: Some(csln_core::options::TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            monograph: Some(csln_core::options::TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            container_monograph: Some(csln_core::options::TitleRendering {
                emph: Some(true),
                // Note: "In " prefix is on the editor component, not here
                ..Default::default()
            }),
            component: Some(csln_core::options::TitleRendering {
                // chapter titles are usually plain in APA
                ..Default::default()
            }),
            default: Some(csln_core::options::TitleRendering {
                ..Default::default()
            }),
            ..Default::default()
        });

        // Add contributor role options for APA
        let mut contributors = options.contributors.unwrap_or_default();
        let mut roles = std::collections::HashMap::new();
        roles.insert(
            "editor".to_string(),
            csln_core::options::RoleRendering {
                name_order: Some(csln_core::template::NameOrder::GivenFirst),
                ..Default::default()
            },
        );
        contributors.role = Some(csln_core::options::RoleOptions {
            roles: Some(roles),
            ..Default::default()
        });
        options.contributors = Some(contributors);
    }

    // Extract author suffix before macro inlining (will be lost during inlining)
    let author_suffix = if let Some(ref bib) = legacy_style.bibliography {
        analysis::bibliography::extract_author_suffix(&bib.layout)
    } else {
        None
    };

    // Extract bibliography-specific 'and' setting (may differ from citation)
    let bib_and = analysis::bibliography::extract_bibliography_and(&legacy_style);

    // 1. Deconstruction
    let inliner = if enable_provenance {
        MacroInliner::with_provenance(&legacy_style, tracker.clone())
    } else {
        MacroInliner::new(&legacy_style)
    };
    let flattened_bib = inliner
        .inline_bibliography(&legacy_style)
        .unwrap_or_default();
    let flattened_cit = inliner.inline_citation(&legacy_style);

    // 2. Semantic Upsampling
    let upsampler = if enable_provenance {
        Upsampler::with_provenance(tracker.clone())
    } else {
        Upsampler::new()
    };
    let raw_bib = upsampler.upsample_nodes(&flattened_bib);
    let raw_cit = upsampler.upsample_nodes(&flattened_cit);

    // 3. Compression (Pattern Recognition)
    let compressor = Compressor;
    let csln_bib = compressor.compress_nodes(raw_bib.clone());
    let csln_cit = compressor.compress_nodes(raw_cit.clone());

    // 4. Template Compilation
    let template_compiler = TemplateCompiler;

    // Detect if this is a numeric style
    let is_numeric = matches!(
        options.processing,
        Some(csln_core::options::Processing::Numeric)
    );

    let (mut new_bib, type_templates) =
        template_compiler.compile_bibliography_with_types(&csln_bib, is_numeric);
    let new_cit = template_compiler.compile_citation(&csln_cit);

    // Record template placements if provenance tracking is enabled
    if enable_provenance {
        for (index, component) in new_bib.iter().enumerate() {
            match component {
                TemplateComponent::Variable(v) => {
                    let var_name = format!("{:?}", v.variable).to_lowercase();
                    tracker.record_template_placement(
                        &var_name,
                        index,
                        "bibliography.template",
                        "Variable",
                    );
                }
                TemplateComponent::Number(n) => {
                    let var_name = format!("{:?}", n.number).to_lowercase();
                    tracker.record_template_placement(
                        &var_name,
                        index,
                        "bibliography.template",
                        "Number",
                    );
                }
                TemplateComponent::Date(d) => {
                    let var_name = format!("{:?}", d.date).to_lowercase();
                    tracker.record_template_placement(
                        &var_name,
                        index,
                        "bibliography.template",
                        "Date",
                    );
                }
                TemplateComponent::Title(t) => {
                    let var_name = format!("{:?}", t.title).to_lowercase();
                    tracker.record_template_placement(
                        &var_name,
                        index,
                        "bibliography.template",
                        "Title",
                    );
                }
                TemplateComponent::Contributor(_) => {
                    tracker.record_template_placement(
                        "contributor",
                        index,
                        "bibliography.template",
                        "Contributor",
                    );
                }
                _ => {}
            }
        }
    }

    // Apply author suffix extracted from original CSL (lost during macro inlining)
    analysis::bibliography::apply_author_suffix(&mut new_bib, author_suffix);

    // Apply bibliography-specific 'and' setting (may differ from citation)
    analysis::bibliography::apply_bibliography_and(&mut new_bib, bib_and);

    // For author-date styles with in-text class, apply standard formatting.
    // Note styles (class="note") should NOT have these transformations applied.
    let is_in_text_class = legacy_style.class == "in-text";
    let is_author_date_processing = matches!(
        options.processing,
        Some(csln_core::options::Processing::AuthorDate)
    );

    // Apply to all in-text styles (both author-date and numeric)
    if is_in_text_class {
        // Add space prefix to volume when it follows parent-serial directly.
        // This handles numeric styles where journal and volume are siblings, not in a List.
        passes::reorder::add_volume_prefix_after_serial(&mut new_bib);
    }

    if is_in_text_class && is_author_date_processing {
        // Detect if the style uses space prefix for volume (Elsevier pattern)
        let volume_list_has_space_prefix = new_bib.iter().any(|c| {
            if let TemplateComponent::List(list) = c {
                let has_volume = list.items.iter().any(|item| {
                    matches!(item, TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Volume)
                });
                if has_volume {
                    // Check if the List has a space-only prefix
                    return list.rendering.prefix.as_deref() == Some(" ");
                }
            }
            false
        });

        // Add type-specific overrides (recursively to handle nested Lists)
        // Pass the extracted volume-pages delimiter for journal article pages
        let vol_pages_delim = options.volume_pages_delimiter;
        let style_id = &legacy_style.info.id;
        for component in &mut new_bib {
            apply_type_overrides(
                component,
                vol_pages_delim,
                volume_list_has_space_prefix,
                style_id,
            );
        }

        // Move DOI/URL to the end of the bibliography template.
        // CSL styles typically have access macros at the end, but during macro
        // expansion they can end up in the middle due to conditional processing.
        passes::reorder::move_access_components_to_end(&mut new_bib);

        // Ensure publisher and publisher-place are unsuppressed for chapters
        passes::reorder::unsuppress_for_type(&mut new_bib, "chapter");
        passes::reorder::unsuppress_for_type(&mut new_bib, "paper-conference");
        passes::reorder::unsuppress_for_type(&mut new_bib, "thesis");
        passes::reorder::unsuppress_for_type(&mut new_bib, "document");

        // Remove duplicate titles from Lists that already appear at top level.
        // This happens when container-title appears in multiple CSL macros.
        passes::deduplicate::deduplicate_titles_in_lists(&mut new_bib);

        // Propagate type-specific overrides within Lists.
        // Ensures sibling components (like volume and container-title) have the same
        // type overrides when they're from the same CSL macro.
        passes::reorder::propagate_list_overrides(&mut new_bib);

        // Remove duplicate nested Lists that have identical contents.
        // This happens when CSL conditions have similar then/else branches.
        passes::deduplicate::deduplicate_nested_lists(&mut new_bib);

        // Reorder serial components: container-title before volume.
        // Due to CSL macro processing, volume often ends up before container-title.
        passes::reorder::reorder_serial_components(&mut new_bib);

        // Combine volume and issue into a grouped structure: volume(issue)
        // MUST run after reorder_serial_components to ensure volume is in correct position first.
        passes::grouping::group_volume_and_issue(&mut new_bib, &options, style_id);

        // Move pages to after the container-title/volume List for serial types.
        passes::reorder::reorder_pages_for_serials(&mut new_bib);

        // Reorder publisher-place for Chicago journal articles.
        // Chicago requires publisher-place to appear immediately after the journal
        // title, before the volume.
        passes::reorder::reorder_publisher_place_for_chicago(&mut new_bib, style_id);

        // Reorder chapters for APA: "In " prefix + editors before book title
        passes::reorder::reorder_chapters_for_apa(&mut new_bib, style_id);

        // Reorder chapters for Chicago: "In" prefix + book title before editors
        passes::reorder::reorder_chapters_for_chicago(&mut new_bib, style_id);

        // Fix Chicago issue placement: suppress issue in parent-monograph lists for journals
        // Chicago puts issue after volume (handled by group_volume_and_issue) but the CSL
        // also has issue in parent-monograph groups which creates duplicates
        passes::deduplicate::suppress_duplicate_issue_for_journals(&mut new_bib, style_id);
    }

    // 5. Build Style in correct format for csln_processor
    let style = Style {
        info: StyleInfo {
            title: Some(legacy_style.info.title.clone()),
            id: Some(legacy_style.info.id.clone()),
            default_locale: legacy_style.default_locale.clone(),
            ..Default::default()
        },
        templates: None,
        options: Some(options.clone()),
        citation: Some({
            let (wrap, prefix, suffix) =
                analysis::citation::infer_citation_wrapping(&legacy_style.citation.layout);
            CitationSpec {
                options: None,
                use_preset: None,
                template: Some(new_cit),
                wrap,
                prefix,
                suffix,
                // Extract delimiter from first group in CSL layout (author-year separator)
                delimiter: analysis::citation::extract_citation_delimiter(
                    &legacy_style.citation.layout,
                ),
                multi_cite_delimiter: legacy_style.citation.layout.delimiter.clone(),
                ..Default::default()
            }
        }),
        bibliography: Some(BibliographySpec {
            options: None,
            use_preset: None,
            template: Some(new_bib),
            // type_templates infrastructure exists but auto-generation is disabled.
            // Different styles have incompatible chapter formats (APA vs others),
            // so we can't apply a single template to all author-date styles.
            type_templates: if type_templates.is_empty() {
                None
            } else {
                Some(type_templates)
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    // Output YAML to stdout
    let yaml = serde_yaml::to_string(&style)?;
    println!("{}", yaml);

    // Output debug information if requested
    if let Some(var_name) = debug_variable {
        eprintln!("\n");
        eprintln!("=== PROVENANCE DEBUG ===\n");
        let debug_output = DebugOutputFormatter::format_variable(&tracker, &var_name);
        eprint!("{}", debug_output);
    }

    Ok(())
}

fn apply_type_overrides(
    component: &mut TemplateComponent,
    volume_pages_delimiter: Option<csln_core::template::DelimiterPunctuation>,
    volume_list_has_space_prefix: bool,
    style_id: &str,
) {
    match component {
        // Primary title: style-specific suffix for articles
        TemplateComponent::Title(t) if t.title == csln_core::template::TitleType::Primary => {
            let is_apa = style_id.contains("apa");
            if is_apa {
                let mut new_ovr = std::collections::HashMap::new();
                new_ovr.insert(
                    "article-journal".to_string(),
                    csln_core::template::Rendering {
                        suffix: Some(". ".to_string()),
                        ..Default::default()
                    },
                );
                // Merge instead of overwrite
                let overrides = t
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                for (k, v) in new_ovr {
                    overrides.insert(k, v);
                }
            }
        }
        // Container-title (parent-monograph): style-specific unsuppression
        TemplateComponent::Title(t)
            if t.title == csln_core::template::TitleType::ParentMonograph =>
        {
            let is_apa = style_id.contains("apa");
            if is_apa {
                let mut new_ovr = std::collections::HashMap::new();
                new_ovr.insert(
                    "paper-conference".to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(true),
                        ..Default::default()
                    },
                );
                // Merge instead of overwrite
                let overrides = t
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                for (k, v) in new_ovr {
                    overrides.insert(k, v);
                }
            }
        }
        // Container-title (parent-serial): style-specific suffix and unsuppression
        // - APA: comma suffix, no prefix
        // - Chicago: space suffix (prevents default period separator)
        // - Elsevier: space prefix (handled by List), no suffix needed
        TemplateComponent::Title(t) if t.title == csln_core::template::TitleType::ParentSerial => {
            let is_chicago = style_id.contains("chicago");
            let mut new_ovr = std::collections::HashMap::new();

            // Always unsuppress article-journal (journal title must show)
            let suffix = if volume_list_has_space_prefix {
                // Elsevier: no suffix, spacing handled by List prefix
                None
            } else if is_chicago {
                Some(" ".to_string())
            } else {
                // APA: comma suffix
                Some(",".to_string())
            };

            new_ovr.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    suffix,
                    suppress: Some(false),
                    ..Default::default()
                },
            );

            // Ensure paper-conference shows container title (proceedings name)
            new_ovr.insert(
                "paper-conference".to_string(),
                csln_core::template::Rendering {
                    suffix: Some(",".to_string()),
                    suppress: Some(false),
                    ..Default::default()
                },
            );

            // Merge instead of overwrite
            let overrides = t
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Publisher: suppress for journal articles (journals don't have publishers in bib)
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::Publisher =>
        {
            let mut new_ovr = std::collections::HashMap::new();
            new_ovr.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                },
            );
            // Merge instead of overwrite
            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Publisher-place: suppress for journal articles
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::PublisherPlace =>
        {
            let mut new_ovr = std::collections::HashMap::new();
            new_ovr.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                },
            );
            // Merge instead of overwrite
            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Pages: apply volume-pages delimiter for journal articles
        TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Pages => {
            if let Some(delim) = volume_pages_delimiter {
                let mut new_ovr = std::collections::HashMap::new();
                new_ovr.insert(
                    "article-journal".to_string(),
                    csln_core::template::Rendering {
                        prefix: Some(match delim {
                            csln_core::template::DelimiterPunctuation::Comma => ", ".to_string(),
                            csln_core::template::DelimiterPunctuation::Colon => ":".to_string(),
                            csln_core::template::DelimiterPunctuation::Space => " ".to_string(),
                            _ => "".to_string(),
                        }),
                        ..Default::default()
                    },
                );
                // Merge instead of overwrite
                let overrides = n
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                for (k, v) in new_ovr {
                    overrides.insert(k, v);
                }
            }
        }
        TemplateComponent::List(list) => {
            for item in &mut list.items {
                apply_type_overrides(
                    item,
                    volume_pages_delimiter,
                    volume_list_has_space_prefix,
                    style_id,
                );
            }
        }
        _ => {}
    }
}
