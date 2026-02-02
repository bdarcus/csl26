use csl_legacy::parser::parse_style;
use csln_core::{
    template::{TemplateComponent, WrapPunctuation},
    BibliographySpec, CitationSpec, Style, StyleInfo,
};
use csln_migrate::{Compressor, MacroInliner, OptionsExtractor, TemplateCompiler, Upsampler};
use roxmltree::Document;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("styles/apa.csl");

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
        extract_author_suffix(&bib.layout)
    } else {
        None
    };

    // Extract bibliography-specific 'and' setting (may differ from citation)
    let bib_and = extract_bibliography_and(&legacy_style);

    // 1. Deconstruction
    let inliner = MacroInliner::new(&legacy_style);
    let flattened_bib = inliner
        .inline_bibliography(&legacy_style)
        .unwrap_or_default();
    let flattened_cit = inliner.inline_citation(&legacy_style);

    // 2. Semantic Upsampling
    let upsampler = Upsampler;
    let raw_bib = upsampler.upsample_nodes(&flattened_bib);
    let raw_cit = upsampler.upsample_nodes(&flattened_cit);

    // 3. Compression (Pattern Recognition)
    let compressor = Compressor;
    let csln_bib = compressor.compress_nodes(raw_bib.clone());
    let csln_cit = compressor.compress_nodes(raw_cit.clone());

    // 4. Template Compilation
    let template_compiler = TemplateCompiler;
    let (mut new_bib, type_templates) =
        template_compiler.compile_bibliography_with_types(&csln_bib);
    let mut new_cit = template_compiler.compile_citation(&csln_cit);

    // Apply author suffix extracted from original CSL (lost during macro inlining)
    apply_author_suffix(&mut new_bib, author_suffix);

    // Apply bibliography-specific 'and' setting (may differ from citation)
    apply_bibliography_and(&mut new_bib, bib_and);

    // For author-date styles with in-text class, apply standard formatting.
    // Note styles (class="note") should NOT have these transformations applied.
    let is_in_text_class = legacy_style.class == "in-text";
    let is_author_date_processing = matches!(
        options.processing,
        Some(csln_core::options::Processing::AuthorDate)
    ) || matches!(
        options.processing,
        Some(csln_core::options::Processing::Custom(ref c)) if c.disambiguate.as_ref().is_some_and(|d| d.year_suffix)
    );

    if is_in_text_class && is_author_date_processing {
        // Citation: ensure author (short) + date (year)
        let has_author = new_cit.iter().any(|c| {
            matches!(c, TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Author)
        });
        if !has_author {
            new_cit.insert(
                0,
                TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                    contributor: csln_core::template::ContributorRole::Author,
                    form: csln_core::template::ContributorForm::Short,
                    name_order: None,
                    delimiter: None,
                    rendering: csln_core::template::Rendering::default(),
                    ..Default::default()
                }),
            );
        }
        let has_date = new_cit.iter().any(|c| {
            matches!(c, TemplateComponent::Date(td) if td.date == csln_core::template::DateVariable::Issued)
        });
        if !has_date {
            let insert_pos = new_cit
                .iter()
                .position(|c| !matches!(c, TemplateComponent::Contributor(_)))
                .unwrap_or(1);
            new_cit.insert(
                insert_pos,
                TemplateComponent::Date(csln_core::template::TemplateDate {
                    date: csln_core::template::DateVariable::Issued,
                    form: csln_core::template::DateForm::Year,
                    rendering: csln_core::template::Rendering::default(),
                    ..Default::default()
                }),
            );
        }
        // Keep only essential citation components for author-date
        new_cit.retain(|c| {
            matches!(c,
                TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Author
            ) || matches!(c,
                TemplateComponent::Date(td) if td.date == csln_core::template::DateVariable::Issued
            )
        });

        // Bibliography: fix author form (long), title emph
        // NOTE: Date wrapping (parentheses vs period) is now inferred from the original
        // CSL style during template compilation, not hard-coded here.
        for component in &mut new_bib {
            match component {
                TemplateComponent::Contributor(tc)
                    if tc.contributor == csln_core::template::ContributorRole::Author =>
                {
                    tc.form = csln_core::template::ContributorForm::Long;
                }
                TemplateComponent::Date(td)
                    if td.date == csln_core::template::DateVariable::Issued =>
                {
                    td.form = csln_core::template::DateForm::Year;
                    // Preserve wrap from original style (already extracted during compilation)
                }
                TemplateComponent::Title(_) => {
                    // Title formatting is now handled by the global TitlesConfig
                }
                _ => {}
            }
        }

        // Check if the volume List has a space prefix (Elsevier-style)
        // vs no prefix (APA-style). This determines whether to add suffix to journal title.
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
        move_access_components_to_end(&mut new_bib);

        // Ensure publisher and publisher-place are unsuppressed for chapters
        unsuppress_for_type(&mut new_bib, "chapter");
        unsuppress_for_type(&mut new_bib, "paper-conference");

        // Remove duplicate titles from Lists that already appear at top level.
        // This happens when container-title appears in multiple CSL macros.
        deduplicate_titles_in_lists(&mut new_bib);

        // Propagate type-specific overrides within Lists.
        // Ensures sibling components (like volume and container-title) have the same
        // type overrides when they're from the same CSL macro.
        propagate_list_overrides(&mut new_bib);

        // Remove duplicate nested Lists that have identical contents.
        // This happens when CSL conditions have similar then/else branches.
        deduplicate_nested_lists(&mut new_bib);

        // Reorder serial components: container-title before volume.
        // Due to CSL macro processing, volume often ends up before container-title.
        reorder_serial_components(&mut new_bib);

        // Combine volume and issue into a grouped structure: volume(issue)
        // MUST run after reorder_serial_components to ensure volume is in correct position first.
        group_volume_and_issue(&mut new_bib, &options);

        // Move pages to after the container-title/volume List for serial types.
        reorder_pages_for_serials(&mut new_bib);

        // Reorder publisher-place for Chicago journal articles.
        // Chicago requires publisher-place to appear immediately after the journal
        // title, before the volume.
        reorder_publisher_place_for_chicago(&mut new_bib, style_id);

        // Reorder chapters for Chicago: "In" prefix + book title before editors
        reorder_chapters_for_chicago(&mut new_bib, style_id);

        // Fix Chicago issue placement: suppress issue in parent-monograph lists for journals
        // Chicago puts issue after volume (handled by group_volume_and_issue) but the CSL
        // also has issue in parent-monograph groups which creates duplicates
        suppress_duplicate_issue_for_journals(&mut new_bib, style_id);
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
            let (wrap, prefix, suffix) = infer_citation_wrapping(&legacy_style.citation.layout);
            CitationSpec {
                options: None,
                use_preset: None,
                template: Some(new_cit),
                wrap,
                prefix,
                suffix,
                // Extract delimiter from first group in CSL layout (author-year separator)
                delimiter: extract_citation_delimiter(&legacy_style.citation.layout),
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

    Ok(())
}

