use super::*;

#[test]
fn test_parse_csl_json() {
    let json = r#"{
        "id": "kuhn1962",
        "type": "book",
        "author": [{"family": "Kuhn", "given": "Thomas S."}],
        "title": "The Structure of Scientific Revolutions",
        "issued": {"date-parts": [[1962]]},
        "publisher": "University of Chicago Press",
        "publisher-place": "Chicago"
    }"#;

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_str(json).unwrap();
    let reference: InputReference = legacy.into();
    assert_eq!(reference.id().unwrap(), "kuhn1962");
    assert_eq!(reference.ref_type(), "book");

    if let Some(Contributor::ContributorList(list)) = reference.author() {
        if let Contributor::StructuredName(name) = &list.0[0] {
            assert_eq!(name.family, MultilingualString::Simple("Kuhn".to_string()));
        }
    }
}
