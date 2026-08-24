# Substituted-Title Bibliography Formatting Specification

**Status:** Draft
**Version:** 1.2
**Date:** 2026-08-26
**Supersedes:** None
**Related:** bean `csl26-0u0f`; bean `csl26-0dca`; `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md`; `docs/adjudication/DIVERGENCE_REGISTER.md` div-011

## Purpose

`SUBSTITUTED_VALUE_FORMATTING.md` answers, for **citation** context, whether a
title promoted into the missing-author slot (`contributors.substitute`) keeps
its own formatting or takes the slot's — explicitly citation-scoped only
(§5). This spec answers the same question for **bibliography** context, for
bean `csl26-0u0f`.

**v1.0 of this spec covered quoting only. That was too narrow — see §1.** This
revision covers every title-formatting axis (quote, italic, and the
candidate-selection question of *which* value gets promoted at all), and
recommends a different mechanism as a result.

## Taxonomy

Every author-less `chicago-author-date-18th` bibliography row, classified by
what's actually wrong (methodology and reproduction command in §2):

| Class | Count | Example | Disposition |
|---|---|---|---|
| **Quote-gap** | 32 | `document`, `manuscript`, `article-journal` — oracle quotes the promoted title, Citum doesn't | **This spec designs a fix** (§4); realistic yield 22/32 — §4.3 |
| **Emphasis-gap** | 8 | `map`, `hearing`, `software`, `song`, `speech` — oracle italicizes, Citum doesn't | Same root cause as quote-gap (§1); `map`/`hearing` get the fix, `software`/`song`/`speech` need a separate style-content fix instead — §4.3, §5 |
| **Candidate-gap** | 5 | `article-magazine`/`article-newspaper` with both `title` and `container-title` — oracle promotes the container, Citum promotes the title | **Out of scope.** Different defect surface (`SubstituteField::Title` resolution), filed separately — §5 |
| Render-when bypass | 12 | `webpage` with no `title` — routes through a template branch with no `contributor: author` at all | Never reaches the substitute path; not this spec's concern regardless of formatting outcome |
| Already matching | 37 | — | No action |
| Unclassified | 9 | needs manual review (see script limits, §2) | Not yet triaged |

Rows sum to 103, the full author-less bibliography set. One of the five
candidate-gap rows (`6188419/92LLEIJT`) also carries a further complication —
a CMOS-14.102 "anonymous review" macro pattern, §5 — but it's one row, not an
additional count.

**40 of 103 author-less bibliography rows are quote-gap or emphasis-gap; 5 are
a real but different defect (candidate-gap, out of scope); the rest are out
of scope or already correct.** Not all 40 close from the mechanism this spec
designs, though — §4.3 hand-simulates it against the real style YAML and
finds the realistic yield is 22, not 40.

## 1. Why v1.0 only covered quoting

The bean's residual was sized using
`scripts/analyze-parity-residuals.js`'s `"B title quote boundary"` label:

```js
['B title quote boundary', (o, c, ops) => ops.some(({ a, b }) => /[""]/.test(a + b))],
```

This matches curly-quote characters in **exact-text (markup-stripped)** diff
ops. Italic markup (`<i>…</i>`, `_..._`) never survives that stripping into a
diff op — a pure italic-vs-plain mismatch produces **zero** ops and is
invisible to this classifier, and to every other label built on the same
exact-text diff. **The tool used to size the original bean is structurally
blind to every formatting axis except quoting.** v1.0 inherited that blind
spot from the bean rather than choosing it; direct comparison of markup-
preserving output (§2) is what surfaces the rest.

