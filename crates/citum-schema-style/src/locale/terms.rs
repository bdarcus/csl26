/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::citation::LocatorType;
use crate::template::ContributorRole;

use super::Locale;
use super::types::{GeneralTerm, GrammaticalGender, MaybeGendered, SimpleTerm, TermForm};

/// Identifies a field in the archive hierarchy for locale term lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveHierarchyField {
    /// Named collection or record group.
    Collection,
    /// Named series or sub-collection.
    Series,
    /// Box or container designation.
    Box,
    /// Folder designation.
    Folder,
    /// Item, file, or reference-code designation.
    Item,
}

impl ArchiveHierarchyField {
    /// Returns the MF2 message ID for this field's locale label.
    fn message_id(self) -> &'static str {
        match self {
            Self::Collection => "term.archive-collection-label",
            Self::Series => "term.archive-series-label",
            Self::Box => "term.archive-box-label",
            Self::Folder => "term.archive-folder-label",
            Self::Item => "term.archive-item-label",
        }
    }
}

impl Locale {
    /// Get a contributor role term.
    fn resolve_gendered_value(
        value: &MaybeGendered<String>,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&str> {
        value
            .resolve_with_fallback(requested_gender)
            .map(String::as_str)
    }

    fn resolve_gendered_value_neutral(value: &MaybeGendered<String>) -> Option<&str> {
        value.resolve_neutral().map(String::as_str)
    }

    fn resolve_no_date_value<'a>(
        value: &'a SimpleTerm,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&'a str> {
        match requested_gender {
            Some(GrammaticalGender::Common) => match *form {
                TermForm::Long => value
                    .long
                    .resolve_strict(Some(GrammaticalGender::Common))
                    .map(String::as_str),
                TermForm::Short => value
                    .short
                    .resolve_strict(Some(GrammaticalGender::Common))
                    .map(String::as_str)
                    .filter(|value| !value.is_empty())
                    .or_else(|| {
                        value
                            .long
                            .resolve_strict(Some(GrammaticalGender::Common))
                            .map(String::as_str)
                    }),
                _ => value
                    .long
                    .resolve_strict(Some(GrammaticalGender::Common))
                    .map(String::as_str),
            },
            _ => match *form {
                TermForm::Long => Self::resolve_gendered_value(&value.long, requested_gender),
                TermForm::Short => {
                    Self::resolve_gendered_value(&value.short, requested_gender.clone())
                        .filter(|value| !value.is_empty())
                        .or_else(|| Self::resolve_gendered_value(&value.long, requested_gender))
                }
                _ => Self::resolve_gendered_value(&value.long, requested_gender),
            },
        }
    }

    /// Get a contributor role term.
    pub fn role_term(
        &self,
        role: &ContributorRole,
        plural: bool,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&str> {
        let term = self.roles.get(role)?;
        let simple = if plural { &term.plural } else { &term.singular };
        let term_text = match *form {
            TermForm::Long => Self::resolve_gendered_value(&simple.long, requested_gender),
            TermForm::Short => {
                Self::resolve_gendered_value(&simple.short, requested_gender.clone())
                    .filter(|value| !value.is_empty())
                    .or_else(|| Self::resolve_gendered_value(&simple.long, requested_gender))
            }
            TermForm::Verb => Self::resolve_gendered_value(&term.verb.long, None),
            TermForm::VerbShort => Self::resolve_gendered_value(&term.verb.short, None)
                .filter(|value| !value.is_empty())
                .or_else(|| Self::resolve_gendered_value(&term.verb.long, None)),
            _ => Self::resolve_gendered_value(&simple.long, requested_gender),
        };

        match term_text {
            Some(value) if !value.is_empty() => Some(value),
            _ => None,
        }
    }

    /// Resolve a contributor role term using only neutral/common values.
    pub fn role_term_neutral(
        &self,
        role: &ContributorRole,
        plural: bool,
        form: &TermForm,
    ) -> Option<&str> {
        let term = self.roles.get(role)?;
        let simple = if plural { &term.plural } else { &term.singular };
        let term_text = match *form {
            TermForm::Long => Self::resolve_gendered_value_neutral(&simple.long),
            TermForm::Short => Self::resolve_gendered_value_neutral(&simple.short)
                .filter(|value| !value.is_empty())
                .or_else(|| Self::resolve_gendered_value_neutral(&simple.long)),
            TermForm::Verb => Self::resolve_gendered_value(&term.verb.long, None),
            TermForm::VerbShort => Self::resolve_gendered_value(&term.verb.short, None)
                .filter(|value| !value.is_empty())
                .or_else(|| Self::resolve_gendered_value(&term.verb.long, None)),
            _ => Self::resolve_gendered_value_neutral(&simple.long),
        };

        match term_text {
            Some(value) if !value.is_empty() => Some(value),
            _ => None,
        }
    }

    /// Resolve a contributor role term, evaluating MF2 messages when configured.
    pub fn resolved_role_term(
        &self,
        role: &ContributorRole,
        plural: bool,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<String> {
        if let Some(message_id) = Self::role_message_id(role, form)
            && let Some(resolved) = self.resolve_message_text(
                message_id,
                Some(u64::from(plural) + 1),
                requested_gender.clone(),
            )
        {
            return Some(resolved);
        }

        self.role_term(role, plural, form, requested_gender)
            .map(ToOwned::to_owned)
    }

    /// Resolve a contributor role term using only neutral/common values.
    pub fn resolved_role_term_neutral(
        &self,
        role: &ContributorRole,
        plural: bool,
        form: &TermForm,
    ) -> Option<String> {
        if let Some(message_id) = Self::role_message_id(role, form)
            && let Some(resolved) = self.resolve_message_text(
                message_id,
                Some(u64::from(plural) + 1),
                Some(GrammaticalGender::Common),
            )
        {
            return Some(resolved);
        }

        self.role_term_neutral(role, plural, form)
            .map(ToOwned::to_owned)
    }

    /// Resolve an authored term for an ordered combination of contributor roles.
    #[must_use]
    pub fn resolved_role_combination_term(
        &self,
        roles: &[ContributorRole],
        plural: bool,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<String> {
        if roles.len() < 2 {
            return None;
        }
        let key = if roles.len() == 2
            && roles.contains(&ContributorRole::Editor)
            && roles.contains(&ContributorRole::Translator)
        {
            "editor-translator".to_string()
        } else {
            roles
                .iter()
                .map(ContributorRole::as_str)
                .collect::<Vec<_>>()
                .join("-")
        };
        let term = self.role_combinations.get(&key)?;
        let simple = if plural { &term.plural } else { &term.singular };
        let value = match *form {
            TermForm::Long => Self::resolve_gendered_value(&simple.long, requested_gender),
            TermForm::Short => {
                Self::resolve_gendered_value(&simple.short, requested_gender.clone())
                    .filter(|value| !value.is_empty())
                    .or_else(|| Self::resolve_gendered_value(&simple.long, requested_gender))
            }
            TermForm::Verb => Self::resolve_gendered_value(&term.verb.long, requested_gender),
            TermForm::VerbShort => {
                Self::resolve_gendered_value(&term.verb.short, requested_gender.clone())
                    .filter(|value| !value.is_empty())
                    .or_else(|| Self::resolve_gendered_value(&term.verb.long, requested_gender))
            }
            _ => Self::resolve_gendered_value(&simple.long, requested_gender),
        };
        value
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    /// Return the locale connector used to compose a missing combined-role term.
    #[must_use]
    pub fn role_conjunction(&self) -> &str {
        self.general_term(&GeneralTerm::RoleConjunction, &TermForm::Long, None)
            .unwrap_or(" & ")
    }

    /// Get a locator term.
    pub fn locator_term(
        &self,
        locator: &LocatorType,
        plural: bool,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&str> {
        let term = self.locators.get(locator)?;
        let form_term = match *form {
            TermForm::Long => &term.long,
            TermForm::Short => &term.short,
            TermForm::Symbol => &term.symbol,
            _ => &term.short, // Fallback
        };

        if let Some(ft) = form_term {
            let value = if plural { &ft.plural } else { &ft.singular };
            Self::resolve_gendered_value(value, requested_gender)
        } else {
            None
        }
    }

    /// Resolve a locator term, evaluating MF2 messages when configured.
    pub fn resolved_locator_term(
        &self,
        locator: &LocatorType,
        plural: bool,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<String> {
        if let Some(message_id) = Self::locator_message_id(locator, form)
            && let Some(resolved) = self.resolve_message_text(
                message_id,
                Some(u64::from(plural) + 1),
                requested_gender.clone(),
            )
        {
            return Some(resolved);
        }

        self.locator_term(locator, plural, form, requested_gender.clone())
            .map(ToOwned::to_owned)
            .or_else(|| {
                if let LocatorType::Custom(key) = locator {
                    self.locator_term_any_form(locator, plural, requested_gender)
                        .map(ToOwned::to_owned)
                        .or_else(|| Some(key.clone()))
                } else {
                    None
                }
            })
    }

    fn locator_term_any_form(
        &self,
        locator: &LocatorType,
        plural: bool,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&str> {
        let term = self.locators.get(locator)?;
        [&term.long, &term.short, &term.symbol]
            .into_iter()
            .flatten()
            .next()
            .map(|forms| {
                if plural {
                    Self::resolve_gendered_value(&forms.plural, requested_gender).unwrap_or("")
                } else {
                    Self::resolve_gendered_value(&forms.singular, requested_gender).unwrap_or("")
                }
            })
            .filter(|value| !value.is_empty())
    }

    /// Resolve a general term to a borrowed string.
    pub fn general_term(
        &self,
        term: &GeneralTerm,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<&str> {
        // Legacy borrowed lookup path: prefer plain v2 messages first, then
        // alias-backed messages, and finally the v1 term tables. The
        // candidate id must be form-aware (`general_message_id`) rather than
        // form-blind, otherwise Long/Short forms collapse onto whichever
        // message happens to exist for the form-blind id (e.g. `no date`
        // always resolving to the short "n.d." even when Long was
        // requested).
        if let Some(candidate_id) = Self::general_message_id(term, form)
            && let Some(msg) = self.messages.get(candidate_id)
        {
            // Only use plain messages here (no ICU variable syntax)
            if !msg.contains('{') {
                return Some(msg.as_str());
            }
        }
        // Check legacy_term_aliases
        let legacy_key = Self::general_term_to_legacy_key(term);
        if let Some(msg_id) = self.legacy_term_aliases.get(legacy_key)
            && let Some(msg) = self.messages.get(msg_id)
            && !msg.contains('{')
        {
            return Some(msg.as_str());
        }

        // First try the flattened map
        if *term != GeneralTerm::NoDate
            && let Some(simple) = self.terms.general.get(term)
        {
            return match *form {
                TermForm::Long => Self::resolve_gendered_value(&simple.long, requested_gender),
                TermForm::Short => {
                    Self::resolve_gendered_value(&simple.short, requested_gender.clone())
                        .filter(|value| !value.is_empty())
                        .or_else(|| Self::resolve_gendered_value(&simple.long, requested_gender))
                }

                _ => Self::resolve_gendered_value(&simple.long, requested_gender),
            };
        }

        // Fallback to specific fields for common terms
        match term {
            GeneralTerm::And => self.terms.and.as_deref(),
            GeneralTerm::EtAl => self.terms.et_al.as_deref(),
            GeneralTerm::AndOthers => self.terms.and_others.as_deref(),
            GeneralTerm::Accessed => self.terms.accessed.as_deref(),
            GeneralTerm::Ibid => self.terms.ibid.as_deref(),
            GeneralTerm::In => self.terms.in_.as_deref(),
            GeneralTerm::NoDate => self
                .terms
                .general
                .get(term)
                .and_then(|value| Self::resolve_no_date_value(value, form, requested_gender))
                .or(self.terms.no_date.as_deref()),
            GeneralTerm::Retrieved => self.terms.retrieved.as_deref(),
            GeneralTerm::At => self.terms.at.as_deref(),
            GeneralTerm::By => self.terms.by.as_deref(),
            GeneralTerm::From => self.terms.from.as_deref(),
            GeneralTerm::Of => self
                .terms
                .general
                .get(term)
                .and_then(|value| Self::resolve_gendered_value(&value.long, requested_gender)),
            GeneralTerm::To => self
                .terms
                .general
                .get(term)
                .and_then(|value| Self::resolve_gendered_value(&value.long, requested_gender)),
            GeneralTerm::Anonymous => {
                Self::resolve_gendered_value(&self.terms.anonymous.long, requested_gender)
            }
            GeneralTerm::Circa => {
                Self::resolve_gendered_value(&self.terms.circa.long, requested_gender)
            }
            // Fallback to locators for shared terms
            GeneralTerm::Volume => {
                self.locator_term(&LocatorType::Volume, false, form, requested_gender)
            }
            GeneralTerm::Issue => {
                self.locator_term(&LocatorType::Issue, false, form, requested_gender)
            }
            GeneralTerm::Page => {
                self.locator_term(&LocatorType::Page, false, form, requested_gender)
            }
            GeneralTerm::Chapter => {
                self.locator_term(&LocatorType::Chapter, false, form, requested_gender)
            }
            GeneralTerm::Section => {
                self.locator_term(&LocatorType::Section, false, form, requested_gender)
            }
            GeneralTerm::Here => self
                .terms
                .general
                .get(term)
                .and_then(|value| Self::resolve_gendered_value(&value.long, requested_gender)),
            GeneralTerm::Deposited => self
                .terms
                .general
                .get(term)
                .and_then(|value| Self::resolve_gendered_value(&value.long, requested_gender)),
            _ => None,
        }
    }

    /// Resolve a general term, evaluating MF2 messages when configured.
    pub fn resolved_general_term(
        &self,
        term: &GeneralTerm,
        form: &TermForm,
        requested_gender: Option<GrammaticalGender>,
    ) -> Option<String> {
        if let Some(message_id) = Self::general_message_id(term, form)
            && let Some(resolved) =
                self.resolve_message_text(message_id, None, requested_gender.clone())
        {
            return Some(resolved);
        }

        self.general_term(term, form, requested_gender)
            .map(ToOwned::to_owned)
    }

    /// Resolve a reference-type description term (e.g. `"dataset"`,
    /// `"article-journal"`) for the `type-label` template component.
    ///
    /// Looks up `ref_type` in [`Locale::type_terms`] directly — unlike
    /// [`Self::resolved_general_term`], there is no MF2 message path for
    /// these terms yet, since they are dynamically keyed by an open
    /// `ref_type` string rather than the closed [`GeneralTerm`] enum.
    #[must_use]
    pub fn resolved_type_term(&self, ref_type: &str, form: &TermForm) -> Option<String> {
        let simple = self.type_terms.get(ref_type)?;
        match form {
            TermForm::Short => Self::resolve_gendered_value(&simple.short, None)
                .filter(|value| !value.is_empty())
                .or_else(|| Self::resolve_gendered_value(&simple.long, None)),
            _ => Self::resolve_gendered_value(&simple.long, None),
        }
        .map(ToOwned::to_owned)
    }

    /// Resolve an archive hierarchy label, using MF2 messages.
    /// Returns singular form (count=1) by default.
    pub fn resolved_archive_term(&self, field: ArchiveHierarchyField) -> Option<String> {
        self.resolve_message_text(field.message_id(), Some(1), None)
    }

    /// Get the "and" term based on style preference.
    pub fn and_term(&self, use_symbol: bool) -> &str {
        if use_symbol {
            self.terms.and_symbol.as_deref().unwrap_or("&")
        } else {
            self.terms.and.as_deref().unwrap_or("and")
        }
    }

    /// Get the "et al." term.
    pub fn et_al(&self) -> &str {
        self.terms.et_al.as_deref().unwrap_or("et al.")
    }

    /// Get a month name for a calendar month (`1`-`12`), by its EDTF
    /// sub-year code. Returns an empty string for an out-of-range month.
    pub fn month_name(&self, month: u8, short: bool) -> &str {
        let Some(code) = super::types::SubYearCode::new(month) else {
            return "";
        };
        let table = if short {
            &self.dates.months.short
        } else {
            &self.dates.months.long
        };
        table.get(&code).map(String::as_str).unwrap_or("")
    }
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
    fn test_en_us_locale_terms() {
        let locale = Locale::en_us();
        assert_eq!(locale.and_term(false), "and");
        assert_eq!(locale.and_term(true), "&");
        assert_eq!(locale.et_al(), "et al.");
    }

    #[test]
    fn test_month_names() {
        let locale = Locale::en_us();
        assert_eq!(locale.month_name(1, false), "January");
        assert_eq!(locale.month_name(1, true), "Jan.");
        assert_eq!(locale.month_name(12, false), "December");
    }

    #[test]
    fn test_role_terms() {
        let locale = Locale::en_us();

        assert_eq!(
            locale.role_term(&ContributorRole::Editor, false, &TermForm::Short, None),
            Some("ed.")
        );
        assert_eq!(
            locale.role_term(&ContributorRole::Editor, true, &TermForm::Short, None),
            Some("eds.")
        );
        assert_eq!(
            locale.role_term(&ContributorRole::Translator, false, &TermForm::Verb, None),
            Some("translated by")
        );
    }

    #[test]
    fn test_no_date_term_resolves_long_and_short_forms() {
        let locale = Locale::en_us();

        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Long, None),
            Some("no date")
        );
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Short, None),
            Some("n.d.")
        );
    }

    #[test]
    fn test_no_date_term_falls_back_to_legacy_short_form() {
        let mut locale = Locale::default();
        locale.terms.no_date = Some("n.d.".to_string());

        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Short, None),
            Some("n.d.")
        );
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Long, None),
            Some("n.d.")
        );
    }

    #[test]
    fn test_yaml_no_date_term_preserves_long_and_short_forms() {
        let yaml = r#"
locale: en-US
dates:
  months:
    long: [January, February, March, April, May, June, July, August, September, October, November, December]
    short: [Jan., Feb., Mar., Apr., May, June, July, Aug., Sept., Oct., Nov., Dec.]
  seasons: [Spring, Summer, Autumn, Winter]
roles: {}
terms:
  no date:
    long: no date
    short: n.d.
"#;

        let locale = Locale::from_yaml_str(yaml).unwrap();
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Long, None),
            Some("no date")
        );
        assert_eq!(
            locale.general_term(&GeneralTerm::NoDate, &TermForm::Short, None),
            Some("n.d.")
        );
        assert_eq!(locale.terms.no_date.as_deref(), Some("n.d."));
    }

    #[test]
    fn test_resolved_locator_term_evaluates_plural_message() {
        let locale = Locale::en_us();

        assert_eq!(
            locale.resolved_locator_term(&LocatorType::Page, false, &TermForm::Short, None),
            Some("p.".to_string())
        );
        assert_eq!(
            locale.resolved_locator_term(&LocatorType::Page, true, &TermForm::Short, None),
            Some("pp.".to_string())
        );
    }

    #[test]
    fn test_resolved_locator_term_falls_back_to_custom_locale_form_then_raw_key() {
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
locators:
  reel:
    long:
      singular: "reel"
      plural: "reels"
"#,
        )
        .expect("custom locale should parse");

        assert_eq!(
            locale.resolved_locator_term(
                &LocatorType::Custom("reel".to_string()),
                false,
                &TermForm::Short,
                None,
            ),
            Some("reel".to_string())
        );
        assert_eq!(
            locale.resolved_locator_term(
                &LocatorType::Custom("movement".to_string()),
                false,
                &TermForm::Short,
                None,
            ),
            Some("movement".to_string())
        );
    }

    #[test]
    fn test_legacy_locator_terms_under_terms_still_populate_locators() {
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
terms:
  page:
    short:
      singular: "pg."
      plural: "pgs."
"#,
        )
        .expect("legacy locator terms should parse");

        assert_eq!(
            locale.resolved_locator_term(&LocatorType::Page, false, &TermForm::Short, None),
            Some("pg.".to_string())
        );
    }

    #[test]
    fn test_explicit_locators_override_legacy_terms_for_builtin_keys() {
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
terms:
  page:
    short:
      singular: "pg."
      plural: "pgs."
locators:
  page:
    short:
      singular: "p."
      plural: "pp."
"#,
        )
        .expect("mixed locator forms should parse");

        assert_eq!(
            locale.resolved_locator_term(&LocatorType::Page, false, &TermForm::Short, None),
            Some("p.".to_string())
        );
    }

    #[test]
    fn test_non_locator_terms_are_not_reclassified_as_custom_locators() {
        let locale = Locale::from_yaml_str(
            r#"
locale: en-US
terms:
  and:
    long: "und"
"#,
        )
        .expect("general terms should parse");

        assert_eq!(locale.terms.and.as_deref(), Some("und"));
        assert!(
            !locale
                .locators
                .contains_key(&LocatorType::Custom("and".to_string()))
        );
    }

    #[test]
    fn test_resolved_role_term_evaluates_plural_message() {
        let locale = Locale::en_us();

        assert_eq!(
            locale.resolved_role_term(&ContributorRole::Editor, false, &TermForm::Long, None),
            Some("editor".to_string())
        );
        assert_eq!(
            locale.resolved_role_term(&ContributorRole::Editor, true, &TermForm::Long, None),
            Some("editors".to_string())
        );
    }

    #[test]
    fn test_role_term_prefers_common_form_for_mixed_gender_requests() {
        let locale = Locale::from_yaml_str(
            r#"
locale: es-ES
roles:
  editor:
    long:
      singular:
        masculine: editor
        feminine: editora
        common: persona editora
      plural:
        masculine: editores
        feminine: editoras
        common: equipo editorial
    short:
      singular: ed.
      plural: eds.
    verb: editado por
"#,
        )
        .expect("gendered locale should parse");

        assert_eq!(
            locale.role_term(
                &ContributorRole::Editor,
                false,
                &TermForm::Long,
                Some(GrammaticalGender::Feminine),
            ),
            Some("editora")
        );
        assert_eq!(
            locale.role_term(
                &ContributorRole::Editor,
                true,
                &TermForm::Long,
                Some(GrammaticalGender::Common),
            ),
            Some("equipo editorial")
        );
    }

    #[test]
    fn test_no_date_term_falls_back_when_requested_gender_has_no_matching_slot() {
        let locale = Locale::from_yaml_str(
            r#"
locale: es-ES
terms:
  no date:
    long:
      masculine: sin fecha
  no_date: s. f.
"#,
        )
        .expect("locale should parse");

        assert_eq!(
            locale.general_term(
                &GeneralTerm::NoDate,
                &TermForm::Long,
                Some(GrammaticalGender::Common),
            ),
            Some("s. f.")
        );
    }

    #[test]
    fn test_es_es_locale_is_embedded() {
        let bytes = crate::embedded::get_locale_bytes("es-ES").expect("es-ES should be embedded");
        let yaml = std::str::from_utf8(bytes).expect("embedded locale should be utf-8");
        let locale = Locale::from_yaml_str(yaml).expect("embedded es-ES should parse");

        assert_eq!(locale.locale, "es-ES");
        assert_eq!(
            locale.resolved_role_term(
                &ContributorRole::Editor,
                false,
                &TermForm::Long,
                Some(GrammaticalGender::Feminine),
            ),
            Some("editora".to_string())
        );
    }

    #[test]
    fn test_es_es_role_term_resolves_gendered_mf2_message() {
        let bytes = crate::embedded::get_locale_bytes("es-ES").expect("es-ES should be embedded");
        let yaml = std::str::from_utf8(bytes).expect("embedded locale should be utf-8");
        let locale = Locale::from_yaml_str(yaml).expect("embedded es-ES should parse");

        assert_eq!(
            locale.resolved_role_term(
                &ContributorRole::Editor,
                true,
                &TermForm::Long,
                Some(GrammaticalGender::Masculine),
            ),
            Some("editores".to_string())
        );
        assert_eq!(
            locale.resolved_role_term(
                &ContributorRole::Translator,
                true,
                &TermForm::Long,
                Some(GrammaticalGender::Feminine),
            ),
            Some("traductoras".to_string())
        );
        assert_eq!(
            locale.resolved_role_term_neutral(&ContributorRole::Editor, true, &TermForm::Long),
            Some("equipo editorial".to_string())
        );
    }

    #[test]
    fn test_role_term_falls_back_when_mf2_message_cannot_evaluate() {
        let locale = Locale::from_yaml_str(
            r#"
locale: es-ES
evaluation:
  message-syntax: mf2
messages:
  role.editor.label-long: |
    .match {$gender :unknown} {$count :plural}
    when feminine one {editora}
roles:
  editor:
    long:
      singular:
        feminine: editora heredada
      plural:
        feminine: editoras heredadas
"#,
        )
        .expect("locale should parse");

        assert_eq!(
            locale.resolved_role_term(
                &ContributorRole::Editor,
                false,
                &TermForm::Long,
                Some(GrammaticalGender::Feminine),
            ),
            Some("editora heredada".to_string())
        );
    }
}