/// Infer citation wrapping from CSL layout.
///
/// Checks both layout-level prefix/suffix AND group-level wrapping.
/// Numeric styles like IEEE use `<group prefix="[" suffix="]">` inside the layout,
/// while author-date styles use `<layout prefix="(" suffix=")">`.
///
/// Returns (wrap, prefix, suffix) - uses wrap when possible, falls back to affixes.
fn infer_citation_wrapping(
    layout: &csl_legacy::model::Layout,
) -> (Option<WrapPunctuation>, Option<String>, Option<String>) {
    use csl_legacy::model::CslNode;

    // First check layout-level prefix/suffix
    let layout_wrap = match (layout.prefix.as_deref(), layout.suffix.as_deref()) {
        (Some("("), Some(")")) => Some((Some(WrapPunctuation::Parentheses), None, None)),
        (Some("["), Some("]")) => Some((Some(WrapPunctuation::Brackets), None, None)),
        _ => None,
    };

    if let Some(wrap) = layout_wrap {
        return wrap;
    }

    // Check for group-level wrapping (common in numeric styles like IEEE)
    // Pattern: <layout><group prefix="[" suffix="]">...</group></layout>
    fn find_group_wrapping(
        nodes: &[CslNode],
    ) -> Option<(Option<WrapPunctuation>, Option<String>, Option<String>)> {
        for node in nodes {
            if let CslNode::Group(g) = node {
                match (g.prefix.as_deref(), g.suffix.as_deref()) {
                    (Some("("), Some(")")) => {
                        return Some((Some(WrapPunctuation::Parentheses), None, None))
                    }
                    (Some("["), Some("]")) => {
                        return Some((Some(WrapPunctuation::Brackets), None, None))
                    }
                    _ => {
                        // Recurse into nested groups
                        if let Some(wrap) = find_group_wrapping(&g.children) {
                            return Some(wrap);
                        }
                    }
                }
            }
        }
        None
    }

    if let Some(wrap) = find_group_wrapping(&layout.children) {
        return wrap;
    }

    // Fall back to layout prefix/suffix if set (edge cases)
    match (layout.prefix.as_deref(), layout.suffix.as_deref()) {
        (None, None) | (Some(""), Some("")) | (Some(""), None) | (None, Some("")) => {
            (None, None, None)
        }
        _ => (None, layout.prefix.clone(), layout.suffix.clone()),
    }
}

