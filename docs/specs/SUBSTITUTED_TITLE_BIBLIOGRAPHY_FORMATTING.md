# Substituted-Title Bibliography Formatting Specification

**Status:** Active
**Version:** 1.3
**Date:** 2026-08-26
**Supersedes:** None
**Related:** bean `csl26-0u0f`; bean `csl26-0dca`; `docs/specs/SUBSTITUTED_VALUE_FORMATTING.md`; `docs/adjudication/DIVERGENCE_REGISTER.md` div-011; bean `csl26-zja7` (candidate-gap); bean `csl26-9ups` (software/song/speech style content); bean `csl26-ckcf` (unrelated pre-existing duplicate-rendering bug found during verification)

## Purpose

`SUBSTITUTED_VALUE_FORMATTING.md` answers, for **citation** context, whether a
title promoted into the missing-author slot (`contributors.substitute`) keeps
its own formatting or takes the slot's — explicitly citation-scoped only
(§5). This spec answers the same question for **bibliography** context, for
bean `csl26-0u0f`.

**v1.0 of this spec covered quoting only. That was too narrow — see §1.** v1.1
broadened it to every title-formatting axis. **v1.2 added the realistic-yield
hand simulation. v1.3 replaces that simulation with measured results from the
implemented mechanism** (§4.3) — the live mechanism runs on every author-less
row, not just the ones known to be wrong, and that turned up one regression
the simulation could not have caught (§4.4).

## Taxonomy

Every author-less `chicago-author-date-18th` bibliography row, classified by
what's actually wrong (methodology and reproduction command in §2):

| Class | Count | Example | Disposition |
|---|---|---|---|
| **Quote-gap** | 32 | `document`, `manuscript`, `article-journal` — oracle quotes the promoted title, Citum doesn't | **Fixed.** 32/32 measured clean (§4.3) — the mechanism this spec designs, plus one co-requisite style fix (§4.4) |
| **Emphasis-gap** | 8 | `map`, `hearing`, `software`, `song`, `speech` — oracle italicizes, Citum doesn't | Same root cause as quote-gap (§1). **`map`/`hearing` fixed** (2/8, measured); `software`/`song`/`speech` need a separate style-content fix instead — bean `csl26-9ups`, §4.3, §5 |
| **Candidate-gap** | 5 | `article-magazine`/`article-newspaper` with both `title` and `container-title` — oracle promotes the container, Citum promotes the title | **Out of scope**, unaffected by this mechanism. Different defect surface (`SubstituteField::Title` resolution) — bean `csl26-zja7`, §5 |
| Render-when bypass | 12 | `webpage` with no `title` — routes through a template branch with no `contributor: author` at all | Never reaches the substitute path; not this spec's concern regardless of formatting outcome |
| Already matching | 37 | — | No action |
| Unclassified | 9 | needs manual review (see script limits, §2) | Not yet triaged |

Rows sum to 103, the full author-less bibliography set. One of the five
candidate-gap rows (`6188419/92LLEIJT`) also carries a further complication —
a CMOS-14.102 "anonymous review" macro pattern, §5 — but it's one row, not an
additional count.

**40 of 103 author-less bibliography rows are quote-gap or emphasis-gap; 5 are
a real but different defect (candidate-gap, out of scope); the rest are out
of scope or already correct. Measured result after implementation: 34 of the
40 now render correctly** (32/32 quote-gap, 2/8 emphasis-gap — `map`,
`hearing`); the remaining 6 emphasis-gap rows (`software`/`song`/`speech`)
need a separate style-content fix, bean `csl26-9ups`. §4.3 has the full
measurement, including a real regression the design-time hand-simulation
could not have predicted, found and fixed before this number was taken.

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
`quote`/`wrap`/`emph`/`strong`/`small-caps` — never `prefix`/`suffix`/`form`,
which describe the node's position in its own template, not a property of
the value itself. `text-case` was a candidate for this whitelist in earlier
revisions of this section; dropped at implementation time (Implementation
Notes) since nothing in the corpus exercises a node-level `text-case`
override on a substituted title, and this spec ships verified behavior only.

