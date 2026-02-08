---
# csl26-l05b
title: Expand test fixtures to 25-30 references
status: todo
type: task
priority: high
created_at: 2026-02-08T00:38:50Z
updated_at: 2026-02-08T00:38:50Z
blocking:
    - csl26-m3lb
---

Current fixture (tests/fixtures/references-expanded.json) has 16 items covering only 7 types: article-journal, book, chapter, report, thesis, paper-conference, webpage.

Add 10-15 items covering missing types critical for style differentiation:
- article-newspaper, article-magazine
- entry-encyclopedia
- dataset
- legal_case (at minimum)
- patent
- motion_picture or broadcast
- interview or personal_communication

Also add edge cases for existing types:
- Book with editors only (no author)
- Multiple editors
- Translated work
- Non-English title
- Corporate/organizational author

~200 lines of JSON. This is a prerequisite for the output-driven template inferrer.