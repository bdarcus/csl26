# Style fallback policy specification

**Status:** Draft
**Version:** 2.0
**Date:** 2026-08-16
**Supersedes:** [`DATE_SUBSTITUTE.md`](./DATE_SUBSTITUTE.md) when activated
**Related:** bean `csl26-62kp`, [`PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`](./PRIMARY_CONTRIBUTOR_SUBSTITUTION.md), [`DISAMBIGUATION.md`](./DISAMBIGUATION.md)

## Purpose

Define missing-author and missing-date behavior as style options. Rendering
templates select values and presentation only; they do not contain fallback
chains. The contract uses `substitute` for semantic values promoted into the
author position and `date-fallback` for output used when an issued date is
absent.

## Scope

This specification covers processing defaults, author substitution, terminal
anonymous messages, date fallback candidates, type selectors, occurrence
lanes, inheritance, scoped options, rendering, sorting, disambiguation, and CSL
migration.

It does not add general template conditionals or arbitrary template components
to options. Date candidates remain limited to date variables and locale
messages. Author `otherwise` remains limited to one locale message.

## Design

### Template boundary

`TemplateContributor` and `TemplateDate` have no `fallback` field. A missing
value is silent unless the effective options select another value or message.
The engine resolves fallback policy while traversing the effective template; it
does not inject options back into template components.

### Author substitution

The author policy uses `substitute`:

```yaml
options:
  substitute:
    candidates: [editor, translator]
    overrides:
      episode:
      - contributor: [writer, director]
    otherwise:
      message: term.anonymous
      form: short
```

`candidates` replaces the former `template` name. Candidates retain their
existing order and type-specific `overrides` retain their existing precedence.
`otherwise` is a singular locale-message candidate rendered only after the
effective-primary chain is empty.

The author-date processing family supplies `[editor, title, translator]` when
the style does not configure `substitute`. This includes all four named
author-date modes, custom processing based on one of those modes, and omitted
processing because the effective processing default is author-date. Numeric,
note, and label processing supply no author candidates.

`substitute: none` disables the processing default and every inherited
substitution field. Within an explicit block, `none` clears inherited
`candidates`, `otherwise`, or a type override. Candidate lists must be
non-empty; authors use `none`, not `[]`, to express an intentional clear.

Rendering, sorting, and disambiguation consume one resolved author policy. An
`otherwise` message provides the rendered and disambiguation identity for an
anonymous work. Author sorting keeps the established title-key fallback when
no contributor or title candidate resolves.

### Date fallback

The date policy uses two issued-date occurrence lanes:

```yaml
options:
  date-fallback:
    first-issued:
      default: standard
      article-journal: none
      book:
      - date: copyright
        form: year
    later-issued:
      default: none
      manuscript:
      - date: accessed
        form: year-month-day
```

`first-issued` applies to the first `date: issued` component found in recursive
authored order. `later-issued` applies to every subsequent issued component.
The count is taken after style inheritance, type-variant selection, and locale
selection. Missing non-issued date variables remain silent.

The whole policy accepts `standard`, `gb-t-7714-2025`,
`gb-t-7714-2025-author-date`, `none`, or an explicit lane map. Each lane accepts
`none` or an insertion-ordered `TypeSelector` map. A selector value accepts:

- `standard`, which expands to `message: term.no-date, form: short`;
- `none`, which selects a blank result and stops resolution; or
- a non-empty ordered list of date or locale-message candidates.

Selectors use first matching non-`default` entry, followed by `default`.
Omitted policies, omitted lanes, and unmatched selectors render blank. Empty
candidate lists are invalid.

Date candidates carry their own form, note suppression, and rendering fields.
Message candidates carry their locale message ID, term form, and rendering
fields. The old `dates.no-date-form` option is removed; styles select long or
short text on the message candidate.

### Inheritance and scoped options

The engine resolves the processing-family author baseline first, followed by
global options and then citation or bibliography options. Explicit values
override inherited values field by field. `none` is a retained clear marker,
not an empty configuration, so it cannot merge as a no-op.

Date lane maps merge per selector. Replacing a selector replaces its complete
rule without changing its insertion position; new selectors append in authored
order. A lane-level `none` clears the inherited lane, and a whole-policy `none`
clears both lanes.

### Disambiguation

The first-issued resolver supplies both visible rendering and the date
collision discriminant. A no-date message participates in year-suffix
disambiguation. An accessed-date fallback may render but does not become work
identity. If a configured blank result still belongs to a collision group, the
existing standalone year-suffix behavior remains in effect.

### Migration and errors

The CSL migrator emits no policy for a bare issued date. It converts supported
no-date branches and alternative-date chains into the appropriate date lane.
It converts a supported terminal anonymous macro into `substitute.otherwise`.
Conditional shapes that the closed policy cannot represent produce an explicit
unsupported-migration diagnostic.

The removed spellings have no compatibility aliases. Style loading reports
targeted errors for `date-substitute`, template `fallback`,
`substitute.template`, and `dates.no-date-form`, naming the replacement or the
new blank default.

## Implementation notes

Keep explicit disabled states in the typed schema until inheritance and scope
cascading finish. Eagerly converting `none` to an empty map or empty candidate
list would make a clear indistinguishable from omission.

Public Rust items introduced or changed by this contract require doc comments.
Schema generation must include the new unions and exclude every removed key.

## Acceptance criteria

- [ ] Templates contain no contributor or date fallback fields.
- [ ] Processing-derived substitution and every `none` clear path are covered by schema and cascade tests.
- [ ] Missing dates are blank unless an effective date-fallback rule matches.
- [ ] First and later issued occurrences resolve independently without template mutation.
- [ ] Rendering, sorting, and disambiguation consume the same effective author policy.
- [ ] Rendering and disambiguation consume the same first-issued date policy.
- [ ] The migrator emits supported author/date policies and rejects unsupported shapes explicitly.
- [ ] Every tracked Citum style validates under the new schema with source-authoritative behavior.
- [ ] The style author guide documents defaults, clears, scopes, and semantic consequences.

## Changelog

- v2.0 (2026-08-16): Drafted the options-only author substitution and date fallback contract.
