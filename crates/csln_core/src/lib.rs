use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-exporting the previous enums (ItemType, Variable, etc. are already there)
// I'll include them in the full file write for completeness and to ensure it compiles.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ItemType {
    Article, ArticleJournal, ArticleMagazine, ArticleNewspaper, Bill, Book, Broadcast, 
    Chapter, Dataset, Entry, EntryDictionary, EntryEncyclopedia, Figure, Graphic, 
    Interview, LegalCase, Legislation, Manuscript, Map, MotionPicture, MusicalScore, 
    Pamphlet, PaperConference, Patent, PersonalCommunication, Post, PostWeblog, 
    Report, Review, ReviewBook, Song, Speech, Thesis, Treaty, Webpage, Software, Standard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Variable {
    Author, CollectionEditor, Composer, ContainerAuthor, Director, Editor, 
    EditorialDirector, Illustrator, Interviewer, OriginalAuthor, Recipient, 
    ReviewedAuthor, Translator, Accessed, AvailableDate, EventDate, Issued, 
    OriginalDate, Submitted, ChapterNumber, CollectionNumber, Edition, Issue, 
    Number, NumberOfPages, NumberOfVolumes, Volume, Abstract, Annote, Archive, 
    ArchiveLocation, ArchivePlace, Authority, CallNumber, CitationLabel, 
    CitationNumber, CollectionTitle, ContainerTitle, ContainerTitleShort, 
    Dimensions, DOI, Event, EventPlace, FirstReferenceNoteNumber, Genre, ISBN, 
    ISSN, Jurisdiction, Keyword, Locator, Medium, Note, OriginalPublisher, 
    OriginalPublisherPlace, OriginalTitle, Page, PageFirst, PMCID, PMID, 
    Publisher, PublisherPlace, References, ReviewedTitle, Scale, Section, 
    Source, Status, Title, TitleShort, URL, Version, YearSuffix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CslnStyle {
    pub info: CslnInfo,
    pub citation: Vec<CslnNode>,
    pub bibliography: Vec<CslnNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CslnInfo {
    pub title: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CslnNode {
    /// A literal string (prefix/suffix/delimiter).
    Text { value: String },
    /// A bibliographic variable with intelligent options.
    Variable(VariableBlock),
    /// A date variable with specialized formatting options.
    Date(DateBlock),
    /// A group of nodes with shared formatting and delimiter.
    Group(GroupBlock),
    /// A conditional block (fallback for logic that can't be upsampled).
    Condition(ConditionBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableBlock {
    pub variable: Variable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LabelOptions>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
    /// Type-specific overrides (e.g. Italics for Books).
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub overrides: HashMap<ItemType, FormattingOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupBlock {
    pub children: Vec<CslnNode>,
    pub delimiter: Option<String>,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionBlock {
    pub if_item_type: Vec<ItemType>,
    pub then_branch: Vec<CslnNode>,
    pub else_branch: Option<Vec<CslnNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelOptions {
    pub form: LabelForm,
    pub pluralize: bool,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    Long,
    Short,
    Symbol,
    Verb,
    VerbShort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateBlock {
    pub variable: Variable,
    #[serde(flatten)]
    pub options: DateOptions,
    #[serde(flatten)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DateOptions {
    pub form: Option<DateForm>,
    pub parts: Option<DateParts>, // "year", "year-month", etc.
    pub delimiter: Option<String>,
    // Per-part formatting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month_form: Option<DatePartForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_form: Option<DatePartForm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateForm {
    Text,
    Numeric,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateParts {
    Year,
    YearMonth,
    YearMonthDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DatePartForm {
    Numeric,
    NumericLeadingZeros,
    Ordinal,
    Long,
    Short,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct FormattingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_variant: Option<FontVariant>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quotes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle { Normal, Italic, Oblique }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontVariant { Normal, SmallCaps }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight { Normal, Bold, Light }
// ... (EtAlOptions etc from previous turn would go here too)
