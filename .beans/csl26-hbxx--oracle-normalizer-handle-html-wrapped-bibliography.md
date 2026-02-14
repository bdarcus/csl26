---
# csl26-hbxx
title: 'Oracle normalizer: handle HTML-wrapped bibliography numbering'
status: todo
type: bug
priority: normal
created_at: 2026-02-14T15:05:47Z
updated_at: 2026-02-14T15:05:47Z
---

The normalizeText() in oracle-utils.js now strips leading numbering (e.g. '1. ') but citeproc-js wraps the number in <div class='csl-left-margin'>1. </div>. After HTML stripping, the number prefix may be preceded by whitespace and not at the start of the string, so the ^\d+\.\s+ regex doesn't match. Need to handle csl-left-margin/csl-right-inline div structure properly, or strip after whitespace normalization.
