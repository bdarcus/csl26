# CSL-Direct Interpretation vs. CSL→Citum Translation

- **Date:** 2026-08-01
- **Bean:** `csl26-2jls`
- **Question:** `citum-migrate` (28.5k LOC) has been a costly, low-yield token
  sink. Does that argue for a different path — extending `citum-engine` to
  interpret CSL 1.0 directly, bypassing translation to Citum YAML entirely?
- **Evidence base:**
  [2026-07-17 migration approach strategic review](2026-07-17_MIGRATION_APPROACH_STRATEGIC_REVIEW.md),
  [2026-07-31 exact-parity refocus](2026-07-31_EXACT_PARITY_REFOCUS.md),
  [2026-06-14 migrate fidelity locus classification](2026-06-14_MIGRATE_FIDELITY_LOCUS_CLASSIFICATION.md),
  `scripts/report-data/embedded-parity-baseline.json`, `crates/csl-legacy/`,
  `crates/citum-engine/src/render/component.rs`, `styles/embedded/*.yaml`.

## Verdict

**Not yet decidable, and the headline number usually cited for "translation
has plateaued" does not hold up under inspection.** Interpretation has one
genuine structural advantage over translation — it skips migrate's hardest
problem — but the real cost (a second, permanently-maintained rendering path
that can never replace the oracle it would be judged against) is severe
enough that it should not be scoped from a metric that turns out to be half
duplicate rows and one unexplained regression. The 2026-07-17 review already
answered a related but different question (deterministic conversion vs.
hand-tuning, both translation); it did not evaluate interpretation. This audit
recommends a single measurement — a parity-targeted tuning wave on one
mid-pack embedded style — that would settle the question either way, before
any interpreter is designed.

## Why this question is live now, not in July

The July review closed on "the plateau is the structural price of the
procedural→declarative uplift; the hybrid strategy is correct." That verdict
was reached under `fidelityScore`, a lenient bag-of-words comparator. Two
things changed since:

1. **The gate changed underneath the styles that were tuned against it.**
   The 2026-07-31 refocus replaced lenient fidelity with exact byte parity as
   the metric that defines "done" for the embedded-core tier. Every embedded
   style currently in the repo was hand-tuned to satisfy the old, lenient
   gate — none has been tuned against the new one.
2. **The measured result under the new metric is low.** The 19 embedded-core
   styles — the maximum-effort, hand-authored tier, the strongest case
   translation can make — sit at 1377/3235 (42.6%) divergence-adjusted exact
   parity in the current baseline.

A 42.6% ceiling on the best-resourced tier, if real, would be a legitimate
argument that translation cannot reach the bar the project now holds itself
to. Before drawing that conclusion, the number needs to survive scrutiny.

## The 42.6% headline does not survive scrutiny

Slicing `scripts/report-data/embedded-parity-baseline.json` (19 embedded-core
styles):

| Slice | Exact parity |
|---|---|
| All 19 embedded styles | 1377/3235 = **42.6%** |
| Chicago cluster (4 rows) | 349/1617 = **21.6%** |
| Non-Chicago (15 rows) | 1028/1618 = **63.5%** |

Two problems compound the low number:

- **The Chicago cluster is exactly half the denominator, and one of its four
  rows is a duplicate measurement, not independent evidence.**
  `taylor-and-francis-chicago-author-date` declares `extends:
  taylor-and-francis-chicago-author-date-core`, which in turn `extends:
  chicago-author-date-18th`. Its baseline entry reports **byte-identical**
  numbers to `chicago-author-date-18th` (165/540, `fidelityScore: 0.917`,
  both). The 241-line T&F delta layered on top — sentence-case titles,
  multilingual mode, page-range format — moves zero measured entries. So 540
  of the 3235 denominator rows are the same underlying defect cluster counted
  twice, not two data points.
- **A second pair shows the same correlated-signature pattern.**
  `springer-basic-brackets` and `springer-vancouver-brackets` both report
  20/67 from separately-declared spines — weak as independent confirmation of
  a portfolio-wide ceiling, whatever its cause.
- **The baseline knowingly carries an unexplained regression.** The refocus
  audit records `springer-vancouver-brackets` dropping from 28/67 to 20/67
  between two recent commits, "confirmed unrelated to [the parity-counting]
  fix," filed as an open follow-up, and deliberately left in the floor rather
  than corrected.

Honest statement of the current state: **Chicago-weighted the tier reads 43%;
excluding the duplicated Chicago cluster, the remaining 15 styles read ~64%;
at least one row is a known, uninvestigated regression.** Neither number is
"the ceiling of translation" — both are two-week-old snapshots of a tier
nobody has yet tuned against the metric that produced them.

## The plateau claim is unfalsified, not established

The strongest evidence against a hard ceiling is already in the record the
refocus audit itself produced, isolating only the parity-*counting* bugfix
from unrelated concurrent style/engine work:

- `american-medical-association`: 23.9% → 71.6% exact parity.
- `chicago-author-date-18th`: 19.3% → 30.6% exact parity.

Both moves happened from ordinary style and engine work in the days around
the 2026-07-31 change — work that was not targeting exact parity as a goal,
because until that date nothing was. That is a steep, live gradient on a
metric that has had zero dedicated optimization effort. A claim that
translation cannot reach exact parity has to survive that table, and nothing
in the current evidence base does that.

## What interpretation would genuinely buy

Set the metric argument aside — even a perfect metric wouldn't settle
architecture on its own. The structural case for interpretation deserves to
be stated on its own terms, because part of it is right.

`citum-migrate` is fundamentally a **decompiler**: CSL 1.0 is procedural
(macros, `choose/if` trees, groups with implicit empty-suppression), and
Citum is declarative and typed by design. Migrate's job is not just to
translate syntax — it has to *recover intent* the source encoding does not
carry: which macro invocation means "render the container title," what a
given `choose` branch structure means for a specific reference type, when an
empty-group suppression is load-bearing versus incidental. That is the
documented source of the compounding-defect tail (2026-06-14 locus
classification): every sampled sub-90 style failed for converter-level
reasons — dropped variables, misclassified processing modes, wrong template
data — the "correct template, wrong render" failure mode was never observed.

An interpreter does not have this problem. It evaluates the CSL layout
against a resolved reference at render time, the same way citeproc-js does —
there is no intent to recover, because there is no target representation to
translate into. It just executes.

This is not merely theoretical:

- `crates/csl-legacy` already provides a complete, frozen CSL 1.0 AST
  (`model.rs`, `CslNode`, `Text`, `Names`, `Group`, `Choose`, `Date`, etc. —
  528 lines) plus CSL-JSON support (`csl_json.rs`, 1,644 lines). The frontend
  a CSL evaluator would need already exists and is exercised daily by
  migrate's parse step.
- The engine-side coupling objection is weaker than it first appears.
  `ProcTemplateComponent` (`crates/citum-engine/src/render/component.rs:13`)
  carries a `template_component: TemplateComponent` field alongside an
  already-resolved `value: String` — but that field is a *formatting-options
  carrier* (emphasis, wrap, affixes, quote marks, semantic class), not
  Citum's semantic model. CSL's `<text font-style="italic" prefix="(">` maps
  onto the engine's `Rendering` options almost directly. A CSL evaluator
  lowering into `ProcTemplate` after evaluating CSL's own layout/choose logic
  is a substantially shallower engineering problem than migrate's, which has
  to statically recover that logic's *meaning* without ever seeing a
  reference flow through it.
