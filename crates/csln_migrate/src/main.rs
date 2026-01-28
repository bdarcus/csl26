use std::fs;
use roxmltree::Document;
use csl_legacy::parser::parse_style;
use csln_migrate::{MacroInliner, Upsampler, Compressor, OptionsExtractor};
use csln_core::{CslnStyle, CslnInfo, CslnLocale};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "styles/apa.csl";
    println!("Migrating {} to CSLN...", path);
    
    let text = fs::read_to_string(path)?;
    let doc = Document::parse(&text)?;
    let legacy_style = parse_style(doc.root_element())?;

    // 0. Extract global options (new CSLN Config)
    println!("Extracting style options...");
    let options = OptionsExtractor::extract(&legacy_style);
    println!("  Processing: {:?}", options.processing);
    if let Some(ref contrib) = options.contributors {
        println!("  Contributors: {:?}", contrib);
    }
    if let Some(ref sub) = options.substitute {
        println!("  Substitute: {:?}", sub);
    }

    // 1. Deconstruction
    let inliner = MacroInliner::new(&legacy_style);
    let flattened_bib = inliner.inline_bibliography(&legacy_style).unwrap_or_default();
    let flattened_cit = inliner.inline_citation(&legacy_style);

    // 2. Semantic Upsampling
    let upsampler = Upsampler;
    let raw_bib = upsampler.upsample_nodes(&flattened_bib);
    let raw_cit = upsampler.upsample_nodes(&flattened_cit);

    // 3. Compression (Pattern Recognition)
    println!("Compressing logic...");
    let compressor = Compressor;
    let csln_bib = compressor.compress_nodes(raw_bib);
    let csln_cit = compressor.compress_nodes(raw_cit);

    // 2.5 Locale Upsampling
    let mut terms = HashMap::new();
    // Assuming first locale is the main one for now
    if let Some(loc) = legacy_style.locale.first() {
        for t in &loc.terms {
            // Key: "name" or "name:form"
            let key = if let Some(form) = &t.form {
                format!("{}:{}", t.name, form)
            } else {
                t.name.clone()
            };
            terms.insert(key, t.value.clone());
        }
    }

    let csln_style = CslnStyle {
        info: CslnInfo {
            title: legacy_style.info.title.clone(),
            id: legacy_style.info.id.clone(),
        },
        locale: CslnLocale { terms },
        citation: csln_cit,
        bibliography: csln_bib,
    };

    // 4. Output legacy format
    let json_path = "csln.json";
    let yaml_path = "csln.yaml";
    println!("Migration complete. Writing to {} and {}...", json_path, yaml_path);
    fs::write(json_path, serde_json::to_string_pretty(&csln_style)?)?;
    fs::write(yaml_path, serde_yaml::to_string(&csln_style)?)?;

    // 5. Output new options format separately for now
    let options_yaml = serde_yaml::to_string(&options)?;
    println!("\n--- Extracted Options (CSLN format) ---\n{}", options_yaml);

    Ok(())
}
