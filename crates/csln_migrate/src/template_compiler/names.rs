use csln_core::{
    template::{
        ContributorForm, ContributorRole, Rendering, TemplateComponent, TemplateContributor,
    },
    NamesBlock, Variable,
};

pub fn compile_names(names: &NamesBlock) -> Option<TemplateComponent> {
    let variable = &names.variable;
    let role = map_variable_to_role(variable)?;
    let rendering = Rendering::default();

    Some(TemplateComponent::Contributor(TemplateContributor {
        contributor: role,
        form: ContributorForm::Long, // Default to Long, will be refined
        rendering,
        ..Default::default()
    }))
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