/// Extract the intra-citation delimiter from the layout.
///
/// CSL styles encode the author-year separator in two ways:
///
/// 1. Group delimiter (most common):
///    <group delimiter=", ">
///    <text macro="author-short"/>
///    <text macro="year-date"/>
///    </group>
///
/// 2. Prefix on date element:
///    <text macro="author-short"/>
///    <text macro="year-date" prefix=" "/>
///
/// We want the OUTERMOST group that contains both author and date macros,
/// not nested groups for locators or other elements.
fn extract_citation_delimiter(layout: &csl_legacy::model::Layout) -> Option<String> {
    use csl_legacy::model::CslNode;

    /// Check if a node references an author-related macro.
    fn is_author_macro(node: &CslNode) -> bool {
        match node {
            CslNode::Text(t) => t
                .macro_name
                .as_ref()
                .is_some_and(|m| m.to_lowercase().contains("author")),
            CslNode::Names(n) => n.variable.contains("author"),
            _ => false,
        }
    }

    /// Check if a node references a date-related macro.
    fn is_date_macro(node: &CslNode) -> bool {
        match node {
            CslNode::Text(t) => t.macro_name.as_ref().is_some_and(|m| {
                let lower = m.to_lowercase();
                lower.contains("date") || lower.contains("year") || lower.contains("issued")
            }),
            CslNode::Date(_) => true,
            _ => false,
        }
    }

    /// Find the outermost group that directly contains author and date macros.
    /// This is the group whose delimiter separates author from year.
    fn find_author_date_group_delimiter(nodes: &[CslNode]) -> Option<String> {
        for node in nodes {
            match node {
                CslNode::Group(group) => {
                    // Check if THIS group directly contains author AND date macros
                    let has_author = group.children.iter().any(is_author_macro);
                    let has_date = group.children.iter().any(is_date_macro);

                    if has_author && has_date && group.delimiter.is_some() {
                        // This is the author-date group!
                        return group.delimiter.clone();
                    }

                    // Otherwise recurse into children
                    if let Some(d) = find_author_date_group_delimiter(&group.children) {
                        return Some(d);
                    }
                }
                CslNode::Choose(choose) => {
                    // Search inside choose branches
                    if let Some(d) = find_author_date_group_delimiter(&choose.if_branch.children) {
                        return Some(d);
                    }
                    for else_if in &choose.else_if_branches {
                        if let Some(d) = find_author_date_group_delimiter(&else_if.children) {
                            return Some(d);
                        }
                    }
                    if let Some(ref else_children) = choose.else_branch {
                        if let Some(d) = find_author_date_group_delimiter(else_children) {
                            return Some(d);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Find prefix on date-related text elements (fallback when no group delimiter).
    /// Looks for macros named "date", "year", "year-date", "issued", etc.
    fn find_date_prefix(nodes: &[CslNode]) -> Option<String> {
        for node in nodes {
            match node {
                CslNode::Text(t) => {
                    if let Some(macro_name) = &t.macro_name {
                        let m = macro_name.to_lowercase();
                        if m.contains("date") || m.contains("year") || m.contains("issued") {
                            if let Some(prefix) = &t.prefix {
                                return Some(prefix.clone());
                            }
                        }
                    }
                }
                CslNode::Group(g) => {
                    if let Some(p) = find_date_prefix(&g.children) {
                        return Some(p);
                    }
                }
                _ => {}
            }
        }
        None
    }

    // First try to find the author-date group delimiter
    if let Some(delim) = find_author_date_group_delimiter(&layout.children) {
        return Some(delim);
    }

    // Fall back to prefix on date element
    if let Some(prefix) = find_date_prefix(&layout.children) {
        return Some(prefix);
    }

    // No delimiter found - return None (processor will use default)
    None
}

/// Recursively apply type-specific overrides to components, including nested Lists.
/// The `volume_pages_delimiter` is extracted from the CSL style's group structure.
/// The `volume_list_has_space_prefix` flag indicates whether the volume List has a space
/// prefix (Elsevier-style, don't add suffix to journal) vs no prefix (APA-style, add comma).
/// The `style_id` is used for style-specific rules (e.g., Chicago suppresses chapter pages).
fn apply_type_overrides(
    component: &mut TemplateComponent,
    volume_pages_delimiter: Option<csln_core::template::DelimiterPunctuation>,
    volume_list_has_space_prefix: bool,
    style_id: &str,
) {
    match component {
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
            // Ensure visible for common monographic types
            let unsuppress = csln_core::template::Rendering {
                suppress: Some(false),
                ..Default::default()
            };
            new_ovr.insert("book".to_string(), unsuppress.clone());
            new_ovr.insert("chapter".to_string(), unsuppress.clone());
            new_ovr.insert("report".to_string(), unsuppress.clone());
            new_ovr.insert("thesis".to_string(), unsuppress);

            // Merge instead of overwrite
            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Publisher-place: style-specific visibility rules
        // - Chicago: show for journals (in parens), suppress for books/reports
        // - APA: suppress for everything (APA 7th)
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::PublisherPlace =>
        {
            let mut new_ovr = std::collections::HashMap::new();
            let is_chicago = style_id.contains("chicago");
            let is_apa = style_id.contains("apa");

            if is_chicago {
                // Chicago: suppress for books and reports, show for journals
                let suppress_rendering = csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                };
                new_ovr.insert("book".to_string(), suppress_rendering.clone());
                new_ovr.insert("report".to_string(), suppress_rendering.clone());
                new_ovr.insert("thesis".to_string(), suppress_rendering);
                new_ovr.insert(
                    "article-journal".to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(false),
                        ..Default::default()
                    },
                );
            } else if is_apa {
                // APA 7th: suppress for everything
                v.rendering.suppress = Some(true);
            } else {
                // Default: suppress for journals
                new_ovr.insert(
                    "article-journal".to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(true),
                        ..Default::default()
                    },
                );
            }
            // Merge instead of overwrite
            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Genre: ensure visible for thesis/reports, with bracket wrap for thesis in APA
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::Genre =>
        {
            let is_apa = style_id.contains("apa");
            let mut new_ovr = std::collections::HashMap::new();

            // Thesis: wrap in brackets for APA style
            // Add space prefix OUTSIDE wrap to suppress the default period separator
            // Add period suffix AFTER the closing bracket
            // Set prefix_inside_wrap: false so the space appears before the bracket
            // The genre renders as " [PhD thesis]." which attaches directly to title
            if is_apa {
                new_ovr.insert(
                    "thesis".to_string(),
                    csln_core::template::Rendering {
                        wrap: Some(WrapPunctuation::Brackets),
                        prefix: Some(" ".to_string()), // Space prefix before bracket
                        suffix: Some(".".to_string()), // Period after closing bracket
                        prefix_inside_wrap: Some(false), // Put prefix OUTSIDE the brackets
                        suppress: Some(false),
                        ..Default::default()
                    },
                );
            } else {
                new_ovr.insert(
                    "thesis".to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(false),
                        ..Default::default()
                    },
                );
            }
            new_ovr.insert(
                "report".to_string(),
                csln_core::template::Rendering {
                    suppress: Some(false),
                    ..Default::default()
                },
            );

            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // URL: unsuppress for webpage and post types (typically show URL when no DOI)
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::Url =>
        {
            let mut new_ovr = std::collections::HashMap::new();
            let unsuppress = csln_core::template::Rendering {
                suppress: Some(false),
                ..Default::default()
            };
            new_ovr.insert("webpage".to_string(), unsuppress.clone());
            new_ovr.insert("post".to_string(), unsuppress.clone());
            new_ovr.insert("post-weblog".to_string(), unsuppress);

            let overrides = v
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Pages: use extracted delimiter for journal articles, (pp. X-Y) for chapters
        TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Pages => {
            let mut new_ovr = std::collections::HashMap::new();
            // Use extracted delimiter or default to comma
            let delim = volume_pages_delimiter
                .unwrap_or(csln_core::template::DelimiterPunctuation::Comma)
                .to_string_with_space();
            new_ovr.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    prefix: Some(delim.to_string()),
                    suppress: Some(false),
                    ..Default::default()
                },
            );
            // Chapter pages: style-specific handling
            // - Chicago: suppress pages (only show in citations, not bibliography)
            // - Elsevier: "pp. X-Y" (no wrap)
            // - APA: "(pp. X-Y)" (wrap in parentheses)
            let is_chicago = style_id.contains("chicago");
            let chapter_rendering = if is_chicago {
                csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                }
            } else {
                let chapter_wrap = if volume_list_has_space_prefix {
                    None // Elsevier: no wrap
                } else {
                    Some(WrapPunctuation::Parentheses) // APA: wrap in parentheses
                };
                csln_core::template::Rendering {
                    prefix: Some("pp. ".to_string()),
                    wrap: chapter_wrap,
                    suppress: Some(false),
                    ..Default::default()
                }
            };
            new_ovr.insert("chapter".to_string(), chapter_rendering);
            // Merge instead of overwrite
            let overrides = n
                .overrides
                .get_or_insert_with(std::collections::HashMap::new);
            for (k, v) in new_ovr {
                overrides.insert(k, v);
            }
        }
        // Edition: wrap in parentheses for book types in APA
        // APA puts edition (and other identifiers) in parentheses after the title
        TemplateComponent::Number(n)
            if n.number == csln_core::template::NumberVariable::Edition =>
        {
            let is_apa = style_id.contains("apa");
            if is_apa {
                let mut new_ovr = std::collections::HashMap::new();
                let wrap_rendering = csln_core::template::Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    suppress: Some(false),
                    ..Default::default()
                };
                // Book types get edition in parentheses
                new_ovr.insert("book".to_string(), wrap_rendering.clone());
                new_ovr.insert("edited-book".to_string(), wrap_rendering.clone());
                new_ovr.insert("report".to_string(), wrap_rendering);

                let overrides = n
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                for (k, v) in new_ovr {
                    overrides.insert(k, v);
                }
            }
        }
        // Recursively process Lists
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

/// Combine volume and issue into a grouped structure: volume(issue).
///
/// CSL styles typically have volume and issue together in the source-serial macro
/// with issue wrapped in parentheses. However, during migration:
/// - Issue may come from a separate label-issue macro (for legal types)
/// - Volume may end up inside a nested List (from source-serial grouping)
///
/// This function handles both cases:
/// 1. If both are at top level, creates a new volume(issue) List
/// 2. If volume is inside a List, adds issue to that List after volume
fn group_volume_and_issue(
    components: &mut Vec<TemplateComponent>,
    options: &csln_core::options::Config,
) {
    use csln_core::template::{
        DelimiterPunctuation, NumberVariable, Rendering, TemplateList, TemplateNumber,
        WrapPunctuation,
    };

    // Volume-issue spacing varies by style:
    // - APA (comma delimiter): no space, e.g., "2(2)"
    // - Chicago (colon delimiter): space, e.g., "2 (2)"
    let vol_issue_delimiter = if options
        .volume_pages_delimiter
        .as_ref()
        .is_some_and(|d| matches!(d, DelimiterPunctuation::Comma))
    {
        DelimiterPunctuation::None
    } else {
        DelimiterPunctuation::Space
    };

    // Check for issue at top level
    let issue_pos = components.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Issue),
    );

    // Check for volume at top level
    let vol_pos = components.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Volume),
    );

    // Case 1: Both at top level - combine into a List
    if let (Some(vol_idx), Some(issue_idx)) = (vol_pos, issue_pos) {
        let min_idx = vol_idx.min(issue_idx);
        let max_idx = vol_idx.max(issue_idx);

        // Remove from end first to preserve indices
        components.remove(max_idx);
        components.remove(min_idx);

        let vol_issue_list = TemplateComponent::List(TemplateList {
            items: vec![
                TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::Volume,
                    form: None,
                    rendering: Rendering::default(),
                    overrides: None,
                    ..Default::default()
                }),
                TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::Issue,
                    form: None,
                    rendering: Rendering {
                        wrap: Some(WrapPunctuation::Parentheses),
                        ..Default::default()
                    },
                    overrides: None,
                    ..Default::default()
                }),
            ],
            delimiter: Some(vol_issue_delimiter),
            rendering: Rendering::default(),
            overrides: None,
            ..Default::default()
        });

        components.insert(min_idx, vol_issue_list);
        return;
    }

    // Case 2: Issue at top level, volume inside a nested List
    // Find the List containing volume and add issue to it
    if let Some(issue_idx) = issue_pos {
        // First, find which List index contains volume (immutable borrow)
        let list_idx = components.iter().enumerate().find_map(|(idx, c)| {
            if let TemplateComponent::List(list) = c {
                if find_volume_in_list(list).is_some() {
                    return Some(idx);
                }
            }
            None
        });

        if let Some(list_idx) = list_idx {
            // Extract the issue's overrides before removing it
            let issue_overrides =
                if let Some(TemplateComponent::Number(n)) = components.get(issue_idx) {
                    n.overrides.clone()
                } else {
                    None
                };

            // Remove issue from top level (adjusting for index shift if needed)
            components.remove(issue_idx);

            // Adjust list_idx if issue was before it
            let adjusted_list_idx = if issue_idx < list_idx {
                list_idx - 1
            } else {
                list_idx
            };

            // Create issue component with parentheses wrap
            let issue_with_parens = TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Issue,
                form: None,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    ..Default::default()
                },
                overrides: issue_overrides,
                ..Default::default()
            });

            // Now mutably access the list and add issue after volume
            if let Some(TemplateComponent::List(list)) = components.get_mut(adjusted_list_idx) {
                // Try to insert issue after volume - recursively searching nested lists
                if insert_issue_after_volume(
                    &mut list.items,
                    issue_with_parens,
                    vol_issue_delimiter,
                ) {
                    // Successfully inserted, update top-level list delimiter if needed
                    list.delimiter = Some(DelimiterPunctuation::Comma);
                }
            }
        }
    }

    // Case 3: Neither at top level - issue is in a nested list somewhere
    // Find issue anywhere in nested lists and try to move it to volume's list
    if issue_pos.is_none() && vol_pos.is_none() {
        // Find the issue in any nested list and create a new one after volume
        let issue_exists_nested = find_issue_in_components(components);
        let volume_exists_nested = components.iter().any(|c| {
            if let TemplateComponent::List(list) = c {
                find_volume_in_list(list).is_some()
            } else {
                false
            }
        });

        if issue_exists_nested && volume_exists_nested {
            // Create issue component with parentheses wrap
            let issue_with_parens = TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::Issue,
                form: None,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    ..Default::default()
                },
                overrides: None,
                ..Default::default()
            });

            // Find the list containing volume and add issue to it
            for component in components.iter_mut() {
                if let TemplateComponent::List(list) = component {
                    if find_volume_in_list(list).is_some()
                        && insert_issue_after_volume(
                            &mut list.items,
                            issue_with_parens.clone(),
                            vol_issue_delimiter,
                        )
                    {
                        break;
                    }
                }
            }
        }
    }
}

