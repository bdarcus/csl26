# Style Taxonomy

**Status:** Active
**Version:** 1.5
**Date:** 2026-04-22
**Bean:** `csl26-v961`, `csl26-nrkn`
**Related:** `STYLE_PRESET_ARCHITECTURE.md`, `STYLE_EDITIONS_AND_FAMILIES.md`, `UNIFIED_SCOPED_OPTIONS.md`, `JOURNAL_PROFILE_TAXONOMY_AUDIT.md`, `../architecture/2026-04-22_JOURNAL_PROFILE_CANDIDATE_AUDIT.md`

## Purpose

Define Citum style taxonomy on two separate axes:

1. semantic class: what kind of style relationship the entry represents
2. implementation form: how that relationship is currently expressed in the repo

This replaces the older single mixed tier table, which overloaded aliases,
config-only wrappers, and structural journal descendants into one journal tier.

This document is the public taxonomy. It does **not** change the runtime
`RegistryEntry.kind` enum in this pass.

## Semantic Class

| Class | Definition | Authority requirement | Verification default |
|------|------------|-----------------------|----------------------|
| `base` | Inheritance root with its own complete rendering structure | none beyond normal style authority | own oracle |
| `profile` | Reusable publisher, society, or standards house style | guide-backed parent-plus-deltas relationship | own oracle plus parent-diff evidence |
| `journal` | Journal-specific descendant of a base or profile | journal or publisher guidance ties it to a parent family | delta from selected parent plus own oracle evidence |
| `independent` | Self-contained style with no evidenced parent relationship | no reusable parent proven | own oracle |

## Implementation Form

| Form | Definition | `extends:` | Local template-bearing fields | Typical use |
|------|------------|------------|-------------------------------|-------------|
| `alias` | Registry pointer only | no | no | exact journal clone |
| `config-wrapper` | Parent selected via `extends:` and tuned only with scoped options / metadata | yes | no | thin publisher or journal wrapper |
| `structural-wrapper` | Inherits a parent but still carries local template-bearing structure | yes | yes | journal descendant with real family match but unresolved structural delta |
| `standalone` | Complete self-contained style | optional but typically no | yes | independent or not-yet-factored style |

## Authority Rules

Authority order remains:

1. current publisher or journal guide
2. current publisher house rules or submission instructions
3. named parent-style manual or standards reference
4. CSL metadata and template links
5. current Citum YAML structure

Output similarity by itself is not enough.

### Profile Rule

A style is a `profile` only when both statements are true:

- semantically, it is a reusable publisher, society, or standards house style
- operationally, the public wrapper is `config-wrapper` only

Public `profile` styles may keep local identity and scoped options, but they may
not keep local templates, `type-variants`, or template-clearing `null` values.

### Journal Rule

`journal` is now semantic only. A journal style may be implemented as:

- an `alias`
- a `config-wrapper`
- a `structural-wrapper`

This is the key correction from the earlier mixed tier model.

## Current Classification

### Base + Standalone

| Style | Notes |
|------|-------|
| `apa-7th` | CSL-derived inheritance root |
| `chicago-notes-18th` | CSL-derived inheritance root |
| `chicago-author-date-18th` | CSL-derived inheritance root |
| `ieee` | CSL-derived inheritance root |
| `american-medical-association` | CSL-derived inheritance root |
| `modern-language-association` | CSL-derived inheritance root |

### Profile + Config-Wrapper

| Style | Notes |
|------|-------|
| `chicago-shortened-notes-bibliography` | public shortened-notes wrapper |
| `elsevier-harvard` | public profile handle over hidden family root |
| `elsevier-vancouver` | public profile handle over hidden family root |
| `elsevier-with-titles` | public profile handle over hidden family root |
| `springer-basic-author-date` | public profile handle over hidden family root |
| `springer-basic-brackets` | public profile handle over hidden family root |
| `springer-vancouver-brackets` | public profile handle over hidden family root |
| `taylor-and-francis-chicago-author-date` | public profile handle over hidden family root |
| `taylor-and-francis-council-of-science-editors-author-date` | public profile handle over hidden family root |
| `taylor-and-francis-national-library-of-medicine` | public profile handle over hidden family root |

### Journal + Alias

