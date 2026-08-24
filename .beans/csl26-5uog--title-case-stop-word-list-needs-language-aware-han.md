---
# csl26-5uog
title: Title-case stop-word list needs language-aware handling (de/von/van particles)
status: todo
type: bug
priority: low
tags:
    - style
    - chicago
    - title
    - multilingual
created_at: 2026-08-24T11:50:28Z
updated_at: 2026-08-24T11:50:28Z
parent: csl26-h7oc
---

Real title-case defect confined to non-English titles: 'de' should stay lowercase in a Spanish title ('Las Bases de Un Conflicto' -> 'Las Bases de Un Conflicto', not 'Las Bases De Un Conflicto'), matching citeproc-js's CSL.SKIP_WORDS which includes de/von/van/d'. Not added to TITLE_CASE_STOP_WORDS in the wave-2-adjacent stop-word fix (csl26-omqk) because these particles can legitimately be capitalized in genuine English contexts (names, e.g. 'De La Soul'), so a blind English-list addition risks a new divergence. Needs either language detection on the title field before applying the English stop-word list, or a separate per-language stop-word set. Scope/approach undetermined -- triage first.
