/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Measured inferred-vs-XML citation template selection.
//!
//! The template inferrer's confidence score is computed against its own
//! reconstruction surface, so it can rate a citation template highly even
//! when the template renders badly. This module settles the choice
//! empirically at migration time: render both candidate styles with the
//! Citum engine over the embedded fixture items, compare each output to the
//! citeproc-js reference rendering of the original CSL style, and keep the
//! candidate that matches better.
//!
//! Citation selection uses the oracle's token-similarity fallback directly,
//! because citation labels can use punctuation as meaningful separators.
//! Bibliography selection uses normalized exact comparison first, including
//! the oracle's case-sensitive rejection for case-only mismatches, then the
//! same token-similarity fallback. Selection therefore optimizes the same
//! measure the oracle reports without hiding strict bibliography case errors.

use crate::error::MigrateError;
use crate::js_runtime::{self, EmbeddedTemplateRuntime, FixtureSet};
use citum_engine::Processor;
use citum_engine::reference::{Bibliography, Citation};
use citum_engine::render::bibliography::render_entry_body_with_format;
use citum_engine::render::plain::PlainText;
use citum_schema::Style;
use citum_schema::options::contributors::NameForm;
use citum_schema::template::{
    ContributorForm, ContributorRole, DateForm, NumberVariable, SimpleVariable, TemplateComponent,
    TemplateVariant, TitleType, TypeSelector,
};
use std::collections::{BTreeMap, BTreeSet};
use std::panic::AssertUnwindSafe;
use std::path::Path;

/// Outcome of measured citation-candidate scoring.
#[derive(Debug, Clone)]
pub struct MeasuredCitationSelection {
    /// Selected style after citation-candidate scoring.
    pub selected_style: Style,
    /// Name of the candidate selected by measured scoring.
    pub selected_candidate: String,
    /// Whether the XML-compiled candidate beat all other candidates.
    pub use_xml: bool,
    /// Items the selected candidate passed at the oracle threshold.
    pub selected_passes: usize,
    /// Items the inferred candidate passed at the oracle threshold.
    pub inferred_passes: usize,
    /// Items the XML-compiled candidate passed at the oracle threshold.
    pub xml_passes: usize,
    /// Number of fixture items that produced a citeproc reference citation.
    pub items: usize,
    /// Held-out validation of the selected candidate, when available.
    pub heldout: Option<HeldOutValidation>,
    /// Mutation rounds the synthesis loop accepted for this selection.
    pub synthesis_rounds: usize,
    /// Names of accepted synthesis mutations, in acceptance order.
    pub accepted_mutations: Vec<String>,
}

/// Held-out validation pass-rate for a selected candidate.
///
/// Rendered on the held-out fixture set after selection; the held-out items
/// never participate in inference or candidate scoring. Reporting-only for
/// the bounded selector; the synthesis loop turns regressions into a
/// rejection gate (`docs/specs/OUTPUT_DRIVEN_TEMPLATE_SYNTHESIS.md`).
#[derive(Debug, Clone, Copy)]
pub struct HeldOutValidation {
    /// Held-out scenarios the selected candidate passed at the threshold.
    pub passes: usize,
    /// Held-out scenarios that produced a citeproc reference.
    pub items: usize,
}

/// Caps on measured candidate generation so scoring stays bounded.
///
/// The defaults are sized above the current candidate families, so the
/// bounded selector's behavior is unchanged; they exist so candidate
/// generation cannot grow unbounded as mutation families are added, and the
/// synthesis loop reuses the same caps to keep each mutation round
/// non-combinatorial (`docs/specs/OUTPUT_DRIVEN_TEMPLATE_SYNTHESIS.md`).
#[derive(Debug, Clone, Copy)]
pub struct CandidateBudget {
    /// Maximum generated candidates per mutation family.
    pub max_per_family: usize,
    /// Maximum candidates scored per selection, seed candidates included.
    pub max_total: usize,
}

impl Default for CandidateBudget {
    fn default() -> Self {
        Self {
            max_per_family: 24,
            max_total: 32,
        }
    }
}

impl CandidateBudget {
    /// Truncate generated mutations so seeds plus mutations stay within the
    /// total budget.
    pub(crate) fn truncate_mutations<T>(self, mutations: &mut Vec<T>, seed_count: usize) {
        mutations.truncate(self.max_total.saturating_sub(seed_count));
    }
}

/// Outcome of measured bibliography-candidate scoring.
#[derive(Debug, Clone)]
pub struct MeasuredBibliographySelection {
    /// Selected style after bibliography-candidate scoring.
    pub selected_style: Style,
    /// Name of the candidate selected by measured scoring.
    pub selected_candidate: String,
    /// Candidate family selected by measured scoring, if this was a generated patch.
    pub selected_family: Option<String>,
    /// Section affected by the selected candidate, if this was a generated patch.
    pub selected_section: Option<String>,
    /// Reference types affected by the selected candidate.
    pub selected_affected_types: Vec<String>,
    /// Whether the XML-compiled candidate beat all other candidates.
    pub use_xml: bool,
    /// Items the selected candidate passed at the oracle threshold.
    pub selected_passes: usize,
    /// Items the inferred candidate passed at the oracle threshold.
    pub inferred_passes: usize,
    /// Items the XML-compiled candidate passed at the oracle threshold.
    pub xml_passes: usize,
    /// Number of fixture items that produced a citeproc reference entry.
    pub items: usize,
    /// Held-out validation of the selected candidate, when available.
    pub heldout: Option<HeldOutValidation>,
    /// Mutation rounds the synthesis loop accepted for this selection.
    pub synthesis_rounds: usize,
    /// Names of accepted synthesis mutations, in acceptance order.
    pub accepted_mutations: Vec<String>,
}

/// Per-item pass threshold, mirroring the oracle's `similarityThreshold`.
const PASS_THRESHOLD: f64 = 0.60;

/// Margin (in summed similarity) a candidate must clear to win a pass-count
/// tie, so tiny floating-point noise cannot flip the inferred status quo.
const TIE_MARGIN: f64 = 0.05;

/// Fixture types that often inherit the style's default bibliography shape
/// more faithfully than an overfit migrated type variant.
const LOCAL_DEFAULT_TYPES: &[&str] = &[
    "article-magazine",
    "article-newspaper",
    "book",
    "broadcast",
    "chapter",
    "dataset",
    "entry-encyclopedia",
    "interview",
    "legal_case",
    "manuscript",
    "motion_picture",
    "paper-conference",
    "patent",
    "personal_communication",
    "report",
    "thesis",
    "webpage",
];