/// Check if issue exists anywhere in nested components.
fn find_issue_in_components(components: &[TemplateComponent]) -> bool {
    use csln_core::template::NumberVariable;

    for component in components {
        match component {
            TemplateComponent::Number(n) if n.number == NumberVariable::Issue => {
                return true;
            }
            TemplateComponent::List(list) => {
                if find_issue_in_components(&list.items) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Insert issue component after volume, handling nested lists.
/// Returns true if successfully inserted.
fn insert_issue_after_volume(
    items: &mut Vec<TemplateComponent>,
    issue: TemplateComponent,
    delimiter: csln_core::template::DelimiterPunctuation,
) -> bool {
    use csln_core::template::{NumberVariable, Rendering, TemplateList};

    // First, check if volume is directly in this list
    if let Some(vol_pos) = items.iter().position(
        |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Volume),
    ) {
        // Remove volume from the list
        let volume = items.remove(vol_pos);

        // Create a new List containing [volume, issue] with no delimiter
        // This preserves the outer list's delimiter for other items
        let vol_issue_group = TemplateComponent::List(TemplateList {
            items: vec![volume, issue],
            delimiter: Some(delimiter), // No space between volume and issue
            rendering: Rendering::default(),
            overrides: None,
            ..Default::default()
        });

        // Insert the new group where volume was
        items.insert(vol_pos, vol_issue_group);
        return true;
    }

    // Otherwise, recurse into nested lists
    for item in items.iter_mut() {
        if let TemplateComponent::List(inner_list) = item {
            if insert_issue_after_volume(&mut inner_list.items, issue.clone(), delimiter) {
                return true;
            }
        }
    }

    false
}

/// Check if a List contains a volume variable (recursively).
fn find_volume_in_list(list: &csln_core::template::TemplateList) -> Option<()> {
    for item in &list.items {
        match item {
            TemplateComponent::Number(n)
                if n.number == csln_core::template::NumberVariable::Volume =>
            {
                return Some(());
            }
            TemplateComponent::List(inner) => {
                if find_volume_in_list(inner).is_some() {
                    return Some(());
                }
            }
            _ => {}
        }
    }
    None
}

/// Move DOI and URL components to the end of the bibliography template.
///
/// CSL styles typically place access macros (DOI, URL) at the end of entries,
/// but during macro expansion they can end up in the middle due to how
/// conditionals are processed. This function moves them to the correct position.
fn move_access_components_to_end(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::SimpleVariable;

    // Find indices of access components (DOI, URL)
    let mut access_indices: Vec<usize> = Vec::new();
    for (i, c) in components.iter().enumerate() {
        if let TemplateComponent::Variable(v) = c {
            if matches!(v.variable, SimpleVariable::Doi | SimpleVariable::Url) {
                access_indices.push(i);
            }
        }
        // Also check for List items containing accessed date (URL + accessed date pattern)
        if let TemplateComponent::List(list) = c {
            let has_access = list.items.iter().any(|item| {
                matches!(item, TemplateComponent::Variable(v) if v.variable == SimpleVariable::Url)
                    || matches!(item, TemplateComponent::Date(d) if d.date == csln_core::template::DateVariable::Accessed)
            });
            if has_access {
                access_indices.push(i);
            }
        }
    }

    // Extract access components in reverse order (to preserve indices)
    let mut access_components: Vec<TemplateComponent> = Vec::new();
    for idx in access_indices.into_iter().rev() {
        access_components.push(components.remove(idx));
    }
    access_components.reverse();

    // Append access components at the end
    components.extend(access_components);
}

/// Remove duplicate title components from Lists that already appear at the top level.
///
/// CSL styles often have the same container-title variable in multiple macros
/// (e.g., once for the container and once in the locators group). This causes
/// the same title to render twice. This function removes duplicates from Lists.
fn deduplicate_titles_in_lists(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::TitleType;

    // Collect title types that appear at top level
    let top_level_titles: Vec<TitleType> = components
        .iter()
        .filter_map(|c| {
            if let TemplateComponent::Title(t) = c {
                Some(t.title.clone())
            } else {
                None
            }
        })
        .collect();

    // Remove duplicates from Lists
    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            list.items.retain(|item| {
                if let TemplateComponent::Title(t) = item {
                    !top_level_titles.contains(&t.title)
                } else {
                    true
                }
            });
        }
    }

    // Remove empty Lists
    components.retain(|c| {
        if let TemplateComponent::List(list) = c {
            !list.items.is_empty()
        } else {
            true
        }
    });
}

/// Propagate type-specific overrides within Lists.
///
/// When components are compiled from different CSL macro branches, they may end
/// up in the same List but with different type overrides. This function ensures
/// that if any component in a List has a type override, all siblings in the same
/// List also get that override (with suppress: false).
///
/// This fixes the issue where volume and container-title are in the same source-serial
/// macro but only container-title gets the article-journal override.
fn propagate_list_overrides(components: &mut [TemplateComponent]) {
    use csln_core::template::Rendering;
    use std::collections::HashSet;

    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            propagate_overrides_in_list(&mut list.items);

            // Recursively process nested lists
            for item in &mut list.items {
                if let TemplateComponent::List(inner_list) = item {
                    propagate_overrides_in_list(&mut inner_list.items);
                }
            }
        }
    }

    fn propagate_overrides_in_list(items: &mut [TemplateComponent]) {
        // Collect all type keys that have overrides in any item
        let mut all_override_types: HashSet<String> = HashSet::new();

        for item in items.iter() {
            if let Some(overrides) = get_component_overrides(item) {
                for key in overrides.keys() {
                    all_override_types.insert(key.clone());
                }
            }
        }

        // For each type that exists in any item, ensure all items have it
        for type_key in &all_override_types {
            for item in items.iter_mut() {
                if let Some(overrides) = get_component_overrides_mut(item) {
                    if !overrides.contains_key(type_key) {
                        // Add the override with suppress: false
                        overrides.insert(
                            type_key.clone(),
                            Rendering {
                                suppress: Some(false),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
    }

    fn get_component_overrides(
        comp: &TemplateComponent,
    ) -> Option<&std::collections::HashMap<String, Rendering>> {
        match comp {
            TemplateComponent::Contributor(c) => c.overrides.as_ref(),
            TemplateComponent::Date(d) => d.overrides.as_ref(),
            TemplateComponent::Title(t) => t.overrides.as_ref(),
            TemplateComponent::Number(n) => n.overrides.as_ref(),
            TemplateComponent::Variable(v) => v.overrides.as_ref(),
            _ => None,
        }
    }

    fn get_component_overrides_mut(
        comp: &mut TemplateComponent,
    ) -> Option<&mut std::collections::HashMap<String, Rendering>> {
        match comp {
            TemplateComponent::Contributor(c) => {
                if c.overrides.is_none() {
                    c.overrides = Some(std::collections::HashMap::new());
                }
                c.overrides.as_mut()
            }
            TemplateComponent::Date(d) => {
                if d.overrides.is_none() {
                    d.overrides = Some(std::collections::HashMap::new());
                }
                d.overrides.as_mut()
            }
            TemplateComponent::Title(t) => {
                if t.overrides.is_none() {
                    t.overrides = Some(std::collections::HashMap::new());
                }
                t.overrides.as_mut()
            }
            TemplateComponent::Number(n) => {
                if n.overrides.is_none() {
                    n.overrides = Some(std::collections::HashMap::new());
                }
                n.overrides.as_mut()
            }
            TemplateComponent::Variable(v) => {
                if v.overrides.is_none() {
                    v.overrides = Some(std::collections::HashMap::new());
                }
                v.overrides.as_mut()
            }
            _ => None,
        }
    }
}

/// Remove duplicate nested Lists within parent Lists.
///
/// When CSL conditions have similar then/else branches (both containing the same
/// components like parent-monograph + issue), the migration creates multiple
/// identical nested Lists. This deduplicates them.
fn deduplicate_nested_lists(components: &mut [TemplateComponent]) {
    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            deduplicate_lists_in_items(&mut list.items);
        }
    }
}

/// Helper to deduplicate nested Lists within a list of items.
fn deduplicate_lists_in_items(items: &mut Vec<TemplateComponent>) {
    use std::collections::HashSet;

    // Build a set of "signatures" for Lists we've seen
    let mut seen_signatures: HashSet<String> = HashSet::new();
    let mut indices_to_remove: Vec<usize> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        if let TemplateComponent::List(inner_list) = item {
            let sig = list_signature(inner_list);
            if seen_signatures.contains(&sig) {
                indices_to_remove.push(i);
            } else {
                seen_signatures.insert(sig);
            }
        }
    }

    // Remove duplicates in reverse order to preserve indices
    for i in indices_to_remove.into_iter().rev() {
        items.remove(i);
    }

    // Recursively deduplicate nested lists
    for item in items.iter_mut() {
        if let TemplateComponent::List(inner_list) = item {
            deduplicate_lists_in_items(&mut inner_list.items);
        }
    }
}

/// Create a signature string for a List based on its component types.
/// Two Lists with the same components (by type) get the same signature.
fn list_signature(list: &csln_core::template::TemplateList) -> String {
    list.items
        .iter()
        .map(|item| match item {
            TemplateComponent::Contributor(c) => format!("contrib:{:?}", c.contributor),
            TemplateComponent::Date(d) => format!("date:{:?}", d.date),
            TemplateComponent::Title(t) => format!("title:{:?}", t.title),
            TemplateComponent::Number(n) => format!("num:{:?}", n.number),
            TemplateComponent::Variable(v) => format!("var:{:?}", v.variable),
            TemplateComponent::List(_) => "list".to_string(),
            _ => "unknown".to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Reorder components within Lists for serial types.
///
/// For journal articles, the correct order is:
/// 1. container-title (parent-serial)
/// 2. volume(issue)
/// 3. pages
///
/// But due to how CSL macros are processed, volume often ends up before
/// container-title. This function reorders the items within Lists to match
/// the expected output.
fn reorder_serial_components(components: &mut [TemplateComponent]) {
    use csln_core::template::{NumberVariable, TitleType};

    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            // Check if this list contains both volume and parent-serial
            let has_volume = list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Number(n) if n.number == NumberVariable::Volume
                )
            });
            let has_parent_serial = list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
                )
            });

            if has_volume && has_parent_serial {
                // Find positions
                let volume_pos = list.items.iter().position(|item| {
                    matches!(
                        item,
                        TemplateComponent::Number(n) if n.number == NumberVariable::Volume
                    )
                });
                let parent_serial_pos = list.items.iter().position(|item| {
                    matches!(
                        item,
                        TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
                    )
                });

                // If volume is before parent-serial, swap them
                if let (Some(vol_pos), Some(ps_pos)) = (volume_pos, parent_serial_pos) {
                    if vol_pos < ps_pos {
                        list.items.swap(vol_pos, ps_pos);
                    }
                }
            }

            // Recursively process nested lists
            for item in &mut list.items {
                if let TemplateComponent::List(inner_list) = item {
                    reorder_serial_components_in_list(inner_list);
                }
            }
        }
    }
}

