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
    let options = OptionsExtractor::extract(&legacy_style);

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

        // Bibliography: fix author form (long), date wrap (parentheses), title emph
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
                    td.rendering.wrap = Some(csln_core::template::WrapPunctuation::Parentheses);
                }
                TemplateComponent::Title(tt)
                    if tt.title == csln_core::template::TitleType::Primary =>
                {
                    tt.rendering.emph = Some(true);
                }
                _ => {}
            }
        }

        // Add editor for chapters: "In Editor (Eds.), Container"
        let has_editor = new_bib.iter().any(|c| {
            matches!(c, TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Editor)
        });
        if !has_editor {
            // Insert editor before parent-monograph title
            // Use given-first name order for "In Editor (Eds.)," context per APA
            let container_pos = new_bib.iter().position(|c| {
                matches!(c, TemplateComponent::Title(tt) if tt.title == csln_core::template::TitleType::ParentMonograph)
            });
            if let Some(pos) = container_pos {
                new_bib.insert(
                    pos,
                    TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                        contributor: csln_core::template::ContributorRole::Editor,
                        form: csln_core::template::ContributorForm::Verb,
                        name_order: Some(csln_core::template::NameOrder::GivenFirst), // APA: "K. A. Ericsson", not "Ericsson, K. A."
                        delimiter: None,
                        rendering: csln_core::template::Rendering {
                            prefix: Some("In ".to_string()),
                            suffix: Some(", ".to_string()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                );
            }
        }

        // Combine volume and issue into a List component
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
                delimiter: Some(csln_core::template::DelimiterPunctuation::None), // No delimiter between volume and (issue)
                rendering: csln_core::template::Rendering::default(),
                overrides: None,
                ..Default::default()
            });

            new_bib.insert(min_idx, vol_issue_list);
        }

        // Add type-specific overrides
        for component in &mut new_bib {
            match component {
                // Container-title (parent-serial): journals use comma suffix
                TemplateComponent::Title(t)
                    if t.title == csln_core::template::TitleType::ParentSerial =>
                {
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
                // Pages: different formatting per type
                TemplateComponent::Number(n)
                    if n.number == csln_core::template::NumberVariable::Pages =>
                {
                    let mut overrides = std::collections::HashMap::new();
                    // Chapters need "(pp. pages)"
                    overrides.insert(
                        "chapter".to_string(),
                        csln_core::template::Rendering {
                            prefix: Some("pp. ".to_string()),
                            wrap: Some(csln_core::template::WrapPunctuation::Parentheses),
                            ..Default::default()
                        },
                    );
                    // Journals: comma prefix to connect with volume
                    overrides.insert(
                        "article-journal".to_string(),
                        csln_core::template::Rendering {
                            prefix: Some(", ".to_string()),
                            ..Default::default()
                        },
                    );
                    n.overrides = Some(overrides);
                }
                // Publisher: suppress for journal articles
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
                _ => {}
            }
        }
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
            let (wrap, prefix, suffix) = infer_citation_wrapping(
                &legacy_style.citation.layout.prefix,
                &legacy_style.citation.layout.suffix,
            );
            CitationSpec {
                options: None,
                template: new_cit,
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
            template: new_bib,
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

/// Infer citation wrapping from CSL prefix/suffix.
/// Returns (wrap, prefix, suffix) - uses wrap when possible, falls back to affixes.
fn infer_citation_wrapping(
    prefix: &Option<String>,
    suffix: &Option<String>,
) -> (Option<WrapPunctuation>, Option<String>, Option<String>) {
    match (prefix.as_deref(), suffix.as_deref()) {
        // Clean cases -> use wrap
        (Some("("), Some(")")) => (Some(WrapPunctuation::Parentheses), None, None),
        (Some("["), Some("]")) => (Some(WrapPunctuation::Brackets), None, None),
        // No affixes
        (None, None) | (Some(""), Some("")) | (Some(""), None) | (None, Some("")) => {
            (None, None, None)
        }
        // Edge cases -> use prefix/suffix
        _ => (None, prefix.clone(), suffix.clone()),
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
/// We prefer the innermost group delimiter, but fall back to date prefix.
fn extract_citation_delimiter(layout: &csl_legacy::model::Layout) -> Option<String> {
    use csl_legacy::model::CslNode;

    /// Find the innermost group that directly contains text/names elements.
    /// Uses depth-first search: prefer children's delimiters over current group's.
    fn find_innermost_text_group_delimiter(nodes: &[CslNode]) -> Option<String> {
        for node in nodes {
            match node {
                CslNode::Group(group) => {
                    // FIRST: recurse into children to find deeper groups
                    // This ensures we prefer innermost groups over outer wrappers
                    if let Some(d) = find_innermost_text_group_delimiter(&group.children) {
                        return Some(d);
                    }

                    // THEN: if no children have a delimiter, check this group
                    // Only return this group's delimiter if it directly contains text/names
                    let has_text_or_names = group
                        .children
                        .iter()
                        .any(|c| matches!(c, CslNode::Text(_) | CslNode::Names(_)));

                    if has_text_or_names && group.delimiter.is_some() {
                        return group.delimiter.clone();
                    }
                }
                CslNode::Choose(choose) => {
                    // Search inside choose if-branch first (most common case for author-date)
                    if let Some(d) = find_innermost_text_group_delimiter(&choose.if_branch.children)
                    {
                        return Some(d);
                    }
                    // Also check else-if and else branches
                    for else_if in &choose.else_if_branches {
                        if let Some(d) = find_innermost_text_group_delimiter(&else_if.children) {
                            return Some(d);
                        }
                    }
                    if let Some(ref else_children) = choose.else_branch {
                        if let Some(d) = find_innermost_text_group_delimiter(else_children) {
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

    // First try to find a group delimiter
    if let Some(delim) = find_innermost_text_group_delimiter(&layout.children) {
        return Some(delim);
    }

    // Fall back to prefix on date element
    if let Some(prefix) = find_date_prefix(&layout.children) {
        return Some(prefix);
    }

    // No delimiter found - return None (processor will use default)
    None
}
