use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Style {
    pub version: String,
    pub xmlns: String,
    pub class: String,
    pub info: Info,
    pub locale: Vec<Locale>,
    pub macros: Vec<Macro>,
    pub citation: Citation,
    pub bibliography: Option<Bibliography>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Info {
    pub title: String,
    pub id: String,
    pub updated: String,
    // Simplification for now
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Locale {
    pub lang: Option<String>,
    // Simplification for now
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Macro {
    pub name: String,
    pub children: Vec<CslNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Citation {
    pub layout: Layout,
    // Attributes
    pub et_al_min: Option<usize>,
    pub et_al_use_first: Option<usize>,
    pub disambiguate_add_year_suffix: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bibliography {
    pub layout: Layout,
    pub sort: Option<Sort>,
    // Attributes
    pub et_al_min: Option<usize>,
    pub et_al_use_first: Option<usize>,
    pub hanging_indent: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Layout {
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub delimiter: Option<String>,
    pub children: Vec<CslNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sort {
    pub keys: Vec<SortKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SortKey {
    pub variable: Option<String>,
    pub macro_name: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CslNode {
    Text(Text),
    Date(Date),
    Label(Label),
    Names(Names),
    Group(Group),
    Choose(Choose),
    Number(Number),
    Name(Name),
    EtAl(EtAl),
    Substitute(Substitute),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Text {
    pub value: Option<String>,
    pub variable: Option<String>,
    pub macro_name: Option<String>,
    pub term: Option<String>,
    pub form: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub quotes: Option<bool>,
    pub text_case: Option<String>,
    pub strip_periods: Option<bool>,
    pub plural: Option<String>,
    #[serde(flatten)]
    pub formatting: Formatting,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    pub and: Option<String>,
    pub delimiter: Option<String>,
    pub name_as_sort_order: Option<String>,
    pub sort_separator: Option<String>,
    pub initialize_with: Option<String>,
    pub form: Option<String>,
    pub delimiter_precedes_last: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EtAl {
    pub term: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Formatting {
    pub font_style: Option<String>,
    pub font_variant: Option<String>,
    pub font_weight: Option<String>,
    pub text_decoration: Option<String>,
    pub vertical_align: Option<String>,
    pub display: Option<String>, // Often specific to Group/Bibliography, but kept here for now
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Substitute {
    pub children: Vec<CslNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Date {
    pub variable: String,
    pub form: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub delimiter: Option<String>,
    pub date_parts: Option<String>,
    pub text_case: Option<String>,
    pub parts: Vec<DatePart>,
    #[serde(flatten)]
    pub formatting: Formatting,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatePart {
    pub name: String,
    pub form: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub variable: Option<String>,
    pub form: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub text_case: Option<String>,
    pub strip_periods: Option<bool>,
    pub plural: Option<String>,
    #[serde(flatten)]
    pub formatting: Formatting,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Names {
    pub variable: String,
    pub delimiter: Option<String>,
    pub children: Vec<CslNode>, // <name>, <label>, <substitute>, <et-al>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub delimiter: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub children: Vec<CslNode>,
    #[serde(flatten)]
    pub formatting: Formatting,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Choose {
    pub if_branch: ChooseBranch,
    pub else_if_branches: Vec<ChooseBranch>,
    pub else_branch: Option<Vec<CslNode>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChooseBranch {
    pub match_mode: Option<String>, // "any", "all", "none" (default "all" usually)
    pub type_: Option<String>,
    pub variable: Option<String>,
    pub is_numeric: Option<String>,
    pub locator: Option<String>,
    pub position: Option<String>,
    pub children: Vec<CslNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Number {
    pub variable: String,
    pub form: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub text_case: Option<String>,
    #[serde(flatten)]
    pub formatting: Formatting,
}
