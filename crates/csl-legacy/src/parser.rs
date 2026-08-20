/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! CSL 1.0 XML → [`model`](crate::model) parser.
//!
//! The entry point is [`crate::parser::parse_style()`], which accepts a [`roxmltree::Node`] rooted
//! at `<style>` and returns a fully-populated [`model::Style`].
//!
//! All helper functions follow the same pattern: they receive a node, extract
//! attributes / children, and return a `Result<T, String>` where `Err` carries
//! a human-readable description of what was missing or unexpected.

use crate::model::{
    Bibliography, Choose, ChooseBranch, Citation, CslNode, Date, DatePart, EtAl, Formatting, Group,
    Info, Label, Layout, Locale, LocalizedLayout, Macro, Name, Names, Number, Sort, SortKey, Style,
    Substitute, Term, Text,
};
use roxmltree::Node;

/// Parse the root `<style>` element into a [`Style`].
///
/// # Errors
/// Returns `Err` if any required child element is malformed or if an unknown
/// top-level tag is encountered.
pub fn parse_style(node: Node) -> Result<Style, String> {
    let version = node.attribute("version").unwrap_or_default().to_string();
    let xmlns = node.attribute("xmlns").unwrap_or_default().to_string();
    let class = node.attribute("class").unwrap_or_default().to_string();
    let default_locale = node
        .attribute("default-locale")
        .map(std::string::ToString::to_string);

    // Style-level name options (inherited by all names)
    let initialize_with = node
        .attribute("initialize-with")
        .map(std::string::ToString::to_string);
    let initialize_with_hyphen = node
        .attribute("initialize-with-hyphen")
        .map(|s| s == "true");
    let names_delimiter = node
        .attribute("names-delimiter")
        .map(std::string::ToString::to_string);
    let name_as_sort_order = node
        .attribute("name-as-sort-order")
        .map(std::string::ToString::to_string);
    let sort_separator = node
        .attribute("sort-separator")
        .map(std::string::ToString::to_string);
    let delimiter_precedes_last = node
        .attribute("delimiter-precedes-last")
        .map(std::string::ToString::to_string);
    let delimiter_precedes_et_al = node
        .attribute("delimiter-precedes-et-al")
        .map(std::string::ToString::to_string);
    let and = node.attribute("and").map(std::string::ToString::to_string);
    let page_range_format = node
        .attribute("page-range-format")
        .map(std::string::ToString::to_string);
    let demote_non_dropping_particle = node
        .attribute("demote-non-dropping-particle")
        .map(std::string::ToString::to_string);

    let mut info = Info::default();
    let mut locale = Vec::new();
    let mut macros = Vec::new();
    let mut citation = Citation {
        layout: Layout {
            children: vec![],
            prefix: None,
            suffix: None,
            delimiter: None,
        },
        localized_layouts: Vec::new(),
        sort: None,
        collapse: None,
        cite_group_delimiter: None,
        year_suffix_delimiter: None,
        et_al_min: None,
        et_al_use_first: None,
        disambiguate_add_year_suffix: None,
        disambiguate_add_names: None,
        disambiguate_add_givenname: None,
        disambiguate_givenname_rule: None,
    };
    let mut bibliography = None;

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "info" => info = parse_info(child)?,
            "locale" => locale.push(parse_locale(child)?),
            "macro" => macros.push(parse_macro(child)?),
            "citation" => citation = parse_citation(child)?,
            "bibliography" => bibliography = Some(parse_bibliography(child)?),
            _ => {
                return Err(format!(
                    "Unknown top-level tag: {}",
                    child.tag_name().name()
                ));
            }
        }
    }

    Ok(Style {
        version,
        xmlns,
        class,
        default_locale,
        initialize_with,
        initialize_with_hyphen,
        names_delimiter,
        name_as_sort_order,
        sort_separator,
        delimiter_precedes_last,
        delimiter_precedes_et_al,
        demote_non_dropping_particle,
        and,
        page_range_format,
        info,
        locale,
        macros,
        citation,
        bibliography,
    })
}

