# Medium Designator Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-09-06
**Supersedes:** None
**Related:** `csl26-zs9y`,
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`

## Purpose

The NLM/Vancouver citation family marks references it has only ever seen
online with a bracketed `[Internet]` designator, an "Available from:" access
phrase, and a `[cited …]` bracket around the accessed date — three
components that all key off the same condition (a URL exists) and, in the
shipped CSL, off reference type (periodicals are excluded; a periodical's own
volume/issue/page locator already establishes it's a real, citable, located
work regardless of how it was accessed).

This is 29+10 = 39 embedded exact-parity rows across
`taylor-and-francis-national-library-of-medicine`,
`springer-vancouver-brackets`, and
`taylor-and-francis-council-of-science-editors-author-date`. It is not
covered by `docs/specs/ALTERNATIVES.md`: there is no fallback here (nothing
is tried and abandoned), it is a marker that appears *in addition to* the
title, not instead of anything.

## Scope

In scope:

- one bibliography option controlling the online-access marker bundle
  (title suffix, access phrase, cited-date bracket) for the vancouver/NLM
  style family;
- the type-class gate (periodical types excluded).

Out of scope:

- other families' access-date conventions (Chicago's own `csl26-fsp0`/`ADD`
  URL/DOI policy work is tracked separately under `csl26-h7oc`);
- a general "conditional literal" mechanism — this spec proposes a named
  option, not a reusable template conditional, per the same reasoning
  `ARTICLE_JOURNAL_NO_PAGE_FALLBACK.md` used for its narrower case.

## Evidence

`taylor-and-francis-national-library-of-medicine.csl:133-150` (`title` macro):

```xml
<macro name="title">
  <text variable="title"/>
  <choose>
    <if type="article-journal article-magazine chapter paper-conference article-newspaper" match="none">
      <choose>
        <if variable="URL">
          <text term="internet" prefix=" [" suffix="]" text-case="capitalize-first"/>
        </if>
      </choose>
      <text macro="edition" prefix=". "/>
    </if>
  </choose>
</macro>
```

`springer-vancouver-brackets.csl:113-120` (`accessed-date` macro):

```xml
<macro name="accessed-date">
  <choose>
    <if variable="URL">
      <group prefix="[" suffix="]" delimiter=" ">
        <text term="cited" text-case="lowercase"/>
        <date variable="accessed" form="text"/>
      </group>
    </if>
  </choose>
</macro>
```

Confirmed against 11 T&F-NLM oracle rows (webpage, dictionary, legislation,
map, standard, bill, hearing, software, regulation — all non-periodical) and
`docs/architecture/audits/2026-09-06_RENDER_WHEN_DISPOSITION.md`'s row counts
for `springer-vancouver-brackets` and
`taylor-and-francis-council-of-science-editors-author-date`.

`taylor-and-francis-council-of-science-editors-author-date.csl:77-83`
(`publisher-place` macro) has the same type-conditioned-fallback shape for
`[place unknown]` (7 rows) — that one is `ALTERNATIVES`-shaped
(`alternatives: [{variable: publisher-place}, {message: term.place-unknown}]`),
not a medium designator; it's listed here only because it appears in the same
oracle rows as the `[Internet]` gap and should not be conflated with it in
implementation.

## Design

### New option

```yaml
bibliography:
  options:
    online-access:
      medium-marker: term.internet
      access-phrase: term.available-from
      cited-date-form: text
      exclude-types: [article-journal, article-magazine, chapter, paper-conference, article-newspaper]
```

Proposed schema shape:

- `BibliographyConfig.online_access: Option<OnlineAccessConfig>`
- `OnlineAccessConfig`:
  - `medium_marker: Option<SubstituteMessage>` — locale message rendered
    bracketed after the title when the reference has a URL and its type is
    not in `exclude_types` (reuses the existing `SubstituteMessage` shape
    from `options/substitute.rs` rather than inventing a second message
    reference type).
  - `access_phrase: Option<SubstituteMessage>` — locale message prefixed
    before the rendered URL (`"Available from: "`).
  - `cited_date_form: Option<DateForm>` — form for the accessed-date bracket;
    `None` disables the bracket even when a URL exists.
  - `exclude_types: Vec<TypeSelector>` — reference types this bundle does not
    apply to (empty means "applies to every type").

This mirrors `ArticleJournalNoPageFallback`'s already-accepted shape
(a small, named, type-scoped bibliography option) rather than a general
conditional. Draft field names are reviewable; the design intent — one
option bundling the three co-varying pieces, gated by URL presence and
excluded types — is fixed.

### Semantics

When a reference has a URL and its type is not in `exclude_types`:

1. The title component appends ` [<medium_marker>]` (locale-message text,
   bracketed, capitalized-first — matching the shipped CSL's
   `text-case="capitalize-first"`).
2. The access/link rendering prefixes the URL with `<access_phrase>: `.
3. The accessed-date component, if present, wraps in `[<term.cited> …]`
   using `cited_date_form`.

When any precondition fails (no URL, excluded type, or the relevant option
field is `None`), that piece of the bundle renders nothing extra — matching
today's behavior for periodicals in this family (no marker) and for styles
that don't set this option at all (unaffected).

## Implementation Notes

Independent of the `render-when` disposition — this is a new domain-specific
bibliography option, not a candidate for `alternatives:` or `render-when`.

## Acceptance Criteria

- [ ] Schema: `OnlineAccessConfig` on `BibliographyConfig`, validated.
- [ ] Engine: title-suffix, access-phrase, and cited-date-bracket wiring,
      each independently gated.
- [ ] `taylor-and-francis-national-library-of-medicine-core.yaml`,
      `springer-vancouver-brackets-core.yaml`, and
      `taylor-and-francis-council-of-science-editors-author-date-core.yaml`
      updated; `report-core.js --diff` shows the ~39 targeted rows flip with
      0 regressions.
- [ ] `just schema-gen` run, schema docs updated.
- [ ] Status promoted to Active in the implementation commit.

## Changelog

- v1.0 (2026-09-06): Initial draft.
