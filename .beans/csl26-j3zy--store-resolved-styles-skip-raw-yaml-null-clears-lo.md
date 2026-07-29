---
# csl26-j3zy
title: Store-resolved styles skip raw_yaml; null-clears lost
status: todo
type: bug
priority: high
tags:
    - schema
    - styles
    - fidelity
created_at: 2026-07-29T12:41:10Z
updated_at: 2026-07-29T12:41:10Z
blocking:
    - csl26-svfg
---

crates/citum_store/src/resolver.rs deserializes styles with serde_yaml::from_slice / serde_json::from_slice / ciborium directly, bypassing Style::from_yaml_bytes, so raw_yaml is never populated for store-resolved styles in ANY format (including YAML). Shipped consequence: explicit-null clearing of inherited fields (e.g. citation.prefix: ~, citation.options: ~ per overlay.rs null-aware semantics) works for file/inline-loaded styles but is silently ignored for the same bytes resolved through the store. Load-path-dependent extends semantics.

Fix: one raw-preserving constructor, e.g. Style::from_document_bytes(bytes, format), that parses to a generic value tree first (YAML/JSON parse directly; CBOR via ciborium::Value transcode, string-keyed maps only) and deserializes the typed Style from that tree; require it on every load path (engine StyleInput, citum_store resolver, CLI convert, server, bindings). Consider a lint/CI guard forbidding direct serde_yaml::from_slice::<Style> outside the constructor.

- [ ] Format-neutral raw tree populated for yaml/json/cbor
- [ ] All style load paths route through the constructor
- [ ] Regression test: null-clear child resolved via store behaves identically to file-loaded
- [ ] Guard against future bypasses (lint or code-structure)

Prerequisite for csl26-svfg (deep merge reads authored presence from the same raw tree); found during the 2026-07-28/29 inheritance audit follow-up.
