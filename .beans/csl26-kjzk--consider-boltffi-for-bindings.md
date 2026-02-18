---
# csl26-kjzk
title: Consider boltffi for bindings
status: todo
type: task
priority: normal
created_at: 2026-02-17T22:02:32Z
updated_at: 2026-02-18T15:00:00Z
---

BoltFFI looks promising as a unifying path for multi-language bindings, especially for TypeScript/WASM plus native targets (Swift/Kotlin). This bean defines a forward-looking plan that keeps current velocity while reducing long-term binding complexity.

## Goal

Establish one minimal, stable Rust binding contract and use it across multiple targets with the least duplicated glue code.

## Current State

- Current C ABI lives in `crates/csln_processor/src/ffi.rs` and is used by Lua bindings.
- `csln_processor` already builds as `cdylib` and `rlib`.
- WASM support remains a roadmap target.

## Strategy: Two Lanes

### Lane A (Baseline): Keep current C ABI

- Keep existing C/Lua FFI in place during exploration.
- Treat this as the stability baseline and fallback path.
- Do not break existing exported symbols or ownership conventions during the spike.

### Lane B (Expansion): Introduce BoltFFI

- Add BoltFFI on top of a small shared Rust API contract.
- Start with one or two high-value targets only (TypeScript/WASM and Swift or Kotlin).
- Expand only after objective checks pass.

## Binding API v0 (Shared Contract)

Define and freeze a tiny API surface for cross-language bindings:

1. `new(style_json, bib_json) -> handle`
2. `new_with_locale(style_json, bib_json, locale_json) -> handle`
3. `render_citation(handle, citation_json, format) -> string/error`
4. `render_bibliography(handle, format) -> string/error`
5. `free(handle)`

Notes:
- Keep async out of v0 unless a concrete workload requires it now.
- Keep JSON-in/string-out for v0 to minimize cross-language type complexity.

## Spike Plan (1-2 weeks)

1. Map one existing render path to Binding API v0 for BoltFFI.
2. Generate bindings for TypeScript/WASM and one native target (Swift or Kotlin).
3. Build tiny smoke consumers for each target.
4. Compare behavior against current C ABI baseline on shared fixtures.

## Acceptance Gates

BoltFFI path is considered viable only if all pass:

1. Constructor/destructor lifecycle works without leaks or crashes.
2. String ownership/free semantics are explicit and validated.
3. Errors are surfaced deterministically (not silent null-only behavior).
4. One golden citation and one bibliography fixture match baseline output.
5. Dev workflow is simpler than hand-rolled glue for the tested targets.

## WASM Decision Rule

- If BoltFFI's WASM output integrates cleanly with packaging/runtime expectations, use it.
- If not, keep WASM as a separate implementation lane while preserving the same Binding API v0 semantics.

## Lua Gap

Lua is not currently a BoltFFI target. Keep C ABI for Lua until support exists (tracked by boltffi/boltffi#49) and migration value is clear.

## Risks

- BoltFFI maturity/API churn may add short-term instability.
- Running dual lanes increases temporary maintenance cost.
- Divergence risk if new features bypass Binding API v0 discipline.

## Exit Criteria

- Proceed with broader BoltFFI adoption if spike passes all acceptance gates and reduces net maintenance.
- Otherwise keep BoltFFI optional, continue C ABI baseline, and revisit after BoltFFI/Lua maturity improves.
