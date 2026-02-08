//! Template resolution for CSLN migration.
//!
//! Resolves bibliography and citation templates from multiple sources in priority order:
//! 1. Hand-authored YAML files (examples/{style-name}-style.yaml)
//! 2. Cached inferred JSON files (templates/inferred/{style-name}.json)
//! 3. Live inference via Node.js (scripts/infer-template.js --fragment)
//! 4. Fallback to XML template compiler (caller handles this case)

use csln_core::template::TemplateComponent;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How the template was resolved.
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// From a hand-authored YAML file.
    HandAuthored(PathBuf),
    /// From a cached inferred JSON file.
    InferredCached(PathBuf),
    /// From live Node.js inference (then cached).
    InferredLive,
    /// XML compiler fallback (resolve_template returns None).
    XmlCompiled,
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::HandAuthored(p) => write!(f, "hand-authored ({})", p.display()),
            TemplateSource::InferredCached(p) => {
                write!(f, "cached inferred ({})", p.display())
            }
            TemplateSource::InferredLive => write!(f, "live inferred"),
            TemplateSource::XmlCompiled => write!(f, "XML compiled"),
        }
    }
}

/// Result of template resolution containing the template and its source.
pub struct ResolvedTemplate {
    pub source: TemplateSource,
    pub bibliography: Vec<TemplateComponent>,
    /// Bibliography delimiter from inferred fragment (e.g., ". ").
    /// Overrides the XML-extracted options.bibliography.separator when present.
    pub delimiter: Option<String>,
}

/// JSON fragment format produced by `infer-template.js --fragment`.
#[derive(serde::Deserialize)]
struct InferredFragment {
    meta: Option<FragmentMeta>,
    bibliography: BibliographyFragment,
}

#[derive(serde::Deserialize)]
struct FragmentMeta {
    delimiter: Option<String>,
}

#[derive(serde::Deserialize)]
struct BibliographyFragment {
    template: Vec<TemplateComponent>,
}

/// Resolve a template for the given style using the priority cascade.
///
/// Returns `None` when no pre-built template is available, signaling the caller
/// to use the XML template compiler.
///
/// Priority: hand-authored YAML > cached inferred JSON > live Node.js inference.
pub fn resolve_template(
    style_path: &str,
    style_name: &str,
    template_dir: Option<&Path>,
    workspace_root: &Path,
) -> Option<ResolvedTemplate> {
    // 1. Check for hand-authored YAML
    let hand_path = workspace_root
        .join("examples")
        .join(format!("{}-style.yaml", style_name));
    if hand_path.exists() {
        if let Some(template) = load_hand_authored(&hand_path) {
            return Some(ResolvedTemplate {
                source: TemplateSource::HandAuthored(hand_path),
                bibliography: template,
                delimiter: None, // Hand-authored styles define their own options
            });
        }
    }

    // 2. Check for cached inferred JSON
    let cache_dir = template_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| workspace_root.join("templates").join("inferred"));
    let cache_path = cache_dir.join(format!("{}.json", style_name));
    if cache_path.exists() {
        if let Some((template, delimiter)) = load_inferred_json(&cache_path) {
            return Some(ResolvedTemplate {
                source: TemplateSource::InferredCached(cache_path),
                bibliography: template,
                delimiter,
            });
        }
    }

    // 3. Try live inference via Node.js
    if let Some((template, delimiter)) =
        infer_live(style_path, &cache_dir, style_name, workspace_root)
    {
        return Some(ResolvedTemplate {
            source: TemplateSource::InferredLive,
            bibliography: template,
            delimiter,
        });
    }

    // 4. No template found — caller should use XML compiler
    None
}

/// Load bibliography template from a hand-authored YAML style file.
fn load_hand_authored(path: &Path) -> Option<Vec<TemplateComponent>> {
    let text = std::fs::read_to_string(path).ok()?;
    let style: csln_core::Style = serde_yaml::from_str(&text).ok()?;
    style.bibliography?.template
}