/// Date-heavy media and online fixture types where CSL often expects more
/// than a bare issued year in bibliography output.
const MEDIA_FULL_DATE_TYPES: &[&str] = &[
    "interview",
    "broadcast",
    "motion_picture",
    "patent",
    "webpage",
    "dataset",
    "report",
];

/// Score both candidate styles against citeproc-js reference citations.
///
/// `inferred_style` is the standalone style assembled with the inferred
/// citation template; `xml_style` is the same style assembled down the
/// XML-compilation path. The XML candidate wins only when it passes strictly
/// more items, or ties on passes with a clearly higher summed similarity.
///
/// # Errors
///
/// Returns an error when the embedded runtime, fixture data, or reference
/// rendering is unavailable; callers should treat that as "keep the
/// inferred candidate".
pub fn select(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<MeasuredCitationSelection, MigrateError> {
    crate::synthesis::synthesize_citation_rounds(
        inferred_style,
        xml_style,
        style_name,
        style_xml,
        workspace_root,
        0,
    )
}

/// Validate the selected citation candidate on the held-out fixture set.
///
/// Reporting-only: failures to load or render the held-out set yield `None`
/// rather than disturbing the selection outcome.
pub(crate) fn heldout_citation_validation(
    runtime: &mut EmbeddedTemplateRuntime,
    style_name: &str,
    style_xml: &str,
    selected_style: &Style,
    workspace_root: &Path,
) -> Option<HeldOutValidation> {
    let reference_json = runtime
        .render_citation_strings(style_name, style_xml, FixtureSet::HeldOut)
        .ok()?;
    let references: BTreeMap<String, Vec<Option<String>>> =
        serde_json::from_str(&reference_json).ok()?;
    let bibliography = heldout_bibliography(workspace_root).ok()?;
    let score = score_candidate(selected_style, &bibliography, &references);
    Some(HeldOutValidation {
        passes: score.passes,
        items: score.items,
    })
}

/// Format the held-out pass-rate for selection debug output.
pub(crate) fn heldout_debug_suffix(heldout: Option<HeldOutValidation>) -> String {
    match heldout {
        Some(validation) => format!(
            "; held-out {}/{} passes",
            validation.passes, validation.items
        ),
        None => "; held-out unavailable".to_string(),
    }
}

/// Score bibliography candidates against citeproc-js reference entries.
///
/// `inferred_style` is the standalone style assembled with the inferred
/// bibliography template; `xml_style` is the same style assembled down the
/// XML-compilation path. Additional output-driven mutation candidates are
/// derived from the inferred style. A candidate wins only when it passes
/// strictly more items, or ties on passes with a clearly higher summed
/// similarity.
///
/// # Errors
///
/// Returns an error when the embedded runtime, fixture data, or reference
/// rendering is unavailable; callers should treat that as "keep the
/// inferred candidate".
pub fn select_bibliography(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<MeasuredBibliographySelection, MigrateError> {
    crate::synthesis::synthesize_bibliography_rounds(
        inferred_style,
        xml_style,
        style_name,
        style_xml,
        workspace_root,
        0,
    )
}

/// Validate the selected bibliography candidate on the held-out fixture set.
///
/// Reporting-only: failures to load or render the held-out set yield `None`
/// rather than disturbing the selection outcome.
pub(crate) fn heldout_bibliography_validation(
    runtime: &mut EmbeddedTemplateRuntime,
    style_name: &str,
    style_xml: &str,
    selected_style: &Style,
    workspace_root: &Path,
) -> Option<HeldOutValidation> {
    let reference_json = runtime
        .render_bibliography_strings(style_name, style_xml, FixtureSet::HeldOut)
        .ok()?;
    let references: BTreeMap<String, Option<String>> =
        serde_json::from_str(&reference_json).ok()?;
    let bibliography = heldout_bibliography(workspace_root).ok()?;
    let score = score_bibliography_candidate(selected_style, &bibliography, &references);
    Some(HeldOutValidation {
        passes: score.passes,
        items: score.items,
    })
}

/// Aggregate score for one candidate over the fixture items.
///
/// `items` always counts scored *reference* units, not rendered output
/// units, so the success path ([`score_bibliography_entries`]) and the
/// render-panic fallback ([`invalid_candidate_score`]) stay comparable.
#[derive(Clone, Copy)]
pub(crate) struct CandidateScore {
    pub(crate) passes: usize,
    pub(crate) similarity_sum: f64,
    pub(crate) items: usize,
}

/// Phase 1 acceptance rule: strictly more passes, or a pass-count tie with a
/// summed similarity clearly above the tie margin.
pub(crate) fn candidate_beats(candidate: &CandidateScore, incumbent: &CandidateScore) -> bool {
    candidate.passes > incumbent.passes
        || (candidate.passes == incumbent.passes
            && candidate.similarity_sum > incumbent.similarity_sum + TIE_MARGIN)
}

/// One named bibliography candidate with its provenance metadata.
#[derive(Debug, Clone)]
pub(crate) struct CandidateStyle {
    pub(crate) name: String,
    pub(crate) family: Option<CandidateFamily>,
    pub(crate) affected_section: Option<AffectedSection>,
    pub(crate) affected_types: Vec<String>,
    pub(crate) style: Style,
}

impl CandidateStyle {
    /// Wrap a seed style with no generated-patch metadata.
    pub(crate) fn baseline(name: &str, style: Style) -> Self {
        Self {
            name: name.to_string(),
            family: None,
            affected_section: None,
            affected_types: Vec::new(),
            style,
        }
    }

    /// Wrap the XML-compiled seed style.
    pub(crate) fn source_xml(style: Style) -> Self {
        Self {
            name: "xml".to_string(),
            family: Some(CandidateFamily::SourceXml),
            affected_section: Some(AffectedSection::Bibliography),
            affected_types: Vec::new(),
            style,
        }
    }

    fn from_patch(patch: &CandidatePatch, style: Style) -> Self {
        Self {
            name: patch.name.clone(),
            family: Some(patch.family),
            affected_section: Some(patch.affected_section),
            affected_types: patch.affected_types.clone(),
            style,
        }
    }

    /// Family label for debug output ("baseline" for seed candidates).
    pub(crate) fn family_label(&self) -> &'static str {
        self.family.map_or("baseline", CandidateFamily::as_str)
    }

    /// Section label for debug output ("none" for seed candidates).
    pub(crate) fn section_label(&self) -> &'static str {
        self.affected_section
            .map_or("none", AffectedSection::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateFamily {
    SourceXml,
    ContributorCase,
    TypeLocalDefault,
    DateGranularity,
    ArticleJournalSuppression,
}

impl CandidateFamily {
    /// Stable family label used in debug output and selection metadata.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SourceXml => "source-xml",
            Self::ContributorCase => "contributor-case",
            Self::TypeLocalDefault => "type-local-default",
            Self::DateGranularity => "date-granularity",
            Self::ArticleJournalSuppression => "article-journal-suppression",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AffectedSection {
    Bibliography,
}

impl AffectedSection {
    /// Stable section label used in debug output and selection metadata.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Bibliography => "bibliography",
        }
    }
}

#[derive(Debug, Clone)]
struct CandidatePatch {
    family: CandidateFamily,
    name: String,
    affected_section: AffectedSection,
    affected_types: Vec<String>,
    kind: CandidatePatchKind,
}

impl CandidatePatch {
    fn bibliography(
        family: CandidateFamily,
        name: impl Into<String>,
        affected_types: Vec<String>,
        kind: CandidatePatchKind,
    ) -> Self {
        Self {
            family,
            name: name.into(),
            affected_section: AffectedSection::Bibliography,
            affected_types,
            kind,
        }
    }

    fn apply(&self, style: &Style) -> Option<Style> {
        match &self.kind {
            CandidatePatchKind::BibliographyContributorsSmallCaps => {
                apply_bibliography_template_mutation(style, set_contributors_small_caps)
            }
            CandidatePatchKind::TypeLocalDefault { ref_type } => {
                apply_type_local_default(style, ref_type)
            }
            CandidatePatchKind::MediaIssuedFullDate { ref_types } => {
                apply_issued_full_date(style, ref_types)
            }
            CandidatePatchKind::ArticleJournalSuppress { matchers } => {
                apply_article_journal_suppression(style, matchers)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum CandidatePatchKind {
    BibliographyContributorsSmallCaps,
    TypeLocalDefault { ref_type: String },
    MediaIssuedFullDate { ref_types: Vec<String> },
    ArticleJournalSuppress { matchers: Vec<ComponentMatcher> },
}

#[derive(Debug, Clone, Copy)]
enum ComponentMatcher {
    PrimaryTitle,
    DoiOrUrl,
    Issue,
}

impl ComponentMatcher {
    fn matches(self, component: &TemplateComponent) -> bool {
        match self {
            Self::PrimaryTitle => {
                matches!(
                    component,
                    TemplateComponent::Title(title) if title.title == TitleType::Primary
                )
            }
            Self::DoiOrUrl => {
                matches!(
                    component,
                    TemplateComponent::Variable(variable)
                        if matches!(variable.variable, SimpleVariable::Doi | SimpleVariable::Url)
                )
            }
            Self::Issue => {
                matches!(
                    component,
                    TemplateComponent::Number(number)
                        if matches!(number.number, NumberVariable::Issue)
                )
            }
        }
    }
}

/// Load the embedded fixture items as an engine bibliography.
pub(crate) fn fixture_bibliography(workspace_root: &Path) -> Result<Bibliography, MigrateError> {
    bibliography_from_fixtures(js_runtime::load_fixtures(workspace_root)?)
}

/// Collect recoverable CSL-JSON conversion warnings from migration fixture sets.
///
/// # Errors
///
/// Returns an error when either fixture file is unavailable or an item cannot
/// be parsed as CSL-JSON.
pub fn fixture_conversion_diagnostics(
    workspace_root: &Path,
) -> Result<Vec<crate::evidence::MigrationDiagnostic>, MigrateError> {
    let mut diagnostics =
        fixture_diagnostics_from_value(js_runtime::load_fixtures(workspace_root)?)?;
    diagnostics.extend(fixture_diagnostics_from_value(
        js_runtime::load_heldout_fixtures(workspace_root)?,
    )?);
    Ok(diagnostics)
}

fn fixture_diagnostics_from_value(
    fixtures: serde_json::Value,
) -> Result<Vec<crate::evidence::MigrationDiagnostic>, MigrateError> {
    let map = fixtures.as_object().ok_or_else(|| {
        MigrateError::Parse("embedded fixture file is not a JSON object".to_string())
    })?;
    let mut diagnostics = Vec::new();
    for (id, item) in map {
        let mut legacy: csl_legacy::csl_json::Reference = serde_json::from_value(item.clone())
            .map_err(|err| {
                MigrateError::Parse(format!(
                    "fixture item {id} failed to parse as CSL JSON: {err}"
                ))
            })?;
        if legacy.id.is_empty() {
            legacy.id.clone_from(id);
        }
        diagnostics.extend(
            legacy
                .parse_note_field_hacks_with_diagnostics()
                .into_iter()
                .map(crate::evidence::MigrationDiagnostic::from),
        );
    }
    Ok(diagnostics)
}

/// Load the held-out fixture items as an engine bibliography.
fn heldout_bibliography(workspace_root: &Path) -> Result<Bibliography, MigrateError> {
    bibliography_from_fixtures(js_runtime::load_heldout_fixtures(workspace_root)?)
}

/// Convert a loaded fixture JSON object into an engine bibliography.
fn bibliography_from_fixtures(fixtures: serde_json::Value) -> Result<Bibliography, MigrateError> {
    let map = fixtures.as_object().ok_or_else(|| {
        MigrateError::Parse("embedded fixture file is not a JSON object".to_string())
    })?;

    let mut bibliography = Bibliography::new();
    for (id, item) in map {
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(item.clone())
            .map_err(|err| {
                MigrateError::Parse(format!(
                    "fixture item {id} failed to parse as CSL JSON: {err}"
                ))
            })?;
        bibliography.insert(id.clone(), legacy.into());
    }
    Ok(bibliography)
}

/// Render the citation scenarios for one candidate and score each against the
/// citeproc references at the oracle's similarity criterion.
///
/// Scenario indices follow the [`scenario_citation`] contract: bare first,
/// first with page locator, subsequent, ibid, and ibid with locator — so
/// locator placement and positional repeat-form failures both count against
/// a candidate.
pub(crate) fn score_candidate(
    style: &Style,
    bibliography: &Bibliography,
    references: &BTreeMap<String, Vec<Option<String>>>,
) -> CandidateScore {
    let processor = Processor::new(style.clone(), bibliography.clone());
    let mut score = CandidateScore {
        passes: 0,
        similarity_sum: 0.0,
        items: 0,
    };
    for (id, scenario_references) in references {
        if !bibliography.contains_key(id) {
            continue;
        }
        for (scenario_index, reference) in scenario_references.iter().enumerate() {
            let Some(reference) = reference else {
                continue;
            };
            if reference.trim().is_empty() {
                continue;
            }
            score.items += 1;
            let rendered = processor
                .process_citation(&scenario_citation(id, scenario_index))
                .unwrap_or_default();
            let similarity = token_jaccard(&rendered, reference);
            if similarity >= PASS_THRESHOLD {
                score.passes += 1;
            }
            score.similarity_sum += similarity;
        }
    }
    score
}

/// Phase 1 bounded citation-local mutation candidates of the inferred style.
pub(crate) fn citation_mutation_candidates(inferred_style: &Style) -> Vec<(String, Style)> {
    let mut candidates = Vec::new();
    if !citation_style_contains_primary_title(inferred_style) {
        add_citation_mutation_candidate(
            inferred_style,
            "citation-author-family-only",
            set_author_contributors_family_only,
            &mut candidates,
        );
    }
    add_compact_locator_delimiter_candidate(inferred_style, &mut candidates);
    candidates
}

fn add_compact_locator_delimiter_candidate(
    inferred_style: &Style,
    candidates: &mut Vec<(String, Style)>,
) {
    let mut style = inferred_style.clone();
    let mut changed = false;
    if let Some(options) = style.options.as_mut()
        && let Some(locators) = options.locators.as_mut()
        && !locators.fallback_delimiter.is_empty()
    {
        locators.fallback_delimiter.clear();
        changed = true;
    }
    if let Some(citation) = style.citation.as_mut()
        && let Some(options) = citation.options.as_mut()
        && let Some(locators) = options.locators.as_mut()
        && !locators.fallback_delimiter.is_empty()
    {
        locators.fallback_delimiter.clear();
        changed = true;
    }
    if let Some(citation) = style.citation.as_mut()
        && citation.delimiter.as_deref() != Some("")
    {
        citation.delimiter = Some(String::new().into());
        changed = true;
    }
    if changed {
        candidates.push(("citation-compact-locator-delimiter".to_string(), style));
    }
}

fn citation_style_contains_primary_title(style: &Style) -> bool {
    style
        .citation
        .as_ref()
        .is_some_and(citation_spec_contains_primary_title)
}

fn citation_spec_contains_primary_title(spec: &citum_schema::CitationSpec) -> bool {
    spec.template
        .as_ref()
        .and_then(TemplateVariant::as_template)
        .is_some_and(template_contains_primary_title)
        || spec.type_variants.as_ref().is_some_and(|type_variants| {
            type_variants.values().any(|variant| {
                variant
                    .as_template()
                    .is_some_and(template_contains_primary_title)
            })
        })
        || spec
            .integral
            .as_ref()
            .is_some_and(|spec| citation_spec_contains_primary_title(spec))
        || spec
            .non_integral
            .as_ref()
            .is_some_and(|spec| citation_spec_contains_primary_title(spec))
        || spec
            .subsequent
            .as_ref()
            .is_some_and(|spec| citation_spec_contains_primary_title(spec))
        || spec
            .ibid
            .as_ref()
            .is_some_and(|spec| citation_spec_contains_primary_title(spec))
}

fn template_contains_primary_title(components: &[TemplateComponent]) -> bool {
    components.iter().any(|component| match component {
        TemplateComponent::Title(title) => title.title == TitleType::Primary,
        TemplateComponent::Group(group) => template_contains_primary_title(&group.group),
        _ => false,
    })
}

fn add_citation_mutation_candidate<F>(
    inferred_style: &Style,
    name: &str,
    mutator: F,
    candidates: &mut Vec<(String, Style)>,
) where
    F: Fn(&mut [TemplateComponent]) -> bool,
{
    let mut style = inferred_style.clone();
    let Some(citation) = style.citation.as_mut() else {
        return;
    };
    if mutate_citation_spec(citation, &mutator) {
        candidates.push((name.to_string(), style));
    }
}

fn mutate_citation_spec<F>(spec: &mut citum_schema::CitationSpec, mutator: &F) -> bool
where
    F: Fn(&mut [TemplateComponent]) -> bool,
{
    let mut changed = false;
    if let Some(template) = spec
        .template
        .as_mut()
        .and_then(TemplateVariant::as_template_mut)
    {
        changed |= mutator(template);
    }
    if let Some(type_variants) = spec.type_variants.as_mut() {
        for variant in type_variants.values_mut() {
            if let TemplateVariant::Full(template) = variant {
                changed |= mutator(template);
            }
        }
    }
    if let Some(spec) = spec.integral.as_mut() {
        changed |= mutate_citation_spec(spec, mutator);
    }
    if let Some(spec) = spec.non_integral.as_mut() {
        changed |= mutate_citation_spec(spec, mutator);
    }
    if let Some(spec) = spec.subsequent.as_mut() {
        changed |= mutate_citation_spec(spec, mutator);
    }
    if let Some(spec) = spec.ibid.as_mut() {
        changed |= mutate_citation_spec(spec, mutator);
    }
    changed
}

fn set_author_contributors_family_only(components: &mut [TemplateComponent]) -> bool {
    let mut changed = false;
    for component in components {
        if let TemplateComponent::Group(group) = component {
            changed |= set_author_contributors_family_only(&mut group.group);
        }
        if let TemplateComponent::Contributor(contributor) = component
            && contributor.contributor == ContributorRole::Author
        {
            if contributor.form != ContributorForm::FamilyOnly {
                contributor.form = ContributorForm::FamilyOnly;
                changed = true;
            }
            if contributor.name_form != Some(NameForm::FamilyOnly) {
                contributor.name_form = Some(NameForm::FamilyOnly);
                changed = true;
            }
        }
    }
    changed
}

/// Render one candidate bibliography and score entries against citeproc output.
pub(crate) fn score_bibliography_candidate(
    style: &Style,
    bibliography: &Bibliography,
    references: &BTreeMap<String, Option<String>>,
) -> CandidateScore {
    let Ok(processed) = catch_candidate_unwind(|| {
        let processor = Processor::new(style.clone(), bibliography.clone());
        processor.process_references_with_format_standalone::<PlainText>()
    }) else {
        return invalid_candidate_score(references);
    };
    let reference_entries: Vec<String> = references
        .values()
        .filter_map(|reference| reference.as_ref())
        .filter(|reference| !reference.trim().is_empty())
        .cloned()
        .collect();
    let rendered_entries: Vec<String> = processed
        .bibliography
        .iter()
        .map(render_entry_body_with_format::<PlainText>)
        .collect();

    score_bibliography_entries(&reference_entries, &rendered_entries)
}

struct BibliographyPairCandidate {
    reference_index: usize,
    rendered_index: usize,
    similarity: f64,
}

/// Score bibliography candidates against citeproc references.
///
/// `CandidateScore::items` always counts scored *reference* units, never
/// rendered output, so this success path stays comparable to
/// [`invalid_candidate_score`]'s failure path: a candidate whose renderer
/// emits spurious extra bibliography entries must not be penalized with an
/// inflated denominator relative to one that failed to render at all.
fn score_bibliography_entries(references: &[String], rendered: &[String]) -> CandidateScore {
    let mut candidates = Vec::new();
    for (reference_index, reference) in references.iter().enumerate() {
        let normalized_reference = normalize_text(reference);
        for (rendered_index, rendered_entry) in rendered.iter().enumerate() {
            let similarity = token_jaccard(&normalized_reference, &normalize_text(rendered_entry));
            if similarity >= 0.20 {
                candidates.push(BibliographyPairCandidate {
                    reference_index,
                    rendered_index,
                    similarity,
                });
            }
        }
    }
    candidates.sort_by(|left, right| {
        right
            .similarity
            .partial_cmp(&left.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut used_references = vec![false; references.len()];
    let mut used_rendered = vec![false; rendered.len()];
    let mut score = CandidateScore {
        passes: 0,
        similarity_sum: 0.0,
        items: 0,
    };

    for candidate in candidates {
        let Some(reference_used) = used_references.get(candidate.reference_index) else {
            continue;
        };
        let Some(rendered_used) = used_rendered.get(candidate.rendered_index) else {
            continue;
        };
        if *reference_used || *rendered_used {
            continue;
        }
        let Some(reference_used) = used_references.get_mut(candidate.reference_index) else {
            continue;
        };
        *reference_used = true;
        let Some(rendered_used) = used_rendered.get_mut(candidate.rendered_index) else {
            continue;
        };
        *rendered_used = true;
        let Some(reference) = references.get(candidate.reference_index) else {
            continue;
        };
        let Some(rendered_entry) = rendered.get(candidate.rendered_index) else {
            continue;
        };
        score.items += 1;
        let comparison = compare_text(reference, rendered_entry);
        if comparison.matches {
            score.passes += 1;
        }
        score.similarity_sum += comparison.similarity;
    }

    score.items += used_references.iter().filter(|used| !**used).count();
    score
}

/// Guards one-time installation of [`candidate_panic_hook`], so repeated
/// candidate scoring never swaps the process-global panic hook per call.
static CANDIDATE_PANIC_HOOK_INIT: std::sync::Once = std::sync::Once::new();

fn catch_candidate_unwind<F, T>(f: F) -> Result<T, Box<dyn std::any::Any + Send>>
where
    F: FnOnce() -> T,
{
    CANDIDATE_PANIC_HOOK_INIT.call_once(|| {
        std::panic::set_hook(Box::new(candidate_panic_hook));
    });
    std::panic::catch_unwind(AssertUnwindSafe(f))
}

/// Replacement panic hook installed once for the process: logs the panic
/// payload instead of the default stderr print, so a candidate render panic
/// stays visible (via `RUST_LOG=citum_migrate=debug`) rather than being fully
/// discarded, without repeatedly swapping the global hook per candidate.
fn candidate_panic_hook(info: &std::panic::PanicHookInfo<'_>) {
    let payload = info
        .payload()
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_string());
    tracing::debug!(panic.message = %payload, location = %info.location().map_or_else(|| "unknown".to_string(), ToString::to_string), "candidate render panicked");
}

fn invalid_candidate_score(references: &BTreeMap<String, Option<String>>) -> CandidateScore {
    CandidateScore {
        passes: 0,
        similarity_sum: 0.0,
        items: references
            .values()
            .filter(|reference| {
                reference
                    .as_ref()
                    .is_some_and(|text| !text.trim().is_empty())
            })
            .count(),
    }
}

/// Phase 1 typed bibliography patch candidates of the inferred style.
pub(crate) fn bibliography_mutation_candidates(
    inferred_style: &Style,
    budget: CandidateBudget,
) -> Vec<CandidateStyle> {
    let mut seen = BTreeSet::new();
    let mut family_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    bibliography_candidate_patches()
        .into_iter()
        .filter(|patch| seen.insert(patch.name.clone()))
        .filter(|patch| {
            let count = family_counts.entry(patch.family.as_str()).or_insert(0);
            *count += 1;
            *count <= budget.max_per_family
        })
        .filter_map(|patch| {
            patch
                .apply(inferred_style)
                .map(|style| CandidateStyle::from_patch(&patch, style))
        })
        .collect()
}

fn bibliography_candidate_patches() -> Vec<CandidatePatch> {
    let mut patches = Vec::new();
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::ContributorCase,
        "bibliography-contributors-small-caps",
        Vec::new(),
        CandidatePatchKind::BibliographyContributorsSmallCaps,
    ));
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::DateGranularity,
        "media-issued-full-date",
        MEDIA_FULL_DATE_TYPES
            .iter()
            .map(|ty| (*ty).to_string())
            .collect(),
        CandidatePatchKind::MediaIssuedFullDate {
            ref_types: MEDIA_FULL_DATE_TYPES
                .iter()
                .map(|ty| (*ty).to_string())
                .collect(),
        },
    ));
    for ref_type in LOCAL_DEFAULT_TYPES {
        patches.push(CandidatePatch::bibliography(
            CandidateFamily::TypeLocalDefault,
            format!("{ref_type}-local-default"),
            vec![(*ref_type).to_string()],
            CandidatePatchKind::TypeLocalDefault {
                ref_type: (*ref_type).to_string(),
            },
        ));
    }
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::TypeLocalDefault,
        "article-journal-local-default",
        vec!["article-journal".to_string()],
        CandidatePatchKind::TypeLocalDefault {
            ref_type: "article-journal".to_string(),
        },
    ));
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::ArticleJournalSuppression,
        "article-journal-suppress-primary-title",
        vec!["article-journal".to_string()],
        CandidatePatchKind::ArticleJournalSuppress {
            matchers: vec![ComponentMatcher::PrimaryTitle],
        },
    ));
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::ArticleJournalSuppression,
        "article-journal-suppress-doi-url",
        vec!["article-journal".to_string()],
        CandidatePatchKind::ArticleJournalSuppress {
            matchers: vec![ComponentMatcher::DoiOrUrl],
        },
    ));
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::ArticleJournalSuppression,
        "article-journal-suppress-primary-title-doi-url",
        vec!["article-journal".to_string()],
        CandidatePatchKind::ArticleJournalSuppress {
            matchers: vec![ComponentMatcher::PrimaryTitle, ComponentMatcher::DoiOrUrl],
        },
    ));
    patches.push(CandidatePatch::bibliography(
        CandidateFamily::ArticleJournalSuppression,
        "article-journal-suppress-issue",
        vec!["article-journal".to_string()],
        CandidatePatchKind::ArticleJournalSuppress {
            matchers: vec![ComponentMatcher::Issue],
        },
    ));
    patches
}

