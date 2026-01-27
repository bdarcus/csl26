use csl_legacy::model::{self as legacy, CslNode as LNode};
use csln_core::{self as csln, Variable, ItemType, FormattingOptions};
use std::collections::HashMap;

pub struct Upsampler;

impl Upsampler {
    /// The entry point for converting a flattened legacy tree into CSLN nodes.
    pub fn upsample_nodes(&self, legacy_nodes: &[LNode]) -> Vec<csln::CslnNode> {
        let mut csln_nodes = Vec::new();
        let mut i = 0;

        while i < legacy_nodes.len() {
            let node = &legacy_nodes[i];

            // HEURISTIC 1: Label + Variable Grouping
            // If we see a Group containing exactly a Label and a Text/Number variable of the same type, collapse them.
            if let LNode::Group(group) = node {
                if let Some(collapsed) = self.try_collapse_label_variable(group) {
                    csln_nodes.push(collapsed);
                    i += 1;
                    continue;
                }
            }

            // HEURISTIC 2: Simple Variable Mapping
            if let Some(mapped) = self.map_node(node) {
                csln_nodes.push(mapped);
            }

            i += 1;
        }

        csln_nodes
    }

    fn map_node(&self, node: &LNode) -> Option<csln::CslnNode> {
        match node {
            LNode::Text(t) => {
                if let Some(var_str) = &t.variable {
                    if let Some(var) = self.map_variable(var_str) {
                        return Some(csln::CslnNode::Variable(csln::VariableBlock {
                            variable: var,
                            label: None,
                            formatting: self.map_formatting(&t.formatting, &t.prefix, &t.suffix, t.quotes),
                            overrides: HashMap::new(),
                        }));
                    }
                }
                if let Some(val) = &t.value {
                    return Some(csln::CslnNode::Text { value: val.clone() });
                }
                None
            }
            LNode::Group(g) => {
                Some(csln::CslnNode::Group(csln::GroupBlock {
                    children: self.upsample_nodes(&g.children),
                    delimiter: g.delimiter.clone(),
                    formatting: self.map_formatting(&g.formatting, &g.prefix, &g.suffix, None),
                }))
            }
            LNode::Date(d) => self.map_date(d),
            LNode::Choose(c) => self.map_choose(c),
            _ => None, // Expand as we add more types
        }
    }

    fn map_choose(&self, c: &legacy::Choose) -> Option<csln::CslnNode> {
        // For now, we just map the structure recursively.
        // We aren't doing intelligent condition mapping yet (that's complex),
        // but we MUST recurse to find the dates inside.
        
        let mut if_item_type = Vec::new();
        // Naive extraction of types from the if-branch for now
        if let Some(types) = &c.if_branch.type_ {
            for t in types.split_whitespace() {
                 if let Some(it) = self.map_item_type(t) {
                     if_item_type.push(it);
                 }
            }
        }

        Some(csln::CslnNode::Condition(csln::ConditionBlock {
            if_item_type,
            then_branch: self.upsample_nodes(&c.if_branch.children),
            else_branch: if let Some(else_children) = &c.else_branch {
                Some(self.upsample_nodes(else_children))
            } else if !c.else_if_branches.is_empty() {
                 // Flatten else-if into nested else for now, or just take the first one
                 // This is lossy but lets us proceed with finding the dates.
                 Some(self.upsample_nodes(&c.else_if_branches[0].children))
            } else {
                None
            },
        }))
    }

