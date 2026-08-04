/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Reference markers: the processor-generated tokens that stand in for a full
//! reference at the point of citation (`[1]`, `[Kuh62]`).
//!
//! A marker is a value in the render model, never a template component. A style
//! declares `label-mode`; this module resolves what that means and renders the
//! token. Nothing downstream inspects the template AST to find a marker, because
//! no template can contain one.
//!
//! See [`docs/specs/REFERENCE_MARKERS.md`](../../../../../../docs/specs/REFERENCE_MARKERS.md).

use crate::reference::Reference;
use crate::values::ProcHints;
use citum_schema::options::{
    BibliographyLabelMode, BibliographyLabelWrap, CitationLabelMode, Config, LabelWrap,
    bibliography::BibliographyConfig,
};

/// Which processor-generated token a marker renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerKind {
    /// A processor-assigned citation number, such as the `1` in `[1]`.
    Numeric,
    /// A generated alphabetic trigraph, such as `Kuh62`.
    Alphabetic,
}

/// A marker's resolved value.
///
/// Numeric markers carry the integer rather than its rendered form, so collapse
/// reads a number instead of parsing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MarkerValue {
    /// A citation number plus an optional compound-entry sub-label (`1a`).
    Number {
        /// Processor-assigned citation number.
        number: usize,
        /// Compound-entry sub-label, such as the `a` in `1a`.
        sub_label: Option<String>,
    },
    /// A generated trigraph, disambiguation suffix already attached.
    Token(String),
    /// A collapsed run of citation numbers, such as `1–3`.
    Range(String),
}

impl MarkerValue {
    /// Render the value as text, before any wrapping, mapping ASCII digits to
    /// the locale's glyphs — a numeric marker is a number like any other.
    pub(crate) fn as_localized_text(
        &self,
        digit_system: &citum_schema::locale::DigitSystem,
    ) -> String {
        crate::values::number::localize_digits(self.as_text(), digit_system)
    }

    /// Render the value as text, before any wrapping.
    pub(crate) fn as_text(&self) -> String {
        match self {
            MarkerValue::Number { number, sub_label } => match sub_label {
                Some(sub_label) => format!("{number}{sub_label}"),
                None => number.to_string(),
            },
            MarkerValue::Token(text) | MarkerValue::Range(text) => text.clone(),
        }
    }

    /// The citation number and sub-label this value carries, if it is numeric.
    pub(crate) fn number_parts(&self) -> Option<(usize, Option<&str>)> {
        match self {
            MarkerValue::Number { number, sub_label } => Some((*number, sub_label.as_deref())),
            _ => None,
        }
    }

    /// The citation number a *range* collapse may merge on. A value carrying a
    /// compound sub-label is excluded: `1a` does not extend a `1–3` run.
    pub(crate) fn collapsible_number(&self) -> Option<usize> {
        match self.number_parts() {
            Some((number, None)) => Some(number),
            _ => None,
        }
    }

    /// Whether this value is numeric, and so still eligible for either collapse.
    pub(crate) fn is_numeric(&self) -> bool {
        self.number_parts().is_some()
    }
}

/// Where a citation marker sits relative to the rest of the item body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerPlacement {
    /// Ahead of the body: `[1, p. 5]`.
    Leading,
    /// Behind the body: `Smith [1]`.
    Trailing,
}

/// A citation marker resolved for one item, before its value is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CitationMarkerSpec {
    /// Which token to generate.
    pub kind: MarkerKind,
    /// Wrap enclosing the marker alone.
    pub label_wrap: Option<LabelWrap>,
    /// Wrap enclosing the marker together with the item body.
    pub item_wrap: Option<LabelWrap>,
    /// Where the marker sits relative to the body.
    pub placement: MarkerPlacement,
}

/// A bibliography marker resolved for one entry, before its value is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BibliographyMarkerSpec {
    /// Which token to generate.
    pub kind: MarkerKind,
    /// Wrap enclosing the marker.
    pub wrap: Option<BibliographyLabelWrap>,
    /// Text joining the marker to the entry body. Empty renders flush.
    pub separator: String,
}

/// Resolve the effective declarative citation label mode for one citation spec.
pub(crate) fn citation_label_mode(
    config: &Config,
    spec: &citum_schema::CitationSpec,
) -> Option<CitationLabelMode> {
    spec.options
        .as_ref()
        .and_then(|options| options.label_mode)
        .or_else(|| {
            matches!(
                config.effective_processing(),
                citum_schema::options::Processing::Numeric
            )
            .then_some(CitationLabelMode::Numeric)
        })
}

/// Resolve the marker one citation item renders, if any.
///
/// Integral mode places the marker behind the body (`Smith [1]`) and excludes
/// `item-wrap`, since there the body is the author and renders outside the
/// marker. See `docs/specs/REFERENCE_MARKERS.md`.
pub(crate) fn resolve_citation_marker(
    config: &Config,
    spec: &citum_schema::CitationSpec,
    mode: &citum_schema::citation::CitationMode,
) -> Option<CitationMarkerSpec> {
    let kind = match citation_label_mode(config, spec)? {
        CitationLabelMode::Numeric => MarkerKind::Numeric,
        CitationLabelMode::Alphabetic => MarkerKind::Alphabetic,
        CitationLabelMode::None => return None,
    };
    let integral = matches!(mode, citum_schema::citation::CitationMode::Integral);
    let options = spec.options.as_ref();
    Some(CitationMarkerSpec {
        kind,
        label_wrap: options.and_then(|options| options.label_wrap),
        item_wrap: if integral {
            None
        } else {
            options.and_then(|options| options.item_wrap)
        },
        placement: if integral {
            MarkerPlacement::Trailing
        } else {
            MarkerPlacement::Leading
        },
    })
}

/// Resolve the marker one bibliography entry renders, if any.
pub(crate) fn resolve_bibliography_marker(
    config: Option<&BibliographyConfig>,
) -> Option<BibliographyMarkerSpec> {
    let config = config?;
    let kind = match config.label_mode? {
        BibliographyLabelMode::Numeric => MarkerKind::Numeric,
        BibliographyLabelMode::Alphabetic => MarkerKind::Alphabetic,
        BibliographyLabelMode::None | BibliographyLabelMode::AuthorDate => return None,
    };
    Some(BibliographyMarkerSpec {
        kind,
        wrap: config.label_wrap,
        separator: config.label_separator.clone().unwrap_or_default(),
    })
}

/// Generate a marker's value for one reference.
///
/// Numeric markers read the processor-assigned citation number; alphabetic
/// markers generate a trigraph and attach the disambiguation suffix that
/// `disambiguation.rs` decided.
pub(crate) fn marker_value(
    kind: MarkerKind,
    config: &Config,
    reference: &Reference,
    citation_number: Option<usize>,
    sub_label: Option<String>,
    hints: Option<&ProcHints>,
) -> Option<MarkerValue> {
    match kind {
        MarkerKind::Numeric => {
            citation_number.map(|number| MarkerValue::Number { number, sub_label })
        }
        MarkerKind::Alphabetic => {
            let citum_schema::options::Processing::Label(label_config) =
                config.processing.as_ref()?
            else {
                return None;
            };
            let base = crate::processor::labels::generate_base_label(
                reference,
                &label_config.effective_params(),
            );
            if base.is_empty() {
                return None;
            }
            let suffix = hints
                .filter(|hints| hints.disamb_condition && hints.group_index > 0)
                .and_then(|hints| crate::values::int_to_letter(hints.group_index as u32))
                .unwrap_or_default();
            Some(MarkerValue::Token(format!("{base}{suffix}")))
        }
    }
}
