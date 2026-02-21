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

    // 2. Scan bibliography and citation scopes independently.
    // Keep bibliography-driven shortening as the global default when both are
    // present; citation-specific shortening is emitted as scoped overrides.
    if let Some(bib_opts) = extract_bibliography_contributor_overrides(style) {
        merge_contributor_config(&mut config, bib_opts, true);
        has_config = true;
    }

    if let Some(cit_opts) = extract_citation_contributor_overrides(style) {
        // Citation options should not clobber an already-extracted bibliography
        // shorten config at global scope.
        let allow_shorten_override = config.shorten.is_none();
        merge_contributor_config_with_shorten_policy(
            &mut config,
            cit_opts,
            false,
            allow_shorten_override,
        );
        has_config = true;
    }

    if has_config { Some(config) } else { None }
}

pub fn extract_citation_contributor_overrides(style: &Style) -> Option<ContributorConfig> {
    let cit_macros = collect_citation_macros(style);
    extract_scope_contributor_overrides(
        &style.citation.layout.children,
        style,
        &cit_macros,
        style.citation.et_al_min,
        style.citation.et_al_use_first,
    )
}

pub fn extract_bibliography_contributor_overrides(style: &Style) -> Option<ContributorConfig> {
    let bib = style.bibliography.as_ref()?;
    let bib_macros = collect_bibliography_macros(style);
    extract_scope_contributor_overrides(
        &bib.layout.children,
        style,
        &bib_macros,
        bib.et_al_min,
        bib.et_al_use_first,
    )
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
                if let Some(macro_name) = &t.macro_name
                    && target_macros.contains(macro_name)
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && let Some(config) =
                        extract_name_options_from_nodes(&m.children, style, target_macros)
                {
                    return Some(config);
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
                if let Some(else_branch) = &c.else_branch
                    && let Some(config) =
                        extract_name_options_from_nodes(else_branch, style, target_macros)
                {
                    return Some(config);
                }
            }
            _ => {}
        }
    }
    None
}

fn extract_scope_contributor_overrides(
    nodes: &[CslNode],
    style: &Style,
    target_macros: &HashSet<String>,
    et_al_min: Option<usize>,
    et_al_use_first: Option<usize>,
) -> Option<ContributorConfig> {
    let mut config =
        extract_name_options_from_nodes(nodes, style, target_macros).unwrap_or_default();
    let mut has_config = config != ContributorConfig::default();

    if apply_et_al_attributes(&mut config, et_al_min, et_al_use_first) {
        has_config = true;
    }

    if has_config { Some(config) } else { None }
}

fn apply_et_al_attributes(
    config: &mut ContributorConfig,
    et_al_min: Option<usize>,
    et_al_use_first: Option<usize>,
) -> bool {
    let Some(min_value) = et_al_min else {
        return false;
    };

    let shorten = config
        .shorten
        .get_or_insert_with(ShortenListOptions::default);
    shorten.min = usize_to_u8(min_value);
    if let Some(use_first) = et_al_use_first {
        shorten.use_first = usize_to_u8(use_first);
    }
    true
}

fn usize_to_u8(value: usize) -> u8 {
    value.min(u8::MAX as usize) as u8
}

fn merge_contributor_config(
    base: &mut ContributorConfig,
    incoming: ContributorConfig,
    overwrite_existing: bool,
) {
    merge_contributor_config_with_shorten_policy(
        base,
        incoming,
        overwrite_existing,
        overwrite_existing,
    );
}

fn merge_contributor_config_with_shorten_policy(
    base: &mut ContributorConfig,
    incoming: ContributorConfig,
    overwrite_existing: bool,
    overwrite_shorten: bool,
) {
    if incoming.shorten.is_some()
        && (overwrite_existing || (overwrite_shorten && base.shorten.is_none()))
    {
        base.shorten = incoming.shorten;
    }
    if incoming.display_as_sort.is_some() && (overwrite_existing || base.display_as_sort.is_none())
    {
        base.display_as_sort = incoming.display_as_sort;
    }
    if incoming.delimiter.is_some() && (overwrite_existing || base.delimiter.is_none()) {
        base.delimiter = incoming.delimiter;
    }
    if incoming.sort_separator.is_some() && (overwrite_existing || base.sort_separator.is_none()) {
        base.sort_separator = incoming.sort_separator;
    }
    if incoming.initialize_with.is_some() && (overwrite_existing || base.initialize_with.is_none())
    {
        base.initialize_with = incoming.initialize_with;
    }
    if incoming.initialize_with_hyphen.is_some()
        && (overwrite_existing || base.initialize_with_hyphen.is_none())
    {
        base.initialize_with_hyphen = incoming.initialize_with_hyphen;
    }
    if incoming.delimiter_precedes_last.is_some()
        && (overwrite_existing || base.delimiter_precedes_last.is_none())
    {
        base.delimiter_precedes_last = incoming.delimiter_precedes_last;
    }
    if incoming.delimiter_precedes_et_al.is_some()
        && (overwrite_existing || base.delimiter_precedes_et_al.is_none())
    {
        base.delimiter_precedes_et_al = incoming.delimiter_precedes_et_al;
    }
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

    if has_config { Some(config) } else { None }
}

pub fn extract_substitute_pattern(style: &Style) -> Option<CslnSubstitute> {
    let bib_macros = collect_bibliography_macros(style);
    let cit_macros = collect_citation_macros(style);

    // Search bibliography first, then citation
    if let Some(bib) = &style.bibliography
        && let Some(sub) = find_substitute_in_nodes(&bib.layout.children, style, &bib_macros)
    {
        return Some(sub);
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
                if let Some(macro_name) = &t.macro_name
                    && target_macros.contains(macro_name)
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && let Some(sub) = find_substitute_in_nodes(&m.children, style, target_macros)
                {
                    return Some(sub);
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
                if let Some(else_branch) = &c.else_branch
                    && let Some(sub) = find_substitute_in_nodes(else_branch, style, target_macros)
                {
                    return Some(sub);
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
