use csl_legacy::model::{CslNode, Names, Style, Substitute};
use csln_core::options::{
    AndOptions, ContributorConfig, DelimiterPrecedesLast, DemoteNonDroppingParticle, DisplayAsSort,
    ShortenListOptions, Substitute as CslnSubstitute, SubstituteKey,
};
use std::collections::{HashMap, HashSet};

pub fn extract_contributor_config(style: &Style) -> Option<ContributorConfig> {
    let mut config = ContributorConfig::default();
    let mut has_config = false;

    // 1. Extract from style-level name attributes
    if let Some(and) = &style.and {
        config.and = Some(match and.as_str() {
            "text" => AndOptions::Text,
            "symbol" => AndOptions::Symbol,
            _ => AndOptions::None,
        });
        has_config = true;
    }

    if let Some(delim) = &style.delimiter_precedes_last {
        config.delimiter_precedes_last = Some(match delim.as_str() {
            "always" => DelimiterPrecedesLast::Always,
            "never" => DelimiterPrecedesLast::Never,
            "contextual" => DelimiterPrecedesLast::Contextual,
            "after-inverted-name" => DelimiterPrecedesLast::AfterInvertedName,
            _ => DelimiterPrecedesLast::Contextual,
        });
        has_config = true;
    }

    if let Some(demote) = &style.demote_non_dropping_particle {
        config.demote_non_dropping_particle = Some(match demote.as_str() {
            "never" => DemoteNonDroppingParticle::Never,
            "sort-only" => DemoteNonDroppingParticle::SortOnly,
            "display-and-sort" => DemoteNonDroppingParticle::DisplayAndSort,
            _ => DemoteNonDroppingParticle::DisplayAndSort,
        });
        has_config = true;
    }

    if let Some(init) = &style.initialize_with {
        config.initialize_with = Some(init.clone());
        has_config = true;
    }

    // 2. Scan citation and bibliography for name options (et-al, display-as-sort)
    let bib_macros = collect_bibliography_macros(style);
    let cit_macros = collect_citation_macros(style);

    if let Some(bib) = &style.bibliography {
        if let Some(bib_opts) =
            extract_name_options_from_nodes(&bib.layout.children, style, &bib_macros)
        {
            if bib_opts.shorten.is_some() {
                config.shorten = bib_opts.shorten;
            }
            if bib_opts.display_as_sort.is_some() {
                config.display_as_sort = bib_opts.display_as_sort;
            }
            if bib_opts.delimiter.is_some() {
                config.delimiter = bib_opts.delimiter;
            }
            if bib_opts.sort_separator.is_some() {
                config.sort_separator = bib_opts.sort_separator;
            }
            if bib_opts.initialize_with.is_some() {
                config.initialize_with = bib_opts.initialize_with;
            }
            if bib_opts.initialize_with_hyphen.is_some() {
                config.initialize_with_hyphen = bib_opts.initialize_with_hyphen;
            }
            if bib_opts.delimiter_precedes_last.is_some() {
                config.delimiter_precedes_last = bib_opts.delimiter_precedes_last;
            }
            if bib_opts.delimiter_precedes_et_al.is_some() {
                config.delimiter_precedes_et_al = bib_opts.delimiter_precedes_et_al;
            }
            has_config = true;
        }
    }

    if let Some(cit_opts) =
        extract_name_options_from_nodes(&style.citation.layout.children, style, &cit_macros)
    {
        if cit_opts.shorten.is_some() {
            config.shorten = cit_opts.shorten;
        }
        if config.display_as_sort.is_none() && cit_opts.display_as_sort.is_some() {
            config.display_as_sort = cit_opts.display_as_sort;
        }
        if config.delimiter.is_none() && cit_opts.delimiter.is_some() {
            config.delimiter = cit_opts.delimiter;
        }
        if config.sort_separator.is_none() && cit_opts.sort_separator.is_some() {
            config.sort_separator = cit_opts.sort_separator;
        }
        if config.initialize_with.is_none() && cit_opts.initialize_with.is_some() {
            config.initialize_with = cit_opts.initialize_with;
        }
        if config.initialize_with_hyphen.is_none() && cit_opts.initialize_with_hyphen.is_some() {
            config.initialize_with_hyphen = cit_opts.initialize_with_hyphen;
        }
        if config.delimiter_precedes_last.is_none() && cit_opts.delimiter_precedes_last.is_some() {
            config.delimiter_precedes_last = cit_opts.delimiter_precedes_last;
        }
        if config.delimiter_precedes_et_al.is_none() && cit_opts.delimiter_precedes_et_al.is_some()
        {
            config.delimiter_precedes_et_al = cit_opts.delimiter_precedes_et_al;
        }
        has_config = true;
    }

    if has_config {
        Some(config)
    } else {
        None
    }
}

