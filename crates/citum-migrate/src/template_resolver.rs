/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Template resolution for Citum migration.
//!
//! Resolves bibliography and citation templates from multiple sources:
//! 1. Hand-authored YAML files (`examples/{style-name}-style.yaml`)
//! 2. Cached inferred JSON files (`templates/inferred/`)
//! 3. Live inference via embedded JS or Node (`scripts/infer-template.js --fragment`)
//! 4. Fallback to XML template compiler (caller handles this case)

use crate::js_runtime::EmbeddedTemplateRuntime;
use citum_schema::template::{TemplateComponent, WrapPunctuation};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Template source preference passed from CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateMode {
    /// Prefer hand-authored, then inferred.
    Auto,
    /// Use hand-authored templates only.
    Hand,
    /// Use inferred templates only.
    Inferred,
    /// Disable template resolution and use XML compiler only.
    Xml,
}

impl std::str::FromStr for TemplateMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::Auto),
            "hand" => Ok(Self::Hand),
            "inferred" => Ok(Self::Inferred),
            "xml" => Ok(Self::Xml),
            other => Err(format!(
                "invalid template mode '{other}': expected auto|hand|inferred|xml"
            )),
        }
    }
}

/// Live inference backend preference passed from CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveInferBackend {
    /// Try embedded JS first, then Node subprocess.
    Auto,
    /// Use the embedded JS runtime only.
    Embedded,
    /// Use the legacy Node subprocess only.
    Node,
}

impl std::str::FromStr for LiveInferBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(Self::Auto),
            "embedded" => Ok(Self::Embedded),
            "node" => Ok(Self::Node),
            other => Err(format!(
                "invalid live inference backend '{other}': expected auto|embedded|node"
            )),
        }
    }
}

/// Template section to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSection {
    Bibliography,
    Citation,
}

impl TemplateSection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bibliography => "bibliography",
            Self::Citation => "citation",
        }
    }

    fn cache_candidates(self, cache_dir: &Path, style_name: &str) -> Vec<PathBuf> {
        match self {
            // Keep support for legacy cache naming (`{style}.json`).
            Self::Bibliography => vec![
                cache_dir.join(format!("{style_name}.bibliography.json")),
                cache_dir.join(format!("{style_name}.json")),
            ],
            Self::Citation => vec![cache_dir.join(format!("{style_name}.citation.json"))],
        }
    }

    fn cache_output_path(self, cache_dir: &Path, style_name: &str) -> PathBuf {
        cache_dir.join(format!("{}.{}.json", style_name, self.as_str()))
    }
}

/// How the template was resolved.
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// From a hand-authored YAML file.
    HandAuthored(PathBuf),
    /// From a cached inferred JSON file.
    InferredCached(PathBuf),
    /// From live Node.js inference (then cached).
    InferredLive,
    /// XML compiler fallback (`resolve_templates` returns None for section).
    XmlCompiled,
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::HandAuthored(p) => write!(f, "hand-authored ({})", p.display()),
            TemplateSource::InferredCached(p) => write!(f, "cached inferred ({})", p.display()),
            TemplateSource::InferredLive => write!(f, "live inferred"),
            TemplateSource::XmlCompiled => write!(f, "XML compiled"),
        }
    }
}

/// Section-level resolved template and metadata.
#[derive(Debug, Clone)]
pub struct ResolvedTemplateSection {
    pub source: TemplateSource,
    pub template: Vec<TemplateComponent>,
    pub confidence: Option<f64>,
    /// Delimiter inferred from output (e.g., ". " for bibliography, ", " for citation).
    pub delimiter: Option<String>,
    /// Bibliography entry suffix inferred from output (e.g., ".").
    pub entry_suffix: Option<String>,
    /// Inferred citation wrap (`parentheses`, `brackets`, etc).
    pub wrap: Option<WrapPunctuation>,
}

/// Templates resolved per section.
#[derive(Debug, Clone, Default)]
pub struct ResolvedTemplates {
    pub bibliography: Option<ResolvedTemplateSection>,
    pub citation: Option<ResolvedTemplateSection>,
}