/// Parse the `<info>` element into an [`Info`] struct.
fn parse_info(node: Node) -> Result<Info, String> {
    let mut info = Info::default();
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "title" => info.title = child.text().unwrap_or_default().to_string(),
            "id" => info.id = child.text().unwrap_or_default().to_string(),
            "updated" => info.updated = child.text().unwrap_or_default().to_string(),
            "summary" => info.summary = child.text().map(std::string::ToString::to_string),
            "category" => {
                if let Some(field) = child.attribute("field").filter(|f| *f != "generic-base") {
                    info.fields.push(field.to_string());
                }
                // citation-format attribute is intentionally ignored here
                // (handled separately by options_extractor/processing.rs)
            }
            "link" => {
                let href = child.attribute("href").unwrap_or_default().to_string();
                let rel = child.attribute("rel").map(std::string::ToString::to_string);
                info.links.push(crate::model::InfoLink { href, rel });
            }
            "author" => info.authors.push(parse_info_person(child)),
            "contributor" => info.contributors.push(parse_info_person(child)),
            "rights" => {
                // Prefer license= attribute; fall back to text content
                info.rights = child
                    .attribute("license")
                    .map(std::string::ToString::to_string)
                    .or_else(|| {
                        child
                            .text()
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                    });
            }
            _ => {}
        }
    }
    Ok(info)
}

/// Parse an `<author>` or `<contributor>` element into an [`InfoPerson`].
fn parse_info_person(node: Node) -> crate::model::InfoPerson {
    let mut person = crate::model::InfoPerson::default();
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "name" => person.name = child.text().map(std::string::ToString::to_string),
            "email" => person.email = child.text().map(std::string::ToString::to_string),
            "uri" => person.uri = child.text().map(std::string::ToString::to_string),
            _ => {}
        }
    }
    person
}

/// Parse a `<locale>` element into a [`Locale`].
///
/// The `xml:lang` attribute is read as `lang`; all `<term>` children inside
/// `<terms>` are collected and parsed individually.
fn parse_locale(node: Node) -> Result<Locale, String> {
    let lang = node.attribute("lang").map(std::string::ToString::to_string);
    let mut terms = Vec::new();

    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "terms" {
            for term_node in child.children() {
                if term_node.is_element() && term_node.tag_name().name() == "term" {
                    terms.push(parse_term(term_node)?);
                }
            }
        }
    }

    Ok(Locale { lang, terms })
}

/// Parse a `<term>` element into a [`Term`].
///
/// Handles both simple terms (text content only) and terms with
/// `<single>` / `<multiple>` child elements.
fn parse_term(node: Node) -> Result<Term, String> {
    let name = node.attribute("name").unwrap_or_default().to_string();
    let form = node.attribute("form").map(std::string::ToString::to_string);
    let value = node.text().unwrap_or_default().to_string();
    let mut single = None;
    let mut multiple = None;

    // Check for single/multiple children
    for child in node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "single" => single = Some(child.text().unwrap_or_default().to_string()),
                "multiple" => multiple = Some(child.text().unwrap_or_default().to_string()),
                _ => {}
            }
        }
    }

    // If no single/multiple, value is the text content
    // Actually, simple terms just have text content.

    Ok(Term {
        name,
        form,
        value,
        single,
        multiple,
    })
}

/// Parse a `<macro>` element into a [`Macro`].
///
/// # Errors
/// Returns `Err` when the `name` attribute is missing.
fn parse_macro(node: Node) -> Result<Macro, String> {
    let name = node
        .attribute("name")
        .ok_or("Macro missing name")?
        .to_string();
    let children = parse_children(node)?;
    Ok(Macro { name, children })
}

