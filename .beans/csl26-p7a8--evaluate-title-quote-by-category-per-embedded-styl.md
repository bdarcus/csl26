---
# csl26-p7a8
title: 'Evaluate title-quote: by-category per embedded style'
status: todo
type: task
priority: normal
tags:
    - title
    - substitute
    - rendering
    - styles
created_at: 2026-08-14T20:04:49Z
updated_at: 2026-08-14T20:04:59Z
---

csl26-0dca (fix/substituted-title-emphasis) completed the engine-side
implementation of contributors.substitute.title-quote: by-category — a
substituted title in citation context now both un-quotes AND picks up its
titles: category emph/strong/small-caps when that mode is set (div-011).

No embedded style opts into by-category yet. This bean is to evaluate, per
style, whether flipping title-quote: by-category is a net-correct change
against citeproc-js/publisher conventions, and if so apply it. Candidates
identified during csl26-0dca's investigation:

- apa-7th: monograph/periodical titles.*.emph: true. Author-less book/
  periodical references cited by title would go from `(\"Some Book
  Title,\" 2020)` to `(_Some Book Title_, 2020)` in citations, matching
  APA's normal title-italicization rule.
- chicago-author-date-18th: monograph/container-monograph/periodical/serial
  all set emph: true. Its citation substitute chain already includes
  parent-serial before title (editor -> translator -> parent-serial ->
  title), so this would mainly affect the `title` fallback tier specifically
  (works with no editor/translator/container title at all).
- Any other embedded style whose titles: categories set emph/strong/
  small_caps and whose substitute chain can reach the title key.

Each style flip is its own style-behavior change with its own parity
surface -- verify with report-core.js before/after per style, not a
blanket change. See docs/adjudication/DIVERGENCE_REGISTER.md div-011.
