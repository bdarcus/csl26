use csl_legacy::parser::parse_style;
use roxmltree::Document;
use serde_json::to_string_pretty;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = fs::read_to_string("../styles-legacy/chicago-author-date.csl")?;
    let doc = Document::parse(&text)?;
    let root = doc.root_element();

    let style = parse_style(root)?;
    println!("{}", to_string_pretty(&style)?);

    Ok(())
}