/// Parse a `<citation>` element into a [`Citation`].
fn parse_citation(node: Node) -> Result<Citation, String> {
    let mut sort = None;
    let collapse = node
        .attribute("collapse")
        .map(std::string::ToString::to_string);
    let cite_group_delimiter = node
        .attribute("cite-group-delimiter")
        .map(std::string::ToString::to_string);
    let year_suffix_delimiter = node
        .attribute("year-suffix-delimiter")
        .map(std::string::ToString::to_string);
    let et_al_min = node.attribute("et-al-min").and_then(|s| s.parse().ok());
    let et_al_use_first = node
        .attribute("et-al-use-first")
        .and_then(|s| s.parse().ok());
    let disambiguate_add_year_suffix = node
        .attribute("disambiguate-add-year-suffix")
        .map(|s| s == "true");
    let disambiguate_add_names = node
        .attribute("disambiguate-add-names")
        .map(|s| s == "true");
    let disambiguate_add_givenname = node
        .attribute("disambiguate-add-givenname")
        .map(|s| s == "true");
    let disambiguate_givenname_rule = node
        .attribute("givenname-disambiguation-rule")
        .map(std::string::ToString::to_string);

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "layout" => {}
            "sort" => sort = Some(parse_sort(child)?),
            _ => {}
        }
    }
    let (layout, localized_layouts) = parse_ordered_layouts(node, "citation")?;
    Ok(Citation {
        layout,
        localized_layouts,
        sort,
        collapse,
        cite_group_delimiter,
        year_suffix_delimiter,
        et_al_min,
        et_al_use_first,
        disambiguate_add_year_suffix,
        disambiguate_add_names,
        disambiguate_add_givenname,
        disambiguate_givenname_rule,
    })
}

/// Parse a `<bibliography>` element into a [`Bibliography`].
fn parse_bibliography(node: Node) -> Result<Bibliography, String> {
    let mut sort = None;
    let et_al_min = node.attribute("et-al-min").and_then(|s| s.parse().ok());
    let et_al_use_first = node
        .attribute("et-al-use-first")
        .and_then(|s| s.parse().ok());
    let hanging_indent = node.attribute("hanging-indent").map(|s| s == "true");
    let second_field_align = node
        .attribute("second-field-align")
        .map(std::string::ToString::to_string);

    let subsequent_author_substitute = node
        .attribute("subsequent-author-substitute")
        .map(std::string::ToString::to_string);
    let subsequent_author_substitute_rule = node
        .attribute("subsequent-author-substitute-rule")
        .map(std::string::ToString::to_string);

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "layout" => {}
            "sort" => sort = Some(parse_sort(child)?),
            _ => {}
        }
    }
    let (layout, localized_layouts) = parse_ordered_layouts(node, "bibliography")?;
    Ok(Bibliography {
        layout,
        localized_layouts,
        sort,
        et_al_min,
        et_al_use_first,
        hanging_indent,
        second_field_align,
        subsequent_author_substitute,
        subsequent_author_substitute_rule,
    })
}

/// Parse conventional or ordered CSL-M layouts while retaining the singular fallback API.
fn parse_ordered_layouts(
    node: Node,
    section: &str,
) -> Result<(Layout, Vec<LocalizedLayout>), String> {
    let mut layouts = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() != "layout" {
            continue;
        }
        let locales = child
            .attribute("locale")
            .map(|value| value.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        layouts.push(LocalizedLayout {
            locales,
            layout: parse_layout(child)?,
        });
    }

    let empty = || Layout {
        children: Vec::new(),
        prefix: None,
        suffix: None,
        delimiter: None,
    };
    match layouts.as_slice() {
        [] => return Ok((empty(), Vec::new())),
        [only] if only.locales.is_empty() => return Ok((only.layout.clone(), Vec::new())),
        _ => {}
    }

    let mut fallback_index = None;
    for (index, localized) in layouts.iter().enumerate() {
        if localized.locales.is_empty() {
            if fallback_index.is_some() {
                return Err(format!(
                    "CSL-M {section} must contain exactly one unscoped fallback layout"
                ));
            }
            fallback_index = Some(index);
        } else if fallback_index.is_some() {
            return Err(format!(
                "CSL-M {section} locale-scoped layout appears after the unscoped fallback"
            ));
        }
    }

    let Some(fallback_index) = fallback_index else {
        return Err(format!(
            "CSL-M {section} locale-scoped layouts require a final unscoped fallback"
        ));
    };
    if fallback_index + 1 != layouts.len() {
        return Err(format!(
            "CSL-M {section} unscoped fallback layout must be last"
        ));
    }

    let fallback = layouts
        .get(fallback_index)
        .ok_or_else(|| format!("CSL-M {section} fallback layout index is invalid"))?
        .layout
        .clone();
    Ok((fallback, layouts))
}

/// Parse a `<layout>` element into a [`Layout`].
fn parse_layout(node: Node) -> Result<Layout, String> {
    let prefix = node
        .attribute("prefix")
        .map(std::string::ToString::to_string);
    let suffix = node
        .attribute("suffix")
        .map(std::string::ToString::to_string);
    let delimiter = node
        .attribute("delimiter")
        .map(std::string::ToString::to_string);
    let children = parse_children(node)?;
    Ok(Layout {
        prefix,
        suffix,
        delimiter,
        children,
    })
}