/// Helper to reorder components in a single list.
fn reorder_serial_components_in_list(list: &mut csln_core::template::TemplateList) {
    use csln_core::template::{NumberVariable, TitleType};

    // Check if this list contains both volume and parent-serial
    let has_volume = list.items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Number(n) if n.number == NumberVariable::Volume
        )
    });
    let has_parent_serial = list.items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
        )
    });

    if has_volume && has_parent_serial {
        // Find positions
        let volume_pos = list.items.iter().position(|item| {
            matches!(
                item,
                TemplateComponent::Number(n) if n.number == NumberVariable::Volume
            )
        });
        let parent_serial_pos = list.items.iter().position(|item| {
            matches!(
                item,
                TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
            )
        });

        // If volume is before parent-serial, swap them
        if let (Some(vol_pos), Some(ps_pos)) = (volume_pos, parent_serial_pos) {
            if vol_pos < ps_pos {
                list.items.swap(vol_pos, ps_pos);
            }
        }
    }
}

/// Move pages component to appear after the container-title/volume List.
///
/// For serial types (journals, magazines), pages should appear AFTER the
/// container-title and volume, not before. The template has pages at top level
/// but it needs to come after the List containing parent-serial.
fn reorder_pages_for_serials(components: &mut Vec<TemplateComponent>) {
    use csln_core::template::{NumberVariable, TitleType};

    // Find the pages component position
    let pages_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Number(n) if n.number == NumberVariable::Pages
        )
    });

    // Find the List containing parent-serial (container-title for journals)
    // Need to search recursively since parent-serial may be in a nested List
    let serial_list_pos = components.iter().position(contains_parent_serial_recursive);

    // If pages is BEFORE the serial list, move it to right after
    if let (Some(p_pos), Some(s_pos)) = (pages_pos, serial_list_pos) {
        if p_pos < s_pos {
            let pages_component = components.remove(p_pos);
            // After removal, indices shift - insert at s_pos (which is now s_pos - 1 + 1 = s_pos)
            components.insert(s_pos, pages_component);
        }
    }

    fn contains_parent_serial_recursive(component: &TemplateComponent) -> bool {
        match component {
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial => true,
            TemplateComponent::List(list) => {
                list.items.iter().any(contains_parent_serial_recursive)
            }
            _ => false,
        }
    }
}

