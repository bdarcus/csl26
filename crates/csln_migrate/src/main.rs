use std::fs;
use roxmltree::Document;
use serde_json::to_string_pretty;
use csl_legacy::parser::parse_style;
use csln_migrate::{MacroInliner, Upsampler};
use csln_core::{CslnStyle, CslnInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "styles/chicago-author-date.csl";
    println!("Migrating {} to CSLN...", path);
    
    let text = fs::read_to_string(path)?;
    let doc = Document::parse(&text)?;
    let legacy_style = parse_style(doc.root_element())?;

    // 1. Deconstruction
    let inliner = MacroInliner::new(&legacy_style);
    let flattened_bib = inliner.inline_bibliography(&legacy_style).unwrap_or_default();
    let flattened_cit = inliner.inline_citation(&legacy_style);

    // 2. Semantic Upsampling
    let upsampler = Upsampler;
    let csln_bib = upsampler.upsample_nodes(&flattened_bib);
    let csln_cit = upsampler.upsample_nodes(&flattened_cit);

    let csln_style = CslnStyle {
        info: CslnInfo {
            title: legacy_style.info.title.clone(),
            id: legacy_style.info.id.clone(),
        },
        citation: csln_cit,
        bibliography: csln_bib,
    };

    // 3. Output
    let output_path = "csln.json";
    println!("Migration complete. Writing to {}...", output_path);
    fs::write(output_path, to_string_pretty(&csln_style)?)?;

    Ok(())
}
