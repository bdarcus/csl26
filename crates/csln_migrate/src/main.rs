use std::fs;
use roxmltree::Document;
use csl_legacy::parser::parse_style;
use csln_migrate::{MacroInliner, Upsampler, Compressor, OptionsExtractor, TemplateCompiler};
use csln_core::{Style, StyleInfo, CitationSpec, BibliographySpec, options::Config, template::TemplateComponent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("styles/apa.csl");
    
    eprintln!("Migrating {} to CSLN...", path);
    
    let text = fs::read_to_string(path)?;
    let doc = Document::parse(&text)?;
    let legacy_style = parse_style(doc.root_element())?;

    // 0. Extract global options (new CSLN Config)
    let options = OptionsExtractor::extract(&legacy_style);

    // 1. Deconstruction
    let inliner = MacroInliner::new(&legacy_style);
    let flattened_bib = inliner.inline_bibliography(&legacy_style).unwrap_or_default();
    let flattened_cit = inliner.inline_citation(&legacy_style);

    // 2. Semantic Upsampling
    let upsampler = Upsampler;
    let raw_bib = upsampler.upsample_nodes(&flattened_bib);
    let raw_cit = upsampler.upsample_nodes(&flattened_cit);

    // 3. Compression (Pattern Recognition)
    let compressor = Compressor;
    let csln_bib = compressor.compress_nodes(raw_bib.clone());
    let csln_cit = compressor.compress_nodes(raw_cit.clone());

    // 4. Template Compilation
    let template_compiler = TemplateCompiler;
    let new_bib = template_compiler.compile_bibliography(&csln_bib);
    let mut new_cit = template_compiler.compile_citation(&csln_cit);
    
    // For author-date styles, ensure citation has author + date
    if matches!(options.processing, Some(csln_core::options::Processing::AuthorDate)) {
        let has_author = new_cit.iter().any(|c| {
            matches!(c, TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Author)
        });
        if !has_author {
            new_cit.insert(0, TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                contributor: csln_core::template::ContributorRole::Author,
                form: csln_core::template::ContributorForm::Short,
                delimiter: None,
                rendering: csln_core::template::Rendering::default(),
            }));
        }
        let has_date = new_cit.iter().any(|c| {
            matches!(c, TemplateComponent::Date(td) if td.date == csln_core::template::DateVariable::Issued)
        });
        if !has_date {
            let insert_pos = new_cit.iter().position(|c| !matches!(c, TemplateComponent::Contributor(_))).unwrap_or(1);
            new_cit.insert(insert_pos, TemplateComponent::Date(csln_core::template::TemplateDate {
                date: csln_core::template::DateVariable::Issued,
                form: csln_core::template::DateForm::Year,
                rendering: csln_core::template::Rendering::default(),
            }));
        }
        // Keep only essential citation components for author-date
        new_cit.retain(|c| {
            matches!(c, 
                TemplateComponent::Contributor(tc) if tc.contributor == csln_core::template::ContributorRole::Author
            ) || matches!(c, 
                TemplateComponent::Date(td) if td.date == csln_core::template::DateVariable::Issued
            )
        });
    }

    // 5. Build Style in correct format for csln_processor
    let style = Style {
        info: StyleInfo {
            title: Some(legacy_style.info.title.clone()),
            id: Some(legacy_style.info.id.clone()),
            description: None,
        },
        templates: None,
        options: Some(options),
        citation: Some(CitationSpec {
            options: None,
            template: new_cit,
        }),
        bibliography: Some(BibliographySpec {
            options: None,
            template: new_bib,
        }),
    };

    // Output YAML to stdout
    let yaml = serde_yaml::to_string(&style)?;
    println!("{}", yaml);

    Ok(())
}