/// JSON fragment format produced by `infer-template.js --fragment`.
#[derive(serde::Deserialize)]
struct InferredFragment {
    meta: Option<FragmentMeta>,
    bibliography: Option<TemplateFragment>,
    citation: Option<TemplateFragment>,
}

#[derive(serde::Deserialize)]
struct FragmentMeta {
    confidence: Option<f64>,
    delimiter: Option<String>,
    #[serde(rename = "entrySuffix")]
    entry_suffix: Option<String>,
    wrap: Option<WrapPunctuation>,
}

#[derive(serde::Deserialize)]
struct TemplateFragment {
    template: Vec<TemplateComponent>,
}

/// Resolve citation and bibliography templates from configured sources.
#[must_use]
pub fn resolve_templates(
    style_path: &str,
    style_name: &str,
    template_dir: Option<&Path>,
    workspace_root: &Path,
    mode: TemplateMode,
    min_confidence: f64,
    live_infer_backend: LiveInferBackend,
) -> ResolvedTemplates {
    if mode == TemplateMode::Xml {
        return ResolvedTemplates::default();
    }

    let hand_path = workspace_root
        .join("examples")
        .join(format!("{style_name}-style.yaml"));
    let hand_authored = if matches!(mode, TemplateMode::Auto | TemplateMode::Hand) {
        load_hand_authored_sections(&hand_path)
    } else {
        None
    };

    let cache_dir = template_dir.map_or_else(
        || workspace_root.join("templates").join("inferred"),
        std::path::Path::to_path_buf,
    );

    let mut ctx = ResolveContext {
        style_path,
        style_name,
        workspace_root,
        cache_dir: &cache_dir,
        hand_authored: hand_authored.as_ref(),
        mode,
        min_confidence,
        live_infer_backend,
        embedded_runtime: EmbeddedRuntimeState::Uninitialized,
        node_available: None,
        style_xml: None,
    };

    let bibliography = resolve_section(TemplateSection::Bibliography, &mut ctx);
    let citation = resolve_section(TemplateSection::Citation, &mut ctx);

    ResolvedTemplates {
        bibliography,
        citation,
    }
}

struct HandAuthoredTemplates {
    path: PathBuf,
    bibliography: Option<Vec<TemplateComponent>>,
    citation: Option<Vec<TemplateComponent>>,
}

/// Load citation and bibliography templates from a hand-authored YAML style file.
fn load_hand_authored_sections(path: &Path) -> Option<HandAuthoredTemplates> {
    if !path.exists() {
        return None;
    }

    let text = std::fs::read_to_string(path).ok()?;
    let style: citum_schema::Style = serde_yaml::from_str(&text).ok()?;
    Some(HandAuthoredTemplates {
        path: path.to_path_buf(),
        bibliography: style
            .bibliography
            .and_then(|b| b.template)
            .and_then(citum_schema::TemplateVariant::into_template),
        citation: style
            .citation
            .and_then(|c| c.template)
            .and_then(citum_schema::TemplateVariant::into_template),
    })
}

struct ResolveContext<'a> {
    style_path: &'a str,
    style_name: &'a str,
    workspace_root: &'a Path,
    cache_dir: &'a Path,
    hand_authored: Option<&'a HandAuthoredTemplates>,
    mode: TemplateMode,
    min_confidence: f64,
    live_infer_backend: LiveInferBackend,
    embedded_runtime: EmbeddedRuntimeState,
    node_available: Option<bool>,
    style_xml: Option<String>,
}

enum EmbeddedRuntimeState {
    Uninitialized,
    Ready(EmbeddedTemplateRuntime),
    Failed,
}

