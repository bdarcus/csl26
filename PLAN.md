# CSLN Refactor Implementation Plan

## Current Status: Phase 3a Complete ✅

We have successfully created the core processing pipeline:
- **Phase 2**: Schema alignment, options extraction, template compilation, oracle verification
- **Phase 3a**: Processor crate with citation/bibliography rendering

**36 tests passing** across the workspace.

---

## What's Been Built

### Crate: `csln_core`
- `options.rs` - Style configuration (substitute, contributors, processing mode)
- `template.rs` - TemplateComponent enum (Contributor, Date, Title, Number, Variable)
- New `Style` struct coexisting with legacy types for migration bridge

### Crate: `csln_migrate`  
- `options_extractor.rs` - Extracts global options from CSL 1.0 styles
- `template_compiler.rs` - Converts CslnNode trees to clean TemplateComponents

### Crate: `csln_processor`
- `reference.rs` - CSL-JSON compatible Reference type
- `values.rs` - ComponentValues trait for value extraction
- `processor.rs` - Core Processor with rendering
- `render.rs` - String output formatting

### Verification
- `scripts/oracle.js` - citeproc-js wrapper for oracle comparison
- Integration tests confirming semantic match with citeproc-js

---

## Semantic Match Achieved

| Citation | CSLN Output | citeproc-js Match |
|----------|-------------|-------------------|
| Single author | `(Kuhn, 1962)` | ✅ |
| 3+ authors | `(LeCun et al., 2015)` | ✅ |
| Bibliography | `Kuhn, T. (1962). _Title_.` | ✅ |

---

## Next Steps

### Phase 3b: Expand Coverage
- [ ] Add locale support for terms ("and", "et al.", "ed.")
- [ ] Add List component rendering
- [ ] Add locator handling in citations
- [ ] Handle more contributor roles (publisher, director, etc.)

### Phase 3c: Full Verification
- [ ] Run full APA test suite vs citeproc-js
- [ ] Migrate Chicago style and verify
- [ ] Document any intentional differences

### Phase 4: Bulk Migration
- [ ] Process all 2,844 styles in `styles/` directory
- [ ] Report migration statistics
- [ ] Identify styles requiring manual review

---

## Key Technical Decisions

1. **Serde flatten** - `#[serde(flatten)]` allows `emph: true` directly on components
2. **Non-exhaustive enums** - Future-proof against new component types
3. **CSL-JSON compatible** - Reference type matches existing ecosystem
4. **Options-first** - Style behavior extracted to config, not embedded in templates

---

## Test Counts by Crate

| Crate | Tests |
|-------|-------|
| csln_core | 12 |
| csln_migrate | 8 |
| csln_processor | 16 |
| **Total** | **36** |
