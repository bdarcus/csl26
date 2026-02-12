use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::locale::TermForm;
use csln_core::template::{NumberVariable, TemplateNumber};

impl ComponentValues for TemplateNumber {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        use csln_core::template::LabelForm;

        let value = match self.number {
            NumberVariable::Volume => reference.volume().map(|v| v.to_string()),
            NumberVariable::Issue => reference.issue().map(|v| v.to_string()),
            NumberVariable::Pages => reference.pages().map(|p| {
                format_page_range(&p.to_string(), options.config.page_range_format.as_ref())
            }),
            NumberVariable::Edition => reference.edition(),
            NumberVariable::CitationNumber => hints.citation_number.map(|n| n.to_string()),
            _ => None,
        };

        value.filter(|s| !s.is_empty()).map(|value| {
            // Handle label if label_form is specified
            let prefix = if let Some(label_form) = &self.label_form {
                if let Some(locator_type) = number_var_to_locator_type(&self.number) {
                    // Check pluralization
                    let plural = check_plural(&value, &locator_type);

                    let term_form = match label_form {
                        LabelForm::Long => TermForm::Long,
                        LabelForm::Short => TermForm::Short,
                        LabelForm::Symbol => TermForm::Symbol,
                    };

                    options
                        .locale
                        .locator_term(&locator_type, plural, term_form)
                        .map(|t| format!("{} ", t))
                } else {
                    None
                }
            } else {
                None
            };

            ProcValues {
                value,
                prefix,
                suffix: None,
                url: crate::values::resolve_effective_url(
                    self.links.as_ref(),
                    options.config.links.as_ref(),
                    reference,
                    csln_core::options::LinkAnchor::Component,
                ),
                substituted_key: None,
            }
        })
    }
}

pub fn number_var_to_locator_type(
    var: &NumberVariable,
) -> Option<csln_core::citation::LocatorType> {
    use csln_core::citation::LocatorType;
    match var {
        NumberVariable::Volume => Some(LocatorType::Volume),
        NumberVariable::Pages => Some(LocatorType::Page),
        NumberVariable::ChapterNumber => Some(LocatorType::Chapter),
        NumberVariable::NumberOfPages => Some(LocatorType::Page),
        NumberVariable::NumberOfVolumes => Some(LocatorType::Volume),
        NumberVariable::Issue => Some(LocatorType::Issue),
        _ => None,
    }
}

pub fn check_plural(value: &str, _locator_type: &csln_core::citation::LocatorType) -> bool {
    // Simple heuristic: if contains ranges or separators, it's plural.
    // "1-10", "1, 3", "1 & 3"
    value.contains('–') || value.contains('-') || value.contains(',') || value.contains('&')
}

/// Format a page range according to the specified format.
///
/// Formats: expanded (default), minimal, minimal-two, chicago, chicago-16
pub fn format_page_range(
    pages: &str,
    format: Option<&csln_core::options::PageRangeFormat>,
) -> String {
    use csln_core::options::PageRangeFormat;

    // First, replace hyphen with en-dash
    let pages = pages.replace("-", "–");

    // If no range or no format specified, return as-is
    let format = match format {
        Some(f) => f,
        None => return pages, // Default: just convert to en-dash
    };

    // Check if this is a range (contains en-dash)
    let parts: Vec<&str> = pages.split('–').collect();
    if parts.len() != 2 {
        return pages; // Not a simple range
    }

    let start = parts[0].trim();
    let end = parts[1].trim();

    // Parse as numbers
    let start_num: Option<u32> = start.parse().ok();
    let end_num: Option<u32> = end.parse().ok();

    match (start_num, end_num) {
        (Some(s), Some(e)) if e > s => {
            let formatted_end = match format {
                PageRangeFormat::Expanded => end.to_string(),
                PageRangeFormat::Minimal => format_minimal(start, end, 1),
                PageRangeFormat::MinimalTwo => format_minimal(start, end, 2),
                PageRangeFormat::Chicago | PageRangeFormat::Chicago16 => format_chicago(s, e),
                _ => end.to_string(), // Future variants: default to expanded
            };
            format!("{}–{}", start, formatted_end)
        }
        _ => pages, // Can't parse or invalid range
    }
}

/// Minimal format: keep only differing digits, with minimum min_digits
pub fn format_minimal(start: &str, end: &str, min_digits: usize) -> String {
    let start_chars: Vec<char> = start.chars().collect();
    let end_chars: Vec<char> = end.chars().collect();

    if start_chars.len() != end_chars.len() {
        return end.to_string();
    }

    // Find first differing position
    let mut first_diff = 0;
    for (i, (s, e)) in start_chars.iter().zip(end_chars.iter()).enumerate() {
        if s != e {
            first_diff = i;
            break;
        }
    }

    // Keep at least min_digits from the end
    let keep_from = first_diff.min(end_chars.len().saturating_sub(min_digits));
    end_chars[keep_from..].iter().collect()
}

/// Chicago Manual of Style page range format
pub fn format_chicago(start: u32, end: u32) -> String {
    // Chicago rules (simplified from CMOS 17th):
    // - Under 100: use all digits (3–10, 71–72, 96–117)
    // - 100+, same hundreds: use changed part only for 2+ digits (107–8, 321–28, 1536–38)
    // - Different hundreds: use all digits (107–108, 321–328 if change of hundreds)

    if start < 100 || end < 100 {
        return end.to_string();
    }

    let start_str = start.to_string();
    let end_str = end.to_string();

    if start_str.len() != end_str.len() {
        return end_str;
    }

    // Check if same hundreds
    let start_prefix = start / 100;
    let end_prefix = end / 100;

    if start_prefix != end_prefix {
        return end_str; // Different hundreds, use full number
    }

    // Same hundreds: use minimal-two style
    format_minimal(&start_str, &end_str, 2)
}
