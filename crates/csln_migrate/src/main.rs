use csl_legacy::{
    model::{CslNode, Layout},
    parser::parse_style,
};
use csln_core::{
    template::{
        DelimiterPunctuation, Rendering, SimpleVariable, TemplateComponent, TemplateList,
        TemplateVariable, WrapPunctuation,
    },
    BibliographySpec, CitationSpec, Style, StyleInfo,
};
use csln_migrate::{
    analysis, debug_output::DebugOutputFormatter, passes, preset_detector,
    provenance::ProvenanceTracker, template_resolver, Compressor, MacroInliner, OptionsExtractor,
    TemplateCompiler, Upsampler,
};
use roxmltree::Document;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let program_name = args
        .first()
        .and_then(|arg| std::path::Path::new(arg).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("csln-migrate");

    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help(program_name);
        return Ok(());
    }

    // Parse command-line arguments
    let mut path = "styles-legacy/apa.csl";
    let mut debug_variable: Option<String> = None;
    let mut template_mode = template_resolver::TemplateMode::Auto;
    let mut template_dir: Option<PathBuf> = None;
    let mut min_template_confidence = 0.70_f64;

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
                    template_mode = match args[i + 1].parse::<template_resolver::TemplateMode>() {
                        Ok(mode) => mode,
                        Err(msg) => {
                            eprintln!("Error: {}", msg);
                            std::process::exit(1);
                        }
                    };
                    i += 2;
                } else {
                    eprintln!(
                        "Error: --template-source requires an argument (auto|hand|inferred|xml)"
                    );
                    std::process::exit(1);
                }
            }
            "--min-template-confidence" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f64>() {
                        Ok(val) if (0.0..=1.0).contains(&val) => {
                            min_template_confidence = val;
                            i += 2;
                        }
                        _ => {
                            eprintln!(
                                "Error: --min-template-confidence requires a number in [0.0, 1.0]"
                            );
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: --min-template-confidence requires a numeric argument");
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
            arg if !arg.starts_with('-') => {
                path = &args[i];
                i += 1;
            }
            _ => {
                eprintln!("Error: unknown argument '{}'", args[i]);
                eprintln!();
                print_help(program_name);
                std::process::exit(1);
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

    let mut resolved = template_resolver::resolve_templates(
        path,
        style_name,
        template_dir.as_deref(),
        &workspace_root,
        template_mode,
        min_template_confidence,
    );

    // Guardrails for inferred citation templates:
    // - Empty citation templates regress fidelity heavily.
    // - Numeric styles require citation-number in citation templates.
    let mut reject_inferred_citation_reason: Option<&str> = None;
    if let Some(resolved_cit) = resolved.citation.as_ref() {
        let is_inferred_source = matches!(
            resolved_cit.source,
            template_resolver::TemplateSource::InferredCached(_)
                | template_resolver::TemplateSource::InferredLive
        );
        if is_inferred_source {
            if resolved_cit.template.is_empty() {
                reject_inferred_citation_reason = Some("empty citation template");
            } else if matches!(
                options.processing,
                Some(csln_core::options::Processing::Numeric)
            ) && !citation_template_has_citation_number(&resolved_cit.template)
            {
                reject_inferred_citation_reason =
                    Some("numeric style citation template missing citation-number");
            } else if legacy_style.class == "note"
                && note_citation_template_is_underfit(&resolved_cit.template)
            {
                reject_inferred_citation_reason =
                    Some("note style citation template is contributor-only underfit");
            }
        }
    }
    if let Some(reason) = reject_inferred_citation_reason {
        eprintln!(
            "Rejecting inferred citation template for {}: {}. Falling back to XML citation template.",
            style_name, reason
        );
        resolved.citation = None;
    }

    // Heuristic normalization for note styles:
    // If inferred citation template is a simple author-year shape, prefer short
    // contributor form to align with typical note citation behavior.
    let should_normalize_author_year_citations = legacy_style.class == "note"
        || matches!(
            options.processing,
            Some(csln_core::options::Processing::AuthorDate)
        );

    if should_normalize_author_year_citations {
        if let Some(resolved_cit) = resolved.citation.as_mut() {
            let is_inferred_source = matches!(
                resolved_cit.source,
                template_resolver::TemplateSource::InferredCached(_)
                    | template_resolver::TemplateSource::InferredLive
            );
            if is_inferred_source
                && citation_template_is_author_year_only(&resolved_cit.template)
                && normalize_contributor_form_to_short(&mut resolved_cit.template)
            {
                eprintln!(
                    "Normalized citation contributor form to short for {} (author-year inferred citation template).",
                    style_name
                );
            }
        }
    }

    let needs_xml_bib = resolved.bibliography.is_none();
    let needs_xml_cit = resolved.citation.is_none();

    let xml_fallback = if needs_xml_bib || needs_xml_cit {
        Some(compile_from_xml(
            &legacy_style,
            &mut options,
            enable_provenance,
            &tracker,
        ))
    } else {
        None
    };

    if let Some(ref resolved_bib) = resolved.bibliography {
        eprintln!("Using {} bibliography template", resolved_bib.source);
        if let Some(conf) = resolved_bib.confidence {
            eprintln!("  bibliography confidence: {:.0}%", conf * 100.0);
        }
    } else {
        eprintln!(
            "Using {} bibliography template",
            template_resolver::TemplateSource::XmlCompiled
        );
    }

    if let Some(ref resolved_cit) = resolved.citation {
        eprintln!("Using {} citation template", resolved_cit.source);
        if let Some(conf) = resolved_cit.confidence {
            eprintln!("  citation confidence: {:.0}%", conf * 100.0);
        }
    } else {
        eprintln!(
            "Using {} citation template",
            template_resolver::TemplateSource::XmlCompiled
        );
    }

    let (new_bib, type_templates) = if let Some(ref resolved_bib) = resolved.bibliography {
        (resolved_bib.template.clone(), None)
    } else {
        let (new_bib, type_templates, _) = xml_fallback
            .as_ref()
            .expect("XML fallback must exist when bibliography is unresolved");
        (new_bib.clone(), type_templates.clone())
    };

    let mut new_cit = if let Some(ref resolved_cit) = resolved.citation {
        resolved_cit.template.clone()
    } else {
        let (_, _, new_cit) = xml_fallback
            .as_ref()
            .expect("XML fallback must exist when citation is unresolved");
        new_cit.clone()
    };

    // Override bibliography options with inferred values when available.
    // The XML options extractor often gets the wrong delimiter because it reads group
    // delimiters rather than rendered output.
    if let Some(ref resolved_bib) = resolved.bibliography {
        let is_inferred_source = matches!(
            resolved_bib.source,
            template_resolver::TemplateSource::InferredCached(_)
                | template_resolver::TemplateSource::InferredLive
        );
        let allow_bib_punctuation_override = !(legacy_style.class == "note" && is_inferred_source);

        if allow_bib_punctuation_override {
            if let Some(ref delim) = resolved_bib.delimiter {
                eprintln!("  Overriding bibliography separator: {:?}", delim);
                let bib_cfg = options.bibliography.get_or_insert_with(Default::default);
                bib_cfg.separator = Some(delim.clone());
            }

            if let Some(ref suffix) = resolved_bib.entry_suffix {
                eprintln!("  Overriding bibliography entry suffix: {:?}", suffix);
                let bib_cfg = options.bibliography.get_or_insert_with(Default::default);
                bib_cfg.entry_suffix = Some(suffix.clone());
            }
        } else {
            eprintln!(
                "  Skipping inferred bibliography separator/entry-suffix override for note style."
            );
        }
    }

    let (mut citation_wrap, mut citation_prefix, mut citation_suffix) =
        analysis::citation::infer_citation_wrapping(&legacy_style.citation.layout);
    let mut citation_delimiter = analysis::citation::extract_citation_delimiter(
        &legacy_style.citation.layout,
        &legacy_style.macros,
    );

    // Output-driven citation metadata is higher fidelity than XML analysis when available.
    if let Some(ref resolved_cit) = resolved.citation {
        if let Some(ref wrap) = resolved_cit.wrap {
            citation_wrap = Some(wrap.clone());
            citation_prefix = None;
            citation_suffix = None;
        }
        if let Some(ref delim) = resolved_cit.delimiter {
            citation_delimiter = Some(delim.clone());
        }
    }

    // Numeric citation fixups informed by migration quality runs:
    // - Keep locator labels when legacy style has a citation-locator macro.
    // - Preserve per-item wrapping for grouped numeric layouts (e.g., IEEE).
    if matches!(
        options.processing,
        Some(csln_core::options::Processing::Numeric)
    ) {
        ensure_numeric_locator_citation_component(&legacy_style.citation.layout, &mut new_cit);
        move_group_wrap_to_citation_items(
            &legacy_style.citation.layout,
            &mut new_cit,
            &mut citation_wrap,
        );
    }

    // 5. Build Style in correct format for csln_processor
    let citation_scope_options =
        csln_migrate::options_extractor::contributors::extract_citation_contributor_overrides(
            &legacy_style,
        )
        .map(|contributors| csln_core::options::Config {
            contributors: Some(contributors),
            ..Default::default()
        });

    let bibliography_scope_options =
        csln_migrate::options_extractor::contributors::extract_bibliography_contributor_overrides(
            &legacy_style,
        )
        .map(|contributors| csln_core::options::Config {
            contributors: Some(contributors),
            ..Default::default()
        });

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
            CitationSpec {
                options: citation_scope_options,
                use_preset: None,
                template: Some(new_cit),
                wrap: citation_wrap,
                prefix: citation_prefix,
                suffix: citation_suffix,
                delimiter: citation_delimiter,
                multi_cite_delimiter: legacy_style.citation.layout.delimiter.clone(),
                ..Default::default()
            }
        }),
        bibliography: Some(BibliographySpec {
            options: bibliography_scope_options,
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

fn print_help(program_name: &str) {
    eprintln!("CSLN style migration tool");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  {program_name} [STYLE.csl] [options]");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  STYLE.csl                       Input CSL 1.0 style path");
    eprintln!("                                  (default: styles-legacy/apa.csl)");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -h, --help                      Show this help text");
    eprintln!("  --debug-variable <name>         Print provenance details for one variable");
    eprintln!("  --template-source <mode>        Template source: auto|hand|inferred|xml");
    eprintln!("  --template-dir <path>           Override directory for hand-authored templates");
    eprintln!("  --min-template-confidence <n>   Minimum inferred confidence [0.0, 1.0]");
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
    Option<std::collections::HashMap<csln_core::template::TypeSelector, Vec<TemplateComponent>>>,
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

    // Detect holistic style preset for semantic fixups
    let style_preset = preset_detector::detect_style_preset(options);
    if let Some(preset) = style_preset {
        eprintln!("Detected style preset: {:?}", preset);
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
        for component in &mut new_bib {
            apply_type_overrides(
                component,
                vol_pages_delim.clone(),
                volume_list_has_space_prefix,
                style_preset,
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
        passes::grouping::group_volume_and_issue(&mut new_bib, options, style_preset);

        // Move pages to after the container-title/volume List for serial types.
        passes::reorder::reorder_pages_for_serials(&mut new_bib);

        // Reorder publisher-place for Chicago journal articles.
        passes::reorder::reorder_publisher_place_for_chicago(&mut new_bib, style_preset);

        // Reorder chapters for APA: "In " prefix + editors before book title
        passes::reorder::reorder_chapters_for_apa(&mut new_bib, style_preset);

        // Reorder chapters for Chicago: "In" prefix + book title before editors
        passes::reorder::reorder_chapters_for_chicago(&mut new_bib, style_preset);

        // Fix Chicago issue placement
        passes::deduplicate::suppress_duplicate_issue_for_journals(&mut new_bib, style_preset);
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
    style_preset: Option<preset_detector::StylePreset>,
) {
    use preset_detector::StylePreset;
    match component {
        // Primary title: style-specific suffix for articles
        TemplateComponent::Title(t) if t.title == csln_core::template::TitleType::Primary => {
            if matches!(style_preset, Some(StylePreset::Apa)) {
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
                use csln_core::template::ComponentOverride;
                for (k, v) in new_ovr {
                    overrides.insert(
                        csln_core::template::TypeSelector::Single(k),
                        ComponentOverride::Rendering(v),
                    );
                }
            }
        }
        // Container-title (parent-monograph): style-specific unsuppression
        TemplateComponent::Title(t)
            if t.title == csln_core::template::TitleType::ParentMonograph =>
        {
            if matches!(style_preset, Some(StylePreset::Apa)) {
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
                use csln_core::template::ComponentOverride;
                for (k, v) in new_ovr {
                    overrides.insert(
                        csln_core::template::TypeSelector::Single(k),
                        ComponentOverride::Rendering(v),
                    );
                }
            }
        }
        // Container-title (parent-serial): style-specific suffix and unsuppression
        // - APA: comma suffix, no prefix
        // - Chicago: space suffix (prevents default period separator)
        // - Elsevier: space prefix (handled by List), no suffix needed
        TemplateComponent::Title(t) if t.title == csln_core::template::TitleType::ParentSerial => {
            let is_chicago = matches!(style_preset, Some(StylePreset::Chicago));
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
            use csln_core::template::ComponentOverride;
            for (k, v) in new_ovr {
                overrides.insert(
                    csln_core::template::TypeSelector::Single(k),
                    ComponentOverride::Rendering(v),
                );
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
            use csln_core::template::ComponentOverride;
            for (k, v) in new_ovr {
                overrides.insert(
                    csln_core::template::TypeSelector::Single(k),
                    ComponentOverride::Rendering(v),
                );
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
            use csln_core::template::ComponentOverride;
            for (k, v) in new_ovr {
                overrides.insert(
                    csln_core::template::TypeSelector::Single(k),
                    ComponentOverride::Rendering(v),
                );
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
                use csln_core::template::ComponentOverride;
                for (k, v) in new_ovr {
                    overrides.insert(
                        csln_core::template::TypeSelector::Single(k),
                        ComponentOverride::Rendering(v),
                    );
                }
            }
        }
        TemplateComponent::List(list) => {
            for item in &mut list.items {
                apply_type_overrides(
                    item,
                    volume_pages_delimiter.clone(),
                    volume_list_has_space_prefix,
                    style_preset,
                );
            }
        }
        _ => {}
    }
}

fn ensure_numeric_locator_citation_component(layout: &Layout, template: &mut [TemplateComponent]) {
    if !layout_uses_citation_locator(layout) || citation_template_has_locator(template) {
        return;
    }

    let locator_component = TemplateComponent::Variable(TemplateVariable {
        variable: SimpleVariable::Locator,
        show_label: Some(true),
        rendering: Rendering {
            prefix: Some(", ".to_string()),
            ..Default::default()
        },
        ..Default::default()
    });

    if let Some(idx) = template.iter().position(component_has_citation_number) {
        match &mut template[idx] {
            TemplateComponent::List(list) => {
                list.items.push(locator_component);
                if list.delimiter.is_none() {
                    list.delimiter = Some(DelimiterPunctuation::None);
                }
            }
            _ => {
                let original = template[idx].clone();
                template[idx] = TemplateComponent::List(TemplateList {
                    items: vec![original, locator_component],
                    delimiter: Some(DelimiterPunctuation::None),
                    ..Default::default()
                });
            }
        }
    }
}

fn move_group_wrap_to_citation_items(
    layout: &Layout,
    template: &mut [TemplateComponent],
    citation_wrap: &mut Option<WrapPunctuation>,
) {
    let Some(wrap) = citation_wrap.clone() else {
        return;
    };

    if !layout_has_group_wrap_for_citation_number(layout, &wrap) {
        return;
    }

    for component in template.iter_mut() {
        if component_has_citation_number(component) {
            apply_wrap_to_component(component, wrap.clone());
        }
    }
    *citation_wrap = None;
}

fn apply_wrap_to_component(component: &mut TemplateComponent, wrap: WrapPunctuation) {
    match component {
        TemplateComponent::Number(n) => {
            if n.rendering.wrap.is_none() {
                n.rendering.wrap = Some(wrap);
            }
        }
        TemplateComponent::List(list) => {
            if list.rendering.wrap.is_none() {
                list.rendering.wrap = Some(wrap);
            }
        }
        _ => {}
    }
}

fn citation_template_has_locator(template: &[TemplateComponent]) -> bool {
    template.iter().any(component_has_locator)
}

fn component_has_locator(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Variable(v) => v.variable == SimpleVariable::Locator,
        TemplateComponent::List(list) => list.items.iter().any(component_has_locator),
        _ => false,
    }
}

fn layout_uses_citation_locator(layout: &Layout) -> bool {
    nodes_use_citation_locator(&layout.children)
}

fn nodes_use_citation_locator(nodes: &[CslNode]) -> bool {
    nodes.iter().any(node_uses_citation_locator)
}

fn node_uses_citation_locator(node: &CslNode) -> bool {
    match node {
        CslNode::Text(t) => {
            t.variable.as_deref() == Some("locator")
                || t.macro_name
                    .as_deref()
                    .is_some_and(|name| name.contains("citation-locator"))
        }
        CslNode::Group(g) => nodes_use_citation_locator(&g.children),
        CslNode::Choose(c) => {
            nodes_use_citation_locator(&c.if_branch.children)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_use_citation_locator(&b.children))
                || c.else_branch
                    .as_ref()
                    .is_some_and(|children| nodes_use_citation_locator(children))
        }
        _ => false,
    }
}

fn layout_has_group_wrap_for_citation_number(layout: &Layout, wrap: &WrapPunctuation) -> bool {
    let (prefix, suffix) = match wrap {
        WrapPunctuation::Brackets => ("[", "]"),
        WrapPunctuation::Parentheses => ("(", ")"),
        _ => return false,
    };
    nodes_have_wrapped_citation_number_group(&layout.children, prefix, suffix)
}

fn nodes_have_wrapped_citation_number_group(nodes: &[CslNode], prefix: &str, suffix: &str) -> bool {
    nodes
        .iter()
        .any(|node| node_has_wrapped_citation_number_group(node, prefix, suffix))
}

fn node_has_wrapped_citation_number_group(node: &CslNode, prefix: &str, suffix: &str) -> bool {
    match node {
        CslNode::Group(g) => {
            if g.prefix.as_deref() == Some(prefix)
                && g.suffix.as_deref() == Some(suffix)
                && nodes_contain_citation_number(&g.children)
            {
                return true;
            }
            nodes_have_wrapped_citation_number_group(&g.children, prefix, suffix)
        }
        CslNode::Choose(c) => {
            nodes_have_wrapped_citation_number_group(&c.if_branch.children, prefix, suffix)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_have_wrapped_citation_number_group(&b.children, prefix, suffix))
                || c.else_branch.as_ref().is_some_and(|children| {
                    nodes_have_wrapped_citation_number_group(children, prefix, suffix)
                })
        }
        _ => false,
    }
}

fn nodes_contain_citation_number(nodes: &[CslNode]) -> bool {
    nodes.iter().any(node_contains_citation_number)
}

fn node_contains_citation_number(node: &CslNode) -> bool {
    match node {
        CslNode::Text(t) => t.variable.as_deref() == Some("citation-number"),
        CslNode::Number(n) => n.variable == "citation-number",
        CslNode::Group(g) => nodes_contain_citation_number(&g.children),
        CslNode::Choose(c) => {
            nodes_contain_citation_number(&c.if_branch.children)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_contain_citation_number(&b.children))
                || c.else_branch
                    .as_ref()
                    .is_some_and(|children| nodes_contain_citation_number(children))
        }
        _ => false,
    }
}

fn citation_template_has_citation_number(template: &[TemplateComponent]) -> bool {
    template.iter().any(component_has_citation_number)
}

fn component_has_citation_number(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Number(n) => {
            n.number == csln_core::template::NumberVariable::CitationNumber
        }
        TemplateComponent::List(list) => list.items.iter().any(component_has_citation_number),
        _ => false,
    }
}

fn note_citation_template_is_underfit(template: &[TemplateComponent]) -> bool {
    template.len() == 1 && component_is_contributor_only(&template[0])
}

fn component_is_contributor_only(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Contributor(_) => true,
        TemplateComponent::List(list) => list.items.iter().all(component_is_contributor_only),
        _ => false,
    }
}

fn citation_template_is_author_year_only(template: &[TemplateComponent]) -> bool {
    let mut has_contributor = false;
    let mut has_date = false;

    for component in template {
        match component {
            TemplateComponent::Contributor(_) => has_contributor = true,
            TemplateComponent::Date(_) => has_date = true,
            TemplateComponent::List(list) => {
                for item in &list.items {
                    match item {
                        TemplateComponent::Contributor(_) => has_contributor = true,
                        TemplateComponent::Date(_) => has_date = true,
                        _ => return false,
                    }
                }
            }
            _ => return false,
        }
    }

    has_contributor && has_date
}

fn normalize_contributor_form_to_short(template: &mut [TemplateComponent]) -> bool {
    let mut changed = false;
    for component in template {
        match component {
            TemplateComponent::Contributor(c) => {
                if c.form == csln_core::template::ContributorForm::Long {
                    c.form = csln_core::template::ContributorForm::Short;
                    changed = true;
                }
            }
            TemplateComponent::List(list) => {
                if normalize_contributor_form_to_short(&mut list.items) {
                    changed = true;
                }
            }
            _ => {}
        }
    }
    changed
}