**Cost that's real, at design time:** `RenderOptions`
(`crates/citum-engine/src/values/mod.rs:950`) carried no reference to the
resolved template. This needed scoped plumbing — a lookup from
`(reference type, render context)` to "the `title: primary` node that would
render for this reference" — not a config read. (Implemented: a
`substitute_title_template` field populated from the render request's own
already-resolved template, not a re-derivation — see Implementation Notes.)

### 4.3 Realistic yield: measured against the real style, post-implementation

Before this spec trusted "this closes the 40-row taxonomy," two rounds of
verification ran, in order: a design-time hand simulation
(`scripts/audit-substitute-bibliography-formatting.py --simulate`, v1.2's
22/13/5 breakdown — superseded, script kept for reproducibility), then a
measured before/after diff of the *implemented* mechanism against the
shipped style, below. The measured numbers are authoritative — a hand
simulation only ever ran against the 40 rows already known to be wrong; the
live mechanism runs on **every** author-less bibliography row, including the
ones that were already correct, and that distinction is exactly what found
the regression in §4.4.

**Method.** Build the release binary with the mechanism landed and
`chicago-author-date-18th` opted in, regenerate the report, and diff the
per-row classification against the pre-implementation baseline:

```bash
cargo build --release --bin citum
node scripts/report-core.js --style chicago-author-date-18th --all-features \
    > /tmp/report-after.json
python3 scripts/audit-substitute-bibliography-formatting.py \
    --report /tmp/report-after.json \
    --fixture tests/fixtures/test-items-library/chicago-18th.json --json \
    > /tmp/taxonomy-after.json
# then diff taxonomy-after.json's per-id class against the pre-implementation
# taxonomy.json by id, not by aggregate count
```

| Type | Clean fix (measured) | Style-content fix needed |
|---|---|---|
| `document` | 30 | — |
| `manuscript` | 1 | — |
| `article-journal` | 1 | — |
| `map` | 1 | — |
| `hearing` | 1 | — |
| `software` | — | 3 |
| `song` | — | 2 |
| `speech` | — | 1 |
| **Total (40)** | **34** | **6** |

- **Clean fix (34, measured — up from 22 simulated):** all 32 quote-gap rows
  and 2 of the 8 emphasis-gap rows (`map`, `hearing`) now render correctly —
  **zero content-gap-remains beyond the software/song/speech bucket**, unlike
  the earlier hand simulation's 13-row "right formatting, wrong content"
  estimate. The simulation both undercounted the fixes (its hand-spliced
  punctuation reconstruction was cruder than the real render pipeline's
  Djot-aware quote handling) and overcounted the residual gap (some of what
  it predicted as "content gap remains" — e.g. a suspected missing
  accessed-date clause — turned out not to matter once the real pipeline,
  not a hand splice, produced the comparison text). The mechanism, run for
  real, does better than the paper prediction on both counts. Cross-checked
  by content-normalized diff (markup, quote characters, and whitespace
  stripped) between the pre- and post-implementation `rawCitum` for every
  affected row, not just aggregate counts.
- **Style-content fix needed (6, unchanged from simulation):** `software`,
  `song`, and `speech` have nothing on their resolved `title: primary` node
  to derive from (the earlier v1.2 simulation traced why in detail;
  unchanged by the switch to measured numbers) — bean `csl26-9ups`.
- **Caveat on 4 of the 32 quote-gap "clean fixes":** the taxonomy script's
  `leads()` heuristic misclassifies 4 candidate-gap rows
  (`6188419/Y7JIURAM`, `L4XXFEU2`, `6V4XJV4M`, `MAWJL9U8`) as quote-gap — see
  §2's documented script limits. Those 4 now also read as "match" once
  quoted, because the script's match check only compares the wrapper around
  the *leading* value, not full content — but the underlying candidate-gap
  defect (wrong value promoted, §5) is untouched. Not counted as genuine
  content fixes; tracked by `csl26-zja7` regardless of what the script's
  wrapper-shape heuristic reports.
