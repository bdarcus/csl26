# Locator Input Specification

**Status:** Active
**Version:** 1.1
**Date:** 2026-06-02
**Related:** bean csl26-j007, bean csl26-8sty,
`crates/citum-schema-data/src/citation.rs`,
`docs/specs/LOCATOR_RENDERING.md`,
`docs/specs/NON_STANDARD_NUMBERING_AND_LOCATOR_KINDS.md`

## Purpose

Authoritatively document how a citation locator is **expressed as input** —
what YAML/JSON forms are accepted, how the canonical in-memory model is
constructed from them, and how plurality is determined. This is the input
counterpart to `LOCATOR_RENDERING.md`, which documents *how a locator is
printed* once in memory.

This spec also records a design review of locator-input **ergonomics** first
raised in bean csl26-j007, with explicit options (including do-nothing) for
each open question.

## Scope

**In scope:**
- Three locator-input surfaces and how each produces `CitationLocator`.
- The `LocatorType`, `LocatorValue`, `LocatorSegment`, and `CitationLocator`
  data model.
- Plurality detection and its failure modes.
- Prose/Djot locator normalisation (`normalize_locator_text`).
- The compact-map syntax that was prototyped and reverted (historical record +
  future option).
- Ergonomic open questions with named options.

**Out of scope:**
- Locator rendering (labels, range-format, patterns) → see `LOCATOR_RENDERING.md`.
- Locator kind inventory and custom kinds → see
  `NON_STANDARD_NUMBERING_AND_LOCATOR_KINDS.md`.
- Style-level `LocatorConfig` → see `LOCATOR_RENDERING.md`.

## Design

### Canonical data model

All three input surfaces collapse to the same in-memory type:

```rust
CitationLocator = Single(LocatorSegment)
               | Compound { segments: Vec<LocatorSegment> }   // ≥2 segments
```

(`crates/citum-schema-data/src/citation.rs`, `CitationLocator`, line ~460)

```rust
LocatorSegment = { label: LocatorType, value: LocatorValue }
```

```rust
LocatorValue = Text(String)                    // plurality by heuristic
             | Explicit { value: String, plural: bool }  // explicit override
```

(`citation.rs` ~376)

```rust
LocatorType = Page | Chapter | Section | … | Custom(String)
```

~35 known kinds plus an open `Custom(String)` variant. See
`NON_STANDARD_NUMBERING_AND_LOCATOR_KINDS.md` for the full inventory.

### Input surface 1 — structured YAML/JSON

The primary machine-facing input. `CitationItem.locator` accepts either form
directly:

```yaml
# Single locator (shorthand — no wrapper needed)
locator:
  label: page
  value: "23"

# Single locator with explicit plural override
locator:
  label: figure
  value:
    value: "A-3"
    plural: false   # override: heuristic would see '-' and say plural

# Compound locator (≥2 segments, explicit 'segments' wrapper required)
locator:
  segments:
    - label: chapter
      value: "3"
    - label: page
      value: "42-45"
```

The deserialiser (`CitationLocatorRepr`, `citation.rs` ~470) is `untagged`:
it tries `Single(LocatorSegment)` first, then `Compound { segments }`. A
one-element `segments` list is rejected at construction time (`compound()`
enforces ≥2).

### Input surface 2 — Djot/Pandoc prose citations

Prose citations embed a locator as free text inside the Djot citation span
and are normalised by `normalize_locator_text` (`citation.rs` ~616):

```djot
[@smith2020, p. 23]
[@smith2020, ch. 3, p. 42]   # compound — comma before a recognised label
[@smith2020, 42]              # bare number → LocatorType::Page
```

**Parsing rules:**

