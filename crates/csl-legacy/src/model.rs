/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use serde::{Deserialize, Serialize};

/// A parsed CSL 1.0 style.
///
/// Corresponds to the root `<style>` element in a `.csl` XML file.
/// After parsing, this struct is the primary IR handed to the migration pipeline.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Style {
    /// CSL specification version declared in the style (e.g., `"1.0"`).
    pub version: String,
    /// XML namespace URI (typically `"http://purl.org/net/xbiblio/csl"`).
    pub xmlns: String,
    /// Citation class: `"in-text"` or `"note"`.
    pub class: String,
    /// The default locale for this style (e.g., `"en-US"`, `"de-DE"`).
    pub default_locale: Option<String>,
    /// Style-level name formatting options (inherited by all names unless overridden).
    pub initialize_with: Option<String>,
    /// Whether to include a hyphen when initialising given names.
    pub initialize_with_hyphen: Option<bool>,
    /// Delimiter inserted between rendered names.
    pub names_delimiter: Option<String>,
    /// Which names are rendered in sort order (`"first"` or `"all"`).
    pub name_as_sort_order: Option<String>,
    /// String placed between family name and given name in sort order (default `", "`).
    pub sort_separator: Option<String>,
    /// When the Oxford comma is inserted before the last name delimiter.
    pub delimiter_precedes_last: Option<String>,
    /// When the name delimiter precedes "et al." truncation.
    pub delimiter_precedes_et_al: Option<String>,
    /// Controls sorting/display of non-dropping particles (e.g., `"display-and-sort"`).
    pub demote_non_dropping_particle: Option<String>,
    /// Conjunction word inserted before the last name (`"text"` → locale term, `"symbol"` → `&`).
    pub and: Option<String>,
    /// Page range formatting algorithm (`"expanded"`, `"minimal"`, `"chicago"`, `"chicago-16"`).
    pub page_range_format: Option<String>,
    /// Style metadata (`<info>` element).
    pub info: Info,
    /// In-style locale overrides (`<locale>` elements).
    pub locale: Vec<Locale>,
    /// Named macro definitions (`<macro>` elements).
    pub macros: Vec<Macro>,
    /// Citation rendering configuration (`<citation>` element).
    pub citation: Citation,
    /// Bibliography rendering configuration (`<bibliography>` element), if present.
    pub bibliography: Option<Bibliography>,
}

/// Metadata block for a CSL style (`<info>` element).
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Info {
    /// Human-readable style title.
    pub title: String,
    /// Canonical URI uniquely identifying this style.
    pub id: String,
    /// ISO 8601 date-time string of the last update.
    pub updated: String,
    /// Subject-area categories (`field=` attribute values; `generic-base` is silently ignored).
    pub fields: Vec<String>,
    /// Optional short description of the style.
    pub summary: Option<String>,
    /// Links to related resources (self-link, template, documentation, etc.).
    pub links: Vec<InfoLink>,
    /// Style authors.
    pub authors: Vec<InfoPerson>,
    /// Style contributors.
    pub contributors: Vec<InfoPerson>,
    /// License URI from `<rights license="…">` or the element's text content.
    pub rights: Option<String>,
}

/// A hyperlink entry inside `<info>` (`<link href="…" rel="…"/>`).
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InfoLink {
    /// Absolute URI of the linked resource.
    pub href: String,
    /// Link relation type (e.g., `"self"`, `"template"`, `"documentation"`).
    pub rel: Option<String>,
}

/// A person credited in `<info>` as an `<author>` or `<contributor>`.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct InfoPerson {
    /// Display name of the person.
    pub name: Option<String>,
    /// Contact email address.
    pub email: Option<String>,
    /// Personal or institutional URI.
    pub uri: Option<String>,
}

/// A locale block inside the style (`<locale xml:lang="…">`).
///
/// Locale blocks supply style-specific term overrides for a given language.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Locale {
    /// BCP 47 language tag (e.g., `"en-US"`); `None` applies to all locales.
    pub lang: Option<String>,
    /// Term definitions provided by this locale block.
    pub terms: Vec<Term>,
}

/// A term definition inside a `<locale>` block (`<term name="…">`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Term {
    /// CSL term name (e.g., `"editor"`, `"page"`).
    pub name: String,
    /// Term form (`"long"`, `"short"`, `"verb"`, `"verb-short"`, `"symbol"`).
    pub form: Option<String>,
    /// Text value for terms without plural/singular distinction.
    pub value: String,
    /// Singular form (used when the term has `<single>` / `<multiple>` children).
    pub single: Option<String>,
    /// Plural form (used when the term has `<single>` / `<multiple>` children).
    pub multiple: Option<String>,
}

