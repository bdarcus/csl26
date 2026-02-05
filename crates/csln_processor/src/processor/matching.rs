use crate::reference::Reference;
use csln_core::options::{Config, Substitute, SubstituteKey};
use csln_core::Style;

pub struct Matcher<'a> {
    style: &'a Style,
    default_config: &'a Config,
}

impl<'a> Matcher<'a> {
    pub fn new(style: &'a Style, default_config: &'a Config) -> Self {
        Self {
            style,
            default_config,
        }
    }

    /// Check if primary contributors (authors/editors) match between two references.
    /// Uses the style's substitution logic to determine the primary contributor.
    pub fn contributors_match(&self, prev: &Reference, current: &Reference) -> bool {
        let substitute = self.get_substitute_config();
        let prev_contributors = self.get_primary_contributors(prev, &substitute);
        let curr_contributors = self.get_primary_contributors(current, &substitute);

        match (prev_contributors, curr_contributors) {
            (Some(p), Some(c)) => p == c,
            _ => false,
        }
    }

    /// Get the substitute configuration from the style or use defaults.
    fn get_substitute_config(&self) -> Substitute {
        self.style
            .options
            .as_ref()
            .and_then(|o| o.substitute.as_ref())
            .map(|s| s.resolve())
            .or_else(|| self.default_config.substitute.as_ref().map(|s| s.resolve()))
            .unwrap_or_default()
    }

    /// Get the primary contributors for a reference based on the style's substitution order.
    /// Follows the substitute template: Author is always first, then the configured fallbacks.
    fn get_primary_contributors(
        &self,
        reference: &Reference,
        substitute: &Substitute,
    ) -> Option<crate::reference::Contributor> {
        // Author is always the primary contributor
        if let Some(author) = reference.author() {
            return Some(author);
        }

        // Fall back through the substitute template order
        for key in &substitute.template {
            let contributor = match key {
                SubstituteKey::Editor => reference.editor(),
                SubstituteKey::Translator => reference.translator(),
                SubstituteKey::Title => None, // Title is not a contributor
            };
            if contributor.is_some() {
                return contributor;
            }
        }

        None
    }
}