**Both axes share one root cause.** `document`/`manuscript`/`article-journal`
(need quote) and `map`/`graphic`/`classic`/`hearing` (need italic) all resolve
to the *same* `component` title category in
`chicago-author-date-18th.yaml`'s `titles.type-mapping`, which configures
neither `quote` nor `emph` — so `resolve_title_substitute`
(`crates/citum-engine/src/values/contributor/substitute.rs:612`) has nothing
to apply either way. More tellingly: `map`/`graphic`/`classic`/`hearing`'s own
bibliography template *already declares* `title: primary` with `emph: true`
at the node level (`chicago-author-date-18th.yaml:930`) for their normal,
non-substitute title rendering — but the substitute path's
`apply_substitute_title_emphasis` only ever looks up the **category**, never
the resolved template node, so it can't see an override the style already
wrote. **The substitute path can see category-level config and nothing
finer** — that ceiling is what produces both the quote-gap and the
emphasis-gap, not two unrelated problems.

## 2. Method and reproduction

```bash
cargo build --release --bin citum
node scripts/report-core.js --all-features --citum-bin target/release/citum \
    --style chicago-author-date-18th > /tmp/report.json
python3 scripts/audit-substitute-bibliography-formatting.py \
    --report /tmp/report.json \
    --fixture tests/fixtures/test-items-library/chicago-18th.json
```

`scripts/audit-substitute-bibliography-formatting.py` (committed alongside
this spec, companion to `scripts/audit-substitute-formatting.py`) compares
markup-preserving oracle/Citum output for every author-less bibliography row
and classifies each into the taxonomy above. It is a heuristic, not a
template parse — its docstring records the specific known false negatives
(a fixed-size "does this value lead the output" window misclassifies four
`article-magazine`/`article-newspaper` rows as quote-gap when they're really
candidate-gap; corrected by hand in the taxonomy table above, the same way
`SUBSTITUTED_VALUE_FORMATTING.md` §3 hand-corrects its own classifier's
short-macro contamination). Spot-check before trusting a row, as that spec's
own §2 already instructs for its citation-context counterpart.

