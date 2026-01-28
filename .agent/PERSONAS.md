# CSLN Design Personas

When evaluating features, consider these three stakeholder perspectives.

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

---

## Example: Evaluating `name-order` Field

**Feature**: Allow per-contributor control of name ordering (given-first vs family-first)

| Persona | Evaluation |
|---------|------------|
| Style Author | ✅ Explicit YAML field, no magic. `name-order: given-first` is readable |
| Web Developer | ✅ Enum with two values, easy dropdown. No hidden interaction |
| Systems Architect | ✅ `NameOrder` enum, not String. Well-documented in template.rs |

**Result**: Feature approved for all personas.
