/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#[cfg(test)]
mod tests {
    use crate::render::component::{ProcTemplateComponent, render_component_with_format};
    use crate::render::djot::Djot;
    use crate::render::html::Html;
    use csln_core::{tc_contributor, tc_title, tc_variable};

    #[test]
    fn test_html_title() {
        let component = ProcTemplateComponent {
            template_component: tc_title!(Primary, emph = true),
            value: "My Title".to_string(),
            ..Default::default()
        };

        let result = render_component_with_format::<Html>(&component);
        assert_eq!(result, r#"<span class="csln-title"><i>My Title</i></span>"#);
    }

    #[test]
    fn test_html_contributor() {
        let component = ProcTemplateComponent {
            template_component: tc_contributor!(Author, Long, small_caps = true),
            value: "Smith".to_string(),
            ..Default::default()
        };

        let result = render_component_with_format::<Html>(&component);
        assert_eq!(
            result,
            r#"<span class="csln-author"><span style="font-variant:small-caps">Smith</span></span>"#
        );
    }

    #[test]
    fn test_djot_title() {
        let component = ProcTemplateComponent {
            template_component: tc_title!(Primary, emph = true),
            value: "My Title".to_string(),
            ..Default::default()
        };

        let result = render_component_with_format::<Djot>(&component);
        assert_eq!(result, "[_My Title_]{.csln-title}");
    }

    #[test]
    fn test_djot_contributor() {
        let component = ProcTemplateComponent {
            template_component: tc_contributor!(Author, Long, small_caps = true),
            value: "Smith".to_string(),
            ..Default::default()
        };

        let result = render_component_with_format::<Djot>(&component);
        assert_eq!(result, "[[Smith]{.small-caps}]{.csln-author}");
    }

    #[test]
    fn test_html_link() {
        let component = ProcTemplateComponent {
            template_component: tc_variable!(Url),
            value: "https://example.com".to_string(),
            url: Some("https://example.com".to_string()),
            ..Default::default()
        };

        let result = render_component_with_format::<Html>(&component);
        assert_eq!(
            result,
            r#"<span class="csln-url"><a href="https://example.com">https://example.com</a></span>"#
        );
    }

    #[test]
    fn test_html_title_link_doi() {
        use csln_core::{
            options::{LinkAnchor, LinkTarget, LinksConfig},
            template::{TemplateTitle, TitleType},
        };
        let component = ProcTemplateComponent {
            template_component: csln_core::template::TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                links: Some(LinksConfig {
                    target: Some(LinkTarget::Doi),
                    anchor: Some(LinkAnchor::Title),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            value: "My Title".to_string(),
            url: Some("https://doi.org/10.1001/test".to_string()),
            ..Default::default()
        };

        let result = render_component_with_format::<Html>(&component);
        assert_eq!(
            result,
            r#"<span class="csln-title"><a href="https://doi.org/10.1001/test">My Title</a></span>"#
        );
    }
}
