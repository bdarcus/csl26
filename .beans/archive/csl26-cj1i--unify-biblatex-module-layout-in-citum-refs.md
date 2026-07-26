---
# csl26-cj1i
title: Unify biblatex module layout in citum-refs
status: completed
type: task
priority: normal
tags:
    - refactor
    - citum-refs
created_at: 2026-07-26T13:38:50Z
updated_at: 2026-07-26T13:45:47Z
---

Move crates/citum-refs/src/biblatex.rs (entry/field mapping) and src/formats/biblatex.rs (I/O wrappers) into a single src/formats/biblatex/ module, matching the layout of csl_json/ris/native. No public API changes.

- [x] Move src/biblatex.rs -> src/formats/biblatex/mapping.rs (git mv)
- [x] Move src/formats/biblatex.rs -> src/formats/biblatex/mod.rs (git mv), add mod mapping + explicit re-exports
- [x] Update src/lib.rs to pub use formats::biblatex; fix intra-doc links
- [x] Update crates/citum-refs/README.md with full crate description
- [x] Add citum-refs row to crates/README.md
- [x] Repoint csl26-11h2 references from src/biblatex.rs to src/formats/biblatex/mapping.rs
- [x] Add refs scope to .alint.yml conventional-commits allowlist
- [x] just pre-commit clean
- [x] cargo doc -p citum-refs (deny warnings) and cargo check -p citum-io -p citum-engine clean

## Summary of Changes

Moved crates/citum-refs/src/biblatex.rs (entry/field mapping) and src/formats/biblatex.rs (I/O wrappers) into a single src/formats/biblatex/ module (mod.rs + private mapping.rs submodule), matching the layout already used by csl_json/ris/native. No public API changes: formats::biblatex::{load_biblatex, parse_biblatex_str} and the citum_refs::biblatex::* glob re-export (used by citum-io) both still resolve, verified via cargo check -p citum-io -p citum-engine and RUSTDOCFLAGS="-D warnings" cargo doc -p citum-refs (one pre-existing, unrelated doc-link warning on contributor_from_person remains, not introduced by this change).

Also: expanded crates/citum-refs/README.md with crate responsibilities/dep-graph position/format table; added a citum-refs row to crates/README.md; repointed the two citum-refs/src/biblatex.rs references in open bean csl26-11h2 to the new mapping.rs path; added a refs scope to .alint.yml's conventional-commits allowlist (previously missing, so this commit could not otherwise use scope refs). just pre-commit (fmt/clippy/nextest, 2194 tests) is clean.
