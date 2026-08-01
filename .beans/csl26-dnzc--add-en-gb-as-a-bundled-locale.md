---
# csl26-dnzc
title: Add en-GB as a bundled locale
status: todo
type: feature
priority: normal
tags:
    - locale
    - i18n
created_at: 2026-08-01T11:58:13Z
updated_at: 2026-08-01T11:58:13Z
---

styles/mhra-notes.yaml declares default-locale: en-GB, which is not in EMBEDDED_LOCALE_IDS (crates/citum-schema-style/src/embedded/locales.rs). load_locale_or_default (crates/citum_store/src/chain.rs) silently substitutes en-US with no diagnostic today, and scripts/report-core.js's resolveStyleLocale() now has to special-case this (skip passing --locale rather than erroring) since the CLI hard-errors on an explicit --locale for an unembedded code. Author a proper en-GB locale (British grammar-options -- punctuation-in-quote: false, quote glyphs, terms) per docs/guides/AUTHORING_LOCALES.md, register it in embedded/locales.rs, regenerate schemas. Confirmed with Bruce: prefer adding the locale over papering over the gap. Follow-up from csl26-1hya.