Measured at commit `4a3c1cb1` (this branch, PR #1231's first commit), against
`styles-legacy` pinned at `ca545f945a676a4b6319ba386ef3adaccacf9df9` — same
revision `SUBSTITUTED_VALUE_FORMATTING.md` §2 pins.

## 3. Non-regression: APA / Elsevier-Harvard

Same measurement against `apa-7th` and `elsevier-harvard`: zero
bibliography-context rows of any class for either style. Neither currently
depends on today's never-format bibliography-substitute default, so neither
is at risk from whatever mechanism ships here — checked, not assumed.

## 4. Design

### 4.1 Why a per-type list (v1.0's recommendation) no longer works

v1.0 recommended an explicit per-type **quote** list. With the emphasis-gap
now in scope, that mechanism would need a second, parallel per-type
**emphasis** list — and a third for `strong`/`small-caps` should a future
style need those — each restating, for the substitute path only, information
the type's own resolved template already encodes. Three lists that must be
kept in sync with the templates forever is worse than one lookup that reads
the templates directly.

### 4.2 Recommendation: extend the merge precedence normal title rendering already uses

This is smaller than "a new mechanism." Normal (non-substitute) title
rendering already resolves formatting in two steps —
`effective_title_quote_depth` (`crates/citum-engine/src/values/title.rs:390–397`)
resolves the title's **category** first, then merges the template node's own
`Rendering` **over** it, node winning wherever it's explicitly set:

```rust
let mut rendering = get_title_category_rendering(&template.title, ..., &options.config)
    .unwrap_or_default();
rendering.merge(&template.rendering);
```

The substitute path (`resolve_title_substitute`,
`apply_substitute_title_emphasis`) never does this second step — it resolves
category and stops. **The fix is to give the substitute path the same
category-then-node-merge precedence the rest of the engine already has**, not
to invent a rule from scratch. This is why it covers quote, italic, and any
other axis with a single change instead of a list per axis: whatever the
node declares, the merge already knows how to apply.

`render_when`-guarded multi-node templates
(`chicago-author-date-18th.yaml:385–398`) — v1.0's objection to this
mechanism — resolve the same way: `render_when` evaluation is already a pure
function of `(reference, condition)` —
`crate::values::group_condition_matches(ctx.reference, condition)`
(`crates/citum-engine/src/processor/rendering/grouped/core.rs:1321`), the
exact call the normal render path already makes. Evaluate the same
conditions against the same reference and take the node that would actually
render — not new logic, a new call site for an existing function.

**Field whitelist.** Only formatting fields transfer —
`quote`/`wrap`/`emph`/`strong`/`small-caps`/`text-case` — never
`prefix`/`suffix`/`form`, which describe the node's position in its own
template, not a property of the value itself.

**Cost that's still real:** `RenderOptions`
(`crates/citum-engine/src/values/mod.rs:950`) carries no reference to the
resolved template today. This needs scoped plumbing — a lookup from
`(reference type, render context)` to "the `title: primary` node that would
render for this reference" — not a config read.

### 4.3 Realistic yield: hand-simulated against the real style

Before trusting "this closes the 40-row taxonomy," `scripts/audit-substitute-bibliography-formatting.py --simulate`
hand-applies each affected type's *actual, currently-shipped* `title: primary`
node formatting to its quote-gap/emphasis-gap rows and checks the result
against the oracle:

```bash
python3 scripts/audit-substitute-bibliography-formatting.py \
    --report /tmp/report.json \
    --fixture tests/fixtures/test-items-library/chicago-18th.json --simulate
```

| Type | Clean fix | Content gap remains | No effect |
|---|---|---|---|
| `document` | 21 | 9 | — |
| `manuscript` | 1 | — | — |
| `article-journal` | — | 1 | — |
| `map` | — | 1 | — |
| `hearing` | — | 1 | — |
| `speech` | — | 1 | — |
| `software` | — | — | 3 |
| `song` | — | — | 2 |
| **Total (40)** | **22** | **13** | **5** |

- **Clean fix (22):** the mechanism alone closes these — dominated by
  `document`, which has no dedicated type-variant and falls to the style's
  default template, whose `title: primary` already declares `wrap: quotes`.
- **Content gap remains (13):** the mechanism gets the formatting right, but
  the row still diverges from the oracle for an unrelated, pre-existing
  reason this spec doesn't touch — 7 of the 9 `document` rows here share one
  specific cause (a missing accessed-date clause); `map`/`hearing` are
  missing separate archival/legal-citation fields; `article-journal` is
  missing volume/series numbering.
- **No effect (5):** `software` and `song` each have their own dedicated
  bibliography template, and neither one's `title: primary` node declares
  `wrap` or `emph` at all — a real, non-substituted title of that type would
  render unformatted too. The mechanism reads the node faithfully; the node
  has nothing in it. This is a pre-existing gap in the type's own template
  content, not a substitute-path limitation, and is better fixed directly
  (a `/style-maintain`-sized YAML change adding `emph: true`), independent of
  this spec.
- **`speech`** deserves its own note: it has no dedicated type-variant either,
  so it falls to the same default template `document` uses — whose
  `wrap: quotes` is right for `document` but wrong for `speech` (oracle
  wants italic). The engine cannot tell these two cases apart; only a style
  author can, by giving `speech` its own type-variant. It belongs with
  `software`/`song` as a style-content fix, not a case for an engine-side
  guard — an engine guard that suppressed prediction for "types with no
  dedicated template" would also suppress `document`'s 21 clean fixes, which
  reach the right answer through exactly the same structural path.

### 4.4 Must be opt-in — node-level formatting can be positional, not intrinsic

Removing the `RenderContext::Citation` gate and reading the template node
unconditionally, for every embedded style, is tempting given §4.2 reframes
this as completing an existing precedence rather than adding a new one. It's
still rejected, on sharper grounds than v1.0 had:

**Node-level formatting can depend on what sits *beside* the node, not just
the type.** `apa-7th.yaml`'s `article-journal` bibliography template sets
`title: primary` to `emph: false`, immediately followed by
`title: parent-serial` at `emph: true` — the article title is deliberately
left plain *because its container-title sibling two lines later carries the
italics in this exact template*. That's not a demonstrated regression today:
`article-journal` maps to APA's `component` category, which itself sets no
`emph`, so category-only and node-merged both currently resolve to plain for
this type — no divergence in the fixture corpus as it exists. But a
substituted title promoted into the author slot has **no such sibling** —
copying a suppression that's only correct beside a container it won't be
standing next to is a real, structural hazard that this specific example
happens not to trigger, not one that's absent. Combined with `apa-7th`
having **zero** currently-exercised bibliography-substitute rows (§3 —
unexercised, not verified-safe) and `speech`'s confirmed misprediction
(§4.3) once a mode like this is live, an unconditional engine-wide switch is
not something this spec is prepared to ship without per-style verification.

This is the general shape of the risk: some node-level formatting is
**intrinsic** to the type (Chicago's `wrap: quotes` on short-work titles
holds regardless of what else is nearby — confirmed safe to copy into any
slot by §4.3's clean fixes) and some is **positional** (APA's `emph: false`
here is conditioned on its template neighbor, not a property of
`article-journal` titles in general). Nothing in the schema today
distinguishes the two. An explicit per-style opt-in *is* that distinction,
enforced by process rather than by a new schema axis: a style only opts in
once its author has checked that the types it's opting in for use intrinsic,
not positional, formatting — exactly the check §4.3 just did for Chicago by
hand. If a style later demonstrates it needs the schema to express the
distinction directly, that's an additive follow-on, not a redesign — nothing
here forecloses it.

Gate behind a new bibliography-scoped substitute mode. **Naming is an open
decision, flagged not resolved here:** the existing field this would extend,
`Substitute.title_quote: Option<SubstituteTitleQuoteMode>`, is quote-named
because it predates this spec; a mode that also governs italic and other
axes needs either a new, more accurately-named field (e.g.
`substitute.title-rendering: from-template`) or a renamed/reinterpreted
enum — a decision for the implementation commit, made deliberately rather
than by whichever name happens to be typed first. Default stays today's
category-only behavior. Only `chicago-author-date-18th` (and its T&F
descendant, resolved in the same commit — inheritance hazard carried forward
from `SUBSTITUTED_VALUE_FORMATTING.md` §3/§7.6) opts in.

**Normative points, unchanged from v1.0's reasoning:**

- Bibliography default stays never-format-from-source unless a style opts in.
- Citation-context behavior and `SUBSTITUTED_VALUE_FORMATTING.md`'s stage-2
  order are untouched.
- div-011's either/or (quote **xor** category emphasis) is *superseded*, not
  silently widened, for styles that opt in — worth stating explicitly in the
  implementation, since the new mechanism can in principle produce both
  together if a template node ever declared both (none does today).

## 5. Explicitly out of scope

**Candidate-gap** (5 rows): `SubstituteField::Title`
(`crates/citum-engine/src/values/contributor/substitute.rs`, `resolve_candidate`)
always resolves `reference.title()` and never considers `container-title`,
even for types (`article-magazine`, `article-newspaper`) where Chicago's real
rule promotes the container over the title when both are present. This is a
**different defect surface** — which value gets chosen, not how the chosen
value renders — with a different fix (the candidate-resolution function, not
the formatting path this spec designs). Filed as a follow-up bean,
cross-referenced from here once opened; not designed in this document.

**Macro-shape gap** (1 row, `6188419/92LLEIJT`): compounds the above with a
CMOS-14.102-specific "anonymous book review" pattern
(`reviewed-author: [{family: Ranke, ...}]` in the fixture), which real Chicago
renders as "Unsigned review of *Title*, by Author" — a distinct clause
structure, not explainable by promoting a different candidate value alone.
Flagged, not investigated further here.

**Unclassified rows** (9): not yet individually triaged; see the script's
documented limits in §2.

**`software`/`song`/`speech` template content** (5 of the 40 quote-gap/
emphasis-gap rows, §4.3): these three types' own bibliography templates have
no formatting on `title: primary` to derive from (`software`/`song`) or fall
to a default template with the wrong axis (`speech`). Fixing this is
ordinary style-YAML content work — add `emph: true` to the existing
`software`/`song` type-variants, give `speech` its own type-variant — not an
engine or schema change, and not designed in this document. Can land before,
after, or independent of this spec's mechanism; it doesn't block it either
way.

## Rejected alternatives

- **`titles.component.quote: true`.** Confirmed harmful via
  `Rendering::merge`/`TitleRendering::merge` semantics
  (`crates/citum-schema-style/src/template.rs:162`,
  `crates/citum-schema-style/src/options/titles.rs:245` — both use
  `merge_options!`, only overwriting when the overlay is `Some`): a category
  `quote: Some(true)` survives into `map`/`graphic`/`classic`/`hearing`'s
  template, which never sets `quote` at all, wrongly quoting alongside the
  `emph: true` that's already correct there.
- **`quote: true` on the `contributor: author` node itself.** Every affected
  row today is author-less; this plants a landmine for the same type *with*
  a real author, whose rendered name would get wrapped in quotes.
- **Drop `title` from `substitute.candidates`.**
  `SUBSTITUTED_VALUE_FORMATTING.md` §1 confirms the real citation-context
  substitute chain does promote title for these same rows; removing the
  candidate fixes bibliography by accident while breaking citation rendering.
- **Flip the bibliography default globally, no opt-in.** Every already-shipped
  embedded style changes output simultaneously with no per-style
  verification — the same objection `SUBSTITUTED_VALUE_FORMATTING.md` §6
  raises against an unconditional citation-context flip.
- **A per-type quote list (v1.0's recommendation).** Not wrong, just
  insufficient once the emphasis-gap is in scope — see §4.1. Superseded here
  by the template-node mechanism, which subsumes it.
- **Unconditional switch to node-derivation, no opt-in.** Rejected per §4.4:
  node-level formatting can be positional (correct only beside a specific
  sibling) rather than intrinsic to the type, `apa-7th` has zero
  currently-exercised bibliography-substitute rows to verify against, and
  `speech` is a proven misprediction once this mode exists at all. Per-style
  opt-in is the cheapest way to require the author verify intrinsic-vs-
  positional before a style is affected.
- **An engine-side guard suppressing prediction for "types with no dedicated
  template."** Considered while investigating `speech`'s misprediction
  (§4.3). Rejected: `document` also has no dedicated type-variant, and that
  structural fact is exactly why 21 of the 22 clean fixes work. The engine
  cannot distinguish "no template, and that's fine" from "no template, and
  that's wrong" — only a style author, by giving the type its own
  type-variant, can. The real fix for `speech` is style content, not an
  engine guard.

## Implementation Notes

Stage 2 (a stacked follow-on PR, only after this spec is reviewed):

- Decide the schema surface's name (§4.4) before writing code, not while
  writing it.
- Add the `(reference type, render context) → resolved title node` lookup,
  restricted to the field whitelist (§4.2: `quote`/`wrap`/`emph`/
  `strong`/`small-caps`/`text-case`, never `prefix`/`suffix`/`form`), and the
  opt-in mode to `crates/citum-schema-style/src/options/substitute.rs` →
  `just schema-gen` in the same commit, regenerating `docs/schemas/`.
- `chicago-author-date-18th` and `taylor-and-francis-chicago-author-date-core`
  are one change surface — both opted in in the same commit, each with its
  own `report-core.js` before/after diff.
- Extend the div-011 test block in `crates/citum-engine/src/values/tests.rs`
  with both quote-gap and emphasis-gap cases; `#[rstest]` BDD
  `given/when/then`, ≥2 cases, `assert_eq!` on captured output.
- `report-core.js` before/after for `chicago-author-date-18th`,
  `taylor-and-francis-chicago-author-date`, `apa-7th`, `elsevier-harvard` —
  the first two should show **22** of the 40-row taxonomy closing (§4.3;
  the 13 content-gap-remains and 5 no-effect rows are expected to stay open —
  don't treat their persistence as a regression), the last two byte-identical.
- The 5 no-effect rows (`software`/`song`/`speech`) are a separate,
  independent style-content fix (§5) — bundle it in this commit only if
  convenient, not because this spec requires it.
- `just pre-commit` gate, verbatim.

## Acceptance Criteria

- [x] Taxonomy covers every author-less bibliography row, not only the rows a
      quote-specific label could see; the tool's own blind spot is named as
      the reason v1.0 was narrow, not asserted as a design choice.
- [x] Quote-gap and emphasis-gap traced to one shared root cause (category-only
      resolution in the substitute path), not presented as two problems.
- [x] Candidate-gap and macro-shape gap identified, documented, and explicitly
      separated from the formatting mechanism this spec designs.
- [x] `apa-7th` / `elsevier-harvard` non-regression confirmed by measurement.
- [x] Mechanism chosen and argued, with the v1.0 recommendation's insufficiency
      explained (not just superseded) and the render-when ambiguity that
      blocked it in v1.0 resolved by name (`group_condition_matches`).
- [x] Mechanism reframed as extending the existing category-then-node-merge
      precedence normal title rendering already uses
      (`effective_title_quote_depth`), not introduced as a new concept.
- [x] Realistic yield hand-simulated against the real, currently-shipped
      style YAML before claiming the taxonomy closes — 22 of 40, not 40 —
      with the simulation committed as a reproducible script flag
      (`--simulate`), not asserted.
- [x] Opt-in requirement re-justified on intrinsic-vs-positional grounds
      (§4.4), with a concrete example (`apa-7th`'s `article-journal`
      `emph: false` beside `title: parent-serial`) checked directly against
      the style YAML rather than assumed, and precisely scoped: demonstrates
      positional intent, not a proven regression.
- [x] `software`/`song`/`speech`'s no-effect/misprediction rows traced to a
      separate style-content cause, with the rejected engine-guard
      alternative recorded and why it fails (`document` shares the same
      structural shape as the rows it would have suppressed).
- [x] Registered in `docs/specs/README.md`.
- [x] div-011 updated to record this revision, without altering its contract.
- [ ] Candidate-gap follow-up bean filed and cross-referenced here.
- [ ] `software`/`song`/`speech` style-content fix filed (bean or direct
      `/style-maintain` pass) and cross-referenced here.
- [ ] Schema surface name decided (§4.4).
- [ ] Stage 2 implements the opt-in template-node mechanism for
      `chicago-author-date-18th` and its T&F descendant in one commit, with
      `just schema-gen` and the before/after diffs listed in Implementation
      Notes.

## Changelog

- v1.0 (2026-08-24): Initial version, quoting only.
- v1.1 (2026-08-26): Broadened from quoting to every title-formatting axis
  after review on PR #1231 identified the scope-narrowing tool blind spot
  (§1). Mechanism recommendation changed from a per-type quote list to
  deriving formatting from the reference type's own resolved template node,
  gated behind an explicit opt-in. Candidate-selection and macro-shape
  findings added and explicitly scoped out. Renamed from
  `SUBSTITUTED_TITLE_BIBLIOGRAPHY_QUOTING.md`.
- v1.2 (2026-08-26): Follow-up review questioned whether this needs schema
  change at all, and whether the design leaves room for slot-specific
  formatting. Reframed the mechanism as extending the category-then-node-merge
  precedence normal title rendering already uses (§4.2), not a new concept.
  Hand-simulated the mechanism against the real style YAML (§4.3,
  `scripts/audit-substitute-bibliography-formatting.py --simulate`): realistic
  yield is 22 of 40, not all 40 — 13 rows have separate, unrelated remaining
  gaps, and 5 (`software`/`song`/`speech`) need a style-content fix the
  mechanism can't provide, moved to §5. Re-justified the opt-in on
  intrinsic-vs-positional formatting grounds, using a concrete `apa-7th`
  example, in place of v1.1's more generic backward-compatibility argument.
  Added and then rejected an engine-guard alternative for template-less
  types.