/// Parse a `<sort>` element into a [`Sort`].
fn parse_sort(node: Node) -> Result<Sort, String> {
    let mut keys = Vec::new();
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        if child.tag_name().name() == "key" {
            keys.push(parse_sort_key(child)?);
        }
    }
    Ok(Sort { keys })
}

/// Parse a `<key>` element into a [`SortKey`].
fn parse_sort_key(node: Node) -> Result<SortKey, String> {
    let variable = node
        .attribute("variable")
        .map(std::string::ToString::to_string);
    let macro_name = node
        .attribute("macro")
        .map(std::string::ToString::to_string);
    let sort = node.attribute("sort").map(std::string::ToString::to_string);
    Ok(SortKey {
        variable,
        macro_name,
        sort,
    })
}

/// Collect all element children of `node` into a [`Vec<CslNode>`].
///
/// Text nodes and processing instructions are ignored.
fn parse_children(node: Node) -> Result<Vec<CslNode>, String> {
    let mut children = Vec::new();
    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        if let Some(csl_node) = parse_node(child)? {
            children.push(csl_node);
        }
    }
    Ok(children)
}

/// Dispatch an XML element to the appropriate node parser.
///
/// Returns `Ok(None)` for unknown tags that should be silently ignored (none
/// currently), or `Err` for genuinely unrecognised tags.
fn parse_node(node: Node) -> Result<Option<CslNode>, String> {
    match node.tag_name().name() {
        "text" => Ok(Some(CslNode::Text(parse_text(node)?))),
        "date" => Ok(Some(CslNode::Date(parse_date(node)?))),
        "label" => Ok(Some(CslNode::Label(parse_label(node)?))),
        "names" => Ok(Some(CslNode::Names(parse_names(node)?))),
        "group" => Ok(Some(CslNode::Group(parse_group(node)?))),
        "choose" => Ok(Some(CslNode::Choose(parse_choose(node)?))),
        "number" => Ok(Some(CslNode::Number(parse_number(node)?))),
        "name" => Ok(Some(CslNode::Name(parse_name(node)?))),
        "et-al" => Ok(Some(CslNode::EtAl(parse_et_al(node)?))),
        "substitute" => Ok(Some(CslNode::Substitute(parse_substitute(node)?))),
        _ => Err(format!("Unknown node tag: {}", node.tag_name().name())),
    }
}

/// Parse a `<text>` element into a [`Text`] node.
///
/// # Errors
/// Returns `Err` when an unrecognised attribute is present.
fn parse_text(node: Node) -> Result<Text, String> {
    for attr in node.attributes() {
        match attr.name() {
            "value" | "variable" | "macro" | "term" | "form" | "prefix" | "suffix" | "quotes"
            | "text-case" | "strip-periods" | "plural" | "font-style" | "font-variant"
            | "font-weight" | "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Text has unknown attribute: {}", attr.name())),
        }
    }

    let formatting = parse_formatting(node);
    Ok(Text {
        value: node
            .attribute("value")
            .map(std::string::ToString::to_string),
        variable: node
            .attribute("variable")
            .map(std::string::ToString::to_string),
        macro_name: node
            .attribute("macro")
            .map(std::string::ToString::to_string),
        term: node.attribute("term").map(std::string::ToString::to_string),
        form: node.attribute("form").map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        quotes: node.attribute("quotes").map(|s| s == "true"),
        text_case: node
            .attribute("text-case")
            .map(std::string::ToString::to_string),
        strip_periods: node.attribute("strip-periods").map(|s| s == "true"),
        plural: node
            .attribute("plural")
            .map(std::string::ToString::to_string),
        macro_call_order: None,
        formatting,
    })
}

