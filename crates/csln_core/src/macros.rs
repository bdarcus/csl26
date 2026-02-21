/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Declarative macros for the CSLN ecosystem.

/// Generates a string-backed enum and its `as_str` method.
/// Preserves any doc comments and derive macros on the enum and its variants.
#[macro_export]
macro_rules! str_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $val:expr
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[non_exhaustive]
        $vis enum $name {
            $(
                $(#[$vmeta])*
                $variant,
            )+
        }

        impl $name {
            #[doc = "Returns the string value associated with this variant."]
            pub fn as_str(&self) -> &'static str {
                match self {
                    $( Self::$variant => $val, )+
                }
            }
        }
    }
}

/// Dispatches an operation across all variants of `TemplateComponent`.
/// Requires `$target` to be a `TemplateComponent` and provides `$inner`
/// to the closure/expression provided in `$action`.
#[macro_export]
macro_rules! dispatch_component {
    ($target:expr, |$inner:ident| $action:expr) => {
        match $target {
            $crate::template::TemplateComponent::Contributor($inner) => $action,
            $crate::template::TemplateComponent::Date($inner) => $action,
            $crate::template::TemplateComponent::Title($inner) => $action,
            $crate::template::TemplateComponent::Number($inner) => $action,
            $crate::template::TemplateComponent::Variable($inner) => $action,
            $crate::template::TemplateComponent::List($inner) => $action,
            $crate::template::TemplateComponent::Term($inner) => $action,
        }
    };
}

/// Merges fields from a target struct `source` into a mutable `target` if `source.field.is_some()`.
/// This simplifies boilerplate in configuration merge implementations.
#[macro_export]
macro_rules! merge_options {
    ($target:expr, $source:expr, $($field:ident),+ $(,)?) => {
        $(
            if $source.$field.is_some() {
                $target.$field = $source.$field.clone();
            }
        )+
    };
}

// AST Builder macros for tests and embedded styles.
// These use a quasi-DSL to quickly stamp out TemplateComponents.

