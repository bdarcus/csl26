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