/// Parse a `<date>` element into a [`Date`] node.
///
/// # Errors
/// Returns `Err` when the mandatory `variable` attribute is absent, or when
/// an unrecognised attribute is present.
fn parse_date(node: Node) -> Result<Date, String> {
    let variable = node
        .attribute("variable")
        .ok_or("Date missing variable")?
        .to_string();

    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "date-parts" | "delimiter"
            | "text-case" | "font-style" | "font-variant" | "font-weight" | "text-decoration"
            | "vertical-align" | "display" => {}
            _ => return Err(format!("Date has unknown attribute: {}", attr.name())),
        }
    }

    let mut parts = Vec::new();
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "date-part" {
            parts.push(parse_date_part(child)?);
        }
    }

    // Dates can also have formatting!
    let formatting = parse_formatting(node);

    Ok(Date {
        variable,
        form: node.attribute("form").map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        delimiter: node
            .attribute("delimiter")
            .map(std::string::ToString::to_string),
        date_parts: node
            .attribute("date-parts")
            .map(std::string::ToString::to_string),
        text_case: node
            .attribute("text-case")
            .map(std::string::ToString::to_string),
        parts,
        macro_call_order: None,
        formatting,
    })
}

/// Parse a `<date-part>` child element into a [`DatePart`].
///
/// # Errors
/// Returns `Err` when the mandatory `name` attribute is absent.
fn parse_date_part(node: Node) -> Result<DatePart, String> {
    Ok(DatePart {
        name: node
            .attribute("name")
            .ok_or("Date-part missing name")?
            .to_string(),
        form: node.attribute("form").map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
    })
}

/// Parse a `<label>` element into a [`Label`] node.
///
/// # Errors
/// Returns `Err` when an unrecognised attribute is present.
fn parse_label(node: Node) -> Result<Label, String> {
    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "text-case" | "strip-periods"
            | "plural" | "font-style" | "font-variant" | "font-weight" | "text-decoration"
            | "vertical-align" | "display" => {}
            _ => return Err(format!("Label has unknown attribute: {}", attr.name())),
        }
    }

    // Labels have formatting too!
    let formatting = parse_formatting(node);

    Ok(Label {
        variable: node
            .attribute("variable")
            .map(std::string::ToString::to_string),
        form: node.attribute("form").map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        text_case: node
            .attribute("text-case")
            .map(std::string::ToString::to_string),
        strip_periods: node.attribute("strip-periods").map(|s| s == "true"),
        plural: node
            .attribute("plural")
            .map(std::string::ToString::to_string),
        macro_call_order: None,
        formatting,
    })
}

/// Parse a `<names>` element into a [`Names`] node.
///
/// # Errors
/// Returns `Err` when the mandatory `variable` attribute is absent.
fn parse_names(node: Node) -> Result<Names, String> {
    let variable = node
        .attribute("variable")
        .ok_or("Names missing variable")?
        .to_string();
    let children = parse_children(node)?;
    let formatting = parse_formatting(node);
    Ok(Names {
        variable,
        delimiter: node
            .attribute("delimiter")
            .map(std::string::ToString::to_string),
        delimiter_precedes_et_al: node
            .attribute("delimiter-precedes-et-al")
            .map(std::string::ToString::to_string),
        et_al_min: node.attribute("et-al-min").and_then(|s| s.parse().ok()),
        et_al_use_first: node
            .attribute("et-al-use-first")
            .and_then(|s| s.parse().ok()),
        et_al_subsequent_min: node
            .attribute("et-al-subsequent-min")
            .and_then(|s| s.parse().ok()),
        et_al_subsequent_use_first: node
            .attribute("et-al-subsequent-use-first")
            .and_then(|s| s.parse().ok()),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        children,
        macro_call_order: None,
        formatting,
    })
}

/// Extract inline formatting attributes from any CSL element node.
fn parse_formatting(node: Node) -> Formatting {
    Formatting {
        font_style: node
            .attribute("font-style")
            .map(std::string::ToString::to_string),
        font_variant: node
            .attribute("font-variant")
            .map(std::string::ToString::to_string),
        font_weight: node
            .attribute("font-weight")
            .map(std::string::ToString::to_string),
        text_decoration: node
            .attribute("text-decoration")
            .map(std::string::ToString::to_string),
        vertical_align: node
            .attribute("vertical-align")
            .map(std::string::ToString::to_string),
        display: node
            .attribute("display")
            .map(std::string::ToString::to_string),
    }
}