- **Zero false regressions confirmed the hard way.** The raw before/after
  diff also showed 8 rows flip from "match" to "unclassified" — all
  `legislation`/`bill` entries with no dedicated bibliography type-variant,
  falling to the same default template `document` uses (`wrap: quotes`).
  Content-normalized diff confirms these 8 had **zero** content-level change
  from this commit — they were never real matches; the classifier's
  wrapper-only check was fooled by a leading-title coincidence while missing
  an entirely separate, large, pre-existing defect (missing Bluebook-style
  `Pub. L. No. …, Stat. …, … Cong. (…)` legal-citation grammar, tracked by
  this repo's existing legal-citation beans, not this spec). Whether
  `legislation`/`bill` *should* quote their act name under the default
  template is a real open question but not a regression this commit
  introduces — the content gap dominating these rows existed before and
  after, unchanged.

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

**A third case, demonstrated (not hypothetical) during implementation:
conditional-within-type.** `chicago-author-date-18th`'s
`manuscript, collection:` bibliography type-variant was one compound
type-selector key sharing a single `title: primary` node with unconditional
`wrap: quotes`. `manuscript` and `collection` are distinct native ref-types
(a note-field `type: collection` override round-trips to `collection`, per
`crates/citum-schema-data/src/reference/conversion/contract_tests.rs`), and
CMOS 18 quotes an individual manuscript item's title but not an archival
collection's — the node's formatting was correct for a *subset* of the
type-variant's instances, not simply right-or-wrong for the type as a whole.
Before this mechanism, the substitute path never read node-level formatting,
so `collection`-typed author-less rows rendered plain and happened to match
the (also plain) oracle by coincidence; deriving from the node's actual
`wrap: quotes` regressed those 6 rows the moment the mechanism went live.
Caught by re-measuring the full author-less corpus (§4.3), not by the
design-time hand simulation, which only ever ran against rows already known
to be wrong — this is the same lesson §4.3 draws from measuring instead of
simulating, generalized: **the live mechanism runs on every author-less row,
not just the previously-wrong ones; previously-matching rows are part of the
blast radius for any style opt-in, and must be checked, not assumed safe.**
Fixed by splitting the compound key into separate `manuscript:` (keeps
`wrap: quotes`) and `collection:` (no wrap) type-variants — the same fix
shape independently validated for the category-config version of this exact
distinction by bean `csl26-jxco`. This is a **co-requisite of opting
`chicago-author-date-18th` in**, not an optional style-content nice-to-have
like `software`/`song`/`speech` (§5): shipping the opt-in without it would
regress 6 real corpus rows.

Gate behind a new bibliography-scoped substitute mode. **Naming, resolved:**
a new field, `Substitute.title_rendering: Option<SubstituteTitleRendering>`
(single variant `FromTemplate`), rather than reusing or renaming the
existing quote-named `Substitute.title_quote: Option<SubstituteTitleQuoteMode>`
— that field predates this spec and nothing sets it today, so overloading it
would have cost nothing technically but would have left the field's name
lying about its own scope going forward. Default stays today's category-only
behavior. **Only `chicago-author-date-18th` opts in this commit — its T&F
descendant does not.** T&F's `article-journal` bibliography type-variant
declares a bare `title: primary` (no `wrap`), unlike the parent's
`wrap: quotes`, so the parent's measured yield (§4.3) does not transfer
without T&F's own before/after verification — exactly the class of failure
the `manuscript`/`collection` finding above demonstrates the cost of
skipping. T&F opt-in is a follow-up, gated on its own audit, not assumed
identical per `SUBSTITUTED_VALUE_FORMATTING.md` §3/§7.6's general
inheritance-hazard caution.

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
the formatting path this spec designs). Filed as bean `csl26-zja7`; not
designed in this document.

**Macro-shape gap** (1 row, `6188419/92LLEIJT`): compounds the above with a
CMOS-14.102-specific "anonymous book review" pattern
(`reviewed-author: [{family: Ranke, ...}]` in the fixture), which real Chicago
renders as "Unsigned review of *Title*, by Author" — a distinct clause
structure, not explainable by promoting a different candidate value alone.
Flagged, not investigated further here.

