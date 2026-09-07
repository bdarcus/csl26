---
# csl26-8z39
title: Extend ArticleJournalNoPageFallback to test volume absence (NLM/CSE)
status: todo
type: task
priority: normal
tags:
    - style
    - engine
    - schema
    - fidelity
created_at: 2026-09-06T23:14:03Z
updated_at: 2026-09-06T23:14:08Z
parent: csl26-ccdt
---

docs/specs/ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md's ArticleJournalNoPageFallback::Doi currently gates on page absence alone (article_journal_bibliography_mode in crates/citum-engine/src/processor/rendering/grouped/template_policy.rs:76-99, reference_has_pages). T&F-NLM's shipped access macro (styles-legacy/taylor-and-francis-national-library-of-medicine.csl:72-88) gates the same DOI-vs-detail-block choice on page AND volume both being absent (if match="none" variable="page volume"), scoped to article-journal only.

Found while resolving a Codex adversarial-review finding on docs/specs/ALTERNATIVES.md (csl26-8b4a): an earlier draft incorrectly proposed generalizing this NLM rule into alternatives:, using the full detail block (including date: issued, which is nearly always present) as the first candidate -- that would never fall through to DOI, so alternatives: cannot express this rule at all. The correct fix is extending the existing, narrower, already-shipped ArticleJournalNoPageFallback option instead, which already implements exactly this type-gated field-presence shape for RSC's page-only case.

## Todo
- [ ] Extend ArticleJournalNoPageFallback (or add a sibling variant) to test volume absence in addition to/instead of page, matching NLM's match="none" variable="page volume" rule
- [ ] Wire taylor-and-francis-national-library-of-medicine-core.yaml and taylor-and-francis-council-of-science-editors-author-date-core.yaml to use it
- [ ] report-core.js --diff verifying the ~11 T&F-NLM DOI rows flip with 0 regressions
- [ ] Update docs/specs/ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md acceptance criteria
