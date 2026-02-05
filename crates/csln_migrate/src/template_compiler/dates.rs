use csln_core::{
    template::{DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate},
    DateBlock, Variable,
};

pub fn compile_date(date: &DateBlock) -> Option<TemplateComponent> {
    let variable = &date.variable;
    let var_type = map_variable_to_date(variable)?;
    let rendering = Rendering::default();

    Some(TemplateComponent::Date(TemplateDate {
        date: var_type,
        form: DateForm::Year, // Default
        rendering,
        ..Default::default()
    }))
}

fn map_variable_to_date(var: &Variable) -> Option<DateVariable> {
    match var {
        Variable::Issued => Some(DateVariable::Issued),
        Variable::Accessed => Some(DateVariable::Accessed),
        Variable::OriginalDate => None, // TODO: Add OriginalDate to Template DateVariable
        Variable::EventDate => Some(DateVariable::EventDate),
        Variable::Submitted => Some(DateVariable::Submitted),
        _ => None,
    }
}
