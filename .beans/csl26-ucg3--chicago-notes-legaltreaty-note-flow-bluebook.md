---
# csl26-ucg3
title: Chicago notes legal/treaty note-flow (Bluebook)
status: todo
type: bug
priority: normal
created_at: 2026-06-21T11:49:07Z
updated_at: 2026-08-02T14:19:28Z
parent: csl26-h7oc
---

Chicago notes (`chicago-notes-18th.yaml`) legal/treaty note-flow defects found
while completing csl26-6qv3. These are **bucket B/D** (Bluebook-specialised,
flagged-not-guessed in `docs/architecture/audits/2026-06-20_STYLE_GUIDE_CONFORMANCE.md`)
and need a content/format decision before implementing — do **not** guess.

Observed in the rendered notes (CITATIONS, references-expanded.json):

- Leading comma before an author-less legal case: `, Brown v. Board of Education,
  347 U.S. Reports 483 (U.S. Supreme Court 1954).` — title substitutes for the
  absent author but a stray leading `, ` remains.
- Repeated treaty fields: `… U.S.T. 14 (1963): 1313, in U.S.T., vol. 14 (1963),
  U.S.T., 1313.` — volume/year/pages and the `U.S.T.` reporter render twice.
- Conference note double-year/pages: `"…Phrases" (2013): 3111–19, in
  _Proceedings of NIPS 2013_ (2013), 3111–19.` — year and pages rendered before
  AND after `in Proceedings`.

These touch the chicago-notes citation type-variants (deep note-flow) and the
legal-case/treaty Bluebook formatting. Confirm the intended Bluebook output with
the maintainer before changing.

Note: the chicago-notes *bibliography* is intentionally empty (notes-only style;
citeproc `chicago-notes.json` also emits `bibliography: []`) — not in scope here.

## Mechanical Diagnosis (2026-07-06 — content decisions still yours, per this bean's rule)

Read against crates/citum-schema-style/embedded/styles/chicago-notes-18th.yaml (extends chicago-18-base):

1. **Stray leading comma (Brown v. Board):** the `legal-case` note variant opens with `contributor: author` followed by `title: primary, prefix: \", \"`. US case references carry no author, so the author component renders empty and the title's component-local prefix is emitted with nothing before it. The note-flow path has no leading-affix trimming (the bibliography cleanup pass would have caught \" ,\"-style artifacts; notes flow does not run it). **Mechanical fix candidates:** (a) engine: suppress a component prefix when no visible output precedes it in the entry — general rule, benefits all note styles; (b) style: give the title `render-when: field-absent: author` twin components. (a) is the right locus if we confirm citeproc behaves that way; it belongs with the csl26-ztxq/zfqr punctuation family.
2. **Treaty double fields (U.S.T. 14 (1963): 1313, in U.S.T., vol. 14…):** there is **no `treaty:` type-variant** in chicago-notes-18th.yaml at all — treaty falls through to the default note template, which renders the container/volume/year/pages group, while reporter-style fields also render from the generic components. The fix is necessarily a new Bluebook treaty variant — pure content decision (Bluebook R21: name, parties, vol, source abbrev, page, date).
3. **Conference double year/pages:** the `paper-conference` variant is an `extends`/remove-diff; its removes do not cover the year+pages group the base renders *before* the 'in Proceedings' group, so both the component-level and container-level renderings survive. Needs either wider removes or a Full variant — decide alongside the intended Chicago 18 conference-paper note shape.

**Decisions needed from you before implementation:** intended Bluebook output strings for (2) treaty and (3) conference-paper notes; for (1), confirm citeproc suppresses the leading separator so the engine-level fix is fidelity-correct.

Repro unchanged: render CITATIONS / references-expanded.json against chicago-notes-18th.