fn apply_bibliography_template_mutation<F>(style: &Style, mutator: F) -> Option<Style>
where
    F: Fn(&mut [TemplateComponent]) -> bool,
{
    let mut style = style.clone();
    let bibliography = style.bibliography.as_mut()?;
    let mut changed = false;
    if let Some(template) = bibliography
        .template
        .as_mut()
        .and_then(TemplateVariant::as_template_mut)
    {
        changed |= mutator(template);
    }
    if let Some(type_variants) = bibliography.type_variants.as_mut() {
        for variant in type_variants.values_mut() {
            if let Some(template) = variant.as_template_mut() {
                changed |= mutator(template);
            }
        }
    }
    changed.then_some(style)
}

fn apply_type_local_default(style: &Style, ref_type: &str) -> Option<Style> {
    let template = style
        .bibliography
        .as_ref()
        .and_then(|bibliography| bibliography.template.as_ref())
        .and_then(TemplateVariant::as_template)
        .map(<[_]>::to_vec)?;
    let mut style = style.clone();
    let bibliography = style.bibliography.as_mut()?;
    bibliography
        .type_variants
        .get_or_insert_with(Default::default)
        .insert(
            TypeSelector::Single(ref_type.to_string()),
            TemplateVariant::Full(template),
        );
    Some(style)
}