    fn map_item_type(&self, s: &str) -> Option<ItemType> {
        match s {
            "article" => Some(ItemType::Article),
            "article-journal" => Some(ItemType::ArticleJournal),
            "article-magazine" => Some(ItemType::ArticleMagazine),
            "article-newspaper" => Some(ItemType::ArticleNewspaper),
            "bill" => Some(ItemType::Bill),
            "book" => Some(ItemType::Book),
            "broadcast" => Some(ItemType::Broadcast),
            "chapter" => Some(ItemType::Chapter),
            "dataset" => Some(ItemType::Dataset),
            "entry" => Some(ItemType::Entry),
            "entry-dictionary" => Some(ItemType::EntryDictionary),
            "entry-encyclopedia" => Some(ItemType::EntryEncyclopedia),
            "figure" => Some(ItemType::Figure),
            "graphic" => Some(ItemType::Graphic),
            "interview" => Some(ItemType::Interview),
            "legal_case" => Some(ItemType::LegalCase),
            "legislation" => Some(ItemType::Legislation),
            "manuscript" => Some(ItemType::Manuscript),
            "map" => Some(ItemType::Map),
            "motion_picture" => Some(ItemType::MotionPicture),
            "musical_score" => Some(ItemType::MusicalScore),
            "pamphlet" => Some(ItemType::Pamphlet),
            "paper-conference" => Some(ItemType::PaperConference),
            "patent" => Some(ItemType::Patent),
            "personal_communication" => Some(ItemType::PersonalCommunication),
            "post" => Some(ItemType::Post),
            "post-weblog" => Some(ItemType::PostWeblog),
            "report" => Some(ItemType::Report),
            "review" => Some(ItemType::Review),
            "review-book" => Some(ItemType::ReviewBook),
            "song" => Some(ItemType::Song),
            "speech" => Some(ItemType::Speech),
            "thesis" => Some(ItemType::Thesis),
            "treaty" => Some(ItemType::Treaty),
            "webpage" => Some(ItemType::Webpage),
            _ => None,
        }
    }

    fn map_date(&self, d: &legacy::Date) -> Option<csln::CslnNode> {
        let variable = self.map_variable(&d.variable)?;
        
        // Infer configuration from date-parts
        let mut year_form = None;
        let mut month_form = None;
        let mut day_form = None;

        for part in &d.parts {
            match part.name.as_str() {
                "year" => year_form = self.map_date_part_form(&part.form),
                "month" => month_form = self.map_date_part_form(&part.form),
                "day" => day_form = self.map_date_part_form(&part.form),
                _ => {}
            }
        }

        Some(csln::CslnNode::Date(csln::DateBlock {
            variable,
            options: csln::DateOptions {
                form: match d.form.as_deref() {
                    Some("text") => Some(csln::DateForm::Text),
                    Some("numeric") => Some(csln::DateForm::Numeric),
                    _ => None,
                },
                parts: match d.date_parts.as_deref() {
                    Some("year") => Some(csln::DateParts::Year),
                    Some("year-month") => Some(csln::DateParts::YearMonth),
                    _ => None, // Default is usually full date
                },
                delimiter: d.delimiter.clone(),
                year_form,
                month_form,
                day_form,
            },
            formatting: self.map_formatting(&d.formatting, &d.prefix, &d.suffix, None),
        }))
    }

    fn map_date_part_form(&self, form: &Option<String>) -> Option<csln::DatePartForm> {
        match form.as_deref() {
            Some("numeric") => Some(csln::DatePartForm::Numeric),
            Some("numeric-leading-zeros") => Some(csln::DatePartForm::NumericLeadingZeros),
            Some("ordinal") => Some(csln::DatePartForm::Ordinal),
            Some("long") => Some(csln::DatePartForm::Long),
            Some("short") => Some(csln::DatePartForm::Short),
            _ => None,
        }
    }

    fn try_collapse_label_variable(&self, group: &legacy::Group) -> Option<csln::CslnNode> {
        if group.children.len() == 2 {
            let first = &group.children[0];
            let second = &group.children[1];

            if let (LNode::Label(l), LNode::Text(t)) = (first, second) {
                if let (Some(l_var), Some(t_var)) = (&l.variable, &t.variable) {
                    if l_var == t_var {
                        if let Some(var) = self.map_variable(t_var) {
                            return Some(csln::CslnNode::Variable(csln::VariableBlock {
                                variable: var,
                                label: Some(csln::LabelOptions {
                                    form: self.map_label_form(&l.form),
                                    pluralize: true, // Upsampled assumption
                                    formatting: self.map_formatting(&l.formatting, &l.prefix, &l.suffix, None),
                                }),
                                formatting: self.map_formatting(&t.formatting, &t.prefix, &t.suffix, t.quotes),
                                overrides: HashMap::new(),
                            }));
                        }
                    }
                }
            }
        }
        None
    }

