/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::locale::{GeneralTerm, TermForm};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Legacy types below - kept for migration bridge from CSL 1.0
// These will be deprecated once migration is complete
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ItemType {
    Article,
    ArticleJournal,
    ArticleMagazine,
    ArticleNewspaper,
    Bill,
    Book,
    Broadcast,
    Chapter,
    Dataset,
    Entry,
    EntryDictionary,
    EntryEncyclopedia,
    Figure,
    Graphic,
    Interview,
    LegalCase,
    Legislation,
    Manuscript,
    Map,
    MotionPicture,
    MusicalScore,
    Pamphlet,
    PaperConference,
    Patent,
    PersonalCommunication,
    Post,
    PostWeblog,
    Report,
    Review,
    ReviewBook,
    Song,
    Speech,
    Thesis,
    Treaty,
    Webpage,
    Software,
    Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Variable {
    Author,
    CollectionEditor,
    Composer,
    ContainerAuthor,
    Director,
    Editor,
    EditorialDirector,
    Illustrator,
    Interviewer,
    OriginalAuthor,
    Recipient,
    ReviewedAuthor,
    Translator,
    Accessed,
    AvailableDate,
    EventDate,
    Issued,
    OriginalDate,
    Submitted,
    ChapterNumber,
    CollectionNumber,
    Edition,
    Issue,
    Number,
    NumberOfPages,
    NumberOfVolumes,
    Volume,
    Abstract,
    Annote,
    Archive,
    ArchiveLocation,
    ArchivePlace,
    Authority,
    CallNumber,
    CitationLabel,
    CitationNumber,
    CollectionTitle,
    ContainerTitle,
    ContainerTitleShort,
    Dimensions,
    DOI,
    Event,
    EventPlace,
    FirstReferenceNoteNumber,
    Genre,
    ISBN,
    ISSN,
    Jurisdiction,
    Keyword,
    Locator,
    Medium,
    Note,
    OriginalPublisher,
    OriginalPublisherPlace,
    OriginalTitle,
    Page,
    PageFirst,
    PMCID,
    PMID,
    Publisher,
    PublisherPlace,
    References,
    ReviewedTitle,
    Scale,
    Section,
    Source,
    Status,
    Title,
    TitleShort,
    URL,
    Version,
    YearSuffix,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CslnStyle {
    pub info: CslnInfo,
    pub locale: CslnLocale,
    pub citation: Vec<CslnNode>,
    pub bibliography: Vec<CslnNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct CslnLocale {
    pub terms: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CslnInfo {
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CslnNode {
    Text { value: String },
    Variable(VariableBlock),
    Date(DateBlock),
    Names(NamesBlock),
    Group(GroupBlock),
    Condition(ConditionBlock),
    Term(TermBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TermBlock {
    pub term: GeneralTerm,
    pub form: TermForm,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    pub source_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableBlock {
    pub variable: Variable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelOptions>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub overrides: HashMap<ItemType, FormattingOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupBlock {
    pub children: Vec<CslnNode>,
    pub delimiter: Option<String>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConditionBlock {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_item_type: Vec<ItemType>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_variables: Vec<Variable>,
    pub then_branch: Vec<CslnNode>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub else_if_branches: Vec<ElseIfBranch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub else_branch: Option<Vec<CslnNode>>,
}

/// An else-if branch in a condition block, capturing type or variable conditions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ElseIfBranch {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_item_type: Vec<ItemType>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub if_variables: Vec<Variable>,
    pub children: Vec<CslnNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LabelOptions {
    pub variable: Variable,
    pub form: LabelForm,
    pub pluralize: bool,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    Long,
    Short,
    Symbol,
    Verb,
    VerbShort,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DateBlock {
    pub variable: Variable,
    #[serde(flatten)]
    pub options: DateOptions,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NamesBlock {
    pub variable: Variable,
    #[serde(flatten)]
    pub options: NamesOptions,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_order: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub struct NamesOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<NameMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<AndTerm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_last: Option<DelimiterPrecedes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_as_sort_order: Option<NameAsSortOrder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub et_al: Option<EtAlOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelOptions>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub substitute: Vec<Variable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NameMode {
    Long,
    Short,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AndTerm {
    Text,
    Symbol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPrecedes {
    Contextual,
    AfterInvertedName,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NameAsSortOrder {
    First,
    All,
}

/// Configuration for et-al abbreviation in names.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EtAlOptions {
    /// Minimum number of names to trigger abbreviation.
    pub min: u8,
    /// Number of names to show when triggered.
    pub use_first: u8,
    /// Optional separate configuration for subsequent citations (CSL 1.0 legacy).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent: Option<Box<EtAlSubsequent>>,
    /// The term to use (e.g., "et al.", "and others").
    pub term: String,
    /// Formatting for the term (italic, bold).
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EtAlSubsequent {
    pub min: u8,
    pub use_first: u8,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DateOptions {
    pub form: Option<DateForm>,
    pub parts: Option<DateParts>,
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_form: Option<DatePartForm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    Text,
    Numeric,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DateParts {
    Year,
    YearMonth,
    YearMonthDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DatePartForm {
    Numeric,
    NumericLeadingZeros,
    Ordinal,
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FormattingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<TextDecoration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecoration {
    None,
    Underline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VerticalAlign {
    Baseline,
    Superscript,
    Subscript,
}
