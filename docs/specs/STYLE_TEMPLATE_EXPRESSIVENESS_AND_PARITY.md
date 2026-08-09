# Style Template Expressiveness and Parity Coverage Specification

**Status:** Draft
**Version:** 1.0
**Date:** 2026-08-09
**Supersedes:** (none)
**Related:** beans `csl26-9zn6`, `csl26-4xg9`, `csl26-02yg`;
[`CHICAGO_VARIANT_AXES.md`](./CHICAGO_VARIANT_AXES.md),
[`TEMPLATE_V3.md`](./TEMPLATE_V3.md),
[`STYLE_COMPATIBILITY_INHERITANCE_REPORT.md`](./STYLE_COMPATIBILITY_INHERITANCE_REPORT.md),
[`REPO_LOCAL_HARNESS.md`](./REPO_LOCAL_HARNESS.md)

## Purpose

Define the smallest template and measurement contracts needed to improve a
style without mistaking aggregate fidelity for correctness. The specification
settles two related questions: how a child style can patch an inherited
fallback template, and how a coverage packet proves which relevant fixture
fields were rendered, intentionally omitted, handled by a fallback, or left
uncovered.

The shortened-notes pilot motivated this design. Its numerical results remain
provisional because the packet was generated from uncommitted inputs and its
row-level parity join failed. This specification preserves the useful findings
without adopting those numbers as a baseline.

## Scope

In scope:

- the admission rule for new template reuse or conditional features;
- inherited diffs for citation and bibliography fallback templates;
- a versioned audit manifest and stable observation identity;
- field relevance, render disposition, comparison eligibility, and exact
  parity denominators;
- reviewer contracts and QA gates for reproducible style evidence;
- adjudication of the shortened-notes pilot as provisional evidence.

Out of scope:

- adding a general macro or expression language;
- implementing the fallback diff or coverage tooling in this documentation
  change;
- changing a style, converter, rendering engine, fixture, or workflow control
  surface;
- committing the pilot packet or individual model review records;
- treating agreement among models as authority.

## Design

### Bounded expressiveness is the default

Citum already provides six bounded composition mechanisms:

1. `extends` for style-family inheritance;
2. full or diff-form `type-variants` for reference-type templates;
3. `render-when` for field-presence conditions;
4. groups for local composition and affix control;
5. presets for named configuration reuse; and
6. locale messages for reusable, reorderable phrases.

No observed shortened-notes residual requires a macro or general conditional.
The measured defects map to style option values, template content, engine
behavior, casing, fixtures, or authority pairing. This is a finding about the
current evidence, not proof that every citation system is expressible. The
legal, treaty, and hearing templates are too incomplete to test that boundary.

A proposal for a new macro, expression, or conditional feature must include a
minimum counterexample with all of the following:

- one fixture reference and the expected authority output;
- the smallest attempted Citum template;
- an explanation of why `extends`, diffs, `render-when`, groups, presets, and
  messages cannot express the result;
- the repetition or maintenance cost that makes a type-specific template
  inappropriate; and
- a schema boundary and deterministic evaluation rule.

Until such a counterexample exists, a residual must first be classified as an
architecture, schema/engine, migration, style-data, fixture, QA, or harness
finding. Missing template content is not evidence for a new language feature.

### Fallback templates use the existing diff type

`BibliographySpec.template` and `CitationSpec.template` are currently
`Option<Template>`, while reference-type templates use the untagged
`TemplateVariant` enum with `Full` and `Diff` cases. This asymmetry makes an
inherited fallback whole-replace only. A child that needs to remove one URL
component must copy the rest of the fallback and then track future parent
changes manually. `CHICAGO_VARIANT_AXES.md`, Gap B, records the concrete
Chicago example and bean `csl26-4xg9` tracks implementation.

The implementation must widen the existing `template` fields to
`Option<TemplateVariant>`. It must not add a second sibling field such as
`template-variant`. Existing YAML sequences continue to deserialize as the
`Full` case, so authored styles do not need migration.

