---
# csl26-ctw8
title: 'csln-migrate: extract strip-periods from CSL source'
status: todo
type: task
priority: normal
created_at: 2026-02-14T15:05:35Z
updated_at: 2026-02-14T15:05:35Z
---

The csln-migrate options extractor does not detect strip-periods from CSL sources. The upsampler.rs map_formatting() hardcodes strip_periods to None (line 648). Many CSL styles use strip-periods='true' on labels (especially editor labels). This should be extracted and set at the appropriate config tier (global options or bibliography.options). Related: strip-periods is available at Tier 1 (global options) and Tier 2 (citation/bibliography options) in CSLN, not just per-component.