/// Reorder publisher-place for Chicago journal articles.
///
/// Chicago style requires publisher-place to appear immediately after the
/// journal title (parent-serial), before the volume. During CSL macro expansion,
/// the `source-serial-name` macro (which groups container-title + publisher-place)
/// gets separated, with publisher-place ending up much later in the template.
///
/// This function moves the publisher-place List to the correct position for
/// Chicago styles.
fn reorder_publisher_place_for_chicago(components: &mut Vec<TemplateComponent>, style_id: &str) {
    use csln_core::template::{SimpleVariable, TitleType};

    // Only apply to Chicago styles
    if !style_id.contains("chicago") {
        return;
    }

    // Find the publisher-place component (it's in a List with wrap: parentheses)
    let publisher_place_pos = components.iter().position(|c| {
        if let TemplateComponent::List(list) = c {
            list.items.iter().any(|item| {
                matches!(
                    item,
                    TemplateComponent::Variable(v)
                    if v.variable == SimpleVariable::PublisherPlace
                )
            })
        } else {
            false
        }
    });

    // Find the parent-serial title position
    let parent_serial_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Title(t) if t.title == TitleType::ParentSerial
        )
    });

    // If we found both, move publisher-place to right after parent-serial
    if let (Some(pp_pos), Some(ps_pos)) = (publisher_place_pos, parent_serial_pos) {
        if pp_pos > ps_pos {
            // Remove the publisher-place List
            let mut publisher_place_component = components.remove(pp_pos);

            // Add space suffix to prevent default period separator
            if let TemplateComponent::List(ref mut list) = publisher_place_component {
                list.rendering.suffix = Some(" ".to_string());
            }

            // Insert it right after parent-serial
            components.insert(ps_pos + 1, publisher_place_component);
        }
    }
}

