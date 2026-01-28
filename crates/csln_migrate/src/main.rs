use std::fs;
use roxmltree::Document;
use csl_legacy::parser::parse_style;
use csln_migrate::{MacroInliner, Upsampler, Compressor, OptionsExtractor, TemplateCompiler};
use csln_core::{CslnStyle, CslnInfo, CslnLocale, Style, options::Config, template::TemplateComponent};
use std::collections::HashMap;

/// Migrated style in new CSLN format
#[derive(Debug, serde::Serialize)]
struct MigratedStyle {
    info: StyleInfo,
    options: Config,
    citation: Vec<TemplateComponent>,
    bibliography: Vec<TemplateComponent>,
}

#[derive(Debug, serde::Serialize)]
struct StyleInfo {
    title: String,
    id: String,
}

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

    // 3. Compression (Pattern Recognition) - legacy path
    println!("Compressing logic (legacy)...");
    let compressor = Compressor;
    let csln_bib = compressor.compress_nodes(raw_bib.clone());
    let csln_cit = compressor.compress_nodes(raw_cit.clone());

    // 4. Template Compilation (new path) - use compressed nodes
    println!("Compiling to TemplateComponents...");
    let template_compiler = TemplateCompiler;
    let new_bib = template_compiler.compile(&csln_bib);
    let new_cit = template_compiler.compile(&csln_cit);
    println!("  Citation: {} components", new_cit.len());
    println!("  Bibliography: {} components", new_bib.len());

    // 5. Locale Upsampling
    let mut terms = HashMap::new();
    if let Some(loc) = legacy_style.locale.first() {
        for t in &loc.terms {
            let key = if let Some(form) = &t.form {
                format!("{}:{}", t.name, form)
            } else {
                t.name.clone()
            };
            terms.insert(key, t.value.clone());
        }
    }

    // Legacy output format
    let csln_style = CslnStyle {
        info: CslnInfo {
            title: legacy_style.info.title.clone(),
            id: legacy_style.info.id.clone(),
        },
        locale: CslnLocale { terms },
        citation: csln_cit,
        bibliography: csln_bib,
    };

    // 6. Output legacy format
    let json_path = "csln.json";
    let yaml_path = "csln.yaml";
    println!("Writing legacy format to {} and {}...", json_path, yaml_path);
    fs::write(json_path, serde_json::to_string_pretty(&csln_style)?)?;
    fs::write(yaml_path, serde_yaml::to_string(&csln_style)?)?;

    // 7. Output new CSLN format
    let migrated = MigratedStyle {
        info: StyleInfo {
            title: legacy_style.info.title.clone(),
            id: legacy_style.info.id.clone(),
        },
        options,
        citation: new_cit,
        bibliography: new_bib,
    };

    let new_yaml_path = "csln-new.yaml";
    println!("Writing new CSLN format to {}...", new_yaml_path);
    fs::write(new_yaml_path, serde_yaml::to_string(&migrated)?)?;

    println!("\n--- New CSLN Style (first 100 lines) ---");
    let yaml = serde_yaml::to_string(&migrated)?;
    for (i, line) in yaml.lines().take(100).enumerate() {
        println!("{:3}. {}", i + 1, line);
    }

    Ok(())
}
