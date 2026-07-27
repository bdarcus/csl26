/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Embedded locale YAML files for common BCP 47 locales.
//!
//! These are baked into the binary at compile time via `include_bytes!`,
//! providing locale data when the CLI is invoked with `--builtin` and there
//! is no `locales/` directory on disk.

use crate::locale::Locale;

/// Raw YAML bytes for an embedded locale by BCP 47 ID.
///
/// Returns `None` for locales not bundled with the binary.
pub fn get_locale_bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        "en-US" => Some(include_bytes!("../../embedded/locales/en-US.yaml")),
        "ar-AR" => Some(include_bytes!("../../embedded/locales/ar-AR.yaml")),
        "de-DE" => Some(include_bytes!("../../embedded/locales/de-DE.yaml")),
        "es-ES" => Some(include_bytes!("../../embedded/locales/es-ES.yaml")),
        "eu-ES" => Some(include_bytes!("../../embedded/locales/eu-ES.yaml")),
        "fr-FR" => Some(include_bytes!("../../embedded/locales/fr-FR.yaml")),
        "fr-CA" => Some(include_bytes!("../../embedded/locales/fr-CA.yaml")),
        "tr-TR" => Some(include_bytes!("../../embedded/locales/tr-TR.yaml")),
        "zh-CN" => Some(include_bytes!("../../embedded/locales/zh-CN.yaml")),
        "ja-JP" => Some(include_bytes!("../../embedded/locales/ja-JP.yaml")),
        "ko-KR" => Some(include_bytes!("../../embedded/locales/ko-KR.yaml")),
        "ru-RU" => Some(include_bytes!("../../embedded/locales/ru-RU.yaml")),
        _ => None,
    }
}

/// Load a fully constructed embedded locale by BCP 47 ID.
///
/// This accessor applies regional inheritance for bundled locale overlays,
/// such as Québec French, before returning the locale to callers.
#[must_use]
pub fn get_locale(id: &str) -> Option<Locale> {
    if id == "fr-CA" {
        return Some(Locale::fr_ca());
    }

    let bytes = get_locale_bytes(id)?;
    let yaml = std::str::from_utf8(bytes).ok()?;
    Locale::from_yaml_str(yaml).ok()
}

/// All available embedded locale IDs.
pub const EMBEDDED_LOCALE_IDS: &[&str] = &[
    "en-US", "ar-AR", "de-DE", "es-ES", "eu-ES", "fr-FR", "fr-CA", "tr-TR", "zh-CN", "ja-JP",
    "ko-KR", "ru-RU",
];

/// Raw YAML bytes for an embedded locale override by ID.
///
/// Returns `None` for overrides not bundled with the binary.
pub fn get_locale_override_bytes(id: &str) -> Option<&'static [u8]> {
    match id {
        "en-US-chicago" => Some(include_bytes!(
            "../../embedded/locales/overrides/en-US-chicago.yaml"
        )),
        "de-DE-chicago" => Some(include_bytes!(
            "../../embedded/locales/overrides/de-DE-chicago.yaml"
        )),
        _ => None,
    }
}

/// All available embedded locale override IDs.
pub const EMBEDDED_LOCALE_OVERRIDE_IDS: &[&str] = &["en-US-chicago", "de-DE-chicago"];

#[cfg(test)]
mod tests {
    use super::{EMBEDDED_LOCALE_IDS, get_locale};
    use crate::locale::types::TermForm;
    use crate::template::ContributorRole;

    #[test]
    #[allow(
        clippy::expect_used,
        reason = "The test must fail when the compile-time embedded locale is absent."
    )]
    fn fr_ca_embedded_locale_inherits_french_term_surfaces() {
        let locale = get_locale("fr-CA").expect("fr-CA should be embedded");

        assert_eq!(locale.locale, "fr-CA");
        assert_eq!(
            locale.resolved_role_term(&ContributorRole::Editor, false, &TermForm::Short, None),
            Some("éd.".to_string())
        );
        assert_eq!(
            locale
                .punctuation_realization
                .as_ref()
                .and_then(|realization| realization.semicolon.as_deref()),
            Some("; ")
        );
    }

    #[test]
    #[allow(
        clippy::expect_used,
        reason = "The test must fail when an embedded locale or required role term is absent."
    )]
    fn editorial_sub_roles_resolve_every_required_form_in_all_locales() {
        let roles = [
            ContributorRole::Annotator,
            ContributorRole::Commentator,
            ContributorRole::ForewordAuthor,
            ContributorRole::IntroductionAuthor,
            ContributorRole::AfterwordAuthor,
        ];
        let forms = [
            TermForm::Long,
            TermForm::Short,
            TermForm::Verb,
            TermForm::VerbShort,
        ];

        for &id in EMBEDDED_LOCALE_IDS {
            let locale = get_locale(id).expect("locale should be embedded");
            for role in &roles {
                for form in &forms {
                    for plural in [false, true] {
                        let resolved = locale
                            .resolved_role_term(role, plural, form, None)
                            .expect("required editorial role term should resolve");
                        assert!(
                            !resolved.trim().is_empty(),
                            "{id} resolved an empty {} ({form:?}, plural={plural})",
                            role.as_str()
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[allow(
        clippy::expect_used,
        reason = "The test must fail when an embedded fallback locale is absent."
    )]
    fn locales_without_upstream_terms_use_documented_english_fallbacks() {
        let english = get_locale("en-US").expect("en-US should be embedded");
        let roles = [
            ContributorRole::Annotator,
            ContributorRole::Commentator,
            ContributorRole::ForewordAuthor,
            ContributorRole::IntroductionAuthor,
            ContributorRole::AfterwordAuthor,
        ];
        let forms = [
            TermForm::Long,
            TermForm::Short,
            TermForm::Verb,
            TermForm::VerbShort,
        ];

        for id in ["ar-AR", "zh-CN", "ja-JP", "ko-KR"] {
            let locale = get_locale(id).expect("fallback locale should be embedded");
            for role in &roles {
                for form in &forms {
                    assert_eq!(
                        locale.resolved_role_term(role, false, form, None),
                        english.resolved_role_term(role, false, form, None),
                        "{id} should use the English fallback for {} ({form:?})",
                        role.as_str()
                    );
                }
            }
        }
    }
}
