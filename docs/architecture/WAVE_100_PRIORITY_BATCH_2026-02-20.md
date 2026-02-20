# Wave 100 Priority Batch (2026-02-20)

This report captures the `styleauthor migrate+enhance` batch that adds the next
58 priority parent styles, expanding the core style catalog from 42 to 100 YAML
styles.

## Batch Metrics

Edited styles (with preset extraction + citation sync from `csln-migrate`):
- Citations: `607/696` (87.2%)
- Bibliography: `1722/1824` (94.4%)
- Combined fidelity: `2329/2520` (92.4%)

Auto-migrate rerun baseline:
- Citations: `607/696` (87.2%)
- Bibliography: `1720/1824` (94.3%)
- Combined fidelity: `2327/2520` (92.3%)

Delta (edited vs rerun):
- Citations: `+0`
- Bibliography: `+2`
- Improved styles: `2`
- Regressed styles: `1`

Per-style deltas:
- Improved: `chicago-author-date` (+2 bibliography),
  `american-sociological-association` (+1 bibliography)
- Regressed: `mary-ann-liebert-harvard` (-1 bibliography)

## Preset Extraction Applied

Across the 58-style wave:
- `options.dates` presetized in 58 styles (`long`/`short`/`numeric`)
- `options.titles` presetized in 39 styles (`humanities`/`journal-emphasis`)
- `options.substitute` presetized in 36 styles (`editor-*` variants)
- contributor presetization in 11 styles (`numeric-compact`/`numeric-medium`)

## Added Styles (58)
1. pensoft-journals
1. springer-vancouver-author-date
1. nlm-citation-sequence-brackets
1. nature-publishing-group-vancouver
1. future-medicine
1. the-institution-of-engineering-and-technology
1. american-society-of-mechanical-engineers
1. african-online-scientific-information-systems-harvard
1. thomson-reuters-legal-tax-and-accounting-australia
1. aims-press
1. bristol-university-press
1. american-geophysical-union
1. mary-ann-liebert-harvard
1. american-society-for-microbiology
1. annual-reviews-author-date
1. future-science-group
1. american-institute-of-physics
1. elsevier-without-titles
1. trends-journals
1. current-opinion
1. frontiers-medical-journals
1. annual-reviews
1. american-physics-society
1. nlm-citation-sequence-superscript-brackets-year-only
1. institute-for-operations-research-and-the-management-sciences
1. american-meteorological-society
1. american-physiological-society
1. elsevier-vancouver-author-date
1. cse-name-year
1. african-online-scientific-information-systems-vancouver
1. american-sociological-association
1. american-association-for-cancer-research
1. spie-journals
1. sage-vancouver
1. international-union-of-crystallography
1. canadian-journal-of-fisheries-and-aquatic-sciences
1. the-geological-society-of-london
1. annual-reviews-alphabetical
1. integrated-science-publishing-journals
1. entomological-society-of-america
1. oxford-journals-scimed-author-date
1. international-journal-of-wildland-fire
1. museum-national-dhistoire-naturelle
1. inter-research-science-center
1. mhra-author-date-publisher-place
1. mhra-shortened-notes-publisher-place
1. proceedings-of-the-royal-society-b
1. spandidos-publications
1. chicago-author-date
1. mhra-notes
1. institute-of-mathematics-and-its-applications
1. hainan-medical-university-journal-publisher
1. plos
1. the-optical-society
1. medicine-publishing
1. karger-journals-author-date
1. medicina-clinica
1. american-institute-of-aeronautics-and-astronautics
