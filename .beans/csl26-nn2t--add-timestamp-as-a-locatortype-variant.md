---
# csl26-nn2t
title: Add timestamp as a LocatorType variant
status: todo
type: task
priority: normal
tags:
    - schema
    - fidelity
    - style
created_at: 2026-09-06T17:47:47Z
updated_at: 2026-09-06T17:47:47Z
parent: csl26-ccdt
---

MLA's citation layout (styles-legacy/modern-language-association.csl:1146)
conditions on locator="line page timestamp", but LocatorType
(crates/citum-schema-data/src/citation.rs:155-231) has no Timestamp variant
-- only Line and Page of that trio exist. Citum's
modern-language-association.yaml locators config (Label Case and
Attachment v1.1, bean csl26-7652) therefore covers page and line only.

Needs: add LocatorType::Timestamp, thread it through
LocatorValue/CitationLocator rendering and any locale term tables that key
off LocatorType, and add the corresponding kinds.timestamp entry to
modern-language-association.yaml. Check citum-migrate's CSL-legacy locator
parsing for whether "timestamp" as a legacy locator value already round-
trips to some other variant (possible silent misclassification today).