/// A named macro (`<macro name="…">`).
///
/// Macros group reusable rendering logic and are referenced by `<text macro="…">`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Macro {
    /// The macro's name as it appears in `macro=` references.
    pub name: String,
    /// Top-level CSL nodes that make up the macro body.
    pub children: Vec<CslNode>,
}

/// Citation configuration (`<citation>` element).
///
/// Controls how in-text or note citations are rendered and sorted.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Citation {
    /// The `<layout>` block that renders each individual citation.
    pub layout: Layout,
    /// Ordered CSL-M layouts, including the final unscoped fallback.
    ///
    /// This is empty for conventional CSL with zero or one unscoped layout.
    #[serde(default)]
    pub localized_layouts: Vec<LocalizedLayout>,
    /// Optional sort order applied to cites within a single citation cluster.
    pub sort: Option<Sort>,
    /// Collapse mode for grouped citations (for example, `"citation-number"`).
    pub collapse: Option<String>,
    /// Delimiter joining items within a collapsed same-author group,
    /// overriding the layout delimiter for that join only.
    pub cite_group_delimiter: Option<String>,
    /// Delimiter preceding a merged/ranged year-suffix token specifically,
    /// under `collapse="year-suffix"` or `"year-suffix-ranged"`.
    pub year_suffix_delimiter: Option<String>,
    /// Minimum number of names before et-al truncation kicks in.
    pub et_al_min: Option<usize>,
    /// Number of names to show before the et-al term.
    pub et_al_use_first: Option<usize>,
    /// If `true`, a year-suffix is appended to disambiguate ambiguous cites.
    pub disambiguate_add_year_suffix: Option<bool>,
    /// If `true`, additional names are expanded for disambiguation.
    pub disambiguate_add_names: Option<bool>,
    /// If `true`, given names are expanded for disambiguation.
    pub disambiguate_add_givenname: Option<bool>,
    /// Rule controlling which author positions receive given-name expansion.
    pub disambiguate_givenname_rule: Option<String>,
}

/// Bibliography configuration (`<bibliography>` element).
///
/// Controls how the reference list is rendered and sorted.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bibliography {
    /// The `<layout>` block that renders each bibliography entry.
    pub layout: Layout,
    /// Ordered CSL-M layouts, including the final unscoped fallback.
    ///
    /// This is empty for conventional CSL with zero or one unscoped layout.
    #[serde(default)]
    pub localized_layouts: Vec<LocalizedLayout>,
    /// Optional sort order applied to bibliography entries.
    pub sort: Option<Sort>,
    /// Minimum number of names before et-al truncation kicks in.
    pub et_al_min: Option<usize>,
    /// Number of names to show before the et-al term.
    pub et_al_use_first: Option<usize>,
    /// If `true`, a hanging indent is applied to each entry.
    pub hanging_indent: Option<bool>,
    /// String substituted for repeated subsequent authors (e.g., `"---"`).
    pub subsequent_author_substitute: Option<String>,
    /// Rule controlling when `subsequent_author_substitute` is applied.
    pub subsequent_author_substitute_rule: Option<String>,
}

/// A layout block (`<layout prefix="…" suffix="…" delimiter="…">`).
///
/// Wraps a sequence of CSL nodes that are rendered as a unit.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layout {
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// String inserted between consecutive rendered children.
    pub delimiter: Option<String>,
    /// The rendering nodes contained in this layout.
    pub children: Vec<CslNode>,
}

/// A locale-scoped CSL-M layout in source order.
///
/// An empty `locales` list identifies the required final fallback layout.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalizedLayout {
    /// Whitespace-separated language tags from the CSL-M `locale` attribute.
    pub locales: Vec<String>,
    /// Rendering layout selected for those locales.
    pub layout: Layout,
}

/// A sort specification (`<sort>`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sort {
    /// Ordered list of sort keys.
    pub keys: Vec<SortKey>,
}

/// A single sort key (`<key variable="…">` or `<key macro="…">`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SortKey {
    /// Variable name to sort by (mutually exclusive with `macro_name`).
    pub variable: Option<String>,
    /// Macro name to sort by (mutually exclusive with `variable`).
    pub macro_name: Option<String>,
    /// Sort direction: `"ascending"` (default) or `"descending"`.
    pub sort: Option<String>,
}