fn collect_bibliography_macros(style: &Style) -> HashSet<String> {
    let mut macros = HashSet::new();
    if let Some(bib) = &style.bibliography {
        collect_macro_refs_from_nodes(&bib.layout.children, &mut macros);
    }
    macros
}

fn collect_citation_macros(style: &Style) -> HashSet<String> {
    let mut macros = HashSet::new();
    collect_macro_refs_from_nodes(&style.citation.layout.children, &mut macros);
    macros
}

fn collect_macro_refs_from_nodes(nodes: &[CslNode], macros: &mut HashSet<String>) {
    for node in nodes {
        match node {
            CslNode::Text(t) => {
                if let Some(name) = &t.macro_name {
                    macros.insert(name.clone());
                }
            }
            CslNode::Group(g) => collect_macro_refs_from_nodes(&g.children, macros),
            CslNode::Choose(c) => {
                collect_macro_refs_from_nodes(&c.if_branch.children, macros);
                for branch in &c.else_if_branches {
                    collect_macro_refs_from_nodes(&branch.children, macros);
                }
                if let Some(else_branch) = &c.else_branch {
                    collect_macro_refs_from_nodes(else_branch, macros);
                }
            }
            CslNode::Names(n) => collect_macro_refs_from_nodes(&n.children, macros),
            _ => {}
        }
    }
}

