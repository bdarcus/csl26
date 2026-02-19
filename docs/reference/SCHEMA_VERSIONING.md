# Schema Versioning Policy

This document defines the versioning strategy for CSL Next (CSLN) style format and processor code.

## Two-Track Versioning

CSLN uses **independent versioning** for code and schema to maintain clarity and stability:

| Track | What | Version Source | Automation |
|-------|------|----------------|------------|
| **Code** | Rust crates (processor, core library, CLI) | `Cargo.toml` workspace version | Fully automated via release-plz |
| **Schema** | Style YAML format specification | `Style.version` default in `csln_core/src/lib.rs` | Semi-automated (manual bump + validation) |

### Why Two Tracks?

**Separation of concerns:**
- Code changes (refactoring, performance, new APIs) don't force schema version bumps
- Schema changes (new fields, format breaking changes) don't force processor releases
- Users see schema version bumps **only when the format actually changes**

**Example:**
- Processor v0.2.3 supports schema v1.0.0
- Add optional `license` field to styles → schema v1.1.0, processor v0.2.4
- Refactor processor internals → processor v0.2.5, schema unchanged (still v1.0.0)

## Version Relationship

**Synchronization Rule:** Code version ≥ Schema version

The processor can support multiple schema versions through backward compatibility:
- Processor v2.3.1 MUST read schema v1.0.0 styles (via `#[serde(default)]` for new fields)
- Processor v2.3.1 SHOULD read schema v2.0.0 styles (current format)

## Git Tag Convention

Different tag prefixes distinguish release types:

```bash
# Code/processor releases
v0.1.0
v0.2.0
v1.0.0

# Schema releases
schema-v1.0.0
schema-v1.1.0
schema-v2.0.0
```

GitHub Releases will use clear naming:
- `v0.2.3 - Processor Release`
- `schema-v1.0.0 - Format Stabilization`

## Pre-1.0 Compatibility

**0.x.y versions (both tracks):**
- Breaking changes allowed in minor version bumps
- No backward compatibility guarantees
- Experimental phase for both code API and style format

**1.0.0+ versions:**
- **Code track**: Stable public API, SemVer guarantees for library users
- **Schema track**: Backward compatibility, processor must read older formats
- Breaking schema changes only in major versions (2.0.0, 3.0.0)

## Independent 1.0 Milestones

Tracks reach 1.0 independently based on their own stability criteria:

**Schema → 1.0.0 first** (most likely):
- Format stabilized, validated against top 10 parent styles
- Backward compatibility guarantees begin
- Style authors get stability while processor evolves

**Code → 1.0.0 later**:
- Library API polished for external consumers
- WASM bindings stable
- Public API guarantees begin

## Schema Versioning Workflow

### Current Schema Version

Check the default schema version in `../crates/csln_core/src/lib.rs`:

```rust
fn default_version() -> String {
    "1.0".to_string()  // Current schema version
}
```

All style files inherit this default unless explicitly overridden in YAML.

### When to Bump Schema Version

**Patch version (1.0.0 → 1.0.1):**
- Documentation clarifications
- Bug fixes in validation logic (no format changes)
- Typo corrections in field descriptions

**Minor version (1.0.0 → 1.1.0):**
- New optional fields (backward compatible)
- New enum variants with `#[non_exhaustive]`
- Deprecation warnings for old fields (not removal)
- New preset types (non-breaking additions)

**Major version (1.0.0 → 2.0.0):**
- Required field additions
- Field removals
- Field type changes (breaking existing styles)
- Enum variant removals
- Changed semantics of existing fields

### Bumping Schema Version

Use the `../scripts/bump-schema.sh` script to update the schema version:

```bash
# Bump to new schema version
./../scripts/bump-schema.sh 1.1.0

# What it does:
# 1. Updates default_version() in csln_core/src/lib.rs
# 2. Validates all styles parse correctly with new version
# 3. Updates ./SCHEMA_VERSIONING.md with timestamp
# 4. Creates git tag: schema-v1.1.0
```

**Manual process:**
1. Update `default_version()` in `../crates/csln_core/src/lib.rs`
2. Run `cargo test` to ensure all styles parse
3. Update this file with schema changelog entry
4. Commit with message: `chore(schema): bump schema version to X.Y.Z`
5. Create git tag: `git tag schema-vX.Y.Z`

### Schema Changelog

Track schema changes separately from code changes:

#### schema-v1.0.0 (Unreleased)
- Initial CSLN schema stabilization
- Core fields: info, options, citation, bibliography
- Supported options: contributors, dates, titles, page-range-format
- Validated against APA 7th, Chicago Author-Date

## CI Validation

The CI pipeline validates schema consistency:

1. **Format check**: All `.yaml` files in `styles/` must parse without errors
2. **Version check**: Styles without explicit `version` field use current default
3. **Backward compatibility**: Processor must read N-1 schema version (after 1.0)

CI fails if:
- Any style file fails to parse
- Schema version regression detected (style requires newer schema than processor supports)

## User-Facing Version Display

Users encounter versions in different contexts:

### CLI (Code Version)
```bash
$ csln --version
csln 0.2.3

$ cargo run --bin csln --features schema -- schema style
# prints style JSON Schema to stdout
```

### Style Files (Schema Version)
```yaml
# styles/apa-7th.yaml
version: "1.0.0"  # Schema version (optional, inherits default)
info:
  title: APA 7th Edition (CSLN)
  id: https://www.zotero.org/styles/apa-7th-csln
```

### Documentation
- README.md shows both versions clearly
- Release notes distinguish code vs schema releases
- Migration guides reference specific schema versions

## FAQ

**Q: Why not unify code and schema versions?**
A: Code refactoring would force schema version bumps (confusing users). Schema changes would force processor releases (unnecessary coupling).

**Q: Can processor v2.0 read schema v1.0 styles?**
A: Yes, after schema v1.0.0 release, backward compatibility is guaranteed via `#[serde(default)]` for new fields.

**Q: Do individual styles have versions?**
A: No, styles inherit the schema version. Style evolution is tracked via Git history, not version numbers. This follows CSL 1.0 precedent.

**Q: When does schema hit 1.0.0?**
A: When the format is validated against top 10 parent styles and stabilized for public use. Code may still be 0.x at that point.

**Q: What if I want to use a newer schema before processor supports it?**
A: Use explicit `version` field in style YAML. Processor will fail gracefully with version mismatch error.

## References

- [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
- [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
- [release-plz Documentation](https://release-plz.ieni.dev/)
- [git-cliff Configuration](https://git-cliff.org/./configuration)
