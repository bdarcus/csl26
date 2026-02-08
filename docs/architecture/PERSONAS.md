# CSLN Design Personas

When evaluating features, consider these four stakeholder perspectives.

---

## 1. Style Author

**Who**: Librarian, publisher, journal editor, bibliography manager developer

**Goals**:
- Express complex, idiosyncratic formatting requirements
- Write styles without understanding Rust internals
- Get clear feedback when something doesn't work

**Priorities**:
- Readable, self-documenting YAML
- Sensible defaults (don't make me specify everything)
- Expressive power for edge cases (APA 7th has many)
- Clear error messages pointing to the problem

**Pain points**:
- "Why doesn't this edge case render correctly?"
- "How do I express 'use comma for journals, period for books'?"
- "What are all the options for contributor formatting?"

### LLM-Assisted Style Authoring

LLMs can act as style authors by following the `/styleauthor` workflow (`.claude/skills/styleauthor/SKILL.md`). This was validated by creating the APA 7th Edition CSLN style, where the LLM:

1. Read style guide references (APA website, university LibGuides)
2. Authored the style YAML using structured blocks, items, delimiters, and overrides
3. Ran the processor and compared output to reference expectations
4. Fixed both style YAML and processor code (adding integral citations, locator support)
5. Iterated until output matched the style guide

This is a **full-stack workflow**: the LLM can evolve the processor and core types when features are missing, not just author YAML. Guard rails (cargo test, clippy, oracle) prevent regressions.

**Key insight**: LLM-authored styles can be higher quality than migration-compiled styles because the LLM understands the style guide's intent, not just the CSL 1.0 XML structure. The APA 7th style created this way achieved 5/5 citation and 5/5 bibliography match.

---

## 2. Web Developer

**Who**: Frontend developer building a style editor, citation manager UI, or API

**Goals**:
- Build a GUI that generates valid styles
- Validate user input without running the processor
- Present enumerable options in dropdowns, not free-text fields

**Priorities**:
- Predictable JSON Schema with no hidden state
- All valid values are enumerable (enums over strings)
- No order-dependent fields or implicit behavior
- Clean serialization/deserialization roundtrip

**Pain points**:
- "What are all valid values for this field?"
- "Is this combination of options valid?"
- "Does field order matter?"
- "Why did my valid-looking YAML fail to parse?"

---

## 3. Systems Architect

**Who**: Rust developer maintaining the processor and migration tools

**Goals**:
- Type-safe, maintainable codebase
- Perfect migration fidelity from CSL 1.0
- Performance suitable for batch processing 2,844+ styles

**Priorities**:
- Strict Rust enums, no stringly-typed values
- Well-commented code with spec references
- Comprehensive test coverage
- Oracle verification for all changes

**Pain points**:
- "This implicit behavior is hard to maintain"
- "Serde is parsing this incorrectly"
- "How do I extend this without breaking existing styles?"

---

## 4. Domain Expert

**Who**: Domain expert, researcher, bibliographer

**Goals**:
- Research existing solutions (CSL 1.0, CSL-M, biblatex, CSLN issues) and apply their lessons
- Ensure semantic correctness and continuity with established standards
- Verify coverage of complex edge cases

**Priorities**:
- "Don't reinvent the wheel"
- Continuity with prior art (CSL 1.0, biblatex)
- Robustness for complex styles (legal, multilingual)

**Pain points**:
- "We solved this 10 years ago, why ignore that?"
- "This model is too simple for real-world bibliography"
- "Inconsistent terminology with standard practices"

---

## Feature Evaluation Checklist

Before adding or modifying a feature, verify it works for all three personas:

### Style Author
- [ ] Can this be expressed in YAML without reading processor code?
- [ ] Are defaults sensible for 80% of use cases?
- [ ] Is the field name self-documenting?
- [ ] Does the error message explain what went wrong?

### Web Developer
- [ ] Is this field enumerable (enum, not free-form string)?
- [ ] Can the schema be validated without running the processor?
- [ ] Does the field have predictable serialization?
- [ ] Is the field independent (no implicit interaction with other fields)?

### Systems Architect
- [ ] Is this type-safe (Rust enum, not String)?
- [ ] Does this maintain oracle parity with citeproc-js?
- [ ] Is the implementation well-commented?
- [ ] Are edge cases tested?

### Domain Expert
- [ ] Has this been implemented or researched in CSL 1.0/M and biblatex?
- [ ] Does this handle known complex edge cases?
- [ ] Is the terminology consistent with domain standards?
- [ ] Are we reinventing the wheel unnecessarily?
- [ ] For multilingual: Does it support locale-specific formatting?
- [ ] For legal: Does it handle jurisdiction hierarchies and parallel citations?

---

## Specialized Checklists

### Multilingual Features
When adding multilingual support, verify:
- [ ] Entry-level `language` field is respected
- [ ] Locale-specific templates can override defaults
- [ ] RTL scripts (Arabic, Hebrew) render correctly
- [ ] CJK conventions (no spaces, different delimiters) are supported
- [ ] Locale terms switch appropriately per item language

### Legal Citation Features
When adding legal support, verify:
- [ ] Extended types (`hearing`, `regulation`) are supported
- [ ] Jurisdiction hierarchies work (e.g., `us:federal:circuit:9`)
- [ ] Court classes can be defined and matched
- [ ] Parallel citations suppress repeated elements
- [ ] `hereinafter` short forms are available
- [ ] Position conditions (`subsequent`, `ibid`, `far-note`) work

---

## Example: Evaluating `name-order` Field

**Feature**: Allow per-contributor control of name ordering (given-first vs family-first)

| Persona | Evaluation |
|---------|------------|
| Style Author | ✅ Explicit YAML field, no magic. `name-order: given-first` is readable |
| Web Developer | ✅ Enum with two values, easy dropdown. No hidden interaction |
| Systems Architect | ✅ `NameOrder` enum, not String. Well-documented in template.rs |
| Domain Expert | ✅ Matches `given-family` distinction in biblatex. Consistent with CSL 1.0 `name-part` logic but simplified |

**Result**: Feature approved for all personas.
