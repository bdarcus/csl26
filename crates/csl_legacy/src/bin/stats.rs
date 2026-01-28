use std::fs;
use roxmltree::Document;
use csl_legacy::parser::parse_style;

fn main() {
    let styles_dir = "styles";
    let entries = match fs::read_dir(styles_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading styles directory: {}", e);
            return;
        }
    };

    let mut total = 0;
    let mut success = 0;
    let mut errors = 0;
    let mut error_types = std::collections::HashMap::new();

    println!("Starting analysis of styles in {}...", styles_dir);

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("csl") {
                total += 1;
                
                // Read file
                let text = match fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(_) => {
                        *error_types.entry("File read error".to_string()).or_insert(0) += 1;
                        errors += 1;
                        continue;
                    }
                };

                // Parse XML
                let doc = match Document::parse(&text) {
                    Ok(d) => d,
                    Err(e) => {
                        *error_types.entry(format!("XML Parse Error: {}", e)).or_insert(0) += 1;
                        errors += 1;
                        continue;
                    }
                };

                // Parse CSL
                match parse_style(doc.root_element()) {
                    Ok(_) => success += 1,
                    Err(e) => {
                        // Simplify error message to group them
                        let simple_err = if e.contains("missing") {
                            e.split_whitespace().collect::<Vec<_>>().join(" ")
                        } else {
                            e.clone()
                        };
                        *error_types.entry(simple_err).or_insert(0) += 1;
                        errors += 1;
                        // Optional: Print first few failures
                        // if errors <= 5 {
                        //     println!("Failed: {:?} -> {}", path.file_name().unwrap(), e);
                        // }
                    }
                }
            }
        }
    }

    println!("\n=== ANALYSIS COMPLETE ===");
    println!("Total Styles: {}", total);
    println!("Success:      {} ({:.1}%)", success, (success as f64 / total as f64) * 100.0);
    println!("Failures:     {} ({:.1}%)", errors, (errors as f64 / total as f64) * 100.0);
    println!("\n=== TOP ERRORS ===");
    
    let mut err_vec: Vec<_> = error_types.iter().collect();
    err_vec.sort_by(|a, b| b.1.cmp(a.1));

    for (msg, count) in err_vec.into_iter().take(20) {
        println!("{:4}x {}", count, msg);
    }
}
