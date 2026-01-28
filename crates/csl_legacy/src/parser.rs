use roxmltree::Node;
use crate::model::*;

pub fn parse_style(node: Node) -> Result<Style, String> {
    let version = node.attribute("version").unwrap_or_default().to_string();
    let xmlns = node.attribute("xmlns").unwrap_or_default().to_string();
    let class = node.attribute("class").unwrap_or_default().to_string();

    let mut info = Info::default();
    let mut locale = Vec::new();
    let mut macros = Vec::new();
    let mut citation = Citation { 
        layout: Layout { children: vec![], prefix: None, suffix: None, delimiter: None }, 
        et_al_min: None, et_al_use_first: None, disambiguate_add_year_suffix: None 
    };
    let mut bibliography = None;

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "info" => info = parse_info(child)?,
            "locale" => locale.push(parse_locale(child)?),
            "macro" => macros.push(parse_macro(child)?),
            "citation" => citation = parse_citation(child)?,
            "bibliography" => bibliography = Some(parse_bibliography(child)?),
            _ => return Err(format!("Unknown top-level tag: {}", child.tag_name().name())),
        }
    }

    Ok(Style {
        version,
        xmlns,
        class,
        info,
        locale,
        macros,
        citation,
        bibliography,
    })
}

fn parse_info(node: Node) -> Result<Info, String> {
    let mut info = Info::default();
    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "title" => info.title = child.text().unwrap_or_default().to_string(),
            "id" => info.id = child.text().unwrap_or_default().to_string(),
            "updated" => info.updated = child.text().unwrap_or_default().to_string(),
            _ => {}
        }
    }
    Ok(info)
}

fn parse_locale(node: Node) -> Result<Locale, String> {
    let lang = node.attribute("lang").map(|s| s.to_string());
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

fn parse_term(node: Node) -> Result<Term, String> {
    let name = node.attribute("name").unwrap_or_default().to_string();
    let form = node.attribute("form").map(|s| s.to_string());
    let mut value = node.text().unwrap_or_default().to_string();
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
    
    Ok(Term { name, form, value, single, multiple })
}

fn parse_macro(node: Node) -> Result<Macro, String> {
    let name = node.attribute("name").ok_or("Macro missing name")?.to_string();
    let children = parse_children(node)?;
    Ok(Macro { name, children })
}

fn parse_citation(node: Node) -> Result<Citation, String> {
    let mut layout = Layout { children: vec![], prefix: None, suffix: None, delimiter: None };
    let et_al_min = node.attribute("et-al-min").and_then(|s| s.parse().ok());
    let et_al_use_first = node.attribute("et-al-use-first").and_then(|s| s.parse().ok());
    let disambiguate_add_year_suffix = node.attribute("disambiguate-add-year-suffix").map(|s| s == "true");

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "layout" => layout = parse_layout(child)?,
            _ => {}
        }
    }
    Ok(Citation { layout, et_al_min, et_al_use_first, disambiguate_add_year_suffix })
}

fn parse_bibliography(node: Node) -> Result<Bibliography, String> {
    let mut layout = Layout { children: vec![], prefix: None, suffix: None, delimiter: None };
    let mut sort = None;
    let et_al_min = node.attribute("et-al-min").and_then(|s| s.parse().ok());
    let et_al_use_first = node.attribute("et-al-use-first").and_then(|s| s.parse().ok());
    let hanging_indent = node.attribute("hanging-indent").map(|s| s == "true");

    for child in node.children() {
        if !child.is_element() { continue; }
        match child.tag_name().name() {
            "layout" => layout = parse_layout(child)?,
            "sort" => sort = Some(parse_sort(child)?),
            _ => {}
        }
    }
    Ok(Bibliography { layout, sort, et_al_min, et_al_use_first, hanging_indent })
}

fn parse_layout(node: Node) -> Result<Layout, String> {
    let prefix = node.attribute("prefix").map(|s| s.to_string());
    let suffix = node.attribute("suffix").map(|s| s.to_string());
    let delimiter = node.attribute("delimiter").map(|s| s.to_string());
    let children = parse_children(node)?;
    Ok(Layout { prefix, suffix, delimiter, children })
}