    fn map_variable(&self, s: &str) -> Option<Variable> {
        match s {
            "title" => Some(Variable::Title),
            "container-title" => Some(Variable::ContainerTitle),
            "collection-title" => Some(Variable::CollectionTitle),
            "original-title" => Some(Variable::OriginalTitle),
            "publisher" => Some(Variable::Publisher),
            "publisher-place" => Some(Variable::PublisherPlace),
            "archive" => Some(Variable::Archive),
            "archive-place" => Some(Variable::ArchivePlace),
            "archive_location" => Some(Variable::ArchiveLocation),
            "event" => Some(Variable::Event),
            "event-place" => Some(Variable::EventPlace),
            "page" => Some(Variable::Page),
            "locator" => Some(Variable::Locator),
            "version" => Some(Variable::Version),
            "volume" => Some(Variable::Volume),
            "number-of-volumes" => Some(Variable::NumberOfVolumes),
            "issue" => Some(Variable::Issue),
            "chapter-number" => Some(Variable::ChapterNumber),
            "medium" => Some(Variable::Medium),
            "status" => Some(Variable::Status),
            "edition" => Some(Variable::Edition),
            "section" => Some(Variable::Section),
            "source" => Some(Variable::Source),
            "genre" => Some(Variable::Genre),
            "note" => Some(Variable::Note),
            "annote" => Some(Variable::Annote),
            "abstract" => Some(Variable::Abstract),
            "keyword" => Some(Variable::Keyword),
            "number" => Some(Variable::Number),
            "URL" => Some(Variable::URL),
            "DOI" => Some(Variable::DOI),
            "ISBN" => Some(Variable::ISBN),
            "ISSN" => Some(Variable::ISSN),
            "PMID" => Some(Variable::PMID),
            "PMCID" => Some(Variable::PMCID),
            "call-number" => Some(Variable::CallNumber),
            "dimensions" => Some(Variable::Dimensions),
            "scale" => Some(Variable::Scale),
            "jurisdiction" => Some(Variable::Jurisdiction),
            "citation-label" => Some(Variable::CitationLabel),
            "citation-number" => Some(Variable::CitationNumber),
            "year-suffix" => Some(Variable::YearSuffix),
            // Names
            "author" => Some(Variable::Author),
            "editor" => Some(Variable::Editor),
            "editorial-director" => Some(Variable::EditorialDirector),
            "translator" => Some(Variable::Translator),
            "illustrator" => Some(Variable::Illustrator),
            "original-author" => Some(Variable::OriginalAuthor),
            "container-author" => Some(Variable::ContainerAuthor),
            "collection-editor" => Some(Variable::CollectionEditor),
            "composer" => Some(Variable::Composer),
            "director" => Some(Variable::Director),
            "interviewer" => Some(Variable::Interviewer),
            "recipient" => Some(Variable::Recipient),
            "reviewed-author" => Some(Variable::ReviewedAuthor),
            // Dates
            "issued" => Some(Variable::Issued),
            "event-date" => Some(Variable::EventDate),
            "accessed" => Some(Variable::Accessed),
            "container" => Some(Variable::Submitted), // Approximate mapping for now
            "original-date" => Some(Variable::OriginalDate),
            "available-date" => Some(Variable::AvailableDate),
            _ => None,
        }
    }

    fn map_label_form(&self, form: &Option<String>) -> csln::LabelForm {
        match form.as_deref() {
            Some("short") => csln::LabelForm::Short,
            Some("symbol") => csln::LabelForm::Symbol,
            _ => csln::LabelForm::Long,
        }
    }

    fn map_formatting(&self, f: &legacy::Formatting, prefix: &Option<String>, suffix: &Option<String>, quotes: Option<bool>) -> FormattingOptions {
        FormattingOptions {
            font_style: f.font_style.as_ref().and_then(|s| match s.as_str() {
                "italic" => Some(csln::FontStyle::Italic),
                _ => None,
            }),
            font_weight: f.font_weight.as_ref().and_then(|s| match s.as_str() {
                "bold" => Some(csln::FontWeight::Bold),
                _ => None,
            }),
            font_variant: f.font_variant.as_ref().and_then(|s| match s.as_str() {
                "small-caps" => Some(csln::FontVariant::SmallCaps),
                _ => None,
            }),
            quotes,
            prefix: prefix.clone(),
            suffix: suffix.clone(),
        }
    }
}