/// Reorder chapter components for Chicago style.
///
/// Chicago chapters require: "Chapter Title." In Book Title, edited by Editors.
/// But the default template has: "Chapter Title." edited by Editors, Book Title.
///
/// This function:
/// 1. Finds the editor and parent-monograph positions
/// 2. Swaps them so parent-monograph comes first
/// 3. Adds "In " prefix to parent-monograph for chapters
/// 4. Adjusts editor prefix to ", edited by " for chapters
fn reorder_chapters_for_chicago(components: &mut Vec<TemplateComponent>, style_id: &str) {
    use csln_core::template::{ContributorRole, TitleType};

    // Only apply to Chicago styles
    if !style_id.contains("chicago") {
        return;
    }

    // Find the editor contributor (form: verb)
    let editor_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Contributor(contrib)
            if contrib.contributor == ContributorRole::Editor
        )
    });

    // Find the parent-monograph title
    let parent_monograph_pos = components.iter().position(|c| {
        matches!(
            c,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph
        )
    });

    // If we found both and editor comes before parent-monograph, swap them
    if let (Some(editor_pos), Some(pm_pos)) = (editor_pos, parent_monograph_pos) {
        if editor_pos < pm_pos {
            // Get mutable references to both components
            let editor_component = components.remove(editor_pos);
            let pm_component = components.remove(pm_pos - 1); // Adjust index after removal

            // Add "In " prefix and ", " suffix to parent-monograph for chapters
            let mut pm_with_prefix = pm_component.clone();
            if let TemplateComponent::Title(ref mut title) = pm_with_prefix {
                // Use type-specific override to add "In " prefix and ", " suffix for chapters
                let mut overrides = title.overrides.clone().unwrap_or_default();
                overrides.insert(
                    "chapter".to_string(),
                    csln_core::template::Rendering {
                        prefix: Some("In ".to_string()),
                        suffix: Some(", ".to_string()),
                        ..Default::default()
                    },
                );
                title.overrides = Some(overrides);
            }

            // Adjust editor for chapters: use ". " suffix and given-first name order
            let mut editor_with_suffix = editor_component.clone();
            if let TemplateComponent::Contributor(ref mut contrib) = editor_with_suffix {
                // For chapters, editors should use given-first name order
                // (K. Anders Ericsson, not Ericsson, K. Anders)
                use csln_core::template::NameOrder;
                contrib.name_order = Some(NameOrder::GivenFirst);

                // Add override to change suffix for chapters
                let mut overrides = contrib.overrides.clone().unwrap_or_default();
                overrides.insert(
                    "chapter".to_string(),
                    csln_core::template::Rendering {
                        suffix: Some(". ".to_string()),
                        ..Default::default()
                    },
                );
                contrib.overrides = Some(overrides);
            }

            // Re-insert in new order: parent-monograph, then editor
            components.insert(editor_pos, pm_with_prefix);
            components.insert(editor_pos + 1, editor_with_suffix);
        }
    }
}

/// Suppress duplicate issue in parent-monograph lists for article-journal types.
///
/// Chicago CSL has issue in multiple places:
/// 1. With volume in the source-serial macro (for journal articles)
/// 2. With parent-monograph in the source-monographic macro (for chapters)
///
/// Our group_volume_and_issue function creates the volume(issue) grouping for journals,
/// but the issue also appears in parent-monograph lists. This creates duplicates.
/// This function suppresses issue for article-journal in those lists.
fn suppress_duplicate_issue_for_journals(components: &mut [TemplateComponent], style_id: &str) {
    // Only apply to Chicago styles
    if !style_id.contains("chicago") {
        return;
    }

    for component in components.iter_mut() {
        if let TemplateComponent::List(list) = component {
            suppress_issue_in_parent_monograph_list(&mut list.items);
        }
    }
}