#[macro_export]
macro_rules! tc_contributor {
    ($role:ident, $form:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Contributor(
            $crate::template::TemplateContributor {
                contributor: $crate::template::ContributorRole::$role,
                form: $crate::template::ContributorForm::$form,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_date {
    ($date_var:ident, $form:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Date(
            $crate::template::TemplateDate {
                date: $crate::template::DateVariable::$date_var,
                form: $crate::template::DateForm::$form,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_title {
    ($title_type:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Title(
            $crate::template::TemplateTitle {
                title: $crate::template::TitleType::$title_type,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_number {
    ($num_var:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Number(
            $crate::template::TemplateNumber {
                number: $crate::template::NumberVariable::$num_var,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_variable {
    ($var:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Variable(
            $crate::template::TemplateVariable {
                variable: $crate::template::SimpleVariable::$var,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_term {
    ($term_var:ident $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::Term(
            $crate::template::TemplateTerm {
                term: $crate::localization::GeneralTerm::$term_var,
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

#[macro_export]
macro_rules! tc_list {
    ([$($item:expr),* $(,)?] $(, $key:ident = $val:expr)*) => {
        $crate::template::TemplateComponent::List(
            $crate::template::TemplateList {
                items: vec![$($item),*],
                rendering: $crate::template::Rendering {
                    $( $key: Some($val.into()), )*
                    ..Default::default()
                },
                ..Default::default()
            }
        )
    };
}

// Reference builder macros for tests and fixtures.
// These construct native CSLN InputReference values without verbose struct literals.

/// Builds an `InputReference::Monograph` (book) with a single structured-name author.
///
/// # Examples
/// ```ignore
/// let r = csln_core::ref_book!("b1", "Smith", "John", 2020, "My Title");
/// ```
#[macro_export]
macro_rules! ref_book {
    ($id:expr, $family:expr, $given:expr, $year:expr, $title:expr) => {
        $crate::reference::InputReference::Monograph(::std::boxed::Box::new(
            $crate::reference::Monograph {
                id: Some($id.to_string()),
                r#type: $crate::reference::MonographType::Book,
                title: $crate::reference::Title::Single($title.to_string()),
                author: Some($crate::reference::Contributor::StructuredName(
                    $crate::reference::StructuredName {
                        family: $crate::reference::MultilingualString::Simple($family.to_string()),
                        given: $crate::reference::MultilingualString::Simple($given.to_string()),
                        ..Default::default()
                    },
                )),
                editor: None,
                translator: None,
                issued: $crate::reference::EdtfString($year.to_string()),
                publisher: None,
                url: None,
                accessed: None,
                language: None,
                note: None,
                isbn: None,
                doi: None,
                edition: None,
                report_number: None,
                collection_number: None,
                genre: None,
                medium: None,
                keywords: None,
                original_date: None,
                original_title: None,
            },
        ))
    };
}

/// Builds an `InputReference::Monograph` (book) with multiple structured-name authors.
///
/// # Examples
/// ```ignore
/// let r = csln_core::ref_book_authors!("b1", [("Smith", "John"), ("Doe", "Jane")], 2020, "Title");
/// ```
#[macro_export]
macro_rules! ref_book_authors {
    ($id:expr, [$(($family:expr, $given:expr)),* $(,)?], $year:expr, $title:expr) => {{
        let _authors: Vec<$crate::reference::Contributor> = vec![
            $(
                $crate::reference::Contributor::StructuredName(
                    $crate::reference::StructuredName {
                        family: $crate::reference::MultilingualString::Simple(
                            $family.to_string(),
                        ),
                        given: $crate::reference::MultilingualString::Simple($given.to_string()),
                        ..Default::default()
                    },
                ),
            )*
        ];
        $crate::reference::InputReference::Monograph(::std::boxed::Box::new(
            $crate::reference::Monograph {
                id: Some($id.to_string()),
                r#type: $crate::reference::MonographType::Book,
                title: $crate::reference::Title::Single($title.to_string()),
                author: Some($crate::reference::Contributor::ContributorList(
                    $crate::reference::ContributorList(_authors),
                )),
                editor: None,
                translator: None,
                issued: $crate::reference::EdtfString($year.to_string()),
                publisher: None,
                url: None,
                accessed: None,
                language: None,
                note: None,
                isbn: None,
                doi: None,
                edition: None,
                report_number: None,
                collection_number: None,
                genre: None,
                medium: None,
                keywords: None,
                original_date: None,
                original_title: None,
            },
        ))
    }};
}

/// Builds an `InputReference::SerialComponent` (journal article) with a single author.
///
/// # Examples
/// ```ignore
/// let r = csln_core::ref_article!("a1", "Doe", "Jane", 2021, "Article Title");
/// ```
#[macro_export]
macro_rules! ref_article {
    ($id:expr, $family:expr, $given:expr, $year:expr, $title:expr) => {
        $crate::reference::InputReference::SerialComponent(::std::boxed::Box::new(
            $crate::reference::SerialComponent {
                id: Some($id.to_string()),
                r#type: $crate::reference::SerialComponentType::Article,
                title: Some($crate::reference::Title::Single($title.to_string())),
                author: Some($crate::reference::Contributor::StructuredName(
                    $crate::reference::StructuredName {
                        family: $crate::reference::MultilingualString::Simple($family.to_string()),
                        given: $crate::reference::MultilingualString::Simple($given.to_string()),
                        ..Default::default()
                    },
                )),
                translator: None,
                issued: $crate::reference::EdtfString($year.to_string()),
                parent: $crate::reference::Parent::Embedded($crate::reference::Serial {
                    r#type: $crate::reference::SerialType::AcademicJournal,
                    title: $crate::reference::Title::Single(String::new()),
                    editor: None,
                    publisher: None,
                    issn: None,
                }),
                url: None,
                accessed: None,
                language: None,
                note: None,
                doi: None,
                pages: None,
                volume: None,
                issue: None,
                genre: None,
                medium: None,
                keywords: None,
            },
        ))
    };
}

/// Builds an `InputReference::SerialComponent` (journal article) with multiple authors.
///
/// # Examples
/// ```ignore
/// let r = csln_core::ref_article_authors!("a1", [("Doe", "Jane"), ("Roe", "John")], 2021, "Title");
/// ```
#[macro_export]
macro_rules! ref_article_authors {
    ($id:expr, [$(($family:expr, $given:expr)),* $(,)?], $year:expr, $title:expr) => {{
        let _authors: Vec<$crate::reference::Contributor> = vec![
            $(
                $crate::reference::Contributor::StructuredName(
                    $crate::reference::StructuredName {
                        family: $crate::reference::MultilingualString::Simple(
                            $family.to_string(),
                        ),
                        given: $crate::reference::MultilingualString::Simple($given.to_string()),
                        ..Default::default()
                    },
                ),
            )*
        ];
        $crate::reference::InputReference::SerialComponent(::std::boxed::Box::new(
            $crate::reference::SerialComponent {
                id: Some($id.to_string()),
                r#type: $crate::reference::SerialComponentType::Article,
                title: Some($crate::reference::Title::Single($title.to_string())),
                author: Some($crate::reference::Contributor::ContributorList(
                    $crate::reference::ContributorList(_authors),
                )),
                translator: None,
                issued: $crate::reference::EdtfString($year.to_string()),
                parent: $crate::reference::Parent::Embedded($crate::reference::Serial {
                    r#type: $crate::reference::SerialType::AcademicJournal,
                    title: $crate::reference::Title::Single(String::new()),
                    editor: None,
                    publisher: None,
                    issn: None,
                }),
                url: None,
                accessed: None,
                language: None,
                note: None,
                doi: None,
                pages: None,
                volume: None,
                issue: None,
                genre: None,
                medium: None,
                keywords: None,
            },
        ))
    }};
}

/// Builds a `CitationItem` with optional named fields.
///
/// Supported fields: `locator`, `prefix`, `suffix`, `label`.
///
/// # Examples
/// ```ignore
/// let item = csln_core::citation_item!("kuhn1962");
/// let item = csln_core::citation_item!("kuhn1962", locator = "42");
/// let item = csln_core::citation_item!("kuhn1962", label = csln_core::citation::LocatorType::Page, locator = "42");
/// let item = csln_core::citation_item!("smith2020", prefix = "cf. ");
/// ```
#[macro_export]
macro_rules! citation_item {
    ($id:expr $(, $key:ident = $val:expr)*) => {{
        #[allow(unused_mut)]
        let mut _item = $crate::citation::CitationItem {
            id: $id.to_string(),
            ..Default::default()
        };
        $( citation_item!(@set _item, $key, $val); )*
        _item
    }};
    (@set $item:ident, locator, $val:expr) => { $item.locator = Some($val.to_string()); };
    (@set $item:ident, prefix, $val:expr) => { $item.prefix = Some($val.to_string()); };
    (@set $item:ident, suffix, $val:expr) => { $item.suffix = Some($val.to_string()); };
    (@set $item:ident, label, $val:expr) => { $item.label = Some($val); };
}

/// Builds a `Citation` from a list of `CitationItem` expressions with optional named fields.
///
/// Supported citation fields: `mode`, `suppress_author`.
///
/// # Examples
/// ```ignore
/// // Two items, no options
/// let c = csln_core::citation!([
///     csln_core::citation_item!("item1"),
///     csln_core::citation_item!("item2", locator = "42"),
/// ]);
///
/// // Integral mode
/// let c = csln_core::citation!(
///     [csln_core::citation_item!("item1"), csln_core::citation_item!("item2")],
///     mode = csln_core::citation::CitationMode::Integral,
/// );
///
/// // Suppress author across all items
/// let c = csln_core::citation!(
///     [csln_core::citation_item!("item1")],
///     suppress_author = true,
/// );
/// ```
#[macro_export]
macro_rules! citation {
    ([$($item:expr),* $(,)?] $(, $key:ident = $val:expr)* $(,)?) => {
        $crate::citation::Citation {
            items: vec![$($item),*],
            $($key: $val,)*
            ..Default::default()
        }
    };
}

/// Builds a `Citation` with one `CitationItem`.
///
/// # Examples
/// ```ignore
/// let c = csln_core::cite!("item1");
/// let c = csln_core::cite!("item1", mode = csln_core::citation::CitationMode::Integral);
/// let c = csln_core::cite!("item1", suppress_author = true);
/// ```
#[macro_export]
macro_rules! cite {
    ($id:expr) => {
        $crate::citation::Citation {
            items: vec![$crate::citation::CitationItem {
                id: $id.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    };
    ($id:expr, $key:ident = $val:expr) => {
        $crate::citation::Citation {
            items: vec![$crate::citation::CitationItem {
                id: $id.to_string(),
                ..Default::default()
            }],
            $key: $val,
            ..Default::default()
        }
    };
}

/// Builds an `IndexMap<String, InputReference>` from key-value pairs.
///
/// Requires `indexmap` as a dependency of the calling crate.
///
/// # Examples
/// ```ignore
/// let bib = csln_core::bib_map![
///     "ref1" => make_book("ref1", "Smith", "J", 2020, "Title"),
///     "ref2" => make_article("ref2", "Doe", "J", 2021, "Article"),
/// ];
/// ```
#[macro_export]
macro_rules! bib_map {
    ($($key:expr => $val:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut _map = indexmap::IndexMap::new();
        $( _map.insert($key.to_string(), $val); )*
        _map
    }};
}
