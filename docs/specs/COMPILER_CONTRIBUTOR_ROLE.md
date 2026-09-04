# Compiler Contributor Role Specification

**Status:** Draft
**Date:** 2026-09-04
**Supersedes:** None
**Related:** [`PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`](./PRIMARY_CONTRIBUTOR_SUBSTITUTION.md),
[`ROLE_SUBSTITUTE_FALLBACK.md`](./ROLE_SUBSTITUTE_FALLBACK.md), bean `csl26-shp4`,
bean `csl26-i7nz`

## Purpose

A reference whose only contributor is a compiler ("Austin, Tim, comp.") is
data librarians and archivists commonly enter this way: no separate author,
just a compiler credited on the title page. Chicago (and AMA, which shares
the same substitution config) is supposed to print that name as the
work's primary credit — "Austin, Tim, comp. 2003. *The Times Style and
Usage Guide*." — the same way it already does for an editor standing in
for a missing author. Today it doesn't: the name disappears entirely and
the title is promoted to the front of the entry, as if the reference had
no contributor at all. This spec defines what's needed to let a style
declare "compiler" as a promotable primary-contributor candidate, so that
gap can close without hand-waving the role into an existing one that
means something else.

## Scope

In scope: adding `compiler` as a selectable role in the template-facing
contributor-role vocabulary, threading it through to the existing
data-model `Compiler` role, and making it usable as a
`options.substitute.candidates`/`overrides` entry so a style can promote a
compiler-only reference's contributor into the primary slot. Also in
scope: the separate, narrower locale-term mismapping that makes an
explicit `compiler` role label render with the composer term when a style
does supply a label.

Out of scope: a general "custom role" escape hatch for the template
vocabulary (a much larger change than this one role); modifying
`options.substitute.role-substitute`, which by its own spec does not
cover primary-author-slot substitution (see Design, "Why not
`role-substitute`" below) and is not the right mechanism for this gap;
changing which roles get an automatic label suffix (`ROLE_LABEL_DEFAULTS.md`'s
concern, not this one).

## Design

### The gap, precisely

Three separate places already know about "compiler" correctly:

- The reference-side data model already has a proper
  `ContributorRole::Compiler` variant
  (`crates/citum-schema-data/src/reference/contributor.rs:210`), and
  CSL-legacy ingestion already classifies a `note: "compiler: ..."`-tagged
  contributor into it.
- `crates/citum-engine/src/values/contributor/mod.rs:101` and
  `crates/citum-engine/src/values/contributor/substitute.rs:176` both
  already map the string `"compiler"` to the right data role when they
  see it.

What's missing is the template-facing vocabulary a style author writes.
`citum_schema::template::ContributorRole`
(`crates/citum-schema-style/src/template.rs:1101-1136`) is the fixed,
compile-time-exhaustive enum that both `contributor: <role>` selectors and
`options.substitute.candidates`/`overrides` draw from — see
`SubstituteContributor { contributor: ContributorRoles }`
(`crates/citum-schema-style/src/options/substitute.rs:362-365`), where
`ContributorRoles` wraps this same enum
(`crates/citum-schema-style/src/template.rs:876-885`). It has no
`Compiler` case. A style cannot write `compiler` in a `candidates` or
`overrides` list today; the schema has no slot for it. That's the actual
defect, not a mapping bug — the mapping bug (below) is real but
independent and much smaller.

### Why not `role-substitute`

`options.substitute.role-substitute` (`ROLE_SUBSTITUTE_FALLBACK.md`,
Active) already accepts arbitrary role strings, including "compiler",
with no enum involved, and already resolves them correctly through
`ResolvedRole::Custom` — this was the first place worth checking.
But that spec's own Scope section excludes primary-author-slot
substitution by name, deferring it to this mechanism instead:
`PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`'s `candidates`/`overrides` chain,
which is what decides whether a compiler-only reference gets a primary
contributor at all. `role-substitute` fills in a named role's own
contributor from a related role (e.g. a missing chapter-author from a
container-author); it doesn't promote a role into the primary slot the
way a missing author needs. Reusing it here would work by accident for
some cases and silently not for others, and would contradict its own
documented scope.

### The fix

1. Add a `Compiler` variant to `template::ContributorRole`:
   ```rust
   Compiler = "compiler",
   ```
   This is an additive, non-breaking enum change (existing style YAML is
   unaffected; only new documents can use the new value).
2. Extend `contributor_role_to_reference_role`
   (`crates/citum-engine/src/values/contributor/mod.rs:45`) with the new
   arm, mapping to the already-existing
   `citum_schema::reference::ContributorRole::Compiler`. This match is
   compile-time exhaustive, so the compiler enforces this step — it
   cannot be forgotten.
3. Add `compiler` to Chicago's (and AMA's, which shares config)
   `options.substitute.candidates` — most naturally alongside `editor`
   and `translator`, the two other missing-author stand-ins those styles
   already declare — so a compiler-only reference resolves to a primary
   contributor the same way an editor-only one already does.
