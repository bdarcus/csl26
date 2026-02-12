use csln_core::reference::{InputReference, Monograph};

#[test]
fn test_monograph_doi_alias() {
    let json = r#"{
        "type": "book",
        "title": "Test Book",
        "issued": "2023",
        "DOI": "10.1001/test"
    }"#;
    let monograph: Monograph = serde_json::from_str(json).unwrap();
    assert_eq!(monograph.doi, Some("10.1001/test".to_string()));
}

#[test]
fn test_input_reference_doi_alias() {
    let json = r#"{
        "type": "book",
        "title": "Test Book",
        "issued": "2023",
        "DOI": "10.1001/test"
    }"#;
    let reference: InputReference = serde_json::from_str(json).unwrap();
    if let InputReference::Monograph(m) = reference {
        assert_eq!(m.doi, Some("10.1001/test".to_string()));
    } else {
        panic!("Expected Monograph");
    }
}

#[test]
fn test_input_reference_url_alias() {
    let json = r#"{
        "type": "book",
        "title": "Test Book",
        "issued": "2023",
        "URL": "https://example.com"
    }"#;
    let reference: InputReference = serde_json::from_str(json).unwrap();
    if let InputReference::Monograph(m) = reference {
        assert_eq!(m.url.unwrap().to_string(), "https://example.com/");
    } else {
        panic!("Expected Monograph");
    }
}