/// Parse a `<group>` element into a [`Group`] node.
///
/// # Errors
/// Returns `Err` when an unrecognised attribute is present.
fn parse_group(node: Node) -> Result<Group, String> {
    for attr in node.attributes() {
        match attr.name() {
            "delimiter" | "prefix" | "suffix" | "font-style" | "font-variant" | "font-weight"
            | "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Group has unknown attribute: {}", attr.name())),
        }
    }
    let children = parse_children(node)?;
    let formatting = parse_formatting(node);
    Ok(Group {
        delimiter: node
            .attribute("delimiter")
            .map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        children,
        macro_call_order: None,
        formatting,
    })
}

/// Parse a `<choose>` element into a [`Choose`] node.
///
/// # Errors
/// Returns `Err` when the mandatory `<if>` child is absent.
fn parse_choose(node: Node) -> Result<Choose, String> {
    let mut if_branch = None;
    let mut else_if_branches = Vec::new();
    let mut else_branch = None;

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "if" => if_branch = Some(parse_choose_branch(child)?),
            "else-if" => else_if_branches.push(parse_choose_branch(child)?),
            "else" => else_branch = Some(parse_children(child)?),
            _ => {}
        }
    }

    Ok(Choose {
        if_branch: if_branch.ok_or("Choose missing if block")?,
        else_if_branches,
        else_branch,
    })
}

/// Parse an `<if>` or `<else-if>` element into a [`ChooseBranch`].
fn parse_choose_branch(node: Node) -> Result<ChooseBranch, String> {
    Ok(ChooseBranch {
        match_mode: node
            .attribute("match")
            .map(std::string::ToString::to_string),
        type_: node.attribute("type").map(std::string::ToString::to_string),
        variable: node
            .attribute("variable")
            .map(std::string::ToString::to_string),
        is_numeric: node
            .attribute("is-numeric")
            .map(std::string::ToString::to_string),
        is_uncertain_date: node
            .attribute("is-uncertain-date")
            .map(std::string::ToString::to_string),
        locator: node
            .attribute("locator")
            .map(std::string::ToString::to_string),
        position: node
            .attribute("position")
            .map(std::string::ToString::to_string),
        children: parse_children(node)?,
    })
}

/// Parse a `<number>` element into a [`Number`] node.
///
/// # Errors
/// Returns `Err` when the mandatory `variable` attribute is absent, or when
/// an unrecognised attribute is present.
fn parse_number(node: Node) -> Result<Number, String> {
    let variable = node
        .attribute("variable")
        .ok_or("Number missing variable")?
        .to_string();

    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "text-case" | "font-style"
            | "font-variant" | "font-weight" | "text-decoration" | "vertical-align" | "display" => {
            }
            _ => return Err(format!("Number has unknown attribute: {}", attr.name())),
        }
    }

    let formatting = parse_formatting(node);

    Ok(Number {
        variable,
        form: node.attribute("form").map(std::string::ToString::to_string),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        text_case: node
            .attribute("text-case")
            .map(std::string::ToString::to_string),
        macro_call_order: None,
        formatting,
    })
}

/// Parse a `<name>` element into a [`Name`] node.
fn parse_name(node: Node) -> Result<Name, String> {
    let formatting = parse_formatting(node);
    Ok(Name {
        and: node.attribute("and").map(std::string::ToString::to_string),
        delimiter: node
            .attribute("delimiter")
            .map(std::string::ToString::to_string),
        name_as_sort_order: node
            .attribute("name-as-sort-order")
            .map(std::string::ToString::to_string),
        sort_separator: node
            .attribute("sort-separator")
            .map(std::string::ToString::to_string),
        initialize_with: node
            .attribute("initialize-with")
            .map(std::string::ToString::to_string),
        initialize_with_hyphen: node
            .attribute("initialize-with-hyphen")
            .map(|s| s == "true"),
        form: node.attribute("form").map(std::string::ToString::to_string),
        delimiter_precedes_last: node
            .attribute("delimiter-precedes-last")
            .map(std::string::ToString::to_string),
        delimiter_precedes_et_al: node
            .attribute("delimiter-precedes-et-al")
            .map(std::string::ToString::to_string),
        et_al_min: node.attribute("et-al-min").and_then(|s| s.parse().ok()),
        et_al_use_first: node
            .attribute("et-al-use-first")
            .and_then(|s| s.parse().ok()),
        et_al_subsequent_min: node
            .attribute("et-al-subsequent-min")
            .and_then(|s| s.parse().ok()),
        et_al_subsequent_use_first: node
            .attribute("et-al-subsequent-use-first")
            .and_then(|s| s.parse().ok()),
        prefix: node
            .attribute("prefix")
            .map(std::string::ToString::to_string),
        suffix: node
            .attribute("suffix")
            .map(std::string::ToString::to_string),
        formatting,
    })
}