**Unclassified rows** (9): not yet individually triaged; see the script's
documented limits in §2.

**`software`/`song`/`speech` template content** (6 of the 40 quote-gap/
emphasis-gap rows, §4.3): these three types' own bibliography templates have
no formatting on `title: primary` to derive from (`software`/`song`) or fall
to a default template with the wrong axis (`speech`). Fixing this is
ordinary style-YAML content work — add `emph: true` to the existing
`software`/`song` type-variants, give `speech` its own type-variant — not an
engine or schema change, and not designed in this document. Filed as bean
`csl26-9ups`. Can land before, after, or independent of this spec's
mechanism; it doesn't block it either way.

**`legislation`/`bill` default-template quoting** (not part of the 40-row
taxonomy; found during §4.3's post-implementation measurement): these types
have no dedicated bibliography type-variant and fall to the same default
template `document` uses, whose `title: primary` declares `wrap: quotes`.
That quoting is now applied to their substituted titles too — consistent
with, not a special case of, how `document`'s 32 clean fixes work. Whether
Chicago's Bluebook-style legal-citation grammar (`Pub. L. No. …, Stat. …, …
Cong. (…)`) should quote the act name at all is a real question, but these
rows already diverge from the oracle overwhelmingly for unrelated reasons
(the entire legal-citation clause structure is missing, tracked by this
repo's existing legal-citation beans) — not a new defect this spec's
mechanism introduces, and not investigated further here.

**Pre-existing `archive-collection` duplicate rendering** (found during
regression verification, unrelated to this spec): `manuscript`/`collection`
entries with an `archive-collection` value render it twice consecutively.
Confirmed via before/after `rawCitum` diff to be present identically before
and after this spec's mechanism — not introduced or affected by it. Filed as
bean `csl26-ckcf`; not investigated further here.

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

Stage 2, implemented (stacked on this spec's PR):

- `RenderOptions.substitute_title_template: Option<&[TemplateComponent]>`
  (`crates/citum-engine/src/values/mod.rs`), populated from
  `TemplateRenderRequest.template` at the one shared render-request
  construction site (`process_template_request_with_format`), gated to
  `RenderContext::Bibliography` — the citation path always sees `None`.
- New field, not a reinterpreted enum (§4.4's naming decision, resolved):
  `Substitute.title_rendering: Option<SubstituteTitleRendering>`, single
  variant `FromTemplate`
  (`crates/citum-schema-style/src/options/substitute.rs`), merged via
  `Substitute::merge`. `just schema-gen` run in the implementation commit.
- `find_template_title_node` walks the resolved template honoring
  `render_when` via the existing `group_condition_matches`;
  `merge_substitute_title_rendering` applies the field whitelist (§4.2:
  `quote`/`emph`/`strong`/`small-caps`, and `wrap` only when it's quoting
  punctuation — never `text-case`/`prefix`/`suffix`/`form`; `text-case` was
  in v1.2's whitelist but dropped here since nothing exercises it and the
  session guidance is not to ship unverified behavior)
  (`crates/citum-engine/src/values/contributor/substitute.rs`).
- `chicago-author-date-18th` opted in. **T&F not opted in this commit** —
  found during implementation, not before it: see §4.4's naming-decision
  paragraph for why.
- Found and fixed one co-requisite style change before landing: split
  `chicago-author-date-18th`'s compound `manuscript, collection:`
  type-variant key so `collection` no longer inherits `manuscript`'s
  `wrap: quotes` (§4.4's conditional-within-type case).
- Extended the div-011 test block in `crates/citum-engine/src/values/tests.rs`
  with the from-template mode: quote-only, emph-only, quote+emph-together
  (the div-011 supersession), positional-wrap-ignored, `render_when`
  branch resolution, and opt-in-gate-off cases — `#[test]`/`#[rstest]`,
  `assert_eq!` on captured output.
- `report-core.js` before/after for `chicago-author-date-18th`: **34 of the
  40-row taxonomy closed** (§4.3, measured — exceeding the 22 simulated),
  with one real regression found and fixed pre-landing (`manuscript`/
  `collection`, §4.4) and zero unexplained regressions elsewhere (the
  apparent 8-row "match → unclassified" flip on `legislation`/`bill` traced
  to a pre-existing, unrelated, unaffected-in-content legal-citation gap,
  §4.3). `apa-7th` / `elsevier-harvard` / `taylor-and-francis-chicago-author-date`
  confirmed byte-identical (not opted in; T&F additionally verified by
  scanning for `title-rendering` in every embedded style — only
  `chicago-author-date-18th` has it).
- The 6 remaining rows (`software`/`song`/`speech`) are a separate,
  independent style-content fix — bean `csl26-9ups`, not bundled in this
  commit.
- `cargo nextest run --workspace`: 2708 passed, 0 failed. `just pre-commit`
  gate, verbatim.

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
- [x] Realistic yield measured against the implemented mechanism, not just
      hand-simulated — 34 of 40, exceeding the 22/40 hand-simulation's
      conservative estimate (§4.3), with the pre-implementation simulation
      script kept for reproducibility (`--simulate`).
- [x] Opt-in requirement re-justified on intrinsic-vs-positional grounds
      (§4.4), with a concrete example (`apa-7th`'s `article-journal`
      `emph: false` beside `title: parent-serial`) checked directly against
      the style YAML rather than assumed, and precisely scoped: demonstrates
      positional intent, not a proven regression.
- [x] A third, demonstrated (not hypothetical) case for §4.4's taxonomy —
      conditional-within-type — found during implementation
      (`manuscript`/`collection`), fixed as a co-requisite before landing,
      and folded back into the spec.
- [x] `software`/`song`/`speech`'s no-effect/misprediction rows traced to a
      separate style-content cause, with the rejected engine-guard
      alternative recorded and why it fails (`document` shares the same
      structural shape as the rows it would have suppressed).
- [x] Registered in `docs/specs/README.md`.
- [x] div-011 updated to record this revision, without altering its contract.
- [x] Candidate-gap follow-up bean filed (`csl26-zja7`) and cross-referenced
      here.
- [x] `software`/`song`/`speech` style-content fix filed (`csl26-9ups`) and
      cross-referenced here.
- [x] Schema surface name decided (§4.4): new field
      `Substitute.title_rendering: Option<SubstituteTitleRendering>`.
- [x] Stage 2 implements the opt-in template-node mechanism for
      `chicago-author-date-18th`, with `just schema-gen` and the before/after
      diffs above. **T&F descendant deliberately not opted in this
      commit** — its own bibliography template diverges from the parent's in
      exactly the way that matters (§4.4), so its yield needs independent
      verification; tracked as a follow-up, not silently deferred.
- [x] Zero unexplained regressions: every classification flip in the
      before/after diff traced to either a genuine content fix, a known
      script limitation (§2, §4.3), or a pre-existing unrelated defect
      confirmed unchanged by content-normalized diff — not merely asserted
      from aggregate counts.

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
- v1.3 (2026-08-26): Stage 2 implemented and measured. Schema surface named
  (`Substitute.title_rendering: Option<SubstituteTitleRendering>`, new
  field). Replaced the design-time hand simulation's 22/40 estimate with
  measured results — 34/40, and zero content-gap-remains beyond
  `software`/`song`/`speech` — from a real before/after diff of the
  implemented mechanism (§4.3): the simulation both undercounted the fixes
  and overcounted the residual gap. Found and fixed one real regression
  before landing — `chicago-author-date-18th`'s compound
  `manuscript, collection:` type-variant needed splitting — documented as a
  third, demonstrated §4.4 taxonomy case (conditional-within-type), distinct
  from the hypothetical `apa-7th` example. Confirmed, by scanning every
  embedded style and by content-normalized diff, that only
  `chicago-author-date-18th` is affected and no other row silently
  regressed. T&F opt-in deferred to a follow-up, gated on its own audit,
  after its bibliography template was found to diverge from the parent's in
  exactly the way §4.4 warns about. Filed the two previously-open follow-up
  beans (`csl26-zja7`, `csl26-9ups`) plus one unrelated pre-existing defect
  found during verification (`csl26-ckcf`).