fn resolve_section(
    section: TemplateSection,
    ctx: &mut ResolveContext<'_>,
) -> Option<ResolvedTemplateSection> {
    if matches!(ctx.mode, TemplateMode::Auto | TemplateMode::Hand)
        && let Some(hand) = ctx.hand_authored
    {
        let hand_template = match section {
            TemplateSection::Bibliography => hand.bibliography.clone(),
            TemplateSection::Citation => hand.citation.clone(),
        };
        if let Some(template) = hand_template {
            return Some(ResolvedTemplateSection {
                source: TemplateSource::HandAuthored(hand.path.clone()),
                template,
                confidence: None,
                delimiter: None,
                entry_suffix: None,
                wrap: None,
            });
        }
    }

    if matches!(ctx.mode, TemplateMode::Auto | TemplateMode::Inferred) {
        // `inferred` mode is cache-only so Rust migration can run without live
        // citeproc-js inference after precompilation.
        let allow_live_infer = matches!(ctx.mode, TemplateMode::Auto);
        if let Some(resolved) = resolve_inferred_section(
            ctx.style_path,
            ctx.style_name,
            section,
            ctx.cache_dir,
            allow_live_infer,
            ctx,
        ) {
            return Some(resolved);
        }
    }

    None
}

fn resolve_inferred_section(
    style_path: &str,
    style_name: &str,
    section: TemplateSection,
    cache_dir: &Path,
    allow_live_infer: bool,
    ctx: &mut ResolveContext<'_>,
) -> Option<ResolvedTemplateSection> {
    for cache_path in section.cache_candidates(cache_dir, style_name) {
        if !cache_path.exists() {
            continue;
        }
        if let Some(mut resolved) = load_inferred_json(&cache_path, section, ctx.min_confidence) {
            resolved.source = TemplateSource::InferredCached(cache_path);
            return Some(resolved);
        }
    }

    if allow_live_infer {
        infer_live(style_path, style_name, section, cache_dir, ctx)
    } else {
        None
    }
}

/// Load an inferred fragment from cache and extract a section template.
fn load_inferred_json(
    path: &Path,
    section: TemplateSection,
    min_confidence: f64,
) -> Option<ResolvedTemplateSection> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            tracing::debug!("  [template_resolver] Failed to read cache file: {e}");
            return None;
        }
    };
    parse_fragment(&text, section, min_confidence)
}

#[allow(clippy::cognitive_complexity, reason = "complex JSON parsing logic")]
fn parse_fragment(
    text: &str,
    section: TemplateSection,
    min_confidence: f64,
) -> Option<ResolvedTemplateSection> {
    let sanitized = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(mut value) => {
            strip_deprecated_overrides(&mut value);
            value
        }
        Err(e) => {
            tracing::debug!("  [template_resolver] Failed to parse fragment JSON: {e}");
            tracing::debug!(
                "  [template_resolver] First 200 chars: {}",
                preview_text(text, 200)
            );
            return None;
        }
    };

    let fragment: InferredFragment = match serde_json::from_value(sanitized) {
        Ok(f) => f,
        Err(e) => {
            tracing::debug!("  [template_resolver] Failed to parse fragment JSON: {e}");
            tracing::debug!(
                "  [template_resolver] First 200 chars: {}",
                preview_text(text, 200)
            );
            return None;
        }
    };

    let template_fragment = match section {
        TemplateSection::Bibliography => fragment
            .bibliography
            .as_ref()
            .or(fragment.citation.as_ref()),
        TemplateSection::Citation => fragment
            .citation
            .as_ref()
            .or(fragment.bibliography.as_ref()),
    }?;

    let confidence = fragment.meta.as_ref().and_then(|m| m.confidence);
    if let Some(score) = confidence
        && score < min_confidence
    {
        tracing::debug!(
            "  [template_resolver] Rejected {} template (confidence {:.2} < {:.2})",
            section.as_str(),
            score,
            min_confidence
        );
        return None;
    }

    let delimiter = fragment.meta.as_ref().and_then(|m| m.delimiter.clone());
    let entry_suffix = fragment.meta.as_ref().and_then(|m| m.entry_suffix.clone());
    let wrap = fragment.meta.as_ref().and_then(|m| m.wrap.clone());

    Some(ResolvedTemplateSection {
        source: TemplateSource::InferredLive,
        template: template_fragment.template.clone(),
        confidence,
        delimiter,
        entry_suffix,
        wrap,
    })
}

