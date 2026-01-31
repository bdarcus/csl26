# CSL Style Priority for Rendering Development

This document identifies which citation styles should be prioritized for rendering development based on real-world usage. The data is derived from analyzing the 7,987 dependent CSL styles that reference parent styles.

## How CSL Style Aliasing Works

CSL 1.0 has two types of styles:

1. **Independent styles** (~2,844 styles): Full style definitions with complete formatting rules
2. **Dependent styles** (~7,987 styles): Lightweight aliases that reference a parent style

For example, hundreds of journals use APA formatting - instead of duplicating the APA style definition, they create dependent styles that point to the APA parent style. This means supporting the top parent styles provides coverage for thousands of journals.

## Priority Ranking by Impact

### Tier 1: High Priority (80%+ dependent coverage)

| Parent Style | Dependents | Format | Coverage |
|-------------|------------|--------|----------|
| apa | 783 | author-date | 9.8% |
| elsevier-with-titles | 672 | numeric | 8.4% |
| elsevier-harvard | 665 | author-date | 8.3% |
| elsevier-vancouver | 502 | numeric | 6.3% |
| springer-vancouver-brackets | 472 | numeric | 5.9% |
| springer-basic-author-date | 460 | author-date | 5.8% |
| springer-basic-brackets | 352 | numeric | 4.4% |
| springer-socpsych-author-date | 317 | author-date | 4.0% |
| american-medical-association | 293 | numeric | 3.7% |
| taylor-and-francis-chicago-author-date | 234 | author-date | 2.9% |

**Combined Tier 1 coverage: ~60% of all dependent styles**

### Tier 2: Medium Priority

| Parent Style | Dependents | Format |
|-------------|------------|--------|
| springer-mathphys-brackets | 201 | numeric |
| multidisciplinary-digital-publishing-institute | 180 | numeric |
| ieee | 176 | numeric |
| nlm-citation-sequence-superscript | 121 | numeric |
| nlm-citation-sequence | 116 | numeric |
| karger-journals | 85 | numeric |
| institute-of-physics-numeric | 82 | numeric |
| thieme-german | 74 | numeric |
| mary-ann-liebert-vancouver | 72 | numeric |
| biomed-central | 66 | numeric |

### Tier 3: Note Styles (19% of corpus)

| Parent Style | Dependents | Format |
|-------------|------------|--------|
| chicago-shortened-notes-bibliography | 38 | note |
| thomson-reuters-legal-tax-and-accounting | 26 | note |
| oscola | 12 | note |
| modern-humanities-research-association-3rd-edition-note | 9 | note |
| chicago-fullnote-bibliography | 8 | note |

## Citation Format Distribution

All dependent styles break down by citation format:

| Format | Count | Percentage |
|--------|-------|------------|
| numeric | 4,581 | 57.4% |
| author-date | 3,231 | 40.4% |
| note | 170 | 2.1% |
| author | 5 | 0.1% |

## AI Agent Instructions

When working on rendering improvements, prioritize work that benefits the most users:

### Priority Order for Development

1. **Author-date styles first** - Start with APA (783 dependents), then Elsevier Harvard (665), Springer Basic Author-Date (460)

2. **Test against multiple parent styles** - Don't optimize for a single style. Changes should work across:
   - APA (American Psychological Association)
   - Chicago Author-Date
   - Harvard variants (Elsevier, Springer)
   - Taylor & Francis Chicago

3. **Numeric styles second** - After author-date works reliably:
   - Elsevier Vancouver/With-Titles
   - Springer Vancouver
   - IEEE
   - AMA (American Medical Association)

4. **Note styles last** - Lower priority due to smaller corpus:
   - Chicago Notes
   - OSCOLA (legal)
   - MHRA

### Measuring Impact

When reporting progress, calculate impact as:
```
Impact = sum(dependent_count for passing parent styles) / 7987 * 100
```

For example, if APA, Elsevier Harvard, and Springer Basic Author-Date all pass:
```
Impact = (783 + 665 + 460) / 7987 * 100 = 23.9% of dependent corpus
```

### Running the Analyzer

To regenerate this data:

```bash
# Full ranking
cargo run --bin csln_analyze -- styles/ --rank-parents

# Filter by citation format
cargo run --bin csln_analyze -- styles/ --rank-parents --format author-date

# JSON output for programmatic use
cargo run --bin csln_analyze -- styles/ --rank-parents --json
```

## Fields/Disciplines by Top Styles

| Parent Style | Primary Fields |
|-------------|----------------|
| apa | psychology, social science, linguistics |
| elsevier-harvard | multidisciplinary |
| elsevier-vancouver | medicine |
| springer-* | science, medicine, engineering |
| ieee | engineering, physics, communications |
| american-medical-association | medicine, biology |
| chicago-* | humanities |

## Recommendations

1. **Focus on author-date first** - 40% of corpus, includes APA which alone covers 10%
2. **Build shared infrastructure** - Many author-date styles share patterns (author + year + title + source)
3. **Test oracle against top 10 parent styles** - Provides coverage for 60% of dependent styles
4. **Track regression** - A bug in APA affects 783+ journals, so oracle verification is critical
