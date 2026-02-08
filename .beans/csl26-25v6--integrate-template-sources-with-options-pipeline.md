---
# csl26-25v6
title: Integrate template sources with options pipeline
status: completed
type: task
priority: normal
created_at: 2026-02-08T00:39:25Z
updated_at: 2026-02-08T12:47:52Z
blocking:
    - csl26-m3lb
    - csl26-z8rc
    - csl26-o3ek
---

Wire up hand-authored and output-inferred templates with the existing XML options pipeline so the migration produces complete CSLN styles.

Currently the migration pipeline (csln_migrate) produces:
- Options: from XML extractor (working, 87-100% citation match)
- Templates: from XML template compiler (broken, 0% bibliography match)

New pipeline:
1. Check if a hand-authored template exists for this style -> use it
2. Otherwise, run output-driven inference -> use inferred template
3. Fallback to XML template compiler for remaining styles
4. Always use XML options pipeline for global options

~200 lines of integration code in crates/csln_migrate/src/lib.rs.

Must not regress citation match rates (currently 87-100%).