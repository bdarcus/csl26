/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Migration-time style lineage and wrapper classification.
//!
//! `diff_value` cannot express deletions: it only emits keys present in the
//! child that differ from the parent, so a key the parent set but the
//! standalone child form lacked is silently inherited by the wrapper rather
//! than being suppressed. This is invisible in the emitted YAML; watch for
//! it via the `tracing::debug!` emitted when a key drops between parent and
//! child.

use crate::evidence::{
    EmittedForm, FamilyCandidate, MeasuredSelectionEvidence, MigrationDiagnostic,
    MigrationEvidence, MinimizationDecisionAudit, ParentDiscoverySource, RegistryAliasStatus,
};
use citum_schema::Style;
use citum_schema::embedded;
use citum_schema::options::{Processing, RegimeFamily};
use citum_schema::registry::{StyleKind, StyleRegistry};
use csl_legacy::model::InfoLink;
use serde_yaml::{Mapping, Value};
use std::fmt;
use std::fs;
use std::path::Path;

/// Taxonomy-level semantic class for the migration target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticClass {
    Base,
    Profile,
    Journal,
    Independent,
    Unknown,
}

/// Current implementation form derived from the checked-in style shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationForm {
    Alias,
    TemplateDescendant,
    ConfigWrapper,
    StructuralWrapper,
    Standalone,
    Unknown,
}

/// Explicit artifact plan for migration output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutputPlan {
    /// Emit a single standalone style without an inherited parent.
    Standalone,
    /// Emit a single wrapper over an established checked-in parent style.
    ExistingWrapper {
        /// Established parent style ID from repo truth.
        parent_style_id: String,
        /// Current wrapper implementation form.
        implementation_form: ImplementationForm,
        /// Whether local template-bearing deltas are allowed in the wrapper.
        preserve_template_deltas: bool,
    },
}

/// Migration-time lineage for one style target.
#[derive(Debug, Clone)]
pub struct StyleLineage {
    /// Canonical style ID derived from the CSL filename.
    pub style_id: String,
    /// Semantic class from the registry or alias surface.
    pub semantic_class: SemanticClass,
    /// Current implementation form derived from the checked-in style.
    pub implementation_form: ImplementationForm,
    /// Established parent style ID, if any.
    pub parent_style_id: Option<String>,
    /// Registry alias target for the legacy ID, if any. Captured for the
    /// migration evidence record so downstream tooling can distinguish a
    /// "true alias" from a "discovered family ancestor".
    pub registry_alias_target: Option<String>,
    /// True when the legacy ID is itself a canonical registry entry (not an
    /// alias). Used by the evidence record to surface
    /// `RegistryAliasStatus::ExactMatch` precisely; checking
    /// `registry_alias_target == style_id` cannot detect this because the
    /// alias field is only populated when the legacy id is an alias OF some
    /// other canonical entry.
    pub registry_exact_match: bool,
    /// Family-candidate parent discovered via a reverse `<info><link
    /// rel="template">` in an embedded canonical style, when no other parent
    /// link is available. Inert by default; the CLI promotes this into
    /// `parent_style_id` only when explicitly opted in via
    /// `--family-candidate`.
    pub family_candidate: Option<String>,
    /// Discovery source for `parent_style_id`, when set at resolve time.
    /// Used by the evidence record to classify the parent precisely
    /// (registry alias vs source CSL link vs local file extends).
    pub parent_source: Option<ParentDiscoverySource>,
    parent_style: Option<Style>,
}

/// Caller-supplied details needed to build a migration evidence sidecar.
pub struct MigrationEvidenceParts {
    /// Output size of the standalone form, in lines.
    pub standalone_lines: usize,
    /// The form actually emitted by the migration invocation.
    pub emitted_form: EmittedForm,
    /// Output size of the emitted form, in lines.
    pub emitted_lines: usize,
    /// Audit trail for explicit minimization decisions.
    pub minimization_decision: MinimizationDecisionAudit,
    /// Template-bearing paths preserved in the emitted wrapper.
    pub preserved_template_paths: Vec<String>,
    /// Template-bearing paths discarded from the emitted wrapper.
    pub discarded_template_paths: Vec<String>,
    /// Output-driven measured candidate selection summaries.
    pub measured_selection: Option<MeasuredSelectionEvidence>,
    /// Recoverable warnings collected while converting migration fixtures.
    pub diagnostics: Vec<MigrationDiagnostic>,
}

