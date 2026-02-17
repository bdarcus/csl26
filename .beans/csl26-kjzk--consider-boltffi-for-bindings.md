---
# csl26-kjzk
title: Consider boltffi for bindings
status: todo
type: task
priority: normal
created_at: 2026-02-17T22:02:32Z
updated_at: 2026-02-17T23:00:00Z
---

BoltFFI seems like it might be an interesting solution for this project:

https://github.com/boltffi/boltffi

It currently has "full support" for Swift, Kotlin, and Typescript/WASM, with plans
for Python, C#, and Ruby.

For languages that support it, it will also autogen async support on the other side, which should be valuable for our purposes. From the docs:

> Rust async functions can be exported just like regular functions.

I just submitted an issue for Lua support:

https://github.com/boltffi/boltffi/issues/49

I am thinking it might make for a more integrated but also targeted approach than separate FFI + WASM?

## Planning Analysis

### Fit Assessment

The current FFI layer (`crates/csln/src/ffi.rs`) is ~300 lines of hand-rolled
`unsafe` C-ABI glue — manual null pointer checks, `CString`/`CStr`
conversions, and no async support. BoltFFI would replace this with
macro-annotated Rust that autogenerates type-safe bindings for:

* Swift — Pandoc-adjacent CLI tooling, macOS/iOS citation apps
* Kotlin — Android/JVM citation managers (Zotero-adjacent)
* TypeScript/WASM — browser and Node.js consumers (style editor vision)

All three targets appear in the feature roadmap and CLAUDE.md. The async
export story is directly relevant to the JSON server mode goal ("run as a
background process to minimize startup latency").

### Lua Gap

Lua is not yet supported (issue filed: https://github.com/boltffi/boltffi/issues/49).
The current C-FFI layer must coexist until that resolves. Strategy:

* Retain `ffi.rs` for Lua (C-ABI, cdylib output already configured)
* Adopt BoltFFI for Swift/Kotlin/WASM targets in parallel
* Merge or deprecate `ffi.rs` once BoltFFI Lua support ships

The existing `crate-type = ["rlib", "cdylib"]` in Cargo.toml already
produces the dylib output BoltFFI needs — no build config changes required.

### Risks

* BoltFFI is early-stage — track issue velocity and API stability before
  committing to it as the primary bindings layer.
* Coexistence of two FFI mechanisms adds maintenance surface temporarily.
* Async export relies on BoltFFI's runtime bridge — verify it integrates
  with Tokio (our async runtime) before adopting.

### Recommended Next Steps

1. Spike: annotate one csln_processor function with BoltFFI macros and
   generate Swift + TypeScript bindings. Verify correctness and ergonomics.
2. Benchmark cold-start overhead vs the hand-rolled C layer.
3. Track boltffi/boltffi#49 for Lua support ETA.
4. If spike passes, file a new bean to migrate Swift/Kotlin/WASM targets.
