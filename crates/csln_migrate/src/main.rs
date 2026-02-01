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

        // Add editor and parent-monograph (container) for chapters
        // Pattern: "In K. A. Ericsson... (Eds.), The Cambridge Handbook..."
        let has_editor = new_bib.iter().any(|c| {
            matches!(c, TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Editor)
        });
        let has_container = new_bib.iter().any(|c| {
            matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::ParentMonograph)
        });

        // If we don't have ParentMonograph, check if ParentSerial exists
        // (some styles use container-title for both)
        let has_serial = new_bib.iter().any(|c| {
            matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::ParentSerial)
        });

        // Add ParentMonograph if missing - for chapters, container-title is the book title
        if !has_container {
            // Find position after primary title
            let title_pos = new_bib.iter().position(|c| {
                matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::Primary)
            });
            if let Some(pos) = title_pos {
                // Insert after primary title, or after ParentSerial if it exists
                let insert_pos = if has_serial {
                    new_bib
                        .iter()
                        .position(|c| {
                            matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::ParentSerial)
                        })
                        .map(|p| p + 1)
                        .unwrap_or(pos + 1)
                } else {
                    pos + 1
                };
                new_bib.insert(
                    insert_pos,
                    TemplateComponent::Title(csln_core::template::TemplateTitle {
                        title: csln_core::template::TitleType::ParentMonograph,
                        form: None,
                        rendering: csln_core::template::Rendering {
                            emph: Some(true), // Book titles are typically italic
                            ..Default::default()
                        },
                        overrides: None,
                        ..Default::default()
                    }),
                );
            }
        }

        // Now add editor before ParentMonograph if missing
        if !has_editor {
            // Find the container position (now guaranteed to exist)
            let container_pos = new_bib.iter().position(|c| {
                matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::ParentMonograph)
            });
            if let Some(pos) = container_pos {
                // Style-specific editor formatting patterns:
                // - Elsevier: ", in: Name (Eds.)," (prefix, Long form with label)
                // - APA: "In Name (Ed.)," (prefix, Long form with label)
                // - Chicago: "edited by Name" (no prefix, Verb form)
                let is_elsevier = legacy_style.info.id.contains("elsevier");
                let is_chicago = legacy_style.info.id.contains("chicago");

                let (editor_form, editor_prefix, editor_suffix) = if is_elsevier {
                    (
                        csln_core::template::ContributorForm::Long,
                        Some(", in: ".to_string()),
                        Some(", ".to_string()),
                    )
                } else if is_chicago {
                    // Chicago uses verb form "edited by" with no prefix
                    (
                        csln_core::template::ContributorForm::Verb,
                        None,
                        Some(", ".to_string()),
                    )
                } else {
                    // Default (APA): Long form with "In" prefix and label suffix
                    (
                        csln_core::template::ContributorForm::Long,
                        Some("In ".to_string()),
                        Some(", ".to_string()),
                    )
                };

                new_bib.insert(
                    pos,
                    TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                        contributor: csln_core::template::ContributorRole::Editor,
                        form: editor_form,
                        name_order: None, // Use global config
                        delimiter: None,
                        rendering: csln_core::template::Rendering {
                            prefix: editor_prefix,
                            suffix: editor_suffix,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                );
            }
        }

        // Combine volume and issue into a List component: volume(issue)
        let vol_pos = new_bib.iter().position(|c| {
            matches!(c, TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Volume)
        });
        let issue_pos = new_bib.iter().position(|c| {
            matches!(c, TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Issue)
        });

        if let (Some(vol_idx), Some(issue_idx)) = (vol_pos, issue_pos) {
            // Remove both and insert a List at the earlier position
            let min_idx = vol_idx.min(issue_idx);
            let max_idx = vol_idx.max(issue_idx);

            // Remove from end first to preserve indices
            new_bib.remove(max_idx);
            new_bib.remove(min_idx);

            // Create volume(issue) list
            // Volume-issue spacing varies by style:
            // - APA (comma delimiter): no space, e.g., "2(2)"
            // - Chicago (colon delimiter): space, e.g., "2 (2)"
            let vol_issue_delimiter = if options
                .volume_pages_delimiter
                .as_ref()
                .is_some_and(|d| matches!(d, csln_core::template::DelimiterPunctuation::Comma))
            {
                csln_core::template::DelimiterPunctuation::None
            } else {
                csln_core::template::DelimiterPunctuation::Space
            };
            let vol_issue_list = TemplateComponent::List(csln_core::template::TemplateList {
                items: vec![
                    TemplateComponent::Number(csln_core::template::TemplateNumber {
                        number: csln_core::template::NumberVariable::Volume,
                        form: None,
                        rendering: csln_core::template::Rendering::default(),
                        overrides: None,
                        ..Default::default()
                    }),
                    TemplateComponent::Number(csln_core::template::TemplateNumber {
                        number: csln_core::template::NumberVariable::Issue,
                        form: None,
                        rendering: csln_core::template::Rendering {
                            wrap: Some(csln_core::template::WrapPunctuation::Parentheses),
                            ..Default::default()
                        },
                        overrides: None,
                        ..Default::default()
                    }),
                ],
                delimiter: Some(vol_issue_delimiter),
                rendering: csln_core::template::Rendering::default(),
                overrides: None,
                ..Default::default()
            });

            new_bib.insert(min_idx, vol_issue_list);
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

        // Remove duplicate titles from Lists that already appear at top level.
        // This happens when container-title appears in multiple CSL macros.
        deduplicate_titles_in_lists(&mut new_bib);
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
        // Container-title (parent-serial): add comma suffix for APA-style (no space prefix on volume)
        // Skip if volume List has space prefix (Elsevier-style handles spacing in List)
        TemplateComponent::Title(t) if t.title == csln_core::template::TitleType::ParentSerial => {
            if !volume_list_has_space_prefix {
                let mut overrides = std::collections::HashMap::new();
                overrides.insert(
                    "article-journal".to_string(),
                    csln_core::template::Rendering {
                        suffix: Some(",".to_string()),
                        ..Default::default()
                    },
                );
                t.overrides = Some(overrides);
            }
        }
        // Publisher: suppress for journal articles (journals don't have publishers in bib)
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::Publisher =>
        {
            let mut overrides = std::collections::HashMap::new();
            overrides.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                },
            );
            v.overrides = Some(overrides);
        }
        // Publisher-place: suppress for journal articles only
        // Other types (books, reports) may show publisher-place depending on style
        TemplateComponent::Variable(v)
            if v.variable == csln_core::template::SimpleVariable::PublisherPlace =>
        {
            let mut overrides = std::collections::HashMap::new();
            overrides.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    suppress: Some(true),
                    ..Default::default()
                },
            );
            v.overrides = Some(overrides);
        }
        // Pages: use extracted delimiter for journal articles, (pp. X-Y) for chapters
        TemplateComponent::Number(n) if n.number == csln_core::template::NumberVariable::Pages => {
            let mut overrides = std::collections::HashMap::new();
            // Use extracted delimiter or default to comma
            let delim = volume_pages_delimiter
                .unwrap_or(csln_core::template::DelimiterPunctuation::Comma)
                .to_string_with_space();
            overrides.insert(
                "article-journal".to_string(),
                csln_core::template::Rendering {
                    prefix: Some(delim.to_string()),
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
                    ..Default::default()
                }
            };
            overrides.insert("chapter".to_string(), chapter_rendering);
            n.overrides = Some(overrides);
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