/// Failure while resolving or rewriting migration lineage.
#[derive(Debug)]
pub enum LineageError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Resolution(citum_schema::ResolutionError),
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineageError::Io(err) => write!(f, "{err}"),
            LineageError::Yaml(err) => write!(f, "{err}"),
            LineageError::Resolution(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for LineageError {}

impl From<std::io::Error> for LineageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for LineageError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

impl StyleLineage {
    /// Resolve migration lineage from the CSL file path and repository root.
    ///
    /// # Errors
    ///
    /// Returns an error when the current style or its established parent cannot
    /// be read or parsed from repo-owned YAML.
    pub fn resolve(
        input_path: &str,
        repo_root: &Path,
        legacy_links: &[InfoLink],
    ) -> Result<Self, LineageError> {
        let style_id = Path::new(input_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string();

        let registry = StyleRegistry::load_default();
        let exact_entry = registry.styles.iter().find(|entry| entry.id == style_id);
        let alias_target = registry
            .styles
            .iter()
            .find(|entry| entry.aliases.iter().any(|alias| alias == &style_id));
        let parent_link_hit = resolve_parent_link_target(&registry, legacy_links);
        let parent_link_target = parent_link_hit.as_ref().map(|(id, _)| id.clone());
        let parent_link_rel = parent_link_hit.as_ref().map(|(_, rel)| *rel);
        // Reverse-template-link discovery: if no other parent has surfaced,
        // scan embedded canonical styles for an `<info><link rel="template">`
        // pointing back at this legacy ID. Such a link means a canonical
        // style historically derived from this legacy file — making the
        // canonical style a candidate ancestor for output-driven compression.
        // Discovery is independent of routing: the candidate is recorded on
        // `StyleLineage` but only acts on `output_plan` after explicit opt-in
        // via `promote_family_candidate`.
        let family_candidate = if parent_link_target.is_none() && alias_target.is_none() {
            discover_reverse_template_parent(&registry, &style_id)
        } else {
            None
        };

        let current_style = load_current_style(repo_root, &style_id, exact_entry)?;
        let semantic_class = if let Some(entry) = exact_entry {
            map_style_kind(entry.kind.as_ref())
        } else if let Some(entry) = alias_target {
            // An alias inherits the semantic class of its canonical target so
            // `output_plan` can route Profile/Base/Journal aliases consistently.
            map_style_kind(entry.kind.as_ref())
        } else if parent_link_target.is_some() {
            SemanticClass::Journal
        } else if let Some(style) = current_style.as_ref() {
            if style.extends.is_some() {
                SemanticClass::Journal
            } else {
                SemanticClass::Independent
            }
        } else {
            SemanticClass::Unknown
        };
        // Track *where* the parent came from so the evidence record can
        // distinguish a registry alias from a CSL info-link from a local
        // extends. Order mirrors the precedence in the next line.
        let local_extends = current_style
            .as_ref()
            .and_then(|style| style.extends.as_ref())
            .map(|base| base.key().to_string());
        let established_parent = local_extends
            .clone()
            .or_else(|| alias_target.map(|entry| entry.id.clone()));
        let mut parent_style_id = established_parent.clone().or(parent_link_target.clone());
        let mut parent_source = if local_extends.is_some() {
            Some(ParentDiscoverySource::LocalExtends)
        } else if alias_target.is_some() {
            Some(ParentDiscoverySource::RegistryAlias)
        } else {
            // `parent_link_rel` was captured by the same predicate that
            // resolved `parent_link_target`, so the rel here is guaranteed
            // to match the actual hit (not just the first matching rel in
            // the legacy links list).
            parent_link_rel.map(|rel| match rel {
                ParentLinkRel::IndependentParent => ParentDiscoverySource::IndependentParentLink,
                ParentLinkRel::Template => ParentDiscoverySource::TemplateLink,
            })
        };
        let parent_style = if let Some(parent) = parent_style_id.clone() {
            match load_style_by_id(repo_root, &parent) {
                Ok(style) => Some(style),
                Err(err) if established_parent.is_none() => {
                    parent_style_id = None;
                    parent_source = None;
                    tracing::debug!(
                        "Ignoring legacy parent link `{parent}` because no Citum parent style could be loaded: {err}"
                    );
                    None
                }
                Err(err) => return Err(err),
            }
        } else {
            None
        };

        let implementation_form = if current_style.is_none() && alias_target.is_some() {
            ImplementationForm::Alias
        } else if current_style.is_none() && parent_style_id.is_some() {
            ImplementationForm::TemplateDescendant
        } else if let Some(style) = current_style.as_ref() {
            derive_implementation_form(style)
        } else {
            ImplementationForm::Unknown
        };

        Ok(Self {
            style_id,
            semantic_class,
            implementation_form,
            parent_style_id,
            registry_alias_target: alias_target.map(|entry| entry.id.clone()),
            registry_exact_match: exact_entry.is_some(),
            family_candidate,
            parent_source,
            parent_style,
        })
    }

    /// Drop a template-linked parent when its citation regime family conflicts
    /// with the child's detected regime.
    ///
    /// CSL `<link rel="template">` links record the style that served as a
    /// formatting template during authoring — not a true semantic parent. When
    /// the linked parent's `processing` regime family differs from the child's
    /// detected regime, inheriting it would produce a wrapper with an
    /// incompatible citation template (e.g., a numeric child inheriting an
    /// author-date `non_integral`). This guard catches that case at migrate time
    /// and falls back to standalone output.
    ///
    /// Only fires when **all** of:
    /// - The parent was discovered via a `TemplateLink` (not `LocalExtends`,
    ///   `RegistryAlias`, or `IndependentParentLink` — those are intentional).
    /// - The parent Citum style has an explicitly declared `processing` field.
    /// - `detected_regime` is `Some` and has a different `regime_family()` than
    ///   the parent's declared processing.
    ///
    /// See `docs/specs/CITATION_REGIME.md`.
    #[must_use]
    pub fn apply_regime_guard(mut self, detected_regime: Option<&Processing>) -> Self {
        // Only template links are subject to the guard.
        if self.parent_source != Some(ParentDiscoverySource::TemplateLink) {
            return self;
        }
        let Some(detected) = detected_regime else {
            return self;
        };
        let detected_family = detected.regime_family();

        // Read the parent style's declared processing, if any.
        let parent_family: Option<RegimeFamily> = self
            .parent_style
            .as_ref()
            .and_then(|s| s.options.as_ref())
            .and_then(|c| c.processing.as_ref())
            .map(|p| p.regime_family());

        let Some(parent_family) = parent_family else {
            // Parent has no explicit processing — no basis to drop it.
            return self;
        };

        if parent_family == detected_family {
            return self;
        }

        // Regime mismatch: drop the template-linked parent and fall back to
        // standalone output. Preserve `family_candidate` in case it was set
        // independently via reverse-template discovery.
        tracing::debug!(
            style_id = %self.style_id,
            parent = ?self.parent_style_id,
            "dropping template-link parent: regime mismatch ({detected_family:?} vs {parent_family:?})"
        );
        self.parent_style_id = None;
        self.parent_source = None;
        self.parent_style = None;
        self.semantic_class = SemanticClass::Independent;
        self.implementation_form = ImplementationForm::Standalone;
        self
    }

    /// Build a `MigrationEvidence` record summarizing the lineage decisions
    /// for downstream consumers (SQI scorecard, future hub UX). The caller
    /// supplies the standalone reference LOC and the emitted form details,
    /// since those depend on running the full migration pipeline.
    #[must_use]
    pub fn build_evidence(&self, parts: MigrationEvidenceParts) -> MigrationEvidence {
        let registry_alias_status = if self.registry_exact_match {
            RegistryAliasStatus::ExactMatch
        } else if let Some(target) = self.registry_alias_target.clone() {
            RegistryAliasStatus::Alias { target }
        } else {
            RegistryAliasStatus::None
        };

        let mut discovered_parents: Vec<FamilyCandidate> = Vec::new();
        if let Some(alias) = self.registry_alias_target.clone() {
            discovered_parents.push(FamilyCandidate {
                canonical_id: alias,
                source: ParentDiscoverySource::RegistryAlias,
            });
        }
        if let Some(parent) = self.parent_style_id.clone()
            && !discovered_parents
                .iter()
                .any(|candidate| candidate.canonical_id == parent)
        {
            // `parent_source` was set at resolve time to record exactly which
            // mechanism produced `parent_style_id` (registry alias, source
            // CSL link, local file extends, or reverse template link via
            // promoted family candidate). Fall back to `TemplateLink` if a
            // future code path forgets to set it.
            let source = self
                .parent_source
                .clone()
                .unwrap_or(ParentDiscoverySource::TemplateLink);
            discovered_parents.push(FamilyCandidate {
                canonical_id: parent,
                source,
            });
        }
        if let Some(candidate) = self.family_candidate.clone()
            && !discovered_parents
                .iter()
                .any(|c| c.canonical_id == candidate)
        {
            discovered_parents.push(FamilyCandidate {
                canonical_id: candidate,
                source: ParentDiscoverySource::ReverseTemplateLink,
            });
        }

        MigrationEvidence {
            style_id: self.style_id.clone(),
            registry_alias_status,
            discovered_parents,
            emitted_form: parts.emitted_form,
            minimization_decision: parts.minimization_decision,
            preserved_template_paths: parts.preserved_template_paths,
            discarded_template_paths: parts.discarded_template_paths,
            measured_selection: parts.measured_selection,
            diagnostics: parts.diagnostics,
            standalone_output_lines: parts.standalone_lines,
            emitted_output_lines: parts.emitted_lines,
        }
    }

    /// Promote a discovered family-candidate parent into the active parent
    /// slot, routing future `output_plan` calls through
    /// `ExistingWrapper { preserve_template_deltas: true }`.
    ///
    /// This is opt-in (CLI: `--family-candidate auto|<id>`). When `override_id`
    /// is `Some`, the supplied ID is used regardless of the discovered
    /// candidate and the parent style is loaded directly from the registry.
    /// When `None`, the previously discovered candidate (if any) is promoted.
    ///
    /// Returns `Ok(true)` if a candidate was successfully promoted (and the
    /// parent style loaded), `Ok(false)` if no candidate was available, or an
    /// error if the parent style could not be loaded.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied or discovered parent style cannot be
    /// resolved from repo-owned YAML or embedded registry.
    pub fn promote_family_candidate(
        &mut self,
        repo_root: &Path,
        override_id: Option<&str>,
    ) -> Result<bool, LineageError> {
        let candidate_id = match override_id {
            Some(id) => id.to_string(),
            None => match self.family_candidate.clone() {
                Some(id) => id,
                None => return Ok(false),
            },
        };
        let parent_style = load_style_by_id(repo_root, &candidate_id)?;
        // Inherit the parent's semantic class so `output_plan` reaches the
        // `(Base|Profile|Journal, TemplateDescendant)` wrapper arm. Without
        // this, a legacy style with no registry entry (`SemanticClass::Unknown`)
        // would fall through to `Standalone` even after promotion.
        let registry = StyleRegistry::load_default();
        if let Some(entry) = registry.styles.iter().find(|e| e.id == candidate_id) {
            self.semantic_class = map_style_kind(entry.kind.as_ref());
        }
        self.parent_style = Some(parent_style);
        self.parent_style_id = Some(candidate_id.clone());
        self.implementation_form = ImplementationForm::TemplateDescendant;
        // Distinguish auto-promoted reverse-template candidates from
        // caller-forced explicit overrides so the evidence sidecar reports
        // the discovery source precisely. Also seed `family_candidate` for
        // explicit overrides so the minimize path (`apply_to_migrated_style_
        // minimized`) sees a promoted candidate regardless of which CLI
        // form selected it.
        if override_id.is_some() {
            self.parent_source = Some(ParentDiscoverySource::ExplicitOverride);
            self.family_candidate = Some(candidate_id);
        } else {
            self.parent_source = Some(ParentDiscoverySource::ReverseTemplateLink);
        }
        Ok(true)
    }

    /// Rewrite a standalone migrated style into wrapper form when the current
    /// repo truth already establishes that relationship.
    ///
    /// # Errors
    ///
    /// Returns an error when the rewritten wrapper cannot be serialized or
    /// deserialized as a valid `Style`.
    pub fn apply_to_migrated_style(&self, style: Style) -> Result<Style, LineageError> {
        let MigrationOutputPlan::ExistingWrapper {
            parent_style_id,
            implementation_form,
            preserve_template_deltas,
        } = self.output_plan()
        else {
            return Ok(style);
        };
        let Some(parent_style) = self.parent_style.as_ref() else {
            return Ok(style);
        };

        // An alias is *defined* to be its canonical target; the converter's
        // attempt to derive options/templates from the legacy CSL is best
        // discarded in favor of the canonical style. Emit an info+extends
        // shell so downstream tools resolve through the canonical entry
        // rather than reading converter-derived deltas that are usually noise.
        if implementation_form == ImplementationForm::Alias {
            // Parse the canonical id back through serde so we hit the
            // `StyleReference` untagged enum (Base variant for known builtin
            // ids, Uri fallback otherwise) without depending on private
            // constructors.
            let extends: citum_schema::style_base::StyleReference =
                serde_yaml::from_value(Value::String(parent_style_id))?;
            return Ok(Style {
                info: style.info,
                extends: Some(extends),
                raw_yaml: None,
                ..Default::default()
            });
        }

        let exclude_template_paths = !preserve_template_deltas;

        // Materialize diff type-variants as full templates before attaching
        // `extends`: a diff derived against this style's local template
        // resolves against the parent's same-selector variant at render
        // time, where its anchors may not exist (hard processor failure).
        let mut style = style;
        citum_schema::template::resolve_local_template_variants(&mut style)
            .map_err(LineageError::Resolution)?;

        let child = serde_yaml::to_value(&style)?;
        let parent = serde_yaml::to_value(parent_style.clone().into_resolved())?;

        let mut diff = match diff_value(&child, &parent, &mut Vec::new(), exclude_template_paths) {
            Some(Value::Mapping(map)) => map,
            Some(other) => {
                let mut map = Mapping::new();
                map.insert(Value::String("style".to_string()), other);
                map
            }
            None => Mapping::new(),
        };

        diff.insert(
            Value::String("extends".to_string()),
            Value::String(parent_style_id),
        );

        let mut rebuilt: Style = serde_yaml::from_value(Value::Mapping(diff))?;
        rebuilt.raw_yaml = None;
        Ok(rebuilt)
    }

    /// Apply lineage routing to a migrated style with optional minimization.
    ///
    /// When `minimize` is true and the output plan is an ExistingWrapper for a
    /// family-candidate parent, emit a minimal wrapper (info + extends only)
    /// instead of the full diff. This is used by the evidence-driven compression
    /// harness to derive output-driven minima gated by oracle equivalence.
    ///
    /// # Errors
    ///
    /// Returns an error when the rewritten wrapper cannot be serialized or
    /// deserialized as a valid `Style`.
    pub fn apply_to_migrated_style_minimized(
        &self,
        style: Style,
        minimize: bool,
    ) -> Result<Style, LineageError> {
        // If minimization is requested and a family-candidate parent has
        // been promoted (either auto-discovered via reverse template link
        // or caller-forced via `--family-candidate <id>`), emit the minimal
        // form. Other parent sources (registry alias, source CSL link,
        // local extends) keep their existing routing — those already go
        // through purpose-built arms of `apply_to_migrated_style`.
        let promoted = matches!(
            self.parent_source,
            Some(
                ParentDiscoverySource::ReverseTemplateLink
                    | ParentDiscoverySource::ExplicitOverride
            )
        );
        if minimize && promoted && self.parent_style_id.is_some() {
            let Some(parent_style_id) = self.parent_style_id.as_ref() else {
                return Ok(style);
            };
            // Evidence-driven minimal wrapper: info + extends only, no diffs.
            let extends: citum_schema::style_base::StyleReference =
                serde_yaml::from_value(Value::String(parent_style_id.clone()))?;
            return Ok(Style {
                info: style.info,
                extends: Some(extends),
                raw_yaml: None,
                ..Default::default()
            });
        }

        // Fall back to standard diff-based wrapping.
        self.apply_to_migrated_style(style)
    }

    /// Return the explicit migration artifact plan derived from repo truth.
    #[must_use]
    pub fn output_plan(&self) -> MigrationOutputPlan {
        let Some(parent_style_id) = self.parent_style_id.clone() else {
            return MigrationOutputPlan::Standalone;
        };

        match (self.semantic_class, self.implementation_form) {
            (
                SemanticClass::Profile | SemanticClass::Journal,
                ImplementationForm::ConfigWrapper,
            ) => MigrationOutputPlan::ExistingWrapper {
                parent_style_id,
                implementation_form: self.implementation_form,
                preserve_template_deltas: false,
            },
            (SemanticClass::Journal, ImplementationForm::StructuralWrapper) => {
                MigrationOutputPlan::ExistingWrapper {
                    parent_style_id,
                    implementation_form: self.implementation_form,
                    preserve_template_deltas: true,
                }
            }
            // A legacy CSL ID that aliases a registered Base/Profile/Journal
            // should migrate to a thin wrapper that extends the canonical id,
            // rather than duplicating the canonical style's templates.
            (
                SemanticClass::Base | SemanticClass::Profile | SemanticClass::Journal,
                ImplementationForm::Alias,
            ) => MigrationOutputPlan::ExistingWrapper {
                parent_style_id,
                implementation_form: self.implementation_form,
                preserve_template_deltas: false,
            },
            (
                SemanticClass::Base | SemanticClass::Profile | SemanticClass::Journal,
                ImplementationForm::TemplateDescendant,
            ) => MigrationOutputPlan::ExistingWrapper {
                parent_style_id,
                implementation_form: self.implementation_form,
                preserve_template_deltas: true,
            },
            _ => MigrationOutputPlan::Standalone,
        }
    }
}

fn map_style_kind(kind: Option<&StyleKind>) -> SemanticClass {
    match kind {
        Some(StyleKind::Base) => SemanticClass::Base,
        Some(StyleKind::Profile) => SemanticClass::Profile,
        Some(StyleKind::Journal) => SemanticClass::Journal,
        Some(StyleKind::Independent) => SemanticClass::Independent,
        None => SemanticClass::Unknown,
    }
}

fn load_current_style(
    repo_root: &Path,
    style_id: &str,
    exact_entry: Option<&citum_schema::RegistryEntry>,
) -> Result<Option<Style>, LineageError> {
    let local_path = repo_root.join("styles").join(format!("{style_id}.yaml"));
    if local_path.exists() {
        return Ok(Some(load_style_from_path(&local_path)?));
    }

    if let Some(entry) = exact_entry
        && let Some(name) = &entry.builtin
        && let Some(style) = embedded::get_embedded_style(name)
    {
        return Ok(Some(style?));
    }

    Ok(None)
}

fn load_style_by_id(repo_root: &Path, style_id: &str) -> Result<Style, LineageError> {
    let local_path = repo_root.join("styles").join(format!("{style_id}.yaml"));
    if local_path.exists() {
        return load_style_from_path(&local_path);
    }

    if let Some(style) = embedded::get_embedded_style(style_id) {
        return Ok(style?);
    }

    let embedded_path = repo_root
        .join("styles")
        .join("embedded")
        .join(format!("{style_id}.yaml"));
    if embedded_path.exists() {
        return load_style_from_path(&embedded_path);
    }

    Err(LineageError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("unable to resolve parent style `{style_id}`"),
    )))
}

