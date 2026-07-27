---
# csl26-q7zo
title: Author locale terms for editorial sub-roles
status: completed
type: task
priority: low
tags:
    - locale
    - conversion
    - contributors
created_at: 2026-07-27T18:07:48Z
updated_at: 2026-07-27T20:56:28Z
---

csl26-11h2 added five new ContributorRole variants (annotator, commentator, foreword-author, introduction-author, afterword-author) for BibLaTeX editorial sub-roles, but no locale currently has role terms for any of them -- labels render empty until terms are authored. Author en-US terms first (see locales/en-US.yaml's existing role.*.label pattern for translator/editor), then the other 8 embedded locales.

## Summary of Changes\n\nAdded first-class style roles and complete long/short/verb/verb-short locale terms for annotator, commentator, foreword author, introduction author, and afterword author across all 12 embedded locales. Used BibLaTeX native vocabulary where available and documented English fallbacks for ar-AR, zh-CN, ja-JP, and ko-KR; fr-CA inherits fr-FR. Added exhaustive locale-resolution and engine substitution tests.
