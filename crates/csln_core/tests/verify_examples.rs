use std::fs;
use std::path::PathBuf;

#[test]
fn test_verify_comprehensive_examples() {
    // Locate the examples directory relative to this test file
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../examples/comprehensive.yaml");

    let content = fs::read_to_string(&path).expect("Failed to read comprehensive.yaml");

    // Attempt to deserialize into the expected InputBibliography structure
    let bib: Result<csln_core::InputBibliography, _> = serde_yaml::from_str(&content);

    match bib {
        Ok(bib) => {
            let refs = bib.references;
            println!("Successfully parsed {} references", refs.len());
            for reference in &refs {
                let id = reference.id().expect("Reference should have an ID");
                println!("Parsed: {}", id);

                // Verify specific fields for Foucault example
                if id == "foucault_discipline" {
                    let keywords = reference.keywords().expect("Should have keywords");
                    assert!(keywords.contains(&"humanities".to_string()));
                    assert!(keywords.contains(&"translation".to_string()));

                    let orig_date = reference
                        .original_date()
                        .expect("Should have original date");
                    assert_eq!(orig_date.0, "1975");
                }
            }
            println!("Successfully verified {} references", refs.len());
        }
        Err(e) => {
            panic!("Failed to parse comprehensive.yaml: {}", e);
        }
    }
}
