---
# csl26-07ww
title: Port localization-integrity detector from scripts/ to citum-schema-style lint.rs
status: todo
type: task
priority: normal
created_at: 2026-08-07T13:21:11Z
updated_at: 2026-08-07T13:21:11Z
parent: csl26-40n4
---

The JS-side detector added in csl26-dfq0 (scripts/, wired into report-core.js as a standalone report field) is a stopgap chosen because that stack explicitly excluded Rust changes. Port the same normalize-and-match-against-locale-value-set check into crates/citum-schema-style/src/lint.rs as a proper style-validation error, following the existing lint pattern there (see lint_raw_locale for the locale-completeness precedent). Once ported, retire or keep the JS version in sync — decide which is authoritative, don't let them drift.
