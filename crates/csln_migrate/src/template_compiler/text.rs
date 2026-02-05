use csln_core::{
    template::{
        ContributorRole, DateVariable, NumberVariable, Rendering, SimpleVariable,
        TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber, TemplateTitle,
        TemplateVariable, TitleType,
    },
    Variable, VariableBlock,
};

pub fn compile_variable(var: &VariableBlock) -> Option<TemplateComponent> {
    let variable = &var.variable;
    let rendering = Rendering::default(); // TODO: Extract from FormattingOptions

    if let Some(role) = map_variable_to_role(variable) {
        return Some(TemplateComponent::Contributor(TemplateContributor {
            contributor: role,
            rendering,
            ..Default::default()
        }));
    }

    if let Some(date) = map_variable_to_date(variable) {
        return Some(TemplateComponent::Date(TemplateDate {
            date,
            rendering,
            ..Default::default()
        }));
    }

    if let Some(title) = map_variable_to_title(variable) {
        return Some(TemplateComponent::Title(TemplateTitle {
            title,
            rendering,
            ..Default::default()
        }));
    }

    if let Some(number) = map_variable_to_number(variable) {
        return Some(TemplateComponent::Number(TemplateNumber {
            number,
            rendering,
            ..Default::default()
        }));
    }

    if let Some(simple) = map_variable_to_simple(variable) {
        return Some(TemplateComponent::Variable(TemplateVariable {
            variable: simple,
            rendering,
            ..Default::default()
        }));
    }

    None
}

fn map_variable_to_role(var: &Variable) -> Option<ContributorRole> {
    match var {
        Variable::Author => Some(ContributorRole::Author),
        Variable::Editor => Some(ContributorRole::Editor),
        Variable::Translator => Some(ContributorRole::Translator),
        Variable::ContainerAuthor => Some(ContributorRole::ContainerAuthor),
        Variable::CollectionEditor => Some(ContributorRole::Editor), // Map to editor
        Variable::EditorialDirector => Some(ContributorRole::Editor), // Map to editor
        Variable::Interviewer => Some(ContributorRole::Interviewer),
        Variable::Illustrator => Some(ContributorRole::Illustrator),
        Variable::Director => Some(ContributorRole::Director),
        _ => None,
    }
}

fn map_variable_to_date(var: &Variable) -> Option<DateVariable> {
    match var {
        Variable::Issued => Some(DateVariable::Issued),
        Variable::Accessed => Some(DateVariable::Accessed),
        Variable::OriginalDate => None, // TODO: Add OriginalDate to DateVariable
        Variable::EventDate => Some(DateVariable::EventDate),
        Variable::Submitted => Some(DateVariable::Submitted),
        _ => None,
    }
}

fn map_variable_to_title(var: &Variable) -> Option<TitleType> {
    match var {
        Variable::Title => Some(TitleType::Primary),
        Variable::ContainerTitle => Some(TitleType::ParentSerial), // Default to Serial, adjust later
        Variable::CollectionTitle => Some(TitleType::ParentMonograph),
        Variable::OriginalTitle => None, // TODO: Add Original to TitleType
        _ => None,
    }
}

fn map_variable_to_number(var: &Variable) -> Option<NumberVariable> {
    match var {
        Variable::Volume => Some(NumberVariable::Volume),
        Variable::Issue => Some(NumberVariable::Issue),
        Variable::Page => Some(NumberVariable::Pages),
        Variable::NumberOfPages => Some(NumberVariable::NumberOfPages),
        Variable::Edition => Some(NumberVariable::Edition),
        Variable::CitationNumber => Some(NumberVariable::CitationNumber),
        Variable::ChapterNumber => Some(NumberVariable::ChapterNumber),
        Variable::CollectionNumber => Some(NumberVariable::CollectionNumber),
        _ => None,
    }
}

fn map_variable_to_simple(var: &Variable) -> Option<SimpleVariable> {
    match var {
        Variable::DOI => Some(SimpleVariable::Doi),
        Variable::URL => Some(SimpleVariable::Url),
        Variable::ISBN => Some(SimpleVariable::Isbn),
        Variable::ISSN => Some(SimpleVariable::Issn),
        Variable::PMID => Some(SimpleVariable::Pmid),
        Variable::PMCID => Some(SimpleVariable::Pmid),
        Variable::Publisher => Some(SimpleVariable::Publisher),
        Variable::PublisherPlace => Some(SimpleVariable::PublisherPlace),
        Variable::Archive => Some(SimpleVariable::Archive),
        Variable::ArchiveLocation => Some(SimpleVariable::ArchiveLocation),
        Variable::Abstract => Some(SimpleVariable::Abstract),
        Variable::Genre => Some(SimpleVariable::Genre),
        Variable::Note => Some(SimpleVariable::Note),
        Variable::CallNumber => None, // TODO: Add CallNumber to SimpleVariable
        Variable::Medium => Some(SimpleVariable::Medium),
        Variable::Status => Some(SimpleVariable::Status),
        Variable::Version => Some(SimpleVariable::Version),
        Variable::Keyword => Some(SimpleVariable::Keyword),
        _ => None,
    }
}