fn extract_name_options_from_nodes(
    nodes: &[CslNode],
    style: &Style,
    target_macros: &HashSet<String>,
) -> Option<ContributorConfig> {
    for node in nodes {
        match node {
            CslNode::Names(n) => {
                if let Some(config) = extract_from_names(n) {
                    return Some(config);
                }
            }
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name {
                    if target_macros.contains(macro_name) {
                        if let Some(m) = style.macros.iter().find(|m| &m.name == macro_name) {
                            if let Some(config) =
                                extract_name_options_from_nodes(&m.children, style, target_macros)
                            {
                                return Some(config);
                            }
                        }
                    }
                }
            }
            CslNode::Group(g) => {
                if let Some(config) =
                    extract_name_options_from_nodes(&g.children, style, target_macros)
                {
                    return Some(config);
                }
            }
            CslNode::Choose(c) => {
                if let Some(config) =
                    extract_name_options_from_nodes(&c.if_branch.children, style, target_macros)
                {
                    return Some(config);
                }
                for branch in &c.else_if_branches {
                    if let Some(config) =
                        extract_name_options_from_nodes(&branch.children, style, target_macros)
                    {
                        return Some(config);
                    }
                }
                if let Some(else_branch) = &c.else_branch {
                    if let Some(config) =
                        extract_name_options_from_nodes(else_branch, style, target_macros)
                    {
                        return Some(config);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_from_names(names: &Names) -> Option<ContributorConfig> {
    let mut config = ContributorConfig::default();
    let mut has_config = false;

    if let Some(min) = names.et_al_min {
        let mut shorten = ShortenListOptions {
            min: min as u8,
            ..Default::default()
        };
        if let Some(use_first) = names.et_al_use_first {
            shorten.use_first = use_first as u8;
        }
        config.shorten = Some(shorten);
        has_config = true;
    }

    if let Some(delim) = &names.delimiter_precedes_et_al {
        config.delimiter_precedes_et_al = Some(match delim.as_str() {
            "always" => DelimiterPrecedesLast::Always,
            "never" => DelimiterPrecedesLast::Never,
            "contextual" => DelimiterPrecedesLast::Contextual,
            "after-inverted-name" => DelimiterPrecedesLast::AfterInvertedName,
            _ => DelimiterPrecedesLast::Contextual,
        });
        has_config = true;
    }

    // Scan children for <name> element options
    for child in &names.children {
        if let CslNode::Name(n) = child {
            if let Some(naso) = &n.name_as_sort_order {
                config.display_as_sort = Some(match naso.as_str() {
                    "all" => DisplayAsSort::All,
                    "first" => DisplayAsSort::First,
                    _ => DisplayAsSort::None,
                });
                has_config = true;
            }
            if let Some(delim) = &n.delimiter {
                config.delimiter = Some(delim.clone());
                has_config = true;
            }
            if let Some(sep) = &n.sort_separator {
                config.sort_separator = Some(sep.clone());
                has_config = true;
            }
            if let Some(init) = &n.initialize_with {
                config.initialize_with = Some(init.clone());
                has_config = true;
            }
            if let Some(init_hyphen) = n.initialize_with_hyphen {
                config.initialize_with_hyphen = Some(init_hyphen);
                has_config = true;
            }
            if let Some(dpl) = &n.delimiter_precedes_last {
                config.delimiter_precedes_last = Some(match dpl.as_str() {
                    "always" => DelimiterPrecedesLast::Always,
                    "never" => DelimiterPrecedesLast::Never,
                    "contextual" => DelimiterPrecedesLast::Contextual,
                    "after-inverted-name" => DelimiterPrecedesLast::AfterInvertedName,
                    _ => DelimiterPrecedesLast::Contextual,
                });
                has_config = true;
            }
            if let Some(dpea) = &n.delimiter_precedes_et_al {
                config.delimiter_precedes_et_al = Some(match dpea.as_str() {
                    "always" => DelimiterPrecedesLast::Always,
                    "never" => DelimiterPrecedesLast::Never,
                    "contextual" => DelimiterPrecedesLast::Contextual,
                    "after-inverted-name" => DelimiterPrecedesLast::AfterInvertedName,
                    _ => DelimiterPrecedesLast::Contextual,
                });
                has_config = true;
            }
        }
    }

    if has_config {
        Some(config)
    } else {
        None
    }
}

pub fn extract_substitute_pattern(style: &Style) -> Option<CslnSubstitute> {
    let bib_macros = collect_bibliography_macros(style);
    let cit_macros = collect_citation_macros(style);

    // Search bibliography first, then citation
    if let Some(bib) = &style.bibliography {
        if let Some(sub) = find_substitute_in_nodes(&bib.layout.children, style, &bib_macros) {
            return Some(sub);
        }
    }
    find_substitute_in_nodes(&style.citation.layout.children, style, &cit_macros)
}

fn find_substitute_in_nodes(
    nodes: &[CslNode],
    style: &Style,
    target_macros: &HashSet<String>,
) -> Option<CslnSubstitute> {
    for node in nodes {
        match node {
            CslNode::Names(n) => {
                for child in &n.children {
                    if let CslNode::Substitute(sub) = child {
                        // Check if parent <names> contains a label
                        let label_form = n.children.iter().find_map(|c| {
                            if let CslNode::Label(l) = c {
                                l.form.as_deref()
                            } else {
                                None
                            }
                        });
                        return Some(convert_substitute(sub, label_form));
                    }
                }
            }
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name {
                    if target_macros.contains(macro_name) {
                        if let Some(m) = style.macros.iter().find(|m| &m.name == macro_name) {
                            if let Some(sub) =
                                find_substitute_in_nodes(&m.children, style, target_macros)
                            {
                                return Some(sub);
                            }
                        }
                    }
                }
            }
            CslNode::Group(g) => {
                if let Some(sub) = find_substitute_in_nodes(&g.children, style, target_macros) {
                    return Some(sub);
                }
            }
            CslNode::Choose(c) => {
                if let Some(sub) =
                    find_substitute_in_nodes(&c.if_branch.children, style, target_macros)
                {
                    return Some(sub);
                }
                for branch in &c.else_if_branches {
                    if let Some(sub) =
                        find_substitute_in_nodes(&branch.children, style, target_macros)
                    {
                        return Some(sub);
                    }
                }
                if let Some(else_branch) = &c.else_branch {
                    if let Some(sub) = find_substitute_in_nodes(else_branch, style, target_macros) {
                        return Some(sub);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn convert_substitute(sub: &Substitute, label_form: Option<&str>) -> CslnSubstitute {
    let mut csln_sub = CslnSubstitute::default();
    if let Some(form) = label_form {
        csln_sub.contributor_role_form = Some(form.to_string());
    }

    let mut template = Vec::new();
    let mut overrides = HashMap::new();

    for node in &sub.children {
        match node {
            CslNode::Choose(c) => {
                if let Some(type_name) = &c.if_branch.type_ {
                    overrides.insert(
                        type_name.clone(),
                        extract_substitute_keys(&c.if_branch.children),
                    );
                }
                for branch in &c.else_if_branches {
                    if let Some(type_name) = &branch.type_ {
                        overrides
                            .insert(type_name.clone(), extract_substitute_keys(&branch.children));
                    }
                }
            }
            _ => {
                template.extend(extract_substitute_keys(std::slice::from_ref(node)));
            }
        }
    }

    csln_sub.template = template;
    csln_sub.overrides = overrides;
    csln_sub
}

fn extract_substitute_keys(nodes: &[CslNode]) -> Vec<SubstituteKey> {
    let mut keys = Vec::new();
    for node in nodes {
        match node {
            CslNode::Names(n) => {
                let vars = &n.variable;
                for var in vars.split(' ') {
                    match var {
                        "editor" => keys.push(SubstituteKey::Editor),
                        "translator" => keys.push(SubstituteKey::Translator),
                        _ => {}
                    }
                }
            }
            CslNode::Text(t) => {
                if t.variable.as_ref().is_some_and(|v| v == "title") {
                    keys.push(SubstituteKey::Title);
                }
            }
            CslNode::Group(g) => keys.extend(extract_substitute_keys(&g.children)),
            _ => {}
        }
    }
    keys
}