Journal aliases live in `registry/default.yaml` under `aliases:` and remain
zero-config pointers.

Representative examples:

| Journal Alias | Parent |
|--------------|--------|
| `acta-medica-portuguesa` | `american-medical-association` |
| `annals-of-the-association-of-american-geographers` | `taylor-and-francis-chicago-author-date` |
| `biochimica-et-biophysica-acta` | `elsevier-with-titles` |

### Journal + Structural-Wrapper

Representative existing journal descendants that already prove a journal style
is not necessarily an alias or thin wrapper:

| Style | Parent | Why structural |
|------|--------|----------------|
| `american-society-of-mechanical-engineers` | `ieee` | local bibliography templates and type variants |
| `american-mathematical-society-label` | `elsevier-with-titles` | local citation and bibliography templates |
| `entomological-society-of-america` | `elsevier-harvard` | local citation template and type variants |
| `international-journal-of-wildland-fire` | `springer-basic-author-date` | local citation template and bibliography structure |
| `elsevier-vancouver-author-date` | `elsevier-vancouver` | local author-date citation and bibliography structure |

### Audited Journal Descendants (2026-04-22)

The `9e13a17b` backlog was re-audited with normalized IDs, CSL metadata, the
alias TSV, current guide evidence, and the new `citum-analyze --identify-profiles`
audit mode. The reduction pass kept only the deltas justified by guide-backed or
converging evidence.

| Legacy Style | Proposed Parent in `9e13a17b` | Corrected Parent | Outcome |
|-------------|--------------------------------|------------------|---------|
| `pharmacoepidemiology-and-drug-safety` | `elsevier-with-titles` | `american-medical-association` | `journal + config-wrapper` |
| `disability-and-rehabilitation` | `elsevier-with-titles` | `elsevier-with-titles` | `journal + config-wrapper` |
| `zoological-journal-of-the-linnean-society` | `springer-basic-author-date` | — | `false-positive` |
| `the-lichenologist` | `springer-basic-author-date` | — | `false-positive` |
| `memorias-do-instituto-oswaldo-cruz` | `springer-basic-author-date` | — | `independent + standalone` |
| `techniques-et-culture` | `taylor-and-francis-council-of-science-editors-author-date` | — | `false-positive` |
| `hawaii-international-conference-on-system-sciences-proceedings` | `taylor-and-francis-national-library-of-medicine` | — | `false-positive; temporary structural hold` |
| `cell-numeric` | `elsevier-with-titles` | `elsevier-with-titles` | `journal + config-wrapper` |

This pass promoted three candidates to `journal + config-wrapper`, retained no
new aliases, dropped one unsupported parent link outright, and left one
temporary structural hold where inherited IEEE behavior still covers legacy CSL
surface that the current guide no longer justifies semantically.

## Embedding And Runtime Note

This pass deliberately keeps the runtime `kind` enum unchanged.

- current embedded base/profile styles remain the same
- registry aliases still resolve through `registry/default.yaml`
- journal wrappers remain ordinary style files unless separately promoted

If machine-readable implementation-form metadata becomes necessary later, add a
new field rather than overloading `kind` again.

## Changelog

- v1.6 (2026-07-28): Cross-referenced
  [STYLE_INHERITANCE.md](./STYLE_INHERITANCE.md), which absorbs the Profile
  Rule into its form-decision table and adds the three-tier portfolio policy.
- v1.5 (2026-04-22): Reframed taxonomy on two axes: semantic class and
  implementation form. Recorded the 2026-04-22 journal-candidate audit,
  reduced three audited descendants to `journal + config-wrapper`, dropped one
  unsupported parent link to a standalone style, and explicitly separated
  aliases, config-only journal wrappers, and structural journal wrappers.
  Confirmed that runtime `kind` is unchanged in this pass.
- v1.4 (2026-04-22): Replaced `options.profile` with normal scoped options.
- v1.2 (2026-04-21): Enforced config-only profile wrappers over hidden compiled
  roots.
- v1.1 (2026-04-21): Clarified that `profile` means evidence-backed parentage,
  not output similarity or already-small YAML.
- v1.0 (2026-04-20): Initial spec. Defined the four-tier model (base, profile,
  journal, independent).