fn apply_issued_full_date(style: &Style, ref_types: &[String]) -> Option<Style> {
    let mut updates = Vec::new();
    for ref_type in ref_types {
        let Some(mut template) = bibliography_template_for_type(style, ref_type) else {
            continue;
        };
        if set_issued_year_dates_to_full(&mut template) {
            updates.push((ref_type.clone(), template));
        }
    }
    if updates.is_empty() {
        return None;
    }

    let mut style = style.clone();
    let bibliography = style.bibliography.as_mut()?;
    let type_variants = bibliography
        .type_variants
        .get_or_insert_with(Default::default);
    for (ref_type, template) in updates {
        type_variants.insert(
            TypeSelector::Single(ref_type),
            TemplateVariant::Full(template),
        );
    }
    Some(style)
}

fn apply_article_journal_suppression(
    style: &Style,
    matchers: &[ComponentMatcher],
) -> Option<Style> {
    let mut template = bibliography_template_for_type(style, "article-journal")?;
    if !suppress_matching_components(&mut template, &|component| {
        matchers.iter().any(|matcher| matcher.matches(component))
    }) {
        return None;
    }

    let mut style = style.clone();
    let bibliography = style.bibliography.as_mut()?;
    bibliography
        .type_variants
        .get_or_insert_with(Default::default)
        .insert(
            TypeSelector::Single("article-journal".to_string()),
            TemplateVariant::Full(template),
        );
    Some(style)
}

