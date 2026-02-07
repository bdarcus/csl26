---
# csl26-o248
title: Remove duplicate and inappropriate components
status: in-progress
type: bug
priority: high
created_at: 2026-02-07T18:20:06Z
updated_at: 2026-02-07T18:37:28Z
parent: csl26-ifiw
---

Migration creates duplicate and inappropriate components: year (31 duplicates), issue (17 extra), editors (12 inappropriate for articles), volume (9 extra), pages (appearing twice). Need to fix component selection logic and suppress rules in template_compiler to match CSL 1.0 behavior.