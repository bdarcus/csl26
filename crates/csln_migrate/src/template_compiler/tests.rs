use super::*;
use csln_core::{
    template::{ContributorRole, DateVariable as TemplateDateVar, TemplateComponent},
    DateBlock, NamesBlock, Variable,
};

#[test]
fn test_compile_names_to_contributor() {
    let compiler = TemplateCompiler;
    let names = NamesBlock {
        variable: Variable::Author,
        options: Default::default(),
        formatting: Default::default(),
    };

    let component = compiler.compile_names(&names).unwrap();
    if let TemplateComponent::Contributor(c) = component {
        assert_eq!(c.contributor, ContributorRole::Author);
    } else {
        panic!("Expected Contributor");
    }
}

#[test]
fn test_compile_date() {
    let compiler = TemplateCompiler;
    let date = DateBlock {
        variable: Variable::Issued,
        options: Default::default(),
        formatting: Default::default(),
    };

    let component = compiler.compile_date(&date).unwrap();
    if let TemplateComponent::Date(d) = component {
        assert_eq!(d.date, TemplateDateVar::Issued);
    } else {
        panic!("Expected Date");
    }
}