fn load_style_from_path(path: &Path) -> Result<Style, LineageError> {
    let yaml = fs::read_to_string(path)?;
    Ok(Style::from_yaml_str(&yaml)?)
}

/// The relationship type of a parent link that resolved against the registry.
/// Returned alongside the canonical id so the evidence record can classify
/// the discovery source without re-scanning `legacy_links` (which would race
/// the same first-link-wins resolution and risk misclassifying when an
/// earlier href doesn't resolve).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentLinkRel {
    Template,
    IndependentParent,
}

fn resolve_parent_link_target(
    registry: &StyleRegistry,
    links: &[InfoLink],
) -> Option<(String, ParentLinkRel)> {
    links.iter().find_map(|link| {
        let rel = link.rel.as_deref()?;
        let parent_rel = match rel {
            "template" => ParentLinkRel::Template,
            "independent-parent" => ParentLinkRel::IndependentParent,
            _ => return None,
        };
        let linked_id = zotero_style_id(&link.href)?;
        resolve_registry_id(registry, linked_id).map(|id| (id, parent_rel))
    })
}

/// Scan embedded canonical styles for a reverse `<info><link rel="template">`
/// that targets `legacy_id`. When found, the embedded style is a candidate
/// ancestor for output-driven compression of the legacy style.
///
/// Returns the canonical ID of the discovered parent, or `None` when no
/// embedded style declares the legacy id as its template source.
fn discover_reverse_template_parent(registry: &StyleRegistry, legacy_id: &str) -> Option<String> {
    for entry in &registry.styles {
        let Some(builtin) = entry.builtin.as_deref() else {
            continue;
        };
        let Some(loaded) = embedded::get_embedded_style(builtin) else {
            continue;
        };
        let Ok(style) = loaded else {
            continue;
        };
        // CSL-derived styles store their original `<info><link>` entries under
        // `info.source.links`. Native or biblatex-derived styles have no
        // `info.source` and contribute no reverse template signal.
        let Some(source) = style.info.source.as_ref() else {
            continue;
        };
        for link in &source.links {
            let Some(rel) = link.rel.as_deref() else {
                continue;
            };
            if rel != "template" {
                continue;
            }
            let Some(linked) = zotero_style_id(&link.href) else {
                continue;
            };
            if linked == legacy_id {
                return Some(entry.id.clone());
            }
        }
    }
    None
}

