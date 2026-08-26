---
# csl26-154x
title: Add yaml-language-server schema hint to all style files
status: completed
type: task
priority: normal
tags:
    - tooling
    - chore
    - style
created_at: 2026-08-26T22:27:43Z
updated_at: 2026-08-26T22:28:18Z
---

50 of 56 style YAML files lack the # yaml-language-server: $schema=... hint line that apa-7th.yaml and the experimental styles already carry. Add it as the first line of each so editors get schema validation/completion.

## Summary of Changes

Added `# yaml-language-server: $schema=https://docs.citum.org/schemas/style.json` as the first line of all 50 style YAML files that lacked it (6 already had it: apa-7th and 5 experimental styles). Verified with `node scripts/validate-schemas.js --scope=styles,locales` — all pass.
