use csln_core::reference::{
    CollectionComponent, EdtfString, MonographComponentType, Parent, SerialComponent,
    SerialComponentType, Title,
};

#[test]
fn test_serial_component_with_parent_id() {
    let parent_id = "journal-1".to_string();
    let component = SerialComponent {
        id: Some("article-1".to_string()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Single("My Article".to_string())),
        author: None,
        translator: None,
        issued: EdtfString("2023".to_string()),
        parent: Parent::Id(parent_id.clone()),
        url: None,
        accessed: None,
        note: None,
        doi: None,
        pages: None,
        volume: None,
        issue: None,
    };

    match component.parent {
        Parent::Id(id) => assert_eq!(id, parent_id),
        Parent::Embedded(_) => panic!("Expected Parent::Id"),
    }
}

#[test]
fn test_collection_component_with_parent_id() {
    let parent_id = "book-1".to_string();
    let component = CollectionComponent {
        id: Some("chapter-1".to_string()),
        r#type: MonographComponentType::Chapter,
        title: Some(Title::Single("My Chapter".to_string())),
        author: None,
        translator: None,
        issued: EdtfString("2023".to_string()),
        parent: Parent::Id(parent_id.clone()),
        pages: None,
        url: None,
        accessed: None,
        note: None,
        doi: None,
    };

    match component.parent {
        Parent::Id(id) => assert_eq!(id, parent_id),
        Parent::Embedded(_) => panic!("Expected Parent::Id"),
    }
}