Resolution follows this contract:

1. Resolve the parent's effective fallback to a concrete template before
   overlaying the child. This includes resolving a parent `template-ref`.
2. A child `Full` value replaces the inherited fallback, matching current
   behavior.
3. A child `Diff` applies to the captured parent fallback in authored
   `modify`, `remove`, and `add` order.
4. A root `Diff` with no inherited fallback is invalid and must name the
   section and style in its diagnostic.
5. `TemplateVariantDiff.extends` is invalid for a fallback diff. A fallback
   has one inherited base and cannot select a reference-type template.
6. Declaring `template-ref` and a `Diff` in the same section is invalid. A
   parent reference can be patched after resolution, but a same-section
   reference must not create a second, implicit base-selection rule.
7. Existing explicit-null overlay behavior remains unchanged: `template:
   null` clears an inherited fallback.
8. Resolution is load-time only. A successfully resolved style exposes a
   concrete `Full` fallback to the rendering engine and contains no pending
   diff.

The same conformance cases must cover citation and bibliography sections:
backward-compatible full YAML, parent full plus child diff, parent
`template-ref` plus child diff, missing parent, forbidden `extends`, explicit
null, and selector failures. Schema generation belongs in the implementation
commit.

### Coverage has three independent dimensions

A populated fixture field is not automatically a style requirement. Coverage
starts by declaring whether the field is relevant to the style, surface, and
fixture. Every exclusion needs a manifest rule and a rationale. Fixture
metadata such as a license or citation key must not inflate the uncovered
count merely because it is populated.

Each relevant observation then receives exactly one render disposition:

| Disposition | Meaning |
|---|---|
| `rendered` | The resolved, non-fallback path consumed the field for this observation. |
| `fallback` | The resolved style-wide fallback consumed the field. |
| `suppressed` | The audit manifest declares an intentional omission and its rationale. |
| `uncovered` | No resolved path consumed the relevant field and no intentional omission applies. |

Comparison eligibility is separate:

| Comparison state | Meaning |
|---|---|
| `comparable` | A paired authority observation exists; `exactMatch` is `true` or `false`. |
| `not-comparable` | No paired authority text exists; `exactMatch` is `null`. |

This replaces the proposed five-value state list. `not-comparable` describes
authority pairing, not rendering. A field may therefore be `rendered` and
`not-comparable`, or `uncovered` and `comparable`, without contradiction.

Static component discovery may explain a path, but it cannot prove that a
conditional component consumed a value at runtime. The target evidence is an
engine-produced trace of the resolved template and consumed semantic fields.
If an external tool mirrors resolution, it must run shared conformance
fixtures against the engine and identify itself as inferred structural
coverage rather than runtime coverage.

### Audit manifest and observation identity

Every packet is generated from a versioned audit manifest. The manifest must
record:

- manifest version and packet ID;
- source revision and whether the worktree was dirty;
- style ID plus the path and content hash of every style in its inheritance
  chain;
- fixture paths and hashes, fixture IDs, reference IDs, and normalized types;
- generator version or revision and normalized command arguments;
- authority name, tool version, input hash, output hash, and evidence-run ID;
- relevant-field rules and intentional omissions with rationales;
- surfaces, comparison eligibility, and any registered authority conflict;
- deterministic codepoint ordering and the expected observation count.

An exploratory packet may describe a dirty worktree, but it cannot establish
or update a gate baseline. Baseline evidence must come from committed inputs
and must regenerate byte for byte from the recorded manifest.

Each observation has a stable ID derived from:

`style / surface / fixture / reference / normalized type / semantic field /
occurrence`

The occurrence discriminator is required only when the preceding fields do
not identify one observation. Anonymous duplicate rows are invalid. Packet
formats must emit the complete observation set; a human-readable view may
paginate, but it must not silently truncate.

When a parity report is supplied, rows join by stable evidence identity, not
array position or display text. The generator must fail if all joined
`exactMatch` values are null. A freshness test must regenerate committed
evidence and compare bytes.

### Denominators and gates