/// A CSL rendering node — one of the ten node types defined by CSL 1.0.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CslNode {
    /// Renders a text value, variable, macro, or term.
    Text(Text),
    /// Renders a date variable.
    Date(Date),
    /// Renders a localised label for a variable.
    Label(Label),
    /// Renders a name list variable.
    Names(Names),
    /// Groups child nodes; suppressed when all children render empty.
    Group(Group),
    /// Conditional branching.
    Choose(Choose),
    /// Renders a numeric variable with optional formatting.
    Number(Number),
    /// Configures name formatting within a `<names>` block.
    Name(Name),
    /// Customises the "et al." term within a `<names>` block.
    EtAl(EtAl),
    /// Fallback rendering when the primary name variable is empty.
    Substitute(Substitute),
}

/// A `<text>` node — renders a literal value, variable, macro reference, or term.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Text {
    /// Literal string value (`value="…"`).
    pub value: Option<String>,
    /// Name of the reference variable to render.
    pub variable: Option<String>,
    /// Name of the macro to call.
    pub macro_name: Option<String>,
    /// Name of the locale term to render.
    pub term: Option<String>,
    /// Term form (`"long"`, `"short"`, etc.).
    pub form: Option<String>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// If `true`, the rendered text is wrapped in locale-defined quotation marks.
    pub quotes: Option<bool>,
    /// Text case transformation (`"uppercase"`, `"lowercase"`, `"capitalize-first"`, etc.).
    pub text_case: Option<String>,
    /// If `true`, trailing periods are stripped from the output.
    pub strip_periods: Option<bool>,
    /// Plural behaviour for term rendering (`"always"`, `"never"`, `"contextual"`).
    pub plural: Option<String>,
    /// Internal: position of this node in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<name>` node — controls how individual names in a `<names>` list are rendered.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    /// Conjunction placed before the last name (`"text"` or `"symbol"`).
    pub and: Option<String>,
    /// Delimiter inserted between names.
    pub delimiter: Option<String>,
    /// Which names are rendered in sort order (`"first"` or `"all"`).
    pub name_as_sort_order: Option<String>,
    /// String placed between family and given name in sort order.
    pub sort_separator: Option<String>,
    /// String appended to each given-name initial (e.g., `"."`).
    pub initialize_with: Option<String>,
    /// Whether to include a hyphen when initialising hyphenated given names.
    pub initialize_with_hyphen: Option<bool>,
    /// Name form: `"long"` (given + family) or `"short"` (family only).
    pub form: Option<String>,
    /// When the Oxford comma is inserted before the last name.
    pub delimiter_precedes_last: Option<String>,
    /// When the name delimiter precedes "et al." truncation.
    pub delimiter_precedes_et_al: Option<String>,
    /// Minimum number of names before et-al truncation.
    pub et_al_min: Option<usize>,
    /// Number of names shown before the et-al term.
    pub et_al_use_first: Option<usize>,
    /// Minimum number of names for et-al truncation on subsequent cites.
    pub et_al_subsequent_min: Option<usize>,
    /// Names shown before et-al on subsequent cites.
    pub et_al_subsequent_use_first: Option<usize>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<et-al>` node — customises the et-al term rendered inside `<names>`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EtAl {
    /// Locale term to use instead of the default `"et-al"`.
    pub term: Option<String>,
}

/// Inline text formatting attributes shared by many CSL node types.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Formatting {
    /// Font style: `"italic"`, `"oblique"`, or `"normal"`.
    pub font_style: Option<String>,
    /// Font variant: `"small-caps"` or `"normal"`.
    pub font_variant: Option<String>,
    /// Font weight: `"bold"`, `"light"`, or `"normal"`.
    pub font_weight: Option<String>,
    /// Text decoration: `"underline"` or `"none"`.
    pub text_decoration: Option<String>,
    /// Vertical alignment: `"sup"`, `"sub"`, or `"baseline"`.
    pub vertical_align: Option<String>,
    /// Display mode (primarily for bibliography entries): `"block"`, `"left-margin"`, `"right-inline"`, `"indent"`.
    pub display: Option<String>,
}

/// A `<substitute>` node — provides fallback rendering when the primary `<names>` variable is empty.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Substitute {
    /// Alternative rendering nodes tried in order until one produces output.
    pub children: Vec<CslNode>,
}

/// A `<date>` node — renders a date variable using a built-in or custom format.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Date {
    /// The date variable to render (e.g., `"issued"`, `"accessed"`).
    pub variable: String,
    /// Built-in date form: `"text"`, `"numeric"`, or `"ordinal"`.
    pub form: Option<String>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// Delimiter inserted between date parts.
    pub delimiter: Option<String>,
    /// Which date parts to render: `"year-month-day"`, `"year-month"`, or `"year"`.
    pub date_parts: Option<String>,
    /// Text case transformation applied to the rendered date.
    pub text_case: Option<String>,
    /// Custom `<date-part>` nodes that override the built-in form.
    pub parts: Vec<DatePart>,
    /// Internal: position in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<date-part>` node — configures rendering of one component of a date.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatePart {
    /// The date component: `"day"`, `"month"`, or `"year"`.
    pub name: String,
    /// Rendering form for the component (e.g., `"numeric"`, `"long"`, `"short"`, `"ordinal"`).
    pub form: Option<String>,
    /// String prepended before this date part.
    pub prefix: Option<String>,
    /// String appended after this date part.
    pub suffix: Option<String>,
}