/// Helper to find and suppress issue in lists containing parent-monograph.
fn suppress_issue_in_parent_monograph_list(items: &mut [TemplateComponent]) {
    use csln_core::template::{NumberVariable, TitleType};

    // Check if this list has parent-monograph (indicating it's the monographic source list)
    let has_parent_monograph = items.iter().any(|item| {
        matches!(
            item,
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph
        ) || matches!(item, TemplateComponent::List(inner_list)
            if inner_list.items.iter().any(|i| matches!(i, TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph)))
    });

    if has_parent_monograph {
        // Suppress issue for article-journal in this list
        for item in items.iter_mut() {
            if let TemplateComponent::Number(n) = item {
                if n.number == NumberVariable::Issue {
                    let overrides = n
                        .overrides
                        .get_or_insert_with(std::collections::HashMap::new);
                    if let Some(rendering) = overrides.get_mut("article-journal") {
                        rendering.suppress = Some(true);
                    } else {
                        overrides.insert(
                            "article-journal".to_string(),
                            csln_core::template::Rendering {
                                suppress: Some(true),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
            // Recursively check nested lists
            if let TemplateComponent::List(inner_list) = item {
                suppress_issue_in_parent_monograph_list(&mut inner_list.items);
            }
        }
    }

    // Recursively process all nested lists
    for item in items.iter_mut() {
        if let TemplateComponent::List(inner_list) = item {
            suppress_issue_in_parent_monograph_list(&mut inner_list.items);
        }
    }
}

/// Recursively ensure specific variables are un-suppressed for a given type.
fn unsuppress_for_type(components: &mut [TemplateComponent], item_type: &str) {
    use csln_core::template::SimpleVariable;

    for component in components {
        match component {
            TemplateComponent::Variable(v)
                if matches!(
                    v.variable,
                    SimpleVariable::Publisher | SimpleVariable::PublisherPlace
                ) =>
            {
                let overrides = v
                    .overrides
                    .get_or_insert_with(std::collections::HashMap::new);
                overrides.insert(
                    item_type.to_string(),
                    csln_core::template::Rendering {
                        suppress: Some(false),
                        ..Default::default()
                    },
                );
            }
            TemplateComponent::List(list) => {
                unsuppress_for_type(&mut list.items, item_type);
            }
            _ => {}
        }
    }
}

/// Extract the suffix on the author macro call from the bibliography layout.
///
/// CSL styles like Elsevier use `<text macro="author" suffix=","/>` to add a comma
/// between author and date. This function extracts that suffix so it can be applied
/// to the author component in the migrated template.
fn extract_author_suffix(layout: &csl_legacy::model::Layout) -> Option<String> {
    use csl_legacy::model::CslNode;

    for node in &layout.children {
        // Check for group containing author macro call
        if let CslNode::Group(g) = node {
            for child in &g.children {
                if let CslNode::Text(t) = child {
                    if t.macro_name.as_deref() == Some("author") {
                        // Found the author macro call - return its suffix
                        return t.suffix.clone();
                    }
                }
            }
        }
        // Check for direct author macro call at top level
        if let CslNode::Text(t) = node {
            if t.macro_name.as_deref() == Some("author") {
                return t.suffix.clone();
            }
        }
    }
    None
}

/// Apply the extracted author suffix to the author component in the template.
fn apply_author_suffix(components: &mut [TemplateComponent], suffix: Option<String>) {
    if let Some(suffix) = suffix {
        for component in components {
            if let TemplateComponent::Contributor(c) = component {
                if c.contributor == csln_core::template::ContributorRole::Author {
                    // Set or update the suffix
                    c.rendering.suffix = Some(suffix.clone());
                }
            }
        }
    }
}

/// Check if the bibliography name element has an 'and' attribute.
///
/// CSL styles can have different 'and' settings for citations vs bibliography.
/// For example, Elsevier uses `and="text"` in citation but no `and` in bibliography.
/// Returns `Some(AndOptions)` if found, or `None` if no bibliography name element
/// or the name element has no `and` attribute (meaning no conjunction).
fn extract_bibliography_and(
    style: &csl_legacy::model::Style,
) -> Option<csln_core::options::AndOptions> {
    // First, look for the "author" macro which is used in bibliography
    // The "author-short" macro is used in citations and may have different 'and' settings
    for macro_def in &style.macros {
        if macro_def.name == "author" {
            // Search for Name nodes in this macro
            if let Some(result) = find_name_and(&macro_def.children) {
                return Some(result);
            }
        }
    }

    // Fallback: search the bibliography layout directly
    if let Some(bib) = &style.bibliography {
        if let Some(result) = find_name_and(&bib.layout.children) {
            return Some(result);
        }
    }

    None
}

// Helper function to find 'and' setting in Name nodes
fn find_name_and(nodes: &[csl_legacy::model::CslNode]) -> Option<csln_core::options::AndOptions> {
    use csl_legacy::model::CslNode;

    for node in nodes {
        match node {
            CslNode::Name(name) => {
                // Found a name element - check its 'and' attribute
                if let Some(and) = &name.and {
                    return Some(match and.as_str() {
                        "text" => csln_core::options::AndOptions::Text,
                        "symbol" => csln_core::options::AndOptions::Symbol,
                        _ => csln_core::options::AndOptions::None,
                    });
                }
                // Name exists but has no 'and' - explicitly return None (no conjunction)
                return Some(csln_core::options::AndOptions::None);
            }
            CslNode::Names(names) => {
                // Check within Names (which may contain Name)
                if let Some(result) = find_name_and(&names.children) {
                    return Some(result);
                }
            }
            CslNode::Group(g) => {
                if let Some(result) = find_name_and(&g.children) {
                    return Some(result);
                }
            }
            CslNode::Choose(c) => {
                if let Some(result) = find_name_and(&c.if_branch.children) {
                    return Some(result);
                }
                for branch in &c.else_if_branches {
                    if let Some(result) = find_name_and(&branch.children) {
                        return Some(result);
                    }
                }
                if let Some(else_branch) = &c.else_branch {
                    if let Some(result) = find_name_and(else_branch) {
                        return Some(result);
                    }
                }
            }
            CslNode::Substitute(s) => {
                if let Some(result) = find_name_and(&s.children) {
                    return Some(result);
                }
            }
            _ => {}
        }
    }
    None
}

/// Apply the bibliography 'and' setting to author components in the template.
///
/// If the bibliography name element has no 'and' attribute (or explicitly sets it to none),
/// set `and: none` on the author contributor component to override the global setting.
fn apply_bibliography_and(
    components: &mut [TemplateComponent],
    bib_and: Option<csln_core::options::AndOptions>,
) {
    if let Some(bib_and) = bib_and {
        for component in components {
            if let TemplateComponent::Contributor(c) = component {
                if c.contributor == csln_core::template::ContributorRole::Author {
                    // Set the 'and' option on the contributor component
                    c.and = Some(bib_and.clone());
                }
            }
        }
    }
}
