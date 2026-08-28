# Citation/Bibliography Option Split Specification

**Status:** Active
**Version:** 1.0
**Date:** 2026-03-25
**Supersedes:** None
**Related:** `csl26-8eom`, `docs/specs/TEMPLATE_V2.md`

## Purpose

Specify a strict schema split between shared, citation-local, and
bibliography-local option scopes. Today top-level `Style.options`,
`CitationSpec.options`, and `BibliographySpec.options` all accept shapes that
blur those boundaries. That lets bibliography-only settings parse in places
where they do not make semantic sense. The goal is to make scope boundaries
explicit in the public schema, reject invalid YAML at parse time, and keep the
runtime model aligned with those scope boundaries.

## Scope

In scope:

- new public schema types for citation-local and bibliography-local options
- changing `CitationSpec.options` and `BibliographySpec.options` to use those
  types
- strict parse-time rejection of out-of-scope option fields
- runtime helpers that merge context-local option types back into effective
  `Config` values
- schema regeneration, tests, and author-facing documentation updates

Out of scope:

- narrowing the top-level `Style.options` surface to shared fields only
- adding compatibility aliases or warning-only fallback paths
- changing rendering semantics for already-valid styles, except for removing the
  invalid top-level bibliography-only config path
- changing template selection or template component semantics

## Design

### Public schema split

Keep top-level style options limited to shared defaults:

```yaml
options:
  processing: author-date
  contributors:
    shorten: {min: 3, use-first: 1}
```

Top-level `Style.options` is the shared default layer for the whole style. It
must not contain bibliography-only entry controls.

Split nested option scopes:

```yaml
citation:
  options:
    contributors:
      shorten: {min: 3, use-first: 1}

bibliography:
  options:
    entry-suffix: "."
    separator: ". "
```

These nested blocks are local override layers:

- `citation.options` may override only citation-relevant fields
- `bibliography.options` may override only bibliography-relevant fields

Rust surface:

- `Style.options: Option<Config>` remains as the shared/default config surface
  for cross-context settings only
- `CitationSpec.options: Option<CitationOptions>`
- `BibliographySpec.options: Option<BibliographyOptions>`

### CitationOptions

`CitationOptions` contains the fields allowed when options are attached locally
to a `CitationSpec`. Shared fields may still be configured globally under
`Style.options`; the restriction is only on the citation-local override shape:

- `substitute`
- `processing`
- `localize`
- `multilingual`
- `contributors`
- `dates`
- `titles`
- `locators`
- `links`
- `punctuation_in_quote`
- `volume_pages_delimiter`
- `strip_periods`
- `notes`
- `integral_names`
- `custom`

It explicitly excludes the nested `bibliography` subtree from citation-local
overrides.

### BibliographyOptions

`BibliographyOptions` contains the fields allowed when options are attached
locally to a `BibliographySpec`. Shared fields may still be configured globally
under `Style.options`. Bibliography-only controls are local-only and must not
be configured under `Style.options`:

- `substitute`
- `processing`
- `localize`
- `multilingual`
- `contributors`
- `dates`
- `titles`
- `links`
- `punctuation_in_quote`
- `volume_pages_delimiter`
- `strip_periods`
- bibliography-entry controls currently modeled in `BibliographyConfig`
- `custom`

The bibliography-entry controls move to this local scope directly rather than
staying hidden behind an extra nested `bibliography` subtree.

`BibliographyOptions` excludes citation-only fields that are not meaningful in
bibliography rendering, especially:

- `locators`
- `notes`
- `integral_names`

### Compatibility decision

This change is intentionally strict and breaking.

Invalid shapes that parse today must fail after the change, including:

```yaml
options:
  bibliography:
    entry-suffix: "."
```

```yaml
citation:
  options:
    bibliography:
      entry-suffix: "."
```

and bibliography-local citation-only fields such as:

```yaml
bibliography:
  options:
    locators:
      form: short
```

No compatibility deserializer, normalization pass, or warning-only acceptance
path is allowed in the first implementation.

### Runtime merge model

The runtime model must follow the same scope split as the schema.

Implementation model:

1. Start from the effective top-level `Config`, which now contains shared
   cross-context defaults only.
2. Apply `CitationOptions` via dedicated conversion or merge helpers to derive
   effective citation config.
3. Derive effective bibliography behavior from:
   - the shared top-level `Config`
   - the local `BibliographyOptions`
   - a bibliography-only runtime `BibliographyConfig`
4. Downstream bibliography code reads bibliography-only behavior from that
   bibliography-specific runtime config, not from top-level `Config`.

This keeps scope and precedence explicit in both YAML and runtime code.

### Schema bump

This is a major schema bump because styles that currently parse with
out-of-scope nested option fields will now fail deserialization and JSON Schema
validation.

## Implementation Notes

- Keep the merge logic centralized near `options::Config`.
- Prefer explicit field mapping helpers over clever generic conversion macros if
  the generic path would obscure which fields are intentionally excluded.
- Preserve doc comments on all new public items.
- Update `docs/specs/TEMPLATE_V2.md` to mark the follow-on complete rather than
  deferred.
- Update the style author guide with one citation example and one bibliography
  example using the new nested shapes.

## Acceptance Criteria

- [ ] `CitationSpec.options` rejects bibliography-only configuration at parse
      time.
- [ ] `BibliographySpec.options` rejects citation-only configuration at parse
      time.
- [ ] Top-level `Style.options` rejects bibliography-only configuration.
- [ ] Runtime consumers derive citation config from shared `Config` plus
      `CitationOptions`, and bibliography behavior from shared `Config` plus
      `BibliographyOptions`.
- [ ] Schema artifacts and author-facing docs reflect the split.

## Changelog

- v1.0 (2026-03-25): Initial draft.
- v1.1 (2026-03-25): Activated with strict nested option schemas, runtime
  merge helpers, tests, and schema/documentation updates.