4. Separately, fix `parse_role_name`
   (`crates/citum-schema-style/src/locale/raw_conversion.rs:395`), which
   maps the CSL-legacy locale-term key `"compiler"` to
   `ContributorRole::Composer`. This only affects the display *label* a
   style shows next to a compiler's name when one is configured (e.g.
   "comp." vs "comp." — worth checking what the composer term currently
   renders as, since this may already look accidentally right or
   noticeably wrong depending on locale content) — it does not affect
   which contributor gets selected, which is items 1-3's job. Land this
   in the same PR since it's a one-line, low-risk correction, not a
   separate spec-gated change.

### Worked example

Chicago 18th, `options.substitute`, after this change:

```yaml
options:
  substitute:
    candidates: [editor, compiler, title, translator]
```

`Austin, Tim, comp.` — a reference with a `compiler` contributor and no
`author` — now resolves through step 3 of `PRIMARY_CONTRIBUTOR_SUBSTITUTION.md`'s
existing resolution order (`candidates`, after the semantic-author check
fails) to the compiler, and renders:

> Austin, Tim, comp. 2003. *The Times Style and Usage Guide*. Third
> edition. London: Times Books.

instead of today's:

> *The Times Style and Usage Guide*. 2003. Third edition. London: Times
> Books.

## Implementation Notes

- `just schema-gen` must run in the implementing commit (this crosses
  `citum-schema-style`'s public schema surface — see root `CLAUDE.md`).
- Exhaustive-match sites to update, found via the compiler once the
  variant is added (do not hunt for these manually — let `cargo build`
  find them): `contributor_role_to_reference_role`
  (confirmed above), and any other `match role { ... }` over
  `template::ContributorRole` the build surfaces.
- No change needed to `role_substitute`, `SubstituteField`, or the
  data-model `ContributorRole` enum — all three already either don't
  apply here or already have the variant.
- Versioning: additive public enum variant, `feat:` scope `schema` per
  the project's Minor-bump convention.

## Acceptance Criteria

- [ ] `template::ContributorRole::Compiler` exists and round-trips through
      style YAML (`contributor: compiler` and as a `candidates`/`overrides`
      list entry).
- [ ] `contributor_role_to_reference_role` maps it to
      `citum_schema::reference::ContributorRole::Compiler`.
- [ ] Chicago 18th and AMA's `options.substitute.candidates` include
      `compiler`; the `6188419/CRTE2HQ7` / `6188419/Q9GCH7RF`-shaped
      fixture (compiler-only reference) renders with the compiler as
      primary contributor, matching oracle.
- [ ] `parse_role_name`'s locale-term mapping is corrected to
      `ContributorRole::Compiler`; a style-supplied compiler label term
      renders, rather than the composer term.
- [ ] `just schema-gen` output (schemas, data-model reference docs) is
      regenerated in the same commit.
- [ ] Full-portfolio `report-core.js --all-features` shows zero
      regressions.

## Changelog
- 2026-09-04: Initial draft.