/// A `<label>` node — renders a localised term that describes a variable.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    /// The variable whose label is rendered (e.g., `"page"`, `"locator"`).
    pub variable: Option<String>,
    /// Term form (`"long"`, `"short"`, `"verb"`, `"verb-short"`, `"symbol"`).
    pub form: Option<String>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// Text case transformation applied to the label.
    pub text_case: Option<String>,
    /// If `true`, trailing periods are stripped from the label.
    pub strip_periods: Option<bool>,
    /// Plural behaviour: `"always"`, `"never"`, or `"contextual"`.
    pub plural: Option<String>,
    /// Internal: position in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<names>` node — renders one or more name-list variables.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Names {
    /// Space-separated list of name variables to render (e.g., `"author editor"`).
    pub variable: String,
    /// Delimiter inserted between names.
    pub delimiter: Option<String>,
    /// When the name delimiter precedes "et al." truncation.
    pub delimiter_precedes_et_al: Option<String>,
    /// Minimum number of names before et-al truncation.
    pub et_al_min: Option<usize>,
    /// Number of names rendered before the et-al term.
    pub et_al_use_first: Option<usize>,
    /// Et-al minimum for subsequent cites.
    pub et_al_subsequent_min: Option<usize>,
    /// Et-al use-first for subsequent cites.
    pub et_al_subsequent_use_first: Option<usize>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// Child nodes: `<name>`, `<label>`, `<substitute>`, `<et-al>`.
    pub children: Vec<CslNode>,
    /// Internal: position in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<group>` node — renders child nodes as a unit, suppressed when all children are empty.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    /// Delimiter inserted between non-empty children.
    pub delimiter: Option<String>,
    /// String prepended before the rendered group.
    pub prefix: Option<String>,
    /// String appended after the rendered group.
    pub suffix: Option<String>,
    /// Child rendering nodes.
    pub children: Vec<CslNode>,
    /// Internal: position in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}

/// A `<choose>` node — conditional rendering based on reference attributes.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Choose {
    /// The mandatory `<if>` branch.
    pub if_branch: ChooseBranch,
    /// Zero or more `<else-if>` branches evaluated in order.
    pub else_if_branches: Vec<ChooseBranch>,
    /// Optional `<else>` branch rendered when all preceding conditions fail.
    pub else_branch: Option<Vec<CslNode>>,
}

/// A conditional branch inside `<choose>` (`<if>` or `<else-if>`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChooseBranch {
    /// How multiple test attributes are combined: `"all"` (default), `"any"`, or `"none"`.
    pub match_mode: Option<String>,
    /// Reference type test (e.g., `"book article-journal"`).
    pub type_: Option<String>,
    /// Variable presence test — branch fires when the variable is non-empty.
    pub variable: Option<String>,
    /// Numeric test — branch fires when the variable value is numeric.
    pub is_numeric: Option<String>,
    /// Uncertain-date test.
    pub is_uncertain_date: Option<String>,
    /// Locator type test.
    pub locator: Option<String>,
    /// Citation position test (`"first"`, `"subsequent"`, `"ibid"`, `"ibid-with-locator"`).
    pub position: Option<String>,
    /// Nodes rendered when this branch matches.
    pub children: Vec<CslNode>,
}

/// A `<number>` node — renders a numeric variable with optional form and formatting.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Number {
    /// The numeric variable to render (e.g., `"volume"`, `"issue"`, `"edition"`).
    pub variable: String,
    /// Rendering form: `"numeric"` (default), `"ordinal"`, `"long-ordinal"`, or `"roman"`.
    pub form: Option<String>,
    /// String prepended before the rendered output.
    pub prefix: Option<String>,
    /// String appended after the rendered output.
    pub suffix: Option<String>,
    /// Text case transformation.
    pub text_case: Option<String>,
    /// Internal: position in the macro call order (set during post-processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_call_order: Option<usize>,
    /// Inline formatting attributes.
    #[serde(flatten)]
    pub formatting: Formatting,
}
