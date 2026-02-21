# Rust 2024 Edition Migration

**Date:** 2026-02
**Status:** Complete
**Branch:** feat/edition-2024

## Summary

The workspace was migrated from Rust edition 2021 to edition 2024 (stabilized in Rust 1.85,
February 2025). The migration is purely mechanical with no behavioral changes to processing
logic or output.

## Why Edition 2024?

Edition 2024 tightens safety guarantees and makes unsafe code more explicit -- which aligns
directly with this project's zero-tolerance approach to `unwrap()` and implicit unsafe. Key
motivations:

- **Explicit unsafe ops in `unsafe fn`**: In edition 2024, unsafe function bodies are no longer
  implicitly unsafe. Each unsafe operation must be wrapped in an explicit `unsafe {}` block. This
  makes the FFI surface in `ffi.rs` clearer about exactly which lines carry safety obligations.
- **Safer match ergonomics**: `ref` and `ref mut` in implicitly-borrowing match patterns are now
  disallowed. The compiler enforces binding modes consistently, catching a class of subtle
  ownership bugs at compile time.
- **Explicit unsafe attributes**: `#[no_mangle]` must be written as `#[unsafe(no_mangle)]` to
  acknowledge that symbol export bypasses Rust's name mangling safety.
- **`gen` keyword reservation**: Reserves `gen` for future generator/iterator syntax (see below).

## Changes Made

### Workspace

`Cargo.toml`: `edition = "2021"` -> `edition = "2024"` (propagates to all 7 member crates via
`[workspace.package]`).

### `csln_processor/src/ffi.rs`

- All `#[no_mangle]` replaced with `#[unsafe(no_mangle)]` (edition 2024 requirement).
- All unsafe operations inside `unsafe fn` bodies wrapped in explicit `unsafe {}` blocks:
  `CStr::from_ptr()`, `&*processor` raw pointer derefs, `Box::from_raw()`, `CString::from_raw()`.

### `csln_core/src/template.rs`

- `schemars::gen::SchemaGenerator` -> `schemars::r#gen::SchemaGenerator` because `gen` is now a
  reserved keyword in edition 2024. The `r#` prefix escapes it as a raw identifier.

### `csln_migrate` (multiple files)

- Removed `ref mut` from patterns in `if let` / `match` arms that operate on `&mut T` values
  (e.g., `get_mut()` returns). Edition 2024 disallows explicit borrows within implicitly-borrowing
  patterns. Affected files: `passes/reorder.rs`, `template_compiler/mod.rs`, `lib.rs`.

### Clippy auto-fixes

~80 `collapsible_if` lints were fixed across the codebase using `cargo clippy --fix`. These were
pre-existing style issues now enforced as errors by the updated clippy version shipping with
Rust 1.85. Fixed files span `csln_migrate`, `csln_processor`, and `csln_core`.

## Why Not Async?

The processor is entirely CPU-bound. The hot path -- template rendering, name formatting, date
formatting, substitution logic -- is pure in-memory computation with no I/O. Adding `tokio` for
async would introduce runtime overhead with zero throughput benefit.

The only file I/O is at startup (loading style, bibliography, citations), which completes before
processing begins. Async file I/O on most OSes uses thread pools internally anyway.

The `cdylib` target (`ffi.rs`) adds a hard constraint: async runtimes cannot cross FFI
boundaries safely. The `RefCell<HashMap<String, usize>>` used for citation number tracking is
also incompatible with `async` (cannot be held across `.await` points).

**Decision:** Keep the processor 100% synchronous. The planned JSON server mode (see roadmap
below) will live in a separate `csln_server` crate using `axum` + `tokio`, keeping the core
library FFI-safe and dependency-free.

## Why Not `gen` Blocks?

The dplanning phase assumed `gen {}` blocks were stable in Rust 1.85 / edition 2024. They are
not. The `gen` keyword is reserved in edition 2024, but `gen {}` blocks (lazy iterators without
heap allocation) remain experimental behind `#![feature(gen_blocks)]` as of Rust 1.85.

Tracked at: <https://github.com/rust-lang/rust/issues/117078>

Once stabilized, the primary candidates for `gen` refactoring are:

- `process_template_with_number_internal_with_format` in `rendering.rs` (lines 1029-1083): the
  `filter_map` chain collects into an intermediate `Vec<ProcTemplateComponent>`. A `gen` block
  would eliminate this allocation while preserving the mutable-state deduplication logic
  (`rendered_vars`, `substituted_bases`).
- `strip_author_component` and related filter pipelines in `rendering.rs`.
- Group membership iteration in `grouping/`.

## Roadmap: JSON Server Mode

The async story for CSLN is deferred to a future `csln_server` crate, separate from the
synchronous processor. Design goals:

- Long-running background process (like Haskell citeproc server) to eliminate startup latency
  for interactive editors.
- JSON-over-stdin/stdout or HTTP (axum) API surface.
- Shares the synchronous processor as a library dependency -- no async in the hot path.
- Keeps `csln_processor` FFI-safe and dependency-minimal.

This is reflected in the "JSON Server Mode (Roadmap)" card on the project website.

## Performance Impact

Benchmarked before and after the edition bump using `./scripts/bench-check.sh`:

| Benchmark                    | Before        | After         | Change   |
|------------------------------|---------------|---------------|----------|
| Process Citation (APA)       | 6.405 µs      | 6.443 µs      | +0.6%    |
| Process Bibliography (10)    | 94.700 µs     | 95.678 µs     | +1.0%    |

Both changes are within criterion's noise threshold. The edition bump has no meaningful effect
on runtime performance, as expected for a purely mechanical migration.

## CI Compatibility

CI uses `dtolnay/rust-toolchain@stable`, which tracks the latest stable Rust (1.85+ as of
February 2025). Edition 2024 is fully supported. No toolchain pinning changes required.