fn parse_sort(node: Node) -> Result<Sort, String> {
    let mut keys = Vec::new();
    for child in node.children() {
        if !child.is_element() { continue; }
        if child.tag_name().name() == "key" {
            keys.push(parse_sort_key(child)?);
        }
    }
    Ok(Sort { keys })
}

fn parse_sort_key(node: Node) -> Result<SortKey, String> {
    let variable = node.attribute("variable").map(|s| s.to_string());
    let macro_name = node.attribute("macro").map(|s| s.to_string());
    let sort = node.attribute("sort").map(|s| s.to_string());
    Ok(SortKey { variable, macro_name, sort })
}

fn parse_children(node: Node) -> Result<Vec<CslNode>, String> {
    let mut children = Vec::new();
    for child in node.children() {
        if !child.is_element() { continue; }
        if let Some(csl_node) = parse_node(child)? {
            children.push(csl_node);
        }
    }
    Ok(children)
}

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

fn parse_text(node: Node) -> Result<Text, String> {
    for attr in node.attributes() {
        match attr.name() {
            "value" | "variable" | "macro" | "term" | "form" | 
            "prefix" | "suffix" | "quotes" | "text-case" |
            "strip-periods" | "plural" |
            "font-style" | "font-variant" | "font-weight" | 
            "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Text has unknown attribute: {}", attr.name())),
        }
    }
    
    let formatting = parse_formatting(node);
    Ok(Text {
        value: node.attribute("value").map(|s| s.to_string()),
        variable: node.attribute("variable").map(|s| s.to_string()),
        macro_name: node.attribute("macro").map(|s| s.to_string()),
        term: node.attribute("term").map(|s| s.to_string()),
        form: node.attribute("form").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
        quotes: node.attribute("quotes").map(|s| s == "true"),
        text_case: node.attribute("text-case").map(|s| s.to_string()),
        strip_periods: node.attribute("strip-periods").map(|s| s == "true"),
        plural: node.attribute("plural").map(|s| s.to_string()),
        formatting,
    })
}

fn parse_date(node: Node) -> Result<Date, String> {
    let variable = node.attribute("variable").ok_or("Date missing variable")?.to_string();
    
    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "date-parts" | 
            "delimiter" | "text-case" |
            "font-style" | "font-variant" | "font-weight" | 
            "text-decoration" | "vertical-align" | "display" => {}
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
        form: node.attribute("form").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
        delimiter: node.attribute("delimiter").map(|s| s.to_string()),
        date_parts: node.attribute("date-parts").map(|s| s.to_string()),
        text_case: node.attribute("text-case").map(|s| s.to_string()),
        parts,
        formatting,
    })
}

fn parse_date_part(node: Node) -> Result<DatePart, String> {
    Ok(DatePart {
        name: node.attribute("name").ok_or("Date-part missing name")?.to_string(),
        form: node.attribute("form").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
    })
}

fn parse_label(node: Node) -> Result<Label, String> {
    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "text-case" |
            "strip-periods" | "plural" |
            "font-style" | "font-variant" | "font-weight" | 
            "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Label has unknown attribute: {}", attr.name())),
        }
    }
    
    // Labels have formatting too!
    let formatting = parse_formatting(node);

    Ok(Label {
        variable: node.attribute("variable").map(|s| s.to_string()),
        form: node.attribute("form").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
        text_case: node.attribute("text-case").map(|s| s.to_string()),
        strip_periods: node.attribute("strip-periods").map(|s| s == "true"),
        plural: node.attribute("plural").map(|s| s.to_string()),
        formatting,
    })
}

fn parse_names(node: Node) -> Result<Names, String> {
    let variable = node.attribute("variable").ok_or("Names missing variable")?.to_string();
    let children = parse_children(node)?;
    Ok(Names {
        variable,
        delimiter: node.attribute("delimiter").map(|s| s.to_string()),
        et_al_min: node.attribute("et-al-min").and_then(|s| s.parse().ok()),
        et_al_use_first: node.attribute("et-al-use-first").and_then(|s| s.parse().ok()),
        et_al_subsequent_min: node.attribute("et-al-subsequent-min").and_then(|s| s.parse().ok()),
        et_al_subsequent_use_first: node.attribute("et-al-subsequent-use-first").and_then(|s| s.parse().ok()),
        children,
    })
}

