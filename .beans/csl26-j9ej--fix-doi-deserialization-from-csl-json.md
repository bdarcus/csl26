---
# csl26-j9ej
title: Fix DOI deserialization from CSL JSON
status: completed
type: bug
priority: high
created_at: 2026-02-12T22:00:03Z
updated_at: 2026-02-12T22:06:46Z
---

JSON has uppercase DOI field but Rust structs expect lowercase doi. Need #[serde(alias = "DOI")] annotations on all doi fields in reference types.

This is a prerequisite for full hyperlink support.

Refs: csln#155 (broader hyperlink configuration feature)