Coverage and parity use separate denominators:

- coverage total is the count of relevant observations;
- coverage dispositions partition that total;
- excluded populated fields are reported separately with their manifest
  rationale;
- exact-parity total is the count of comparable authority observations;
- `not-comparable` observations are reported but excluded from exact parity.

Fidelity remains a lenient compatibility tripwire. It does not prove field
coverage or exact text. Exact parity also does not prove coverage because two
renderers can omit the same relevant field. A completion claim therefore needs
both coverage accounting and paired exact-parity evidence.

A style or bounded cluster may move no exact observations for a valid reason,
such as a localization-only change. The result must state its intended effect
and classify every unchanged residual. A rendering cluster cannot be called
complete solely because a floor did not regress.

The QA gate for a baseline packet rejects:

- a dirty or incomplete provenance manifest;
- missing or duplicate observation IDs;
- relevant fields without a render disposition;
- suppressed or excluded fields without a rationale;
- a supplied parity report whose row join produces no non-null exact matches;
- a mismatched, unstable, or silently truncated denominator;
- non-deterministic ordering or a stale generated artifact; and
- a correctness claim based only on aggregate fidelity.

### Review and adjudication

Review packets should be examined from at least three perspectives:
architecture and schema boundaries, CSL and style semantics, and QA and
evaluation. Reviewers must label facts and inferences, cite stable observation
IDs or repository paths, classify findings using the seven categories above,
and flag missing evidence. Reviews are advisory. A maintainer adjudicates
conflicts against the packet and authority contract; agreement among models
does not change the specification.

The repository does not need provider-specific invocation code to enforce this
contract. Review records may be retained when they add durable evidence, but
they are not substitutes for the manifest, trace, or maintainer decision.

## Pilot Adjudication

**Verdict:** approve-with-revisions.

The three strongest agreements with the proposal are that bounded reuse is
sufficient on current evidence, inherited fallback diffs close a concrete
composition gap, and coverage plus exact parity is stronger than aggregate
fidelity alone. The three strongest objections are that the packet provenance
was not reproducible, the five proposed coverage states mixed two dimensions,
and intentional omission had no auditable declaration channel.

The uncommitted 2026-08-09 shortened-notes pilot reported 23/482 exact parity,
13 not-comparable authority observations, and 119 uncovered field
observations. Those are observed generator outputs, not accepted baselines.
The packet named a Git revision while reading modified inherited styles,
hashed only the leaf style, referenced a report in `/tmp`, emitted null
row-level exact matches, and silently limited its Markdown table to 80 of 535
rows. These facts prevent independent regeneration and row-level attribution.

The pilot still supports the following provisional findings. Zero-based row
offsets refer to its uncommitted JSON array and are included only to preserve
the evidence trail until stable observation IDs replace them.