fn parse_formatting(node: Node) -> Formatting {
    Formatting {
        font_style: node.attribute("font-style").map(|s| s.to_string()),
        font_variant: node.attribute("font-variant").map(|s| s.to_string()),
        font_weight: node.attribute("font-weight").map(|s| s.to_string()),
        text_decoration: node.attribute("text-decoration").map(|s| s.to_string()),
        vertical_align: node.attribute("vertical-align").map(|s| s.to_string()),
        display: node.attribute("display").map(|s| s.to_string()),
    }
}

fn parse_group(node: Node) -> Result<Group, String> {
    for attr in node.attributes() {
        match attr.name() {
            "delimiter" | "prefix" | "suffix" |
            "font-style" | "font-variant" | "font-weight" | 
            "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Group has unknown attribute: {}", attr.name())),
        }
    }
    let children = parse_children(node)?;
    let formatting = parse_formatting(node);
    Ok(Group {
        delimiter: node.attribute("delimiter").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
        children,
        formatting,
    })
}

fn parse_choose(node: Node) -> Result<Choose, String> {
    let mut if_branch = None;
    let mut else_if_branches = Vec::new();
    let mut else_branch = None;

    for child in node.children() {
        if !child.is_element() { continue; }
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

fn parse_choose_branch(node: Node) -> Result<ChooseBranch, String> {
    Ok(ChooseBranch {
        match_mode: node.attribute("match").map(|s| s.to_string()),
        type_: node.attribute("type").map(|s| s.to_string()),
        variable: node.attribute("variable").map(|s| s.to_string()),
        is_numeric: node.attribute("is-numeric").map(|s| s.to_string()),
        locator: node.attribute("locator").map(|s| s.to_string()),
        position: node.attribute("position").map(|s| s.to_string()),
        children: parse_children(node)?,
    })
}

fn parse_number(node: Node) -> Result<Number, String> {
    let variable = node.attribute("variable").ok_or("Number missing variable")?.to_string();
    
    for attr in node.attributes() {
        match attr.name() {
            "variable" | "form" | "prefix" | "suffix" | "text-case" |
            "font-style" | "font-variant" | "font-weight" | 
            "text-decoration" | "vertical-align" | "display" => {}
            _ => return Err(format!("Number has unknown attribute: {}", attr.name())),
        }
    }
    
    let formatting = parse_formatting(node);

    Ok(Number {
        variable,
        form: node.attribute("form").map(|s| s.to_string()),
        prefix: node.attribute("prefix").map(|s| s.to_string()),
        suffix: node.attribute("suffix").map(|s| s.to_string()),
        text_case: node.attribute("text-case").map(|s| s.to_string()),
        formatting,
    })
}

fn parse_name(node: Node) -> Result<Name, String> {
    Ok(Name {
        and: node.attribute("and").map(|s| s.to_string()),
        delimiter: node.attribute("delimiter").map(|s| s.to_string()),
        name_as_sort_order: node.attribute("name-as-sort-order").map(|s| s.to_string()),
        sort_separator: node.attribute("sort-separator").map(|s| s.to_string()),
        initialize_with: node.attribute("initialize-with").map(|s| s.to_string()),
        form: node.attribute("form").map(|s| s.to_string()),
        delimiter_precedes_last: node.attribute("delimiter-precedes-last").map(|s| s.to_string()),
        et_al_min: node.attribute("et-al-min").and_then(|s| s.parse().ok()),
        et_al_use_first: node.attribute("et-al-use-first").and_then(|s| s.parse().ok()),
        et_al_subsequent_min: node.attribute("et-al-subsequent-min").and_then(|s| s.parse().ok()),
        et_al_subsequent_use_first: node.attribute("et-al-subsequent-use-first").and_then(|s| s.parse().ok()),
    })
}

fn parse_et_al(node: Node) -> Result<EtAl, String> {
    Ok(EtAl {
        term: node.attribute("term").map(|s| s.to_string()),
    })
}

fn parse_substitute(node: Node) -> Result<Substitute, String> {
    let children = parse_children(node)?;
    Ok(Substitute { children })
}