fn zotero_style_id(href: &str) -> Option<&str> {
    href.strip_prefix("http://www.zotero.org/styles/")
        .or_else(|| href.strip_prefix("https://www.zotero.org/styles/"))
        .filter(|id| !id.is_empty())
}

fn resolve_registry_id(registry: &StyleRegistry, style_id: &str) -> Option<String> {
    registry
        .styles
        .iter()
        .find(|entry| entry.id == style_id)
        .or_else(|| {
            registry
                .styles
                .iter()
                .find(|entry| entry.aliases.iter().any(|alias| alias == style_id))
        })
        .map(|entry| entry.id.clone())
}

fn derive_implementation_form(style: &Style) -> ImplementationForm {
    if style.extends.is_none() {
        return ImplementationForm::Standalone;
    }

    if has_template_bearing_structure(style) {
        ImplementationForm::StructuralWrapper
    } else {
        ImplementationForm::ConfigWrapper
    }
}

fn has_template_bearing_structure(style: &Style) -> bool {
    if style.templates.is_some() || yaml_path_present(style.raw_yaml.as_ref(), &["templates"]) {
        return true;
    }

    TEMPLATE_BEARING_PATHS
        .iter()
        .any(|path| yaml_path_present(style.raw_yaml.as_ref(), path))
}

