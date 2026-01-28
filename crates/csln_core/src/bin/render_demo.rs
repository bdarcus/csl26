use std::fs;
use std::collections::HashMap;
use csln_core::{CslnStyle, Renderer, CitationItem, ItemType, Variable};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load the Migrated Style
    let json = fs::read_to_string("csln.json")
        .expect("Please run 'cargo run --bin csln_migrate' first to generate csln.json");
    let style: CslnStyle = serde_json::from_str(&json)?;

    println!("Loaded Style: {}", style.info.title);

    // 2. Create a Mock Citation Item (Book)
    let mut variables = HashMap::new();
    variables.insert(Variable::Author, "Doe, John".to_string());
    variables.insert(Variable::Issued, "2020".to_string());
    variables.insert(Variable::Title, "The Rust Programming Language".to_string());
    variables.insert(Variable::Publisher, "No Starch Press".to_string());

    let item = CitationItem {
        item_type: ItemType::Book,
        variables,
    };

    // 3. Render Citation
    let renderer = Renderer;
    let cit_output = renderer.render_citation(&style.citation, &item);

    println!("\n=== RENDERED CITATION ===");
    println!("{}", cit_output);
    println!("=========================");

    // 4. Render Bibliography
    let bib_output = renderer.render_citation(&style.bibliography, &item);
    println!("\n=== RENDERED BIBLIOGRAPHY ===");
    println!("{}", bib_output);
    println!("=============================");

    Ok(())
}
