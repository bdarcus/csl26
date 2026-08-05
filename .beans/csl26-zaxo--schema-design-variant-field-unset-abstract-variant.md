---
# csl26-zaxo
title: 'Schema design: variant field unset, abstract variants, locale vocab override'
status: in-progress
type: feature
priority: high
tags:
    - schema
    - style
    - engine
created_at: 2026-08-05T12:35:20Z
updated_at: 2026-08-05T14:06:27Z
parent: csl26-ccdt
---

Docs-only design PR covering three schema gaps found while measuring ieee exact parity (88/149). Review gate before any implementation.

G1 -- `modify` cannot unset an inherited rendering field. `Rendering::merge` goes through `merge_options!` (citum-schema-style/src/macros.rs:109), which assigns only when the source is Some. Dropping the base template's `wrap: quotes` in a type-variant is unexpressible, so authors must `remove` + `add` and restate component position. Direct cause of ieee.yaml's copy-pasted variants (personal_communication is byte-identical to the base template; broadcast/dataset/report are identical to each other).

G2 -- no name for an intermediate variant. `resolve_variant_parent_template` (template/resolution.rs:213) resolves `extends` via `original.contains_key`, so a parent must be a concrete reference type. entry-encyclopedia currently says `extends: broadcast`, which is semantically arbitrary.

G3 -- `LocaleOverride` carries no `vocab`. types.rs:748, raw.rs:596, raw_conversion.rs:795 handle messages/grammar-options/legacy-term-aliases/dates but not vocab.genre or vocab.medium. Blocks a per-style genre vocabulary (IEEE's published examples use "Ph.D. dissertation", not en-US's generic "PhD thesis").

Scope: docs only. No Rust, no schema regen. Implementation lands after review, stacked.

## Todo

- [x] TEMPLATE_V3.md section for G1 + G2 (syntax, resolution semantics, JSON-schema shape, migrate impact, what stays forbidden)
- [x] G3 spec placement decided and written
- [x] Worked ieee.yaml before/after showing the duplication collapse
- [x] Resolve the three open syntax questions rather than assume
- [x] Doc link check
- [ ] CI green

## Decisions taken (for review)

- **G1** -> `unset: [wrap, prefix]` on a modify op. Rejected explicit YAML `null`:
  serde cannot tell an absent key from an explicit null on `Option<T>` without
  lifting every `Rendering` field to `Option<Option<T>>` or hand-writing a
  deserializer for the most-touched struct in the schema. `unset` is additive
  and leaves `Rendering` untouched. Spec: TEMPLATE_V3.md 5.1.
- **G2** -> sibling `abstract-variants:` map. Rejected a reserved sigil inside
  the `type-variants` key space: it makes `TypeSelector` mean two things by
  leading character and needs a carve-out in `matches`, `unknown_type_names`,
  validation, lint, and the JSON schema. Spec: TEMPLATE_V3.md 5.2.
- **G3** -> `vocab` on `LocaleOverride`, merged key-by-key like `messages` and
  `dates` (not whole-replace like `grammar-options`). Spec: LOCALE_MESSAGES.md 4.1.

Also specified: `citum-migrate` should *not* start emitting the new forms on
this change -- converter output is compared against a pinned corpus, so a shape
change would move migration-fidelity numbers for reasons unrelated to converter
quality.

## Outcome (2026-08-05): G1 and G2 withdrawn

Review pushed back on scoping `unset` to type-variants, and suggested the rule belonged in the title preset. That was correct, and checking it dismantled both proposals.

`ieee.yaml` set `titles: humanities` (component titles plain), then compensated with `wrap: quotes` on the base `title: primary`, then needed six variants to undo it per type. Moving the policy to `options.titles` (PR #1143) removed 170 lines at **identical exact parity** (95/149), and the remaining variants became pure `modify` diffs -- nothing left to clear.

Corpus measurement: across all 32 embedded styles, 155 full-replacement type-variants, of which 148 are structurally different templates. Only 6 differ by rendering alone, and every field needing to be *cleared* was `title.wrap` -- 5 occurrences, all ieee, all this same workaround. Post-fix demand for `unset` is zero. G2 was only ever a readability win over multi-type keys.

**G3 (`LocaleOverride.vocab`) stands** -- independently motivated, untouched by this.

Replaced with: the structure-vs-policy layering rule, the `all` selector with specificity-based precedence, and the presets-are-not-macros articulation the review asked for.
