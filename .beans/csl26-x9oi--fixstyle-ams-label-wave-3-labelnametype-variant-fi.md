---
# csl26-x9oi
title: 'fix(style): AMS-label Wave 3 — label/name/type-variant fixes'
status: in-progress
type: task
priority: normal
created_at: 2026-06-22T21:52:33Z
updated_at: 2026-08-03T22:05:31Z
---

Fix american-mathematical-society-label from 72.1% to ≥85%.

Root causes:
1. update_label_mode(Numeric) prepends citation-number before citation-label templates → '19, [Kuhn62]'
2. 15 entry types inherit elsevier-with-titles-core citation-number templates + URLs  
3. name-form inherits 'initials' from parent; AMS CSL uses initialize=false (full names)

Fixes needed:
- [x] Engine: scoped.rs update_label_mode(Numeric) — count CitationLabel as 'has_label' (done in csl26-slfx; label-mode now names the label variable and strips the other one)
- [ ] YAML: add name-form: full to options.contributors
- [ ] YAML: add type-variants for article-magazine, interview, article-newspaper, broadcast, dataset, webpage, entry-encyclopedia, map, software, bill, hearing, legislation, regulation, standard, entry-dictionary
- [ ] Run oracle, verify ≥85% fidelity
- [ ] pre-commit gate