| Classification | Fact or inference | Finding and evidence | Required action |
|---|---|---|---|
| architecture | Inference | No observed defect requires a macro or general conditional; legal, treaty, and hearing coverage is too incomplete to close that question. | Require a minimum counterexample before expanding the language. |
| schema/engine | Fact | Fallback `template` fields are whole-replace while `type-variants` accept `TemplateVariant`; see `BibliographySpec.template`, `CitationSpec.template`, and `CHICAGO_VARIANT_AXES.md` Gap B. | Implement the widening under `csl26-4xg9`. |
| QA | Inference | Render disposition and comparability were combined, so the five proposed states could not classify both at once. | Store them as independent dimensions. |
| harness | Fact | The parity join yielded null `exactMatch` values, the Markdown view truncated rows, and provenance omitted dirty inherited inputs. | Implement the manifest and failure gates under `csl26-02yg`. |
| fixture | Inference | About 37 of 119 uncovered observations were populated metadata such as `citation-key`, `license`, `language`, and `note`, not Chicago render obligations. Examples include pilot row offsets 121, 123-125, 182, and 184-185. | Declare relevance and report exclusions separately. |
| style-data | Inference | Roughly 39 observations were likely intentional omissions, including bibliography `publisher-place` and fields omitted from shortened citations. Examples include row offsets 76, 143-148, 278-279, and 437-441. | Declare intentional omission with rationale before counting these as suppressed. |
| style-data | Inference | Roughly 43 observations were likely real template gaps. Journal `issue` appears at offsets 40-45; legal and patent segments at 120, 126, 210, and 253-255; media and interview fields at 164-165, 204-205, and 241-244. | Repair one bounded defect family and remeasure with a valid packet. |
| schema/engine | Inference | Citation punctuation-in-quote behavior, an authorless leading delimiter, and possible name-suffix ordering explain a material part of the exact drift. The pilot analysis attributed 14 of 38 citation failures solely to punctuation-in-quote. | Test the punctuation boundary as the first single-variable engine experiment. |
| fixture | Fact | Reference `6188419/SMZBX82P` had divergent title text between compared inputs. | Compare the fixture with the authority input snapshot before assigning fault. |
| QA | Fact | Thirteen authority observations were unpaired and therefore excluded from the 482 exact-parity denominator. | Preserve `not-comparable` separately and retain evidence-run identity. |
| migration | Fact | The reviewed style chain was hand-authored, and the pilot identified no converter defect. | Do not assign any residual to migration without new converter evidence. |

The numerical grouping above is a manual interpretation of flawed evidence.
It explains why 119 is not one kind of defect, but it must not become a gate or
be presented as an exact causal decomposition. Exact failures often contain
several overlapping defects.

## Three Smallest High-Leverage Follow-ups

1. Implement the audit manifest, stable row join, complete output, and
   regeneration test under `csl26-02yg`.
2. Test and fix punctuation-in-quote at citation assembly, then regenerate the
   same committed evidence run with an unchanged denominator.
3. Repair the article-journal issue and year grammar in
   `chicago-shortened-notes-bibliography-core.yaml`, then measure the inherited
   effect independently of other residuals.

The recommended next experiment is follow-up 2 on top of follow-up 1. It is a
single engine variable with a predicted citation effect and exercises the full
manifest, trace, join, and parity-ratchet path.

## Unresolved Questions

- Once legal, treaty, and hearing templates render their basic fields, does a
  minimal authority case expose a real conditional gap?
- Is the `6188419/SMZBX82P` title divergence in the fixture, the authority
  input, or the pairing?
- Which engine trace format can expose resolved component and consumed-field
  identity without coupling the audit to internal renderer types?
- Should manifest relevance rules live beside fixtures or in a separate
  versioned audit profile? They must not become hidden style semantics.

## Implementation Notes

- `csl26-4xg9` owns only the schema, resolver, schema-generation, and
  conformance work for fallback diffs.
- `csl26-02yg` owns the manifest and packet implementation. It should consume
  engine resolution evidence where available instead of maintaining a second
  style resolver.
- Style and engine fixes discovered by the pilot remain separate changes with
  their own before-and-after evidence.
- Workflow control surfaces should be updated only after the packet gates are
  executable and proven. This Draft does not change contributor policy.

## Acceptance Criteria

- [x] The macro admission rule is tied to a concrete counterexample.
- [x] Fallback diff syntax, resolution order, error cases, null behavior, and
      `template-ref` interaction are specified.
- [x] Relevance, render disposition, comparison eligibility, and exact-parity
      denominators are defined independently.
- [x] The pilot findings are separated into facts and inferences without
      adopting the reported totals as a baseline.
- [ ] `csl26-4xg9` implements fallback diffs and passes citation and
      bibliography conformance tests.
- [ ] `csl26-02yg` implements the manifest, trace or conformance evidence,
      stable joins, complete output, and byte-regeneration gate.
- [ ] A committed shortened-notes packet passes the new QA gates and replaces
      the provisional pilot numbers.

## Changelog

- 2026-08-09: Initial Draft. Adjudicated the shortened-notes pilot, narrowed
  the fallback design to the existing `template` field, and split coverage
  disposition from comparison eligibility.
