---
# csl26-kohl
title: Deferred config-presets analyzer improvements
status: todo
type: task
priority: deferred
created_at: 2026-08-02T18:16:36Z
updated_at: 2026-08-02T18:16:36Z
---

Follow-ups from csl26-4aml, not built there because unsupported by current corpus evidence: (1) savings-weighted candidate ranking (authored-YAML-line reduction, not just corpus_count); (2) array-order normalization in the shape hash (e.g. LocatorConfig.patterns order-sensitivity); (3) extend the analyzed concern set beyond contributors/dates/titles/locators to SubstitutePreset and SortPreset, which OptionsExtractor also populates via preset-name shorthand; (4) subsumption clustering between candidates (e.g. a shape that is a strict superset of another candidate's fields).
