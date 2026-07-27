---
# csl26-veeg
title: Data model and ingest mapping reference docs
status: todo
type: task
priority: normal
tags:
    - docs
    - schema
    - conversion
created_at: 2026-07-27T12:20:16Z
updated_at: 2026-07-27T12:20:22Z
blocked_by:
    - csl26-qtur
---

Outward-facing docs: docs/reference/DATA_MODEL.md (hand-written narrative: ingest architecture, reference classes, containers, contributors, dates, titles, rich text, forward compat, biblatex sec 2.2.1 prior art), generated docs/reference/generated/DATA_MODEL_FIELDS.md and CSL_JSON_MAPPING.md and docs/reference/BIBLATEX_MAPPING.md rendered from docs/schemas/{bib,type-map}.json via new scripts/build-data-model-reference.js, docs/reference/NATIVE_FORMAT.md with test-backed worked examples under examples/data-model/, plus drift-control CI wiring. Blocked by csl26-qtur (the declarative mapping tables it generates from).