fn bibliography_template_for_type(style: &Style, ref_type: &str) -> Option<Vec<TemplateComponent>> {
    let bibliography = style.bibliography.as_ref()?;
    if let Some(type_variants) = bibliography.type_variants.as_ref() {
        for (selector, variant) in type_variants {
            if selector.matches(ref_type)
                && let TemplateVariant::Full(template) = variant
            {
                return Some(template.clone());
            }
        }
    }
    bibliography
        .template
        .as_ref()
        .and_then(TemplateVariant::as_template)
        .map(<[_]>::to_vec)
}

fn suppress_matching_components<F>(
    components: &mut [TemplateComponent],
    should_suppress: &F,
) -> bool
where
    F: Fn(&TemplateComponent) -> bool,
{
    let mut changed = false;
    for component in components.iter_mut() {
        if let TemplateComponent::Group(group) = component {
            changed |= suppress_matching_components(&mut group.group, should_suppress);
        }
        if should_suppress(component) && component.rendering().suppress != Some(true) {
            component.rendering_mut().suppress = Some(true);
            changed = true;
        }
    }
    changed
}

fn set_contributors_small_caps(components: &mut [TemplateComponent]) -> bool {
    let mut changed = false;
    for component in components {
        if let TemplateComponent::Group(group) = component {
            changed |= set_contributors_small_caps(&mut group.group);
        }
        if let TemplateComponent::Contributor(contributor) = component
            && contributor.rendering.small_caps != Some(true)
        {
            contributor.rendering.small_caps = Some(true);
            changed = true;
        }
    }
    changed
}