fn strip_deprecated_overrides(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.remove("overrides");
            for nested in map.values_mut() {
                strip_deprecated_overrides(nested);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_deprecated_overrides(item);
            }
        }
        _ => {}
    }
}

fn preview_text(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }

    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    #[allow(clippy::string_slice, reason = "end is verified as char boundary")]
    &text[..end]
}

/// Run live inference using the selected backend and cache the result.
fn infer_live(
    style_path: &str,
    style_name: &str,
    section: TemplateSection,
    cache_dir: &Path,
    ctx: &mut ResolveContext<'_>,
) -> Option<ResolvedTemplateSection> {
    tracing::debug!(
        "Inferring {} template for {}...",
        section.as_str(),
        style_name
    );

    let stdout = match ctx.live_infer_backend {
        LiveInferBackend::Auto => infer_with_embedded(style_name, section, ctx)
            .or_else(|| infer_with_node(style_path, section, ctx)),
        LiveInferBackend::Embedded => infer_with_embedded(style_name, section, ctx),
        LiveInferBackend::Node => infer_with_node(style_path, section, ctx),
    }?;

    let mut resolved = parse_fragment(&stdout, section, ctx.min_confidence)?;
    resolved.source = TemplateSource::InferredLive;

    if std::fs::create_dir_all(cache_dir).is_ok() {
        let cache_path = section.cache_output_path(cache_dir, style_name);
        let _ = std::fs::write(cache_path, &stdout);
    }

    Some(resolved)
}

fn infer_with_embedded(
    style_name: &str,
    section: TemplateSection,
    ctx: &mut ResolveContext<'_>,
) -> Option<String> {
    style_xml(ctx)?;
    let style_xml = ctx.style_xml.take()?;
    let result = if let Some(runtime) = embedded_runtime(ctx) {
        runtime.infer_fragment(style_name, &style_xml, section.as_str())
    } else {
        ctx.style_xml = Some(style_xml);
        return None;
    };
    ctx.style_xml = Some(style_xml);

    result
        .map_err(|err| {
            tracing::debug!(
                "  [template_resolver] Embedded inference failed for {}: {err}",
                section.as_str()
            );
            err
        })
        .ok()
}

fn infer_with_node(
    style_path: &str,
    section: TemplateSection,
    ctx: &mut ResolveContext<'_>,
) -> Option<String> {
    if let Some(false) = ctx.node_available {
        return None;
    }

    let script = ctx.workspace_root.join("scripts").join("infer-template.js");
    if !script.exists() {
        ctx.node_available = Some(false);
        return None;
    }

    if ctx.node_available.is_none() {
        ctx.node_available = Some(Command::new("node").arg("--version").output().is_ok());
    }
    if ctx.node_available != Some(true) {
        return None;
    }

    let output = Command::new("node")
        .arg(&script)
        .arg(style_path)
        .arg(format!("--section={}", section.as_str()))
        .arg("--fragment")
        .current_dir(ctx.workspace_root)
        .output()
        .ok()?;

    if !output.status.success() {
        tracing::debug!(
            "  [template_resolver] Node inference failed for {}: {}",
            section.as_str(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }

    String::from_utf8(output.stdout).ok()
}

fn embedded_runtime<'a>(
    ctx: &'a mut ResolveContext<'_>,
) -> Option<&'a mut EmbeddedTemplateRuntime> {
    if matches!(ctx.embedded_runtime, EmbeddedRuntimeState::Uninitialized) {
        ctx.embedded_runtime = match EmbeddedTemplateRuntime::new(ctx.workspace_root) {
            Ok(runtime) => EmbeddedRuntimeState::Ready(runtime),
            Err(err) => {
                tracing::debug!("  [template_resolver] Embedded runtime unavailable: {err}");
                EmbeddedRuntimeState::Failed
            }
        };
    }

    match &mut ctx.embedded_runtime {
        EmbeddedRuntimeState::Ready(runtime) => Some(runtime),
        EmbeddedRuntimeState::Failed | EmbeddedRuntimeState::Uninitialized => None,
    }
}

