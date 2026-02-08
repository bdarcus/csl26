---
# csl26-lhxi
title: Enhance template inferrer
status: todo
type: feature
priority: normal
created_at: 2026-02-08T06:35:02Z
updated_at: 2026-02-08T06:35:02Z
---

Enhancements to scripts/lib/template-inferrer.js for higher fidelity templates:

1. Fix confidence metric - measure per-type coverage (does each journal have all journal-expected components?) instead of global all-components check, which is always 0% since no single entry has all 11 components.

2. Prefix/suffix inference - detect "In " before editors, "pp." before pages, "https://doi.org/" before DOI by examining text between matched components.

3. Items grouping - detect volume(issue) as a grouped unit with delimiter: none, based on adjacency without separator.

4. Formatting inference - italics on parent-serial, quotes on component titles. Requires parsing HTML output from citeproc-js instead of plain text.

5. Parent-monograph detection - currently only infers parent-serial, misses book container titles (chapters "In Editor, Book Title").

6. Wrap inference - issue wrapped in parentheses, pages wrapped for chapters. Detect by examining surrounding punctuation.