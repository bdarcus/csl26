# Test Strategy: Oracle vs CSLN-Native

## Current Approach: Oracle Tests

**Purpose**: Validate CSL 1.0 backward compatibility

**Method**:
- Test fixtures use CSL JSON format (what citeproc-js expects)
- Compare CSLN output against citeproc-js (the oracle)
- Located: `tests/fixtures/references-expanded.json`
- Test harness: `scripts/oracle-e2e.js`

**Limitations**:
- Cannot test CSLN-specific features beyond CSL 1.0
- Constrained by CSL JSON schema

## Integration Test Organization

Integration tests for the `csln_processor` crate are organized into functional targets to allow for focused testing:

- **citations**: Citation rendering, disambiguation, and group logic.
- **bibliography**: Bibliography rendering, sorting, and author substitution.
- **metadata**: Name parsing, contributor extraction, and date normalization.
- **i18n**: Locale-specific terms, date formatting, and translation resolution.
- **document**: Full document processing and semantic output.

To run a specific target:
```bash
cargo nextest run --test citations
```

## Future: CSLN-Native Tests

**Purpose**: Test features that go beyond CSL 1.0

**Features requiring native format**:
1. **Title/subtitle separation** - CSL 1.0 treats as single string
2. **EDTF dates** - CSL JSON uses simple date-parts arrays
3. **Scoped multilingual fields** - CSL 1.0 doesn't support (csln#66)
4. **Enhanced citation model** - mode, locator types (from csln_core)
5. **Math in variables** - Need proper encoding (csln#64)
6. **Structured name particles** - More nuanced than CSL JSON

**Implementation plan** (deferred):
1. Create `tests/fixtures/csln-native-references.yaml` (or .json - serde supports both)
2. Build separate test harness (no oracle comparison - we ARE the reference)
3. Test CSLN-specific rendering against expected outputs
4. Document intentional divergences from CSL 1.0

## Two-Phase Testing Strategy

### Phase 1: CSL 1.0 Parity (Current)
- Expand oracle test coverage to 15+ reference types
- Achieve high fidelity for tier 1 styles (Chicago, APA, Elsevier)
- Use CSL JSON format throughout
- **Goal**: Prove migration works for existing styles

### Phase 2: CSLN Extensions (Future)
- Add CSLN-native test fixtures
- Test features beyond CSL 1.0
- No oracle comparison (we define the behavior)
- **Goal**: Validate new capabilities

## Decision: Phase 1 First

We're deferring CSLN-native tests to focus on CSL 1.0 parity. This is the right prioritization because:
- Need to prove the migration approach works
- Most styles will initially be migrated from CSL 1.0
- New features can be tested incrementally as they're added
- Keeps current work focused and achievable

## Related
- csln#64 - Math in variables
- csln#66 - Multilingual support
- Citation model in csln_core (mode, locator types)
