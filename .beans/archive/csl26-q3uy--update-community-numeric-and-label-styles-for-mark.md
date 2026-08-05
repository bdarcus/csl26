---
# csl26-q3uy
title: Update community numeric and label styles for marker schema
status: completed
type: task
priority: high
created_at: 2026-08-04T22:42:00Z
updated_at: 2026-08-04T22:48:34Z
---

Remove processor-owned citation-number and citation-label template components from citum-styles and replace them with declarative label-mode options compatible with the main schema after PR #1138. Validate the corpus and open a PR in citum-styles.

## Summary of Changes

Migrated 40 citum-styles styles (39 numeric and 1 alphabetic) from processor-owned citation marker template components to declarative label-mode options, validated every style against the current schema, and opened citum-styles PR #2.
