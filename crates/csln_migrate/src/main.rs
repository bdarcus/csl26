use csl_legacy::parser::parse_style;
use csln_core::{template::TemplateComponent, BibliographySpec, CitationSpec, Style, StyleInfo};
use csln_migrate::{
    analysis, debug_output::DebugOutputFormatter, passes, provenance::ProvenanceTracker,
    template_resolver, Compressor, MacroInliner, OptionsExtractor, TemplateCompiler, Upsampler,
};
use roxmltree::Document;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Parse command-line arguments
    let mut path = "styles-legacy/apa.csl";
    let mut debug_variable: Option<String> = None;
    let mut template_source: Option<String> = None;
    let mut template_dir: Option<PathBuf> = None;

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
            "--template-source" => {
                if i + 1 < args.len() {
                    template_source = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!(
                        "Error: --template-source requires an argument (auto|hand|inferred|xml)"
                    );
                    std::process::exit(1);
                }
            }
            "--template-dir" => {
                if i + 1 < args.len() {
                    template_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --template-dir requires a path argument");
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

    // Resolve template: try hand-authored, cached inferred, or live inference
    // before falling back to the XML compiler pipeline.
    let style_name = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Determine workspace root by finding the Cargo workspace directory.
    // For relative paths like "styles-legacy/foo.csl", this is the current directory.
    // For absolute paths, walk up from the style file to find the workspace.
    let workspace_root = {
        let style_path = std::path::Path::new(path);
        if style_path.is_absolute() {
            // Walk up to find Cargo.toml
            style_path
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .unwrap_or(style_path.parent().unwrap_or(std::path::Path::new(".")))
                .to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    };

    let use_xml = template_source.as_deref() == Some("xml");
    let resolved = if use_xml {
        None
    } else {
        template_resolver::resolve_template(
            path,
            style_name,
            template_dir.as_deref(),
            &workspace_root,
        )
    };

    // If we have a resolved template, use it directly and skip the XML pipeline.
    // Otherwise, run the full XML compilation pipeline.
    let (new_bib, type_templates, new_cit) = if let Some(ref resolved_tmpl) = resolved {
        eprintln!("Using {} template", resolved_tmpl.source);

        // Override bibliography options with inferred values when available.
        // The XML options extractor often gets the wrong delimiter (e.g., ", " instead of ". ")
        // because it reads from group delimiters rather than actual rendered output.
        if let Some(ref delim) = resolved_tmpl.delimiter {
            eprintln!("  Overriding bibliography separator: {:?}", delim);
            let bib_cfg = options.bibliography.get_or_insert_with(Default::default);
            bib_cfg.separator = Some(delim.clone());
        }

        if let Some(ref suffix) = resolved_tmpl.entry_suffix {
            eprintln!("  Overriding bibliography entry suffix: {:?}", suffix);
            let bib_cfg = options.bibliography.get_or_insert_with(Default::default);
            bib_cfg.entry_suffix = Some(suffix.clone());
        }

        // Still need citation from XML pipeline
        let inliner = MacroInliner::new(&legacy_style);
        let flattened_cit = inliner.inline_citation(&legacy_style);
        let mut upsampler = Upsampler::new();
        upsampler.et_al_min = legacy_style.citation.et_al_min;
        upsampler.et_al_use_first = legacy_style.citation.et_al_use_first;
        let raw_cit = upsampler.upsample_nodes(&flattened_cit);
        let compressor = Compressor;
        let csln_cit = compressor.compress_nodes(raw_cit);
        let template_compiler = TemplateCompiler;
        let new_cit = template_compiler.compile_citation(&csln_cit);
        (resolved_tmpl.bibliography.clone(), None, new_cit)
    } else {
        // Full XML pipeline for both bibliography and citation
        compile_from_xml(&legacy_style, &mut options, enable_provenance, &tracker)
    };

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
                    &legacy_style.macros,
                ),
                multi_cite_delimiter: legacy_style.citation.layout.delimiter.clone(),
                ..Default::default()
            }
        }),
        bibliography: Some(BibliographySpec {
            options: None,
            use_preset: None,
            template: Some(new_bib),
            type_templates,
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

/// Run the full XML compilation pipeline for bibliography and citation templates.
/// This is the fallback when no hand-authored or inferred template is available.
#[allow(clippy::type_complexity)]
fn compile_from_xml(
    legacy_style: &csl_legacy::model::Style,
    options: &mut csln_core::options::Config,
    enable_provenance: bool,
    tracker: &csln_migrate::provenance::ProvenanceTracker,
) -> (
    Vec<TemplateComponent>,
    Option<std::collections::HashMap<String, Vec<TemplateComponent>>>,
    Vec<TemplateComponent>,
) {
    // Extract author suffix before macro inlining (will be lost during inlining)
    let author_suffix = if let Some(ref bib) = legacy_style.bibliography {
        analysis::bibliography::extract_author_suffix(&bib.layout)
    } else {
        None
    };

    // Extract bibliography-specific 'and' setting (may differ from citation)
    let bib_and = analysis::bibliography::extract_bibliography_and(legacy_style);

    // 1. Deconstruction
    let inliner = if enable_provenance {
        MacroInliner::with_provenance(legacy_style, tracker.clone())
    } else {
        MacroInliner::new(legacy_style)
    };
    let flattened_bib = inliner
        .inline_bibliography(legacy_style)
        .unwrap_or_default();
    let flattened_cit = inliner.inline_citation(legacy_style);

    // 2. Semantic Upsampling
    let mut upsampler = if enable_provenance {
        Upsampler::with_provenance(tracker.clone())
    } else {
        Upsampler::new()
    };

    // Set citation-specific thresholds for citation upsampling
    upsampler.et_al_min = legacy_style.citation.et_al_min;
    upsampler.et_al_use_first = legacy_style.citation.et_al_use_first;
    let raw_cit = upsampler.upsample_nodes(&flattened_cit);

    // Set bibliography-specific thresholds for bibliography upsampling
    if let Some(ref bib) = legacy_style.bibliography {
        upsampler.et_al_min = bib.et_al_min;
        upsampler.et_al_use_first = bib.et_al_use_first;
    }
    let raw_bib = upsampler.upsample_nodes(&flattened_bib);

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
        let vol_pages_delim = options.volume_pages_delimiter.clone();
        let style_id = &legacy_style.info.id;
        for component in &mut new_bib {
            apply_type_overrides(
                component,
                vol_pages_delim.clone(),
                volume_list_has_space_prefix,
                style_id,
            );
        }

        // Move DOI/URL to the end of the bibliography template.
        passes::reorder::move_access_components_to_end(&mut new_bib);

        // Ensure publisher and publisher-place are unsuppressed for chapters
        passes::reorder::unsuppress_for_type(&mut new_bib, "chapter");
        passes::reorder::unsuppress_for_type(&mut new_bib, "paper-conference");
        passes::reorder::unsuppress_for_type(&mut new_bib, "thesis");
        passes::reorder::unsuppress_for_type(&mut new_bib, "document");

        // Remove duplicate titles from Lists that already appear at top level.
        passes::deduplicate::deduplicate_titles_in_lists(&mut new_bib);

        // Propagate type-specific overrides within Lists.
        passes::reorder::propagate_list_overrides(&mut new_bib);

        // Remove duplicate nested Lists that have identical contents.
        passes::deduplicate::deduplicate_nested_lists(&mut new_bib);

        // Reorder serial components: container-title before volume.
        passes::reorder::reorder_serial_components(&mut new_bib);

        // Combine volume and issue into a grouped structure: volume(issue)
        passes::grouping::group_volume_and_issue(&mut new_bib, options, style_id);

        // Move pages to after the container-title/volume List for serial types.
        passes::reorder::reorder_pages_for_serials(&mut new_bib);

        // Reorder publisher-place for Chicago journal articles.
        passes::reorder::reorder_publisher_place_for_chicago(&mut new_bib, style_id);

        // Reorder chapters for APA: "In " prefix + editors before book title
        passes::reorder::reorder_chapters_for_apa(&mut new_bib, style_id);

        // Reorder chapters for Chicago: "In" prefix + book title before editors
        passes::reorder::reorder_chapters_for_chicago(&mut new_bib, style_id);

        // Fix Chicago issue placement
        passes::deduplicate::suppress_duplicate_issue_for_journals(&mut new_bib, style_id);
    }

    let type_templates_opt = if type_templates.is_empty() {
        None
    } else {
        Some(type_templates)
    };

    (new_bib, type_templates_opt, new_cit)
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
                    volume_pages_delimiter.clone(),
                    volume_list_has_space_prefix,
                    style_id,
                );
            }
        }
        _ => {}
    }
}
