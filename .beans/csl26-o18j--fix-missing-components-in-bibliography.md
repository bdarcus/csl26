---
# csl26-o18j
title: Fix missing components in bibliography
status: todo
type: bug
priority: high
created_at: 2026-02-07T18:20:10Z
updated_at: 2026-02-07T19:02:15Z
parent: csl26-ifiw
---

Templates missing critical components: containerTitle (6 occurrences), doi (5 occurrences). These components exist in CSL 1.0 output but are not being included in CSLN templates during migration. Investigate template_compiler component selection logic.