1. Trim whitespace; empty → `None`.
2. Split on `,` only when the token *after* the comma begins with a recognised
   locator label or abbreviation (from the style's `aliases` table). This
   avoids splitting `"1, 3, 5"` into multiple locators.
3. For each segment: strip the leading label (longest alias wins), remainder
   is the value. A bare number with no label defaults to `Page`.
4. 0 segments → `None`. 1 segment → `Single`. ≥2 → `Compound`.

**Important limitation:** the prose path always produces `LocatorValue::Text`.
`Explicit { plural }` is unreachable from prose input — there is no syntax to
override the plurality heuristic.

### Input surface 3 — forward-compat / CSL-JSON bridge

`crates/citum-engine/src/api/forward_compat.rs` translates legacy CSL-JSON
`locator: String` + `label: String` fields into `CitationLocator::Single`. This
path also produces `LocatorValue::Text`.

### Plurality detection and the heuristic

`LocatorValue::is_plural` (`citation.rs` ~401):

```rust
match self {
    Text(s) => s.contains('–') || s.contains('-') || s.contains(',') || s.contains('&'),
    Explicit { plural, .. } => *plural,
}
```

The heuristic answers "does this value reference more than one unit?" which
controls term selection (`"p."` vs `"pp."`). The characters it watches for are:

| Character | Intended meaning |
|-----------|------------------|
| `-` or `–` | Range (`42–45` → plural) |
| `,` | Discontinuous list (`1, 3, 5`) |
| `&` | Paired items (`A & B`) |

**Known false positive:** a hyphen in a non-numeric identifier (`"figure A-3"`,
`"sec. 3-2"`) triggers plural even though the value denotes a single item. The
`Explicit` variant exists to override this for structured input, but has no
equivalent on the prose path.

**Two separate checks:** the engine's `apply_range_format`
(`crates/citum-engine/src/values/locator.rs`) detects ranges for formatting
purposes by scanning for `-`/`–`/`—`. This is *not* the same code as
`is_plural`. The two checks can agree or disagree independently.

---

## Historical note: compact map syntax (reverted)

Bean csl26-j007 originally specified a compact YAML map form:

```yaml
# Compact map — NOT currently implemented
locator:
  page: "23"
  line: "13"
```

This was implemented (`IndexMap`-backed `LocatorsInput`) but subsequently
reverted during the cite-site compound-grouping refactor (commits `8e38a8ca`,
`74920ed0`). There are no remaining references to `LocatorsInput` or the map
form in the codebase as of 2026-06-02.

See **Option B** below for the tradeoffs of reviving it.

---

## Decisions and options

### A — Plural-detection heuristic — **decided: A1**

**Decision:** tighten the heuristic so a hyphen/en-dash is only treated as a
range indicator when it sits between digit-or-roman-numeral sequences:

```text
is_plural ≡ value matches /\d\s*[–-]\s*\d/
          ∨ value contains ','
          ∨ value contains '&'
```

This keeps `"42–45"`, `"1, 3, 5"`, and `"A & B"` plural while making
`"figure A-3"` and `"sec. 3-2"` singular without any author override. No
schema change; the `Explicit { plural }` override is still available for
edge cases this regex doesn't cover.

*Known gap:* all-letter roman-numeral ranges (`"xii–xv"`) would read as
singular under this rule. If that proves a real-world problem, extend the
pattern to cover `[a-z]+\s*[–-]\s*[a-z]+` runs — but that risks being
over-broad, so defer until a concrete case surfaces.

*Deferred options (no decision):*
- **A2** — unify `is_plural` and the engine's `apply_range_format` so the
  two checks cannot disagree. Crosses a crate boundary; revisit if a
  real inconsistency is found.
- **A3** — prose-path override syntax (`[@key, fig. A-3!s]`), since
  `Explicit { plural }` is unreachable from Djot/Pandoc today. Large enough
  for its own bean.

---

### B — Compact map syntax — **decided: B0 (leave reverted)**

The compact map form (`page: "23"` / `line: "13"`) that was prototyped in
csl26-j007 remains reverted. The current `{label, value}` single and
`{segments: […]}` compound forms are the only structured input. Costs of
reviving it (schema ambiguity, no place for `Explicit`, ordering fragility)
outweigh the hand-authoring convenience.

---

### C — Single-to-compound authoring friction (open)

Adding a second locator segment to a citation currently requires restructuring
the YAML, not just appending an element. Concretely:

```yaml
# Single locator — a flat object
locator:
  label: page
  value: "23"

# Compound locator — must switch to a completely different shape
locator:
  segments:
    - label: chapter
      value: "3"
    - label: page
      value: "23"
```

There is no "just add another item" path. `segments` with a single element is
*rejected* by `CitationLocator::compound` (enforces ≥2), so you cannot use the
list form as a uniform container regardless of count.

Two options if this friction warrants addressing:

**C1 — Accept a bare list as input.** Allow `locator:` to be a YAML sequence
as well as an object. A one-element list normalises to `Single`; two or more
become `Compound`. Authors then always write a list and just append:

```yaml
locator:
  - label: page
    value: "23"
# → going compound is just adding an element, no restructuring needed
locator:
  - label: chapter
    value: "3"
  - label: page
    value: "23"
```

*Cost:* a third variant in `CitationLocatorRepr` (currently `Single |
Compound`); slightly more complex deserialiser; the flat shorthand still works
in parallel.

**C2 — Normalise single-element `segments`.** Smaller change: accept
`segments: [{…}]` (one item) and silently coerce it to `Single` instead of
rejecting it. Does not help with the flat-object-vs-list authoring mismatch
but does remove the hard error on one-element `segments`.

*No decision yet.* File a bean if single-to-compound transitions prove
friction-heavy in practice.

---

## Implementation notes

- `LocatorValue` uses `#[serde(untagged)]`. The `Text` arm must come first so a
  bare string is never mistakenly interpreted as `Explicit`.
- `normalize_locator_text` delegates alias lookup to the caller-supplied
  `aliases: &[(String, LocatorType)]` slice (populated from the style's locale
  term table). It does not hard-code any alias strings.
- `split_locator_segments` splits on `,` only before a recognised label, so
  discontinuous page lists (`"1, 3, 5"`) are preserved as single values.

## Acceptance criteria

*This spec documents shipped behaviour. These criteria track spec accuracy, not
new implementation.*

- [x] Three input surfaces are documented with examples.
- [x] `CitationLocator` / `LocatorSegment` / `LocatorValue` model documented.
- [x] Heuristic documented with known false-positive example.
- [x] Prose-path limitation (no `Explicit` override) documented.
- [x] Compact-map history recorded.
- [x] Decisions recorded: A1 (tighten heuristic), B0 (compact map stays reverted).
- [x] Option C (single-to-compound friction) documented with concrete examples and options.
- [ ] Bean filed to implement A1 heuristic tightening in `LocatorValue::is_plural`.

## Changelog

- v1.1 (2026-06-02): Record decisions (A1, B0). Rewrite Option C with concrete
  YAML examples illustrating the single-to-compound restructuring friction;
  add C1 (bare list) and C2 (normalise single-element segments) as named options.
- v1.0 (2026-06-02): Initial version. Spec covers shipped behaviour as of
  commit `2757aa3c` and subsequent refactors through `74920ed0`.