fn set_issued_year_dates_to_full(components: &mut [TemplateComponent]) -> bool {
    let mut changed = false;
    for component in components {
        if let TemplateComponent::Group(group) = component {
            changed |= set_issued_year_dates_to_full(&mut group.group);
        }
        if let TemplateComponent::Date(date) = component
            && date.date == citum_schema::template::DateVariable::Issued
            && date.form == DateForm::Year
        {
            date.form = DateForm::Full;
            changed = true;
        }
    }
    changed
}

/// Build the engine-side citation matching a citeproc reference scenario.
///
/// Scenario order is a cross-renderer contract with the JS side
/// (`renderCitationScenarioStrings` in `scripts/lib/template-inferrer-core.js`):
/// index 0 is a bare first citation, index 1 a first citation with the page
/// locator the JS side renders (`p. 23`), index 2 a subsequent citation,
/// index 3 an ibid citation, and index 4 an ibid citation with the locator.
fn scenario_citation(id: &str, scenario_index: usize) -> Citation {
    use citum_schema::citation::Position;

    let mut citation = Citation::simple(id);
    citation.position = match scenario_index {
        0 | 1 => None,
        2 => Some(Position::Subsequent),
        3 => Some(Position::Ibid),
        _ => Some(Position::IbidWithLocator),
    };
    let wants_locator = matches!(scenario_index, 1 | 4);
    if wants_locator && let Some(item) = citation.items.first_mut() {
        item.locator = Some(citum_schema::citation::CitationLocator::single(
            citum_schema::citation::LocatorType::Page,
            "23",
        ));
    }
    citation
}