/// Load bibliography template and delimiter from a cached inferred JSON fragment.
fn load_inferred_json(path: &Path) -> Option<(Vec<TemplateComponent>, Option<String>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let fragment: InferredFragment = serde_json::from_str(&text).ok()?;
    let delimiter = fragment.meta.and_then(|m| m.delimiter);
    Some((fragment.bibliography.template, delimiter))
}

/// Run the Node.js template inferrer and cache the result.
fn infer_live(
    style_path: &str,
    cache_dir: &Path,
    style_name: &str,
    workspace_root: &Path,
) -> Option<(Vec<TemplateComponent>, Option<String>)> {
    // Check if node is available
    if Command::new("node").arg("--version").output().is_err() {
        return None;
    }

    let script = workspace_root.join("scripts").join("infer-template.js");
    if !script.exists() {
        return None;
    }

    eprintln!("Inferring template for {}...", style_name);

    let output = Command::new("node")
        .arg(&script)
        .arg(style_path)
        .arg("--fragment")
        .current_dir(workspace_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let fragment: InferredFragment = serde_json::from_str(&stdout).ok()?;
    let delimiter = fragment.meta.and_then(|m| m.delimiter);

    // Cache the result for next time
    if std::fs::create_dir_all(cache_dir).is_ok() {
        let cache_path = cache_dir.join(format!("{}.json", style_name));
        let _ = std::fs::write(&cache_path, &stdout);
    }

    Some((fragment.bibliography.template, delimiter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inferred_json_deserialization() {
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.95 },
            "bibliography": {
                "template": [
                    { "contributor": "author", "form": "long" },
                    { "date": "issued", "form": "year", "wrap": "parentheses" },
                    { "title": "primary" },
                    { "number": "volume" },
                    { "variable": "doi" }
                ]
            }
        }"#;

        let fragment: InferredFragment = serde_json::from_str(json).unwrap();
        assert_eq!(fragment.bibliography.template.len(), 5);

        match &fragment.bibliography.template[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, csln_core::template::ContributorRole::Author);
            }
            other => panic!("Expected Contributor, got {:?}", other),
        }

        match &fragment.bibliography.template[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.date, csln_core::template::DateVariable::Issued);
                assert_eq!(
                    d.rendering.wrap,
                    Some(csln_core::template::WrapPunctuation::Parentheses)
                );
            }
            other => panic!("Expected Date, got {:?}", other),
        }
    }

    #[test]
    fn test_live_fragment_with_list_and_delimiter() {
        // Matches actual output from infer-template.js --fragment
        let json = r#"{
            "meta": { "style": "elsevier-harvard", "confidence": 0.97, "delimiter": ". " },
            "bibliography": {
                "template": [
                    { "contributor": "author", "form": "long" },
                    { "date": "issued", "form": "year" },
                    { "title": "primary" },
                    { "contributor": "editor", "form": "verb" },
                    { "title": "parent-monograph" },
                    { "title": "parent-serial" },
                    { "variable": "publisher" },
                    { "items": [{ "number": "volume" }, { "number": "issue" }] },
                    { "number": "pages" },
                    { "variable": "publisher-place" },
                    { "variable": "doi", "prefix": "https://doi.org/" }
                ]
            }
        }"#;

        let fragment: InferredFragment = serde_json::from_str(json).unwrap();
        assert_eq!(fragment.bibliography.template.len(), 11);
        assert_eq!(
            fragment.meta.and_then(|m| m.delimiter),
            Some(". ".to_string())
        );
    }

    #[test]
    fn test_fragment_without_delimiter() {
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.9 },
            "bibliography": {
                "template": [{ "contributor": "author", "form": "long" }]
            }
        }"#;

        let fragment: InferredFragment = serde_json::from_str(json).unwrap();
        assert_eq!(fragment.meta.and_then(|m| m.delimiter), None);
    }

    #[test]
    fn test_invalid_json_returns_none() {
        let dir = std::env::temp_dir().join("csln_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, "not valid json").unwrap();
        assert!(load_inferred_json(&path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_file_returns_none() {
        let path = Path::new("/nonexistent/path/style.json");
        assert!(load_inferred_json(path).is_none());
    }
}
