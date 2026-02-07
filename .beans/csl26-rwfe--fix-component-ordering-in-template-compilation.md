---
# csl26-rwfe
title: Fix component ordering in template compilation
status: in-progress
type: bug
priority: high
created_at: 2026-02-07T18:20:01Z
updated_at: 2026-02-07T18:24:57Z
parent: csl26-ifiw
---

Templates have components in wrong sequence vs CSL 1.0 output (51 ordering issues across 10 styles). Example: pages appearing before containerTitle instead of after. Root cause is in csln_migrate/src/template_compiler/ where CSL 1.0 layouts are converted to CSLN templates. Need to preserve CSL 1.0 layout order during migration.