# Primary Contributor Substitution Specification

**Status:** Active
**Version:** 2.0
**Date:** 2026-08-16
**Supersedes:** None
**Related:** PR #1052,
[`CROSS_ROLE_CONTRIBUTOR_LISTS.md`](./CROSS_ROLE_CONTRIBUTOR_LISTS.md),
[`ROLE_SUBSTITUTE_FALLBACK.md`](./ROLE_SUBSTITUTE_FALLBACK.md),
[`DATE_FALLBACK.md`](./DATE_FALLBACK.md)

## Purpose

Define how a style promotes type-specific contributor roles into the primary
contributor slot while retaining the ordinary missing-author substitution
chain. Selection belongs in `options.substitute`; templates continue to render
`contributor: author`, and contributor presentation remains style-wide under
`options.contributors`.

## Scope

In scope: scalar and merged contributor substitution candidates, type-specific
overrides, default fallback order, exact native type matching, validation, and
the effective primary names used by rendering, sorting, and disambiguation.
Out of scope: automatic membership of a secondary contributor block, new role
taxonomy values, and media distinctions the reference schema cannot represent.

## Design

### Schema

`options.substitute.candidates` and `options.substitute.overrides` use the same
ordered candidate type. Existing scalar keys remain valid; contributor objects
add arbitrary scalar or ordered multi-role candidates:

```yaml
options:
  substitute:
    candidates: [editor, title, translator]
    overrides:
      episode:
        - contributor: [writer, director]
      film:
        - contributor: director
    otherwise:
      message: term.anonymous
      form: short

  contributors:
    role:
      defaults: apa
```

The author-date processing family supplies `editor`, `title`, then `translator`.
Numeric, note, and label processing supply no candidates. The chain does not
contain `author`: author resolution is the operation being supplemented, not a
substitution candidate. `otherwise` is a single closed locale-message value,
not an arbitrary template.

### Resolution

For a primary contributor component, resolve in this order:

1. Select the first non-empty candidate from the first matching type override.
2. If the override is absent or empty, resolve the established semantic author,
   including an explicit native `author` and existing compatibility fallbacks.
3. If no semantic author resolves, select the first non-empty candidate from
   `candidates`.
4. If the chain is exhausted, render `otherwise` when configured.

An override therefore expresses the style guide's authoritative primary credit
for that type. A merged candidate is non-empty when at least one declared role
has data; missing roles are omitted without invalidating the candidate. Exact
native reference discriminators are tested before legacy aliases. Candidate
lists are ordered and the first non-empty result wins.

`substitute: none` disables the complete inherited and processing-derived
policy. Field-level `none` clears inherited `candidates`, `otherwise`, or one
type override. Empty candidate lists are invalid.

Contributor candidates reuse the normal scalar or merged name pipeline,
including multilingual resolution, role suppression, same-person combination,
et-al selection, and role-label presentation. A title candidate retains the
existing title-substitution behavior.

Native subtype keys such as `episode` and `film` are checked before their CSL
compatibility aliases (`broadcast` and `motion-picture`). This permits an
authored native override to coexist with a broader migrated compatibility rule.

### Presentation boundary

Substitution selects people; it does not specify how their roles appear.
`options.contributors.role` supplies style-wide label presentation and
`options.contributors.merge` supplies style-wide merged-list defaults.
Component `label` and `merge` values remain higher-precedence exceptions.

### Semantic consumers

Rendering, bibliography sorting, citation sorting, and name-based
disambiguation must ask one effective-primary resolver for the selected names.
They must not independently inspect templates or repeat type-selection logic.

## Implementation Notes

- Keep scalar `SubstituteKey` YAML source-compatible through an untagged
  candidate representation.
- Validate contributor role lists with the same distinct-role rules as merged
  template components.
- Reject invalid substitution candidates while loading the complete style,
  even if the containing type override is not selected for the current item.

## Acceptance Criteria

- [x] Scalar `candidates` and `overrides` values round-trip unchanged.
- [x] Scalar and merged contributor candidates round-trip and appear in the
      generated style schema.
- [x] Exact-type overrides win when non-empty; semantic author and the default
      candidate chain remain ordered fallbacks.
- [x] Partially populated merged candidates render only available roles.
- [x] Rendering, sorting, and disambiguation share effective-primary names.
- [x] Invalid candidates fail style loading and CLI rendering before output.
- [x] Processing defaults, terminal messages, and explicit clear markers are
      resolved once for every semantic consumer.

## Changelog

- v1.0 (2026-07-14): Defined type-aware primary contributor substitution.
- v2.0 (2026-08-16): Renamed `template` to `candidates`, added processing
  defaults, terminal `otherwise`, and explicit `none` clears.