/// Result of oracle-compatible text comparison for measured scoring.
struct TextComparison {
    matches: bool,
    similarity: f64,
}

/// Compare rendered text with normalized exact equality first, then
/// case-sensitive case-only rejection, then token similarity.
fn compare_text(expected_text: &str, actual_text: &str) -> TextComparison {
    let expected = normalize_text(expected_text);
    let actual = normalize_text(actual_text);

    if expected == actual {
        return TextComparison {
            matches: true,
            similarity: 1.0,
        };
    }

    if expected.to_lowercase() == actual.to_lowercase() {
        return TextComparison {
            matches: false,
            similarity: 1.0,
        };
    }

    let similarity = token_jaccard(&expected, &actual);
    TextComparison {
        matches: similarity >= PASS_THRESHOLD,
        similarity,
    }
}

/// Normalize rendered text for measured scoring, matching the oracle's most
/// important case-sensitive equality behavior without pulling in a regex
/// dependency.
fn normalize_text(text: &str) -> String {
    let without_html = strip_html_tags(text);
    let normalized = without_html
        .replace("&#38;", "&")
        .replace('\u{00a0}', " ")
        .replace(['_', '*'], "");
    let collapsed = collapse_whitespace(&normalized);
    let without_label = strip_bibliography_label(&collapsed);
    without_label
        .trim_end_matches(['.', ',', ';', ':'])
        .trim()
        .to_string()
}

fn strip_html_tags(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn collapse_whitespace(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut previous_was_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !previous_was_space {
                output.push(' ');
                previous_was_space = true;
            }
        } else {
            output.push(ch);
            previous_was_space = false;
        }
    }
    output
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" ;", ";")
        .replace(" :", ":")
        .trim()
        .to_string()
}

fn strip_bibliography_label(text: &str) -> String {
    let trimmed = text.trim_start_matches(|ch: char| {
        matches!(
            ch,
            '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
    });
    let Some((dot_index, _)) = trimmed.char_indices().find(|(_, ch)| *ch == '.') else {
        return text.to_string();
    };
    if dot_index == 0 {
        return text.to_string();
    }
    let (label, after_label) = trimmed.split_at(dot_index);
    if !label.chars().all(|ch| ch.is_ascii_digit()) {
        return text.to_string();
    }
    let Some(after_dot) = after_label.strip_prefix('.') else {
        return text.to_string();
    };
    if after_dot
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        return text.to_string();
    }
    after_dot.trim_start().to_string()
}

/// Bag-of-words Jaccard similarity over lowercased alphanumeric tokens.
fn token_jaccard(left_text: &str, right_text: &str) -> f64 {
    let left = tokenize(left_text);
    let right = tokenize(right_text);
    token_jaccard_from_tokens(&left, &right)
}

fn token_jaccard_from_tokens(left: &[String], right: &[String]) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for token in left {
        counts.entry(token).or_insert((0, 0)).0 += 1;
    }
    for token in right {
        counts.entry(token).or_insert((0, 0)).1 += 1;
    }

    let mut intersection = 0usize;
    let mut union = 0usize;
    for (left_count, right_count) in counts.values() {
        intersection += left_count.min(right_count);
        union += left_count.max(right_count);
    }

    if union == 0 {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "citation token counts are far below f64 integer precision"
    )]
    let ratio = intersection as f64 / union as f64;
    ratio
}

/// Split text into lowercased alphanumeric tokens, dropping single-character
/// tokens.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.chars().count() > 1)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_cmp,
    reason = "Panicking and exact comparison are acceptable in tests."
)]
mod tests {
    use super::{
        CandidateBudget, CandidateScore, PASS_THRESHOLD, bibliography_candidate_patches,
        bibliography_mutation_candidates, candidate_beats, catch_candidate_unwind, compare_text,
        fixture_diagnostics_from_value, normalize_text, scenario_citation, token_jaccard, tokenize,
    };
    use citum_schema::Style;
    use citum_schema::citation::Position;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn tokenize_splits_on_punctuation_and_drops_short_tokens() {
        let given = "T.S. Kuhn, ‘The Structure’ (1962)";
        let tokens = tokenize(given);
        assert_eq!(tokens, vec!["kuhn", "the", "structure", "1962"]);
    }

    #[test]
    fn scenario_citation_positions_mirror_js_scenario_contract() {
        let positions: Vec<Option<Position>> = (0..5)
            .map(|index| scenario_citation("kuhn1962", index).position)
            .collect();
        assert_eq!(
            positions,
            vec![
                None,
                None,
                Some(Position::Subsequent),
                Some(Position::Ibid),
                Some(Position::IbidWithLocator),
            ]
        );
    }

    #[test]
    fn scenario_citation_sets_locator_only_on_locator_scenarios() {
        let locator_scenarios: Vec<bool> = (0..5)
            .map(|index| {
                scenario_citation("kuhn1962", index)
                    .items
                    .first()
                    .is_some_and(|item| item.locator.is_some())
            })
            .collect();
        assert_eq!(locator_scenarios, vec![false, true, false, false, true]);
    }

    #[test]
    fn token_jaccard_is_one_for_equal_bags_regardless_of_order() {
        let left = "Kuhn, The Structure of Scientific Revolutions (1962)";
        let right = "1962 Kuhn: of Scientific Revolutions, The Structure";
        assert_eq!(token_jaccard(left, right), 1.0);
    }