fn style_xml<'a>(ctx: &'a mut ResolveContext<'_>) -> Option<&'a str> {
    if ctx.style_xml.is_none() {
        ctx.style_xml = match std::fs::read_to_string(ctx.style_path) {
            Ok(text) => Some(text),
            Err(err) => {
                tracing::debug!(
                    "  [template_resolver] Failed to read style XML {}: {err}",
                    ctx.style_path
                );
                None
            }
        };
    }

    ctx.style_xml.as_deref()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;

    #[test]
    fn test_inferred_json_deserialization_for_bibliography() {
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
        assert_eq!(fragment.bibliography.unwrap().template.len(), 5);
    }

    #[test]
    fn test_citation_section_key_is_used() {
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.90, "wrap": "parentheses" },
            "citation": {
                "template": [
                    { "contributor": "author", "form": "short" },
                    { "date": "issued", "form": "year" }
                ]
            }
        }"#;

        let resolved = parse_fragment(json, TemplateSection::Citation, 0.70).unwrap();
        assert_eq!(resolved.template.len(), 2);
        assert_eq!(resolved.wrap, Some(WrapPunctuation::Parentheses));
    }

    #[test]
    fn test_legacy_fragment_works_for_citation() {
        // Legacy infer-template fragment only emitted `bibliography`.
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.90 },
            "bibliography": {
                "template": [
                    { "contributor": "author", "form": "short" },
                    { "date": "issued", "form": "year" }
                ]
            }
        }"#;

        let resolved = parse_fragment(json, TemplateSection::Citation, 0.70).unwrap();
        assert_eq!(resolved.template.len(), 2);
    }

    #[test]
    fn test_fragment_with_deprecated_component_overrides_still_parses() {
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.90 },
            "citation": {
                "template": [
                    {
                        "contributor": "author",
                        "form": "long",
                        "overrides": {
                            "legal_case": { "suppress": true }
                        }
                    },
                    {
                        "variable": "doi",
                        "overrides": {
                            "article-journal": { "prefix": "https://doi.org/" }
                        }
                    }
                ]
            }
        }"#;

        let resolved = parse_fragment(json, TemplateSection::Citation, 0.70).unwrap();
        assert_eq!(resolved.template.len(), 2);
    }

    #[test]
    fn test_preview_text_respects_utf8_boundaries() {
        let text = "alpha-é-beta";
        let preview = preview_text(text, 7);
        assert_eq!(preview, "alpha-");
    }

    #[test]
    fn test_low_confidence_fragment_rejected() {
        let json = r#"{
            "meta": { "style": "test", "confidence": 0.20 },
            "bibliography": {
                "template": [{ "contributor": "author", "form": "long" }]
            }
        }"#;

        assert!(parse_fragment(json, TemplateSection::Bibliography, 0.70).is_none());
    }

    #[test]
    fn test_invalid_json_returns_none() {
        let dir = std::env::temp_dir().join("ir_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, "not valid json").unwrap();
        assert!(load_inferred_json(&path, TemplateSection::Bibliography, 0.70).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_file_returns_none() {
        let path = Path::new("/nonexistent/path/style.json");
        assert!(load_inferred_json(path, TemplateSection::Bibliography, 0.70).is_none());
    }

    #[test]
    fn test_live_infer_backend_from_str() {
        assert_eq!(
            "auto".parse::<LiveInferBackend>().unwrap(),
            LiveInferBackend::Auto
        );
        assert_eq!(
            "embedded".parse::<LiveInferBackend>().unwrap(),
            LiveInferBackend::Embedded
        );
        assert_eq!(
            "node".parse::<LiveInferBackend>().unwrap(),
            LiveInferBackend::Node
        );
        assert!("bogus".parse::<LiveInferBackend>().is_err());
    }
}
