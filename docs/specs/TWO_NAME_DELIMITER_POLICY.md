# Two-Name Delimiter Policy Specification

**Status:** Active
**Version:** 1.0
**Date:** 2026-08-12
**Supersedes:** None
**Related:** bean `csl26-fqz2`; `docs/adjudication/DIVERGENCE_REGISTER.md` div-013

## Purpose

Define how a style controls the delimiter before the conjunction in a
two-contributor name list without repeating policy on template components.

## Scope

This specification covers contributor-option schema, option inheritance, and
name-list rendering for exactly two contributors. It does not change delimiter
realization, conjunction localization, et-al delimiters, or CSL migration.

## Design

`ContributorConfig` exposes an optional `two-name-delimiter-policy` field:

```yaml
contributors:
  delimiter-precedes-last: always
  two-name-delimiter-policy: suppress-in-citation-or-given-first
```

The field accepts two values:

- `follow-rule` applies `delimiter-precedes-last` literally and is the default.
- `suppress-in-citation-or-given-first` omits the delimiter for exactly two
  names when rendering a citation or a given-first name list. In all other
  cases it follows `delimiter-precedes-last`.

The policy participates in the existing global, citation, and bibliography
contributor-option cascade. An explicitly scoped `follow-rule` can therefore
override a global suppression policy.

The underlying delimiter rule has these semantics:

| Rule | Two names | Three or more names |
|---|---|---|
| `always` | delimiter | delimiter |
| `contextual` | no delimiter | delimiter |
| `never` | no delimiter | no delimiter |
| `after-inverted-name` | existing inversion rule | existing inversion rule |

The suppression policy is a Citum style option. CSL migration does not infer it
because CSL defines `delimiter-precedes-last` without this contextual exception.

## Implementation Notes

Centralize delimiter-rule evaluation in contributor name joining, then apply
the optional two-name suppression. APA 7th declares the suppression policy once
in its global contributor options.

## Acceptance Criteria

- [x] Unset policy and `follow-rule` both honor `delimiter-precedes-last`
  literally for two names.
- [x] The suppression policy affects only two-name citations and given-first
  lists.
- [x] Global, citation, and bibliography contributor-option overrides resolve
  through the existing cascade.
- [x] APA 7th citation and bibliography fidelity do not regress.
- [x] Generated schemas expose the new option and enum values.

## Changelog

- v1.0 (2026-08-12): Initial specification.
