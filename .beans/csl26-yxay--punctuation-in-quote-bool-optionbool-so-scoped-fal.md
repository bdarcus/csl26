---
# csl26-yxay
title: 'Let scoped punctuation-in-quote: false override a style-level true'
status: todo
type: task
priority: low
tags:
    - punctuation
    - schema
created_at: 2026-08-01T11:58:13Z
updated_at: 2026-08-01T12:13:18Z
---

Problem: punctuation_in_quote is a plain bool on three structs (style-wide Config, per-citation CitationOptions, per-bibliography BibliographyOptions). The merge logic at crates/citum-schema-style/src/options/mod.rs:718/832/978 is OR-only: `if other.punctuation_in_quote { self.punctuation_in_quote = true }`. A scoped block can turn the feature ON over a style-level false, but a scoped `punctuation-in-quote: false` can never turn it OFF when the style-level value is true -- false cannot be distinguished from unset.

Fix: change the field to Option<bool> on all three structs (Config, CitationOptions, BibliographyOptions) so None means "inherit from parent" and Some(false) means "explicitly disabled here." Regenerate schemas (just schema-gen) in the same commit.

No embedded style currently needs this (none sets punctuation-in-quote: false anywhere), so it is low priority. Follow-up from csl26-1hya (fix punctuation-in-quote at all join boundaries).
