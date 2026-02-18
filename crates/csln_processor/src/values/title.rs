/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::reference::Parent;
use csln_core::template::{TemplateTitle, TitleType};

impl ComponentValues for TemplateTitle {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        // Resolve effective rendering options (base merged with type-specific override)
        let mut effective_rendering = self.rendering.clone();
        if let Some(overrides) = &self.overrides {
            use csln_core::template::ComponentOverride;
            let ref_type = reference.ref_type();
            let mut match_found = false;
            for (selector, ov) in overrides {
                if selector.matches(&ref_type) {
                    if let ComponentOverride::Rendering(r) = ov {
                        effective_rendering.merge(r);
                        match_found = true;
                    }
                }
            }
            if !match_found {
                for (selector, ov) in overrides {
                    if selector.matches("default") {
                        if let ComponentOverride::Rendering(r) = ov {
                            effective_rendering.merge(r);
                        }
                    }
                }
            }
        }

        // Get the raw title based on type and template requirement
        let raw_title = match self.title {
            TitleType::Primary => reference.title(),
            TitleType::ParentSerial => match reference {
                Reference::SerialComponent(r) => match &r.parent {
                    Parent::Embedded(p) => Some(&p.title),
                    _ => None,
                },
                _ => None,
            }
            .cloned(),
            TitleType::ParentMonograph => match reference {
                Reference::CollectionComponent(r) => match &r.parent {
                    Parent::Embedded(p) => p.title.as_ref(),
                    _ => None,
                },
                _ => None,
            }
            .cloned(),
            _ => None,
        };

        // Resolve multilingual title if configured
        let value = raw_title.map(|title| {
            use csln_core::reference::types::Title;

            match title {
                Title::Single(s) => s.clone(),
                Title::Multilingual(m) => {
                    let mode = options
                        .config
                        .multilingual
                        .as_ref()
                        .and_then(|ml| ml.title_mode.as_ref());
                    let preferred_script = options
                        .config
                        .multilingual
                        .as_ref()
                        .and_then(|ml| ml.preferred_script.as_ref());
                    let locale_str = options.locale.locale.as_str();

                    let complex =
                        csln_core::reference::types::MultilingualString::Complex(m.clone());
                    crate::values::resolve_multilingual_string(
                        &complex,
                        mode,
                        preferred_script,
                        locale_str,
                    )
                }
                _ => title.to_string(),
            }
        });

        value.filter(|s: &String| !s.is_empty()).map(|value| {
            use csln_core::options::LinkAnchor;
            let url = crate::values::resolve_effective_url(
                self.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                LinkAnchor::Title,
            );
            ProcValues {
                value,
                prefix: None,
                suffix: None,
                url,
                substituted_key: None,
                pre_formatted: false,
            }
        })
    }
}
