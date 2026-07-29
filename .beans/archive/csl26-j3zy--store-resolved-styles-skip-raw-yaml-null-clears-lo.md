---
# csl26-j3zy
title: Store-resolved styles skip raw_yaml; null-clears lost
status: completed
type: bug
priority: high
tags:
    - schema
    - styles
    - fidelity
created_at: 2026-07-29T12:41:10Z
updated_at: 2026-07-29T13:22:21Z
blocking:
    - csl26-svfg
---

crates/citum_store/src/resolver.rs deserializes styles with serde_yaml::from_slice / serde_json::from_slice / ciborium directly, bypassing Style::from_yaml_bytes, so raw_yaml is never populated for store-resolved styles in ANY format (including YAML). Shipped consequence: explicit-null clearing of inherited fields (e.g. citation.prefix: ~, citation.options: ~ per overlay.rs null-aware semantics) works for file/inline-loaded styles but is silently ignored for the same bytes resolved through the store. Load-path-dependent extends semantics.

Fix: one raw-preserving constructor, e.g. Style::from_document_bytes(bytes, format), that parses to a generic value tree first (YAML/JSON parse directly; CBOR via ciborium::Value transcode, string-keyed maps only) and deserializes the typed Style from that tree; require it on every load path (engine StyleInput, citum_store resolver, CLI convert, server, bindings). Consider a lint/CI guard forbidding direct serde_yaml::from_slice::<Style> outside the constructor.

- [x] Format-neutral raw tree populated for yaml/json/cbor
- [x] All style load paths route through the constructor
- [x] Regression test: null-clear child resolved via store behaves identically to file-loaded
- [x] Guard against future bypasses (lint or code-structure)

Prerequisite for csl26-svfg (deep merge reads authored presence from the same raw tree); found during the 2026-07-28/29 inheritance audit follow-up.

## Summary of Changes

Added `Style::from_document_bytes(bytes, StyleDocumentFormat)` in
`crates/citum-schema-style/src/style/model.rs` as the single raw-preserving
constructor for YAML/JSON/CBOR (CBOR is rejected if any map uses a non-string
key, since overlay null-clear lookups key on string field names). Factored
`from_yaml_str`/`from_yaml_bytes`/`from_document_bytes` onto a shared
`from_raw_value` tail.

Routed every remaining Style load path through it:
- `citum_store::resolver` — `StoreResolver::resolve_item`/`load_item_at` now
  take a parse closure so `Style` uses the raw-preserving path while `Locale`
  keeps the plain generic path; `FileResolver::resolve_style` likewise.
- `citum-schema-style::style::resolution::resolve_style_reference_uri` — the
  `file://` `extends` fallback used when no external resolver is supplied.
- `citum-cli`'s `convert style` command.
- `citum-analyze`'s batch-test YAML round-trip check (found by the new guard
  test, though this one only discards the parsed value — no correctness bug,
  fixed for consistency).

Added a regression test (`citum_store::resolver_tests::store_raw_ingest_regression`)
proving a child style with `citation.options: ~` resolved through the store in
each of YAML/JSON/CBOR clears the inherited option identically to a
file-loaded child — verified to fail against the pre-fix code, then pass
after.

Added `crates/citum-schema-style/tests/raw_ingest_guard.rs`, a workspace-wide
test forbidding the turbofish form of a direct bypass
(`serde_yaml::from_slice::<Style>(..)` and siblings) outside the constructor,
plus a `CODING_STANDARDS.md` rule. The guard can't catch a bypass hidden
behind return-type inference (the actual shape of all three bugs found here);
that residual gap is documented for manual review.

Updated `docs/specs/STYLE_INHERITANCE.md` Implementation Note 2 from
"currently bypasses" to fixed, referencing the new constructor and guard.

`ResolverError` mapping: `parse_style_bytes` preserves the existing
`YamlError`/`JsonError`/`CborError` variants (mapped from `StyleDocumentError`)
rather than collapsing to `InvalidStyle`, since `resolver_arch.rs` already
asserts on those variants for `FileResolver`/`HttpResolver`.

Full workspace: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
and `cargo nextest run` (2276 tests) all pass. `just schema-gen` produced no
diff (Style's schema-visible field surface is unchanged).
