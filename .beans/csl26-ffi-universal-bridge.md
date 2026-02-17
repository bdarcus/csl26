---
# csl26-ffi-universal-bridge
title: Universal C-FFI Bridge
status: completed
type: feature
priority: high
created_at: 2026-02-17T12:00:00Z
updated_at: 2026-02-17T12:00:00Z
---

Implement a C-compatible Foreign Function Interface (FFI) to allow CSLN to serve as a high-performance citation engine for multiple languages.

Goals:
- [x] Create C-compatible lifecycle management (new/free) for Processor
- [x] Implement JSON-based data exchange for styles, bibliographies, and citations
- [x] Support stateful processing to avoid re-parsing overhead
- [x] Feature-gate FFI exports to maintain safe Rust defaults
- [x] Provide multi-format rendering exports (LaTeX, HTML, Plain Text)
- [x] Implement proof-of-concept LuaJIT binding for LuaLaTeX
- [x] Document general-purpose integration for Python/JS developers

Deliverables:
- crates/csln_processor/src/ffi.rs
- bindings/README.md
- bindings/lua/csln.lua
- High-performance bridge for LuaLaTeX integration

Refs: #105, #106
