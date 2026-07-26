---
# csl26-95cq
title: Contributor folding is unenforced on direct construction
status: todo
type: task
priority: normal
tags:
    - schema
    - conversion
    - architecture
created_at: 2026-07-26T16:23:59Z
updated_at: 2026-07-26T16:23:59Z
---

The contributor-drop bug YDX found via gb7714-bench (csl26-7ab8, fixed in ae0d9352) was a symptom of a structural asymmetry that is still there. Only the instance was fixed, not the class.

## The asymmetry

All five structural types carry `#[serde(from = "...Deser")]` (crates/citum-schema-data/src/reference/types/structural.rs:108, 482, 709, 958, 1240), so **reading** always folds the `author`/`editor`/`translator` shorthands into the canonical `contributors` vec via `reconcile_contributors`.

**Writing has no equivalent.** None of the five uses `serde(into = ...)`. The shorthands are `#[serde(skip_serializing)]` and `contributors` is the only field serialization preserves, so a reference built directly in Rust with `author: Some(...)` and an empty `contributors` serializes with **no contributors at all**, silently.

`normalize_contributors()` (accessors.rs:1664) exists to bridge this, but it is opt-in — 4c67fc08's own commit message says it is exposed "so any direct-construction converter *can* call it". There is exactly **one** production call site: crates/citum-refs/src/formats/biblatex/mapping.rs:277. Nothing in the type system or the test suite stops the next one from forgetting.

## Why it is easy to miss

`author` does double duty: input shorthand *and* derived read view (`reconcile_contributors` rebuilds the views from the folded vec). So a directly-built reference **renders correctly in memory** — the view field is populated — and only loses its contributors when it crosses a serialization boundary. In-memory rendering tests therefore cannot catch it. It took an external benchmark to find, and it cost real fidelity: the .bib-vs-.json corpus diff was 301/344 differing before the fix, 206/344 after (see csl26-2xjn).

## The documented usage pattern has the trap

crates/citum-engine/src/lib.rs:70 — the crate's module-level doc example — builds a `Monograph` with `author: Some(...)` directly. Anyone who follows the front-page example and then serializes the result loses the author. Benches and many unit tests use the same shape (e.g. processor/disambiguation.rs, sorting.rs, api/session.rs, citum-schema-style/src/macros.rs).

## Options

1. **Make it symmetric (preferred).** Add `#[serde(into = "...Ser")]` alongside the existing `from = "...Deser")]` on the five types, folding on write exactly as we fold on read. The invariant then holds by construction, direct construction cannot drop contributors, and `normalize_contributors` becomes an optimization rather than a correctness requirement. Requires `Clone` (already derived) and a `From<T> for TSer` per type.
2. **Guard it with tests.** A round-trip property test per structural type: build directly with only the shorthand set, serialize, deserialize, assert contributors survive. Cheap, catches regressions, does not prevent the class.
3. Constructor/builder discipline that always normalizes.

Option 1 subsumes 2; do 1 and keep a round-trip test as the regression pin.

## Todo

- [ ] Decide between symmetric serde (option 1) and test-only guards
- [ ] Implement, with a round-trip test per structural type
- [ ] Fix the lib.rs:70 doc example, or make it correct-by-construction
- [ ] Audit remaining direct-construction sites for the same latent issue

## Related

- csl26-7ab8 (the original fix; closed)
- csl26-2xjn (residual biblatex fidelity gaps — a different problem, not this one)
