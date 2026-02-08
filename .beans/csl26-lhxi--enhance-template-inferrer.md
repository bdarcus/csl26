---
# csl26-lhxi
title: Enhance template inferrer
status: completed
type: feature
priority: normal
created_at: 2026-02-08T06:35:02Z
updated_at: 2026-02-08T06:35:02Z
---

Enhancements to scripts/lib/template-inferrer.js for higher fidelity templates:

1. [DONE] Fix confidence metric - per-type coverage. APA 95%, IEEE 96%, Elsevier 97%.

2. [DONE] Prefix/suffix inference - detects "pp." (IEEE pages), "https://doi.org/" (APA/Elsevier DOI). "In " detection wired but needs editor name position improvement.

3. [DONE] Items grouping - volume(issue) detected as grouped unit with delimiter: none. Works for APA, correctly skipped for IEEE.

4. [DONE] Formatting inference - detects italics (emph: true) and quotes (wrap: quotes) by parsing raw HTML output from citeproc-js. Majority vote across entries per component.

5. [DONE] Parent-monograph detection - splits containerTitle into parent-serial and parent-monograph based on reference types (chapter, entry-encyclopedia, entry-dictionary, paper-conference).

6. [DONE] Wrap inference - issue wrap: parentheses detected (APA). Year wrap: parentheses already handled. Section-level delimiter emitted when not default ". ".