- Three independent, mature CSL processors already exist (citeproc-js,
  citeproc-rs, and Zotero's fork); the semantics are specified and have
  reference implementations to check against, unlike migrate's inference
  problem, which has no oracle for the intermediate step.

## What interpretation would cost, and why it doesn't win on its own

None of the above makes interpretation the missing piece. Both `csl26-bv8w`
(the July review) and this section arrive at the same underlying reason:

- **Dual semantics forever.** A CSL evaluator does not replace `citum-migrate`
  or `citum-schema-style`'s template model — Citum-native and hand-tuned
  styles still need the declarative model on its own merits (typed data,
  `extends`, presets — see `DESIGN_PRINCIPLES.md` §4). It would sit alongside
  the existing engine as a second, permanently maintained evaluation path,
  with its own test matrix, its own performance profile, and its own surface
  in every binding (FFI, WASM, CLI).
- **It inherits citeproc-js bug-compatibility with no stopping point.** Exact
  parity with citeproc-js as the target means matching citeproc-js's bugs,
  indefinitely. `DESIGN_PRINCIPLES.md` §1 already treats citeproc-js as "an
  executable authority or fallback," not the final word — a native CSL
  evaluator built to match it byte-for-byte has no natural point at which it
  stops importing that authority's defects as requirements.
- **It cannot replace the oracle, which removes its most attractive secondary
  justification.** citeproc-js *is* the ground truth CSL-derived styles are
  measured against. Diffing a native Rust CSL evaluator against citeproc-js
  output is diffing against the thing it exists to reproduce — useful as an
  internal consistency check, but it cannot become a faster or more available
  oracle, because a divergence from citeproc-js proves nothing about which
  side is correct.
- **Strategic dilution.** If CSL styles could render natively at high fidelity
  without ever becoming Citum YAML, the incentive to author, review, or
  maintain hand-tuned Citum styles for the ~2k+ independent CSL corpus
  weakens — directly undercutting the declarative-schema goal the project
  exists to pursue (`DESIGN_PRINCIPLES.md`, Project Goal in `CLAUDE.md`).

No line-count or effort estimate for an interpreter is given here — nothing
in the current evidence base supports one, and inventing a figure would be
the least defensible claim in this document.

## A narrower, real process gap

Separately from the architecture question: the refocus audit's own escalation
path concedes that `docs/adjudication/DIVERGENCE_REGISTER.md` is too
heavyweight for parity-shaped residuals — a register that added 15 entries in
five months would be swamped by the thousands of largely
punctuation/spacing-shaped exact-parity mismatches the new gate surfaces.
High-volume parity adjudication currently has no process of its own. That gap
is worth closing regardless of which architecture direction is chosen, and is
out of scope for the interpretation question — noted here so it isn't lost.

## Recommendation: measure before scoping

Do not decide the architecture question from evidence that cannot support it
either way. The discriminating measurement is cheap relative to scoping a
second rendering path, and it directly extends the already-open
`csl26-m2t1` (tuning cost telemetry) rather than opening a new direction:

**Run one parity-targeted tuning wave on a single mid-pack embedded style,
with cost recorded, and fix the decision rule before starting:**

- **Probe candidate: `ieee`** (currently 84/149, 56.4% exact parity).
  Confirmed standalone — no style-level `extends`; the `extends:` occurrences
  in `styles/embedded/ieee.yaml` are template-variant selectors, not a parent
  style. Alternative: `modern-language-association` (38/115, 33.0%), also
  confirmed standalone. Both avoid the Chicago duplication and the springer
  correlated-signature pair, so either gives an independent read on the
  tier's real ceiling.
- **Decision rule, fixed now:**
  - If the wave reaches roughly 90%+ exact parity at a sane, recorded cost →
    translation is not structurally capped by this metric; the interpreter
    is unnecessary and this question closes.
  - If it stalls in the 50s–60s after a genuinely thorough tuning effort →
    that stall is the number that justifies scoping `citum-csl` as a
    follow-on design spec, with the cost data from this wave as its opening
    evidence.

This keeps the two decisions properly separated: whether translation can
still reach the bar the project just raised on itself, versus whether a
second rendering path is worth building. Only the first is answerable today.

## Addendum (2026-08-02): the wave ran — here's what to do next

**Verdict:** `ieee` moved 84/149 → 88/149 exact parity (56.4% → 59.1%),
fidelity 100%, zero regressions. Still inside the "50s–60s stall" band this
document's decision rule named — but the wave found one general engine bug,
not an ieee quirk, and that bug retroactively improved
`american-medical-association` too (48/67 → 49/67, verified against the
checked-in baseline) once cross-checked against already-landed work
(`csl26-j7uc`). One wave on one style doesn't settle the interpreter
question either way; what it's good for is the table below. Full record:
bean `csl26-unyu`, commit `12865760` on `style/ieee-exact-parity-wave`.

**Next action: `csl26-ww77`.** `csl26-arly` already triaged all 2,501
embedded+exemplar mismatches into named classes on 2026-07-30, closed its
own scope, and left a 37% "Z-unclassified" bucket never broken down further.
`csl26-ww77` checks whether that bucket hides more bugs shaped like this
one, rather than re-running the triage from scratch. It lives under
`csl26-ccdt` ("Embedded-tier non-Chicago parity," `csl26-arly`'s successor
epic — `csl26-arly` itself is closed and shouldn't be reopened), which is a
child of the top-level milestone `csl26-w0hf`. No new epic was needed once
the tree was checked properly — creating one would have repeated the exact
disconnected-thread mistake `csl26-w0hf` exists to prevent.

### What was found, what it costs to fix

| Class | Found this wave | Fix cost |
|---|---|---|
| General engine bug | Numeric bibliography label wrongly counted as content when its sibling is empty → spurious separator (`label_only` fix) | Fixed once, benefits every numeric style — free |
| Shared-preset entanglement | Role-label suffix shared by ieee/chicago/AMA; fixing ieee regressed the other two (172→164/540, 49→48/67) — caught by hand, reverted | Needs per-style override or smarter separator logic, not a preset patch — `csl26-g6bi` |
| Missing type-variants | `legal_case`/`treaty`/`patent`/apparatus never migrated from `ieee.csl` | Hand-authoring — `csl26-y49d` |
| Schema gap | No way to override month abbreviations or zero-pad days per style | Bounded engine+schema work — `csl26-3az5` |
| Data leaks | Raw `phd-thesis` enum, one fabricated `no. 1` | Small, untriaged — `csl26-lhrl` |

**Process lesson, not optional going forward:** the shared-preset regression
was caught by manually re-checking two styles I happened to think of, not by
any automated check. Any change to engine machinery shared by more than one
style must run `report-core.js --all-features` across the full
embedded-core tier before landing.

**For the interpretation question:** both real bugs found here are
artifacts of translation's own shared-abstraction machinery (a
label-injection pass, a role-label preset) — not something a CSL
interpreter executing each style's layout literally would need to get
right. A real point in its favor on this narrow failure mode; it doesn't
touch the structural costs (dual semantics, no oracle substitute) that
actually decide the question.