const TEMPLATE_BEARING_PATHS: [&[&str]; 9] = [
    &["templates"],
    &["citation", "template"],
    &["citation", "type-variants"],
    &["citation", "integral", "template"],
    &["citation", "integral", "type-variants"],
    &["citation", "non-integral", "template"],
    &["citation", "non-integral", "type-variants"],
    &["bibliography", "template"],
    &["bibliography", "type-variants"],
];

// Paths whose mappings deserialize as an untagged `Preset | Explicit` enum.
// A partial diff at one of these paths can produce a fragment that satisfies
// neither variant (e.g. an `Explicit` `DateConfig` is missing required fields),
// so emit the full child value as an atomic unit when it differs from parent.
const ATOMIC_CONFIG_LEAVES: &[&str] =
    &["dates", "contributors", "titles", "locators", "processing"];
const ATOMIC_CONFIG_PARENTS: &[&[&str]] = &[
    &["options"],
    &["citation", "options"],
    &["bibliography", "options"],
];

fn yaml_path_present(value: Option<&Value>, path: &[&str]) -> bool {
    let Some(mut current) = value else {
        return false;
    };
    for segment in path {
        let Value::Mapping(map) = current else {
            return false;
        };
        let key = Value::String((*segment).to_string());
        let Some(next) = map.get(&key) else {
            return false;
        };
        current = next;
    }
    true
}

fn diff_value(
    child: &Value,
    parent: &Value,
    path: &mut Vec<String>,
    exclude_template_paths: bool,
) -> Option<Value> {
    if child == parent {
        return None;
    }

    match (child, parent) {
        (Value::Mapping(child_map), Value::Mapping(parent_map)) => {
            let mut diff = Mapping::new();
            for (key, child_value) in child_map {
                let Some(segment) = key.as_str() else {
                    diff.insert(key.clone(), child_value.clone());
                    continue;
                };
                path.push(segment.to_string());
                if exclude_template_paths && is_template_bearing_path(path) {
                    path.pop();
                    continue;
                }

                if is_atomic_config_path(path) {
                    if !parent_map.get(key).is_some_and(|p| p == child_value) {
                        diff.insert(key.clone(), child_value.clone());
                    }
                    path.pop();
                    continue;
                }

                match parent_map.get(key) {
                    Some(parent_value) => {
                        if let Some(child_diff) =
                            diff_value(child_value, parent_value, path, exclude_template_paths)
                        {
                            diff.insert(key.clone(), child_diff);
                        }
                    }
                    None => {
                        diff.insert(key.clone(), child_value.clone());
                    }
                }
                path.pop();
            }

            for (key, _) in parent_map {
                let Some(segment) = key.as_str() else {
                    continue;
                };
                if !child_map.contains_key(key) {
                    path.push(segment.to_string());
                    tracing::debug!(
                        "diff_value: child dropped parent key at {}; wrapper will silently inherit it from parent",
                        path.join(".")
                    );
                    path.pop();
                }
            }

            if diff.is_empty() {
                None
            } else {
                Some(Value::Mapping(diff))
            }
        }
        _ => Some(child.clone()),
    }
}