    #[test]
    fn token_jaccard_punishes_run_on_components() {
        let reference =
            "Kuhn, ‘The Structure of Scientific Revolutions’, International Encyclopedia (1962)";
        let run_on =
            "Kuhn, ‘The Structure of Scientific RevolutionsInternational Encyclopedia’ (1962)";
        let separated =
            "Kuhn, 1962, ‘The Structure of Scientific Revolutions’, International Encyclopedia";
        assert!(token_jaccard(separated, reference) > token_jaccard(run_on, reference));
    }

    #[test]
    fn token_jaccard_threshold_separates_wrong_shape_from_near_miss() {
        let reference =
            "Thomas Kuhn, The Structure of Scientific Revolutions, Chicago (1962), p. 23";
        let near_miss = "Thomas Kuhn, The Structure of Scientific Revolutions, Chicago (1962)";
        let wrong_shape = "Kuhn 1962";
        assert!(token_jaccard(near_miss, reference) >= PASS_THRESHOLD);
        assert!(token_jaccard(wrong_shape, reference) < PASS_THRESHOLD);
    }

    #[test]
    fn token_jaccard_handles_empty_inputs() {
        assert_eq!(token_jaccard("", ""), 1.0);
        assert_eq!(token_jaccard("Kuhn 1962", ""), 0.0);
        assert_eq!(token_jaccard("", "Kuhn 1962"), 0.0);
    }

    #[test]
    fn citation_token_similarity_preserves_bibtex_label_separators() {
        let rendered = "Kuhn_1962";
        let reference = "Kuhn 1962";

        assert_eq!(tokenize(rendered), vec!["kuhn", "1962"]);
        assert_eq!(token_jaccard(rendered, reference), 1.0);
    }

    #[test]
    fn candidate_tie_breaking_prioritizes_pass_count() {
        let incumbent = CandidateScore {
            passes: 2,
            similarity_sum: 10.0,
            items: 3,
        };
        let more_passes = CandidateScore {
            passes: 3,
            similarity_sum: 1.0,
            items: 3,
        };

        assert!(candidate_beats(&more_passes, &incumbent));
    }

    #[test]
    fn candidate_tie_breaking_requires_similarity_margin() {
        let incumbent = CandidateScore {
            passes: 2,
            similarity_sum: 10.0,
            items: 3,
        };
        let tiny_similarity_gain = CandidateScore {
            passes: 2,
            similarity_sum: 10.01,
            items: 3,
        };
        let clear_similarity_gain = CandidateScore {
            passes: 2,
            similarity_sum: 10.06,
            items: 3,
        };

        assert!(!candidate_beats(&tiny_similarity_gain, &incumbent));
        assert!(candidate_beats(&clear_similarity_gain, &incumbent));
    }

    #[test]
    fn compare_text_rejects_case_only_mismatch() {
        let comparison = compare_text("ADAMS, J. (1797)", "Adams, J. (1797)");

        assert!(!comparison.matches);
        assert_eq!(comparison.similarity, 1.0);
    }

    #[test]
    fn compare_text_accepts_non_case_near_match() {
        let comparison = compare_text(
            "Thomas Kuhn, The Structure of Scientific Revolutions, Chicago (1962), p. 23",
            "Thomas Kuhn, The Structure of Scientific Revolutions, Chicago (1962)",
        );

        assert!(comparison.matches);
        assert!(comparison.similarity >= PASS_THRESHOLD);
    }

    #[test]
    fn normalize_text_strips_bibliography_labels_and_markup() {
        assert_eq!(
            normalize_text("12. <i>Smith</i>, **Jane**. "),
            "Smith, Jane"
        );
    }

    #[test]
    fn bibliography_candidate_families_have_unique_patch_names() {
        let patches = bibliography_candidate_patches();
        let mut names = BTreeSet::new();

        for patch in &patches {
            assert!(
                names.insert(patch.name.clone()),
                "duplicate candidate patch name: {}",
                patch.name
            );
        }
        assert!(!patches.is_empty());
    }

    #[test]
    fn bibliography_candidates_skip_noop_style_without_bibliography() {
        let candidates =
            bibliography_mutation_candidates(&Style::default(), CandidateBudget::default());

        assert!(candidates.is_empty());
    }

    #[test]
    fn candidate_budget_defaults_do_not_truncate_current_patches() {
        let budget = CandidateBudget::default();
        let patches = bibliography_candidate_patches();
        let mut family_counts: BTreeMap<&str, usize> = BTreeMap::new();
        for patch in &patches {
            *family_counts.entry(patch.family.as_str()).or_insert(0) += 1;
        }

        let seed_count = 2;
        assert!(patches.len() + seed_count <= budget.max_total);
        for (family, count) in family_counts {
            assert!(
                count <= budget.max_per_family,
                "family {family} exceeds the default per-family budget: {count}"
            );
        }
    }

    #[test]
    fn candidate_budget_truncates_over_cap_mutations() {
        let budget = CandidateBudget {
            max_per_family: 1,
            max_total: 3,
        };
        let mut mutations: Vec<usize> = (0..10).collect();

        budget.truncate_mutations(&mut mutations, 2);

        assert_eq!(mutations, vec![0]);
    }

    #[test]
    fn catch_candidate_unwind_reports_panics_as_err_without_reinstalling_the_hook_per_call() {
        let first = catch_candidate_unwind(|| panic!("bad candidate template"));
        let second = catch_candidate_unwind(|| panic!("another bad candidate template"));
        let ok = catch_candidate_unwind(|| 42);

        assert!(first.is_err());
        assert!(second.is_err());
        assert_eq!(ok.unwrap(), 42);
    }

    #[test]
    fn fixture_conversion_diagnostic_keeps_item_id_and_stable_code() {
        let fixtures = serde_json::json!({
            "fixture-key": {
                "id": "cstr-item",
                "type": "article",
                "note": "CSTR: direct-value\ntex.cstr: alternate-value"
            }
        });

        let diagnostics = fixture_diagnostics_from_value(fixtures)
            .expect("fixture diagnostics should parse valid CSL-JSON");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "conflicting-supplementary-identifier");
        assert_eq!(diagnostics[0].item_id.as_deref(), Some("cstr-item"));
    }
}