/// Parse an `<et-al>` element into an [`EtAl`] node.
fn parse_et_al(node: Node) -> Result<EtAl, String> {
    Ok(EtAl {
        term: node.attribute("term").map(std::string::ToString::to_string),
    })
}

/// Parse a `<substitute>` element into a [`Substitute`] node.
fn parse_substitute(node: Node) -> Result<Substitute, String> {
    let children = parse_children(node)?;
    Ok(Substitute { children })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use roxmltree::Document;

    /// Minimal valid CSL XML wrapper — provides the mandatory `<citation><layout/></citation>`.
    fn wrap_style(inner: &str) -> String {
        format!(
            r#"<style version="1.0" xmlns="http://purl.org/net/xbiblio/csl" class="in-text">
  <info><title>Test</title><id>test</id><updated>2024-01-01T00:00:00+00:00</updated></info>
  {inner}
  <citation><layout/></citation>
</style>"#
        )
    }

    fn parse(xml: &str) -> Result<Style, String> {
        let doc = Document::parse(xml).map_err(|e| e.to_string())?;
        parse_style(doc.root_element())
    }

    // ------------------------------------------------------------------
    // parse_style
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_minimal_style() {
        let xml = wrap_style("");
        let style = parse(&xml).unwrap();
        assert_eq!(style.version, "1.0");
        assert_eq!(style.class, "in-text");
        assert_eq!(style.info.title, "Test");
        assert!(style.bibliography.is_none());
    }

    #[test]
    fn test_parse_style_name_options() {
        let xml = wrap_style("");
        // Inject name-option attrs on the root <style> element
        let xml = xml.replace(
            r#"class="in-text""#,
            r#"class="in-text" initialize-with="." names-delimiter="; " and="text""#,
        );
        let style = parse(&xml).unwrap();
        assert_eq!(style.initialize_with.as_deref(), Some("."));
        assert_eq!(style.names_delimiter.as_deref(), Some("; "));
        assert_eq!(style.and.as_deref(), Some("text"));
    }

    #[test]
    fn test_parse_citation_collapse() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            "<citation collapse=\"citation-number\"><layout/></citation>",
        );
        let style = parse(&xml).unwrap();
        assert_eq!(style.citation.collapse.as_deref(), Some("citation-number"));
    }

    #[test]
    fn test_parse_citation_collapse_delimiters() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            r#"<citation collapse="year-suffix" cite-group-delimiter=", " year-suffix-delimiter="-"><layout/></citation>"#,
        );
        let style = parse(&xml).unwrap();
        assert_eq!(style.citation.collapse.as_deref(), Some("year-suffix"));
        assert_eq!(style.citation.cite_group_delimiter.as_deref(), Some(", "));
        assert_eq!(style.citation.year_suffix_delimiter.as_deref(), Some("-"));
    }

    #[test]
    fn test_parse_csl_m_layouts_preserves_order_and_locale_list() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            r#"<citation>
              <layout locale="en-US en-GB"><text value="English"/></layout>
              <layout locale="zh-CN"><text value="Chinese"/></layout>
              <layout><text value="Fallback"/></layout>
            </citation>"#,
        );

        let style = parse(&xml).unwrap();

        assert_eq!(style.citation.localized_layouts.len(), 3);
        assert_eq!(
            style.citation.localized_layouts[0].locales,
            ["en-US", "en-GB"]
        );
        assert_eq!(style.citation.localized_layouts[1].locales, ["zh-CN"]);
        assert!(style.citation.localized_layouts[2].locales.is_empty());
        match &style.citation.layout.children[0] {
            CslNode::Text(text) => assert_eq!(text.value.as_deref(), Some("Fallback")),
            other => panic!("expected fallback text node, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_conventional_layout_keeps_localized_list_empty() {
        let style = parse(&wrap_style("")).unwrap();

        assert!(style.citation.localized_layouts.is_empty());
        assert!(style.citation.layout.children.is_empty());
    }

    #[test]
    fn test_parse_csl_m_layouts_rejects_missing_fallback() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            r#"<citation><layout locale="zh-CN"><text value="Chinese"/></layout></citation>"#,
        );

        assert_eq!(
            parse(&xml).unwrap_err(),
            "CSL-M citation locale-scoped layouts require a final unscoped fallback"
        );
    }

    #[test]
    fn test_parse_csl_m_layouts_rejects_multiple_fallbacks() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            r#"<citation><layout/><layout/></citation>"#,
        );

        assert_eq!(
            parse(&xml).unwrap_err(),
            "CSL-M citation must contain exactly one unscoped fallback layout"
        );
    }

    #[test]
    fn test_parse_csl_m_layouts_rejects_scoped_layout_after_fallback() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            r#"<citation><layout/><layout locale="en"><text value="English"/></layout></citation>"#,
        );

        assert_eq!(
            parse(&xml).unwrap_err(),
            "CSL-M citation locale-scoped layout appears after the unscoped fallback"
        );
    }

    #[test]
    fn test_parse_style_unknown_top_level_tag_errors() {
        let xml = wrap_style("<not-a-csl-tag/>");
        assert!(parse(&xml).is_err());
    }

    // ------------------------------------------------------------------
    // parse_info (rights)
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_info_rights_license_attr() {
        let xml = wrap_style(
            r"<!-- rights override handled in info block -->",
        )
        .replace(
            "<updated>2024-01-01T00:00:00+00:00</updated>",
            "<updated>2024-01-01T00:00:00+00:00</updated><rights license=\"https://example.com/license\">Some text</rights>",
        );
        let style = parse(&xml).unwrap();
        // license= attribute takes priority over element text
        assert_eq!(
            style.info.rights.as_deref(),
            Some("https://example.com/license")
        );
    }

    #[test]
    fn test_parse_info_rights_text_fallback() {
        let xml = wrap_style("").replace(
            "<updated>2024-01-01T00:00:00+00:00</updated>",
            "<updated>2024-01-01T00:00:00+00:00</updated><rights>MIT License</rights>",
        );
        let style = parse(&xml).unwrap();
        assert_eq!(style.info.rights.as_deref(), Some("MIT License"));
    }

    // ------------------------------------------------------------------
    // parse_locale / parse_term
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_locale_terms() {
        let xml = wrap_style(
            r#"<locale xml:lang="en-US">
              <terms>
                <term name="editor" form="short">ed.<single>ed.</single><multiple>eds.</multiple></term>
              </terms>
            </locale>"#,
        );
        let style = parse(&xml).unwrap();
        assert_eq!(style.locale.len(), 1);
        let term = &style.locale[0].terms[0];
        assert_eq!(term.name, "editor");
        assert_eq!(term.form.as_deref(), Some("short"));
        assert_eq!(term.single.as_deref(), Some("ed."));
        assert_eq!(term.multiple.as_deref(), Some("eds."));
    }

    // ------------------------------------------------------------------
    // parse_choose
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_choose_requires_if() {
        // A <choose> with only an <else> but no <if> should fail.
        let xml = wrap_style(
            r#"<citation>
              <layout>
                <choose><else><text value="x"/></else></choose>
              </layout>
            </citation>"#,
        )
        // Remove the auto-generated citation wrapper by supplying our own
        .replace("<citation><layout/></citation>", "");
        assert!(parse(&xml).is_err());
    }

    // ------------------------------------------------------------------
    // parse_node
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_node_unknown_tag_errors() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            "<citation><layout><unknown-tag/></layout></citation>",
        );
        assert!(parse(&xml).is_err());
    }

    // ------------------------------------------------------------------
    // parse_date
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_date_missing_variable_errors() {
        let xml = wrap_style("").replace(
            "<citation><layout/></citation>",
            "<citation><layout><date/></layout></citation>",
        );
        assert!(parse(&xml).is_err());
    }

    // ------------------------------------------------------------------
    // parse_macro
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_macro_missing_name_errors() {
        let xml = wrap_style("<macro><text value=\"x\"/></macro>");
        assert!(parse(&xml).is_err());
    }
}