fn is_template_bearing_path(path: &[String]) -> bool {
    TEMPLATE_BEARING_PATHS.iter().any(|candidate| {
        candidate.len() == path.len()
            && candidate
                .iter()
                .zip(path.iter())
                .all(|(expected, actual)| *expected == actual)
    })
}

fn is_atomic_config_path(path: &[String]) -> bool {
    let Some((leaf, parent)) = path.split_last() else {
        return false;
    };
    if !ATOMIC_CONFIG_LEAVES.contains(&leaf.as_str()) {
        return false;
    }
    ATOMIC_CONFIG_PARENTS.iter().any(|candidate| {
        candidate.len() == parent.len()
            && candidate
                .iter()
                .zip(parent.iter())
                .all(|(expected, actual)| *expected == actual)
    })
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
    use citum_schema::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, SimpleVariable,
        TemplateComponent, TemplateContributor, TemplateDate, TemplateVariable,
    };
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate dir should have workspace root")
            .to_path_buf()
    }

    fn minimal_migrated_style() -> Style {
        Style {
            info: citum_schema::StyleInfo {
                title: Some("Migrated Test".to_string()),
                id: Some("https://example.org/migrated-test".to_string()),
                ..Default::default()
            },
            citation: Some(citum_schema::CitationSpec {
                template: Some(vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Short,
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            }),
            bibliography: Some(citum_schema::BibliographySpec {
                template: Some(vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Long,
                        ..Default::default()
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Doi,
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn resolves_embedded_profile_as_config_wrapper() {
        let lineage =
            StyleLineage::resolve("styles-legacy/elsevier-harvard.csl", &repo_root(), &[]).unwrap();

        assert_eq!(lineage.style_id, "elsevier-harvard");
        assert_eq!(lineage.semantic_class, SemanticClass::Profile);
        assert_eq!(
            lineage.implementation_form,
            ImplementationForm::ConfigWrapper
        );
        assert_eq!(
            lineage.parent_style_id.as_deref(),
            Some("elsevier-harvard-core")
        );
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::ExistingWrapper {
                parent_style_id: "elsevier-harvard-core".to_string(),
                implementation_form: ImplementationForm::ConfigWrapper,
                preserve_template_deltas: false,
            }
        );
    }

    #[test]
    fn resolves_exemplar_structural_wrapper_from_local_style() {
        let lineage = StyleLineage::resolve(
            "styles-legacy/entomological-society-of-america.csl",
            &repo_root(),
            &[],
        )
        .unwrap();

        assert_eq!(lineage.semantic_class, SemanticClass::Journal);
        assert_eq!(
            lineage.implementation_form,
            ImplementationForm::StructuralWrapper
        );
        assert_eq!(lineage.parent_style_id.as_deref(), Some("elsevier-harvard"));
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::ExistingWrapper {
                parent_style_id: "elsevier-harvard".to_string(),
                implementation_form: ImplementationForm::StructuralWrapper,
                preserve_template_deltas: true,
            }
        );
    }

    #[test]
    fn resolves_journal_structural_wrapper_from_local_style() {
        let lineage = StyleLineage::resolve(
            "styles-legacy/american-society-of-mechanical-engineers.csl",
            &repo_root(),
            &[],
        )
        .unwrap();

        assert_eq!(lineage.semantic_class, SemanticClass::Journal);
        assert_eq!(
            lineage.implementation_form,
            ImplementationForm::StructuralWrapper
        );
        assert_eq!(lineage.parent_style_id.as_deref(), Some("ieee"));
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::ExistingWrapper {
                parent_style_id: "ieee".to_string(),
                implementation_form: ImplementationForm::StructuralWrapper,
                preserve_template_deltas: true,
            }
        );
    }

    #[test]
    fn resolves_unknown_style_as_unknown() {
        let lineage = StyleLineage::resolve(
            "styles-legacy/definitely-unknown-style.csl",
            &repo_root(),
            &[],
        )
        .unwrap();

        assert_eq!(lineage.semantic_class, SemanticClass::Unknown);
        assert_eq!(lineage.implementation_form, ImplementationForm::Unknown);
        assert!(lineage.parent_style_id.is_none());
        assert_eq!(lineage.output_plan(), MigrationOutputPlan::Standalone);
    }

    #[test]
    fn config_wrapper_output_sets_extends_and_strips_templates() {
        let lineage =
            StyleLineage::resolve("styles-legacy/elsevier-harvard.csl", &repo_root(), &[]).unwrap();
        let rewritten = lineage
            .apply_to_migrated_style(minimal_migrated_style())
            .unwrap();

        assert_eq!(
            rewritten.extends.as_ref().map(|base| base.key()),
            Some("elsevier-harvard-core")
        );
        assert!(
            rewritten
                .citation
                .as_ref()
                .and_then(|citation| citation.template.as_ref())
                .is_none(),
            "config-wrapper profiles must not keep local citation templates"
        );
        assert!(
            rewritten
                .bibliography
                .as_ref()
                .and_then(|bibliography| bibliography.template.as_ref())
                .is_none(),
            "config-wrapper profiles must not keep local bibliography templates"
        );
    }

    #[test]
    fn aliased_legacy_style_resolves_as_existing_wrapper() {
        // `styles-legacy/apa.csl` declares its CSL ID as `apa`, which the
        // registry exposes as an alias of the canonical `apa-7th` Base entry.
        // The migration plan must route through `ExistingWrapper` so the
        // converter emits `extends: apa-7th` instead of a duplicated standalone.
        let lineage = StyleLineage::resolve("styles-legacy/apa.csl", &repo_root(), &[]).unwrap();
        assert_eq!(lineage.semantic_class, SemanticClass::Base);
        assert_eq!(lineage.implementation_form, ImplementationForm::Alias);
        assert_eq!(lineage.parent_style_id.as_deref(), Some("apa-7th"));

        let rewritten = lineage
            .apply_to_migrated_style(minimal_migrated_style())
            .unwrap();
        assert_eq!(
            rewritten.extends.as_ref().map(|base| base.key()),
            Some("apa-7th"),
        );
        assert!(
            rewritten
                .bibliography
                .as_ref()
                .and_then(|bibliography| bibliography.template.as_ref())
                .is_none(),
            "alias wrapper should not duplicate the canonical style's templates",
        );
    }

    #[test]
    fn template_descendant_resolves_canonical_parent_alias() {
        let links = vec![InfoLink {
            href: "http://www.zotero.org/styles/chicago-author-date".to_string(),
            rel: Some("template".to_string()),
        }];
        let lineage =
            StyleLineage::resolve("styles-legacy/anglia.csl", &repo_root(), &links).unwrap();

        assert_eq!(lineage.semantic_class, SemanticClass::Journal);
        assert_eq!(
            lineage.implementation_form,
            ImplementationForm::TemplateDescendant
        );
        assert_eq!(
            lineage.parent_style_id.as_deref(),
            Some("chicago-author-date-18th")
        );
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::ExistingWrapper {
                parent_style_id: "chicago-author-date-18th".to_string(),
                implementation_form: ImplementationForm::TemplateDescendant,
                preserve_template_deltas: true,
            }
        );
    }

    #[test]
    fn unresolved_template_parent_preserves_standalone_output() {
        let links = vec![InfoLink {
            href: "http://www.zotero.org/styles/apa-6th-edition".to_string(),
            rel: Some("template".to_string()),
        }];
        let lineage = StyleLineage::resolve(
            "styles-legacy/effective-altruism-wiki.csl",
            &repo_root(),
            &links,
        )
        .unwrap();

        assert_eq!(lineage.semantic_class, SemanticClass::Unknown);
        assert_eq!(lineage.implementation_form, ImplementationForm::Unknown);
        assert!(lineage.parent_style_id.is_none());
        assert_eq!(lineage.output_plan(), MigrationOutputPlan::Standalone);
    }

    #[test]
    fn atomic_config_path_emits_full_child_value() {
        // Diffs at `options.dates` (and similar untagged-enum config paths)
        // must emit the full child mapping when it differs, because a partial
        // mapping does not satisfy the `DateConfigEntry::Explicit` variant.
        let child = serde_yaml::from_str::<Value>(
            "options:\n  dates:\n    month: long\n    range-delimiter: \"\\u2013\"\n",
        )
        .unwrap();
        let parent =
            serde_yaml::from_str::<Value>("options:\n  dates:\n    month: long\n").unwrap();
        let diff = diff_value(&child, &parent, &mut Vec::new(), false).unwrap();
        let Value::Mapping(options) = diff.get(Value::String("options".to_string())).unwrap()
        else {
            panic!("expected options mapping in diff");
        };
        let dates = options.get(Value::String("dates".to_string())).unwrap();
        assert_eq!(
            dates,
            child
                .get(Value::String("options".to_string()))
                .unwrap()
                .get(Value::String("dates".to_string()))
                .unwrap(),
            "atomic config paths must emit the full child value",
        );
    }

    #[test]
    fn structural_wrapper_output_keeps_extends_and_structural_deltas() {
        let lineage = StyleLineage::resolve(
            "styles-legacy/american-society-of-mechanical-engineers.csl",
            &repo_root(),
            &[],
        )
        .unwrap();
        let rewritten = lineage
            .apply_to_migrated_style(minimal_migrated_style())
            .unwrap();

        assert_eq!(
            rewritten.extends.as_ref().map(|base| base.key()),
            Some("ieee")
        );
        assert!(
            rewritten
                .citation
                .as_ref()
                .and_then(|citation| citation.template.as_ref())
                .is_some(),
            "structural wrappers should preserve local structural citation deltas"
        );
    }

    #[test]
    fn reverse_template_link_discovers_apa_7th_for_apa_6th_edition() {
        // `styles/embedded/apa-7th.yaml` declares `apa-6th-edition` as its
        // historical template source via `info.source.links`. Lineage
        // resolution should surface this inertly as a family candidate when
        // the legacy file has no other parent link of its own.
        let lineage =
            StyleLineage::resolve("styles-legacy/apa-6th-edition.csl", &repo_root(), &[]).unwrap();

        assert!(
            lineage.parent_style_id.is_none(),
            "discovery must be inert by default; no parent promoted"
        );
        assert_eq!(
            lineage.family_candidate.as_deref(),
            Some("apa-7th"),
            "expected reverse template link to surface apa-7th as family candidate"
        );
        // Without promotion, the output plan stays Standalone so the
        // converter's default behavior is unchanged for callers that don't
        // opt in via `--family-candidate`.
        assert_eq!(lineage.output_plan(), MigrationOutputPlan::Standalone);
    }

    #[test]
    fn promote_family_candidate_routes_through_template_descendant_wrapper() {
        let mut lineage =
            StyleLineage::resolve("styles-legacy/apa-6th-edition.csl", &repo_root(), &[]).unwrap();

        let promoted = lineage
            .promote_family_candidate(&repo_root(), None)
            .expect("promotion should succeed when candidate is present");
        assert!(promoted, "expected the discovered candidate to be promoted");
        assert_eq!(lineage.parent_style_id.as_deref(), Some("apa-7th"));
        assert_eq!(lineage.semantic_class, SemanticClass::Base);
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::ExistingWrapper {
                parent_style_id: "apa-7th".to_string(),
                implementation_form: ImplementationForm::TemplateDescendant,
                preserve_template_deltas: true,
            }
        );
    }

    #[test]
    fn promote_family_candidate_returns_false_when_no_candidate_discovered() {
        let mut lineage = StyleLineage::resolve(
            "styles-legacy/definitely-unknown-style.csl",
            &repo_root(),
            &[],
        )
        .unwrap();
        let promoted = lineage
            .promote_family_candidate(&repo_root(), None)
            .expect("promotion should not error when no candidate is present");
        assert!(
            !promoted,
            "promotion must return false when no candidate was discovered"
        );
        assert!(lineage.parent_style_id.is_none());
    }

    // ── Regime guard tests ──────────────────────────────────────────────────

    #[test]
    fn regime_guard_drops_template_link_parent_on_regime_mismatch() {
        // given: bio-protocol.csl is a numeric CSL style that declares a
        // `<link rel="template">` pointing at APA (author-date).
        // Parse the real CSL file so the links reflect what the migration sees.
        let csl_path = repo_root().join("styles-legacy/bio-protocol.csl");
        if !csl_path.exists() {
            return;
        }
        let text = std::fs::read_to_string(&csl_path).expect("bio-protocol.csl must exist");
        let doc = roxmltree::Document::parse(&text).expect("bio-protocol.csl must parse");
        let legacy = csl_legacy::parser::parse_style(doc.root_element())
            .expect("bio-protocol.csl must be valid CSL");
        let detected = crate::options_extractor::processing::detect_processing_mode(&legacy);

        // when: lineage is resolved with the real links and the regime guard applied
        let lineage = StyleLineage::resolve(
            "styles-legacy/bio-protocol.csl",
            &repo_root(),
            &legacy.info.links,
        )
        .unwrap()
        .apply_regime_guard(detected.as_ref());

        // then: the incompatible author-date parent is dropped; style is standalone
        assert!(
            lineage.parent_style_id.is_none(),
            "numeric style with author-date template link must not inherit author-date parent; \
             got: {:?}",
            lineage.parent_style_id
        );
        assert_eq!(
            lineage.semantic_class,
            SemanticClass::Independent,
            "class must reset to Independent after guard drops the parent"
        );
        assert_eq!(
            lineage.output_plan(),
            MigrationOutputPlan::Standalone,
            "output plan must be Standalone after regime guard fires"
        );
    }

    #[test]
    fn regime_guard_preserves_same_regime_template_link() {
        // given: apa-6th-edition.csl has no template link but if we call
        // apply_regime_guard with a matching detected regime, nothing changes.
        // Use a simple case: resolve anglia (author-date child), pass AuthorDate.
        let csl_path = repo_root().join("styles-legacy/anglia-ruskin-university.csl");
        if !csl_path.exists() {
            // Skip if the file isn't present in this working copy.
            return;
        }
        let text = std::fs::read_to_string(&csl_path).unwrap();
        let doc = roxmltree::Document::parse(&text).unwrap();
        let legacy = csl_legacy::parser::parse_style(doc.root_element()).unwrap();
        let detected = crate::options_extractor::processing::detect_processing_mode(&legacy);

        let lineage_before = StyleLineage::resolve(
            "styles-legacy/anglia-ruskin-university.csl",
            &repo_root(),
            &legacy.info.links,
        )
        .unwrap();
        let parent_before = lineage_before.parent_style_id.clone();

        let lineage_after = lineage_before.apply_regime_guard(detected.as_ref());

        // The guard must not change anything when the regime is compatible.
        assert_eq!(
            lineage_after.parent_style_id, parent_before,
            "same-regime template link must not be dropped by the regime guard"
        );
    }

    #[test]
    fn build_evidence_reports_reverse_template_discovery() {
        let lineage =
            StyleLineage::resolve("styles-legacy/apa-6th-edition.csl", &repo_root(), &[]).unwrap();
        let evidence = lineage.build_evidence(MigrationEvidenceParts {
            standalone_lines: 5662,
            emitted_form: crate::evidence::EmittedForm::Standalone,
            emitted_lines: 5662,
            minimization_decision: crate::evidence::MinimizationDecisionAudit::none(),
            preserved_template_paths: Vec::new(),
            discarded_template_paths: Vec::new(),
            measured_selection: None,
            diagnostics: Vec::new(),
        });
        assert_eq!(
            evidence.registry_alias_status,
            crate::evidence::RegistryAliasStatus::None
        );
        let candidate = evidence
            .discovered_parents
            .iter()
            .find(|c| c.canonical_id == "apa-7th")
            .expect("evidence should include apa-7th as a discovered parent");
        assert_eq!(
            candidate.source,
            crate::evidence::ParentDiscoverySource::ReverseTemplateLink
        );
        assert!(!evidence.compressed());
    }

    #[test]
    fn minimize_wrapper_emits_info_and_extends_only() {
        // When promote_family_candidate is called and minimize is true,
        // apply_to_migrated_style_minimized should emit only info + extends,
        // dropping all template-bearing deltas.
        let mut lineage =
            StyleLineage::resolve("styles-legacy/apa-6th-edition.csl", &repo_root(), &[]).unwrap();

        let promoted = lineage
            .promote_family_candidate(&repo_root(), None)
            .expect("promotion should succeed");
        assert!(promoted);

        let mut test_style = minimal_migrated_style();
        test_style.info = citum_schema::StyleInfo {
            title: Some("APA 6th Edition".to_string()),
            id: Some("apa-6th-edition".to_string()),
            ..Default::default()
        };

        let minimized = lineage
            .apply_to_migrated_style_minimized(test_style.clone(), true)
            .expect("minimization should succeed");

        assert!(
            minimized.extends.is_some(),
            "minimized wrapper must have extends set"
        );
        // Verify the extends points to apa-7th by converting to YAML and back
        let yaml_val = serde_yaml::to_value(&minimized.extends).unwrap();
        let extends_str = yaml_val.as_str();
        assert_eq!(
            extends_str,
            Some("apa-7th"),
            "expected extends to reference apa-7th"
        );
        assert!(
            minimized.bibliography.is_none(),
            "minimized wrapper should not carry bibliography template diff"
        );
        assert!(
            minimized.citation.is_none(),
            "minimized wrapper should not carry citation template diff"
        );
    }
}
