# Style Editions and Families

**Status:** Active
**Version:** 1.0
**Date:** 2026-05-21
**Bean:** `csl26-atyp`
**Related:** `STYLE_PRESET_ARCHITECTURE.md`, `STYLE_TAXONOMY.md`, `STYLE_REGISTRY.md`, `FORWARD_COMPATIBILITY.md`

## Purpose

Citum has explicit style inheritance (`extends:`), a two-axis taxonomy
(`STYLE_TAXONOMY.md`), and a `StyleBase` enum of compiled-in roots — none of
which CSL 1.0 has. The mechanisms exist; the long-term *policy* for how
editions and families evolve over years and decades has not been written down.

This spec codifies that policy. It answers five questions:

1. When APA 8 ships in 2030 with minor differences from 7, what is the
   inheritance chain? Should APA 7 descend from APA 6?
2. What gets embedded in the engine, and when do editions drop off — including
   the base parent edition of a family?
3. If a base style drops out of the registry, how does the first descendant
   become the new base?
4. Do styles need an explicit *version* field beyond what already exists?
5. Do `citum-migrate` and the `/style-*` skills need changes to support this
   strategy?

The spec is normative for new editions, new family heads, and any future
retirement of a `StyleBase`. Mechanism changes that are not needed immediately
are tracked as follow-up beans (see §7).

---

## §1 Edition inheritance shape

**Recommendation.** Editions are siblings under an optional hidden archetype,
not stacked edition-on-edition.

> *Hidden* here means: present in the `StyleBase` enum (reachable as an
> `extends:` target) but not enumerated in `registry/default.yaml`, so the
> entry does not appear in `citum styles`, `citum registry list`, or the
> citum-hub style picker. The existing `*-Core` variants
> (`ElsevierHarvardCore`, `SpringerBasicAuthorDateCore`, …) follow this
> pattern today. See [STYLE_REGISTRY.md](STYLE_REGISTRY.md) for the discovery
> surface and three-layer resolution that drives it.

Publishers always *describe* a new edition as a delta from the prior edition —
that is how style manuals are written. Citum consumes that delta once, during
the authoring of the new edition's YAML. It does not encode the delta as a
live `extends:` edge, because doing so would bind the new edition to whatever
the prior edition's structure happens to be at any moment, even after the
publisher has moved on. **The authoring input is a delta; the runtime
representation is a peer.**

### Rule

When a publisher releases a new edition of a style family (e.g. APA 7 → APA 8),
the new edition is added as a new Tier-1 `StyleBase` variant with its own
YAML in `styles/embedded/`. It **must not** declare `extends: <prior-edition>`.
There are two acceptable shapes:

**Default — both editions standalone.** Neither edition has an `extends:`
field. This is the path to take unless ≥2 editions share enough structure to
justify factoring out an archetype.

```yaml
# styles/embedded/apa-8th.yaml — hypothetical 2030 release
info:
  title: "APA Style"
  short_name: "APA"
  edition: "8th"
# … full style definition, no `extends:` field …
```

**Optional — archetype factored out** when ≥2 editions of the same family
share substantial structure and differ only in additive, well-documented
deltas. A hidden archetype root carries the shared structure; each edition
extends the archetype. Edition N+1 still never `extends:` edition N — both
extend the archetype peer-to-peer.

```yaml
# styles/embedded/apa-7th.yaml
extends: apa-author-date-archetype-core
info:
  short_name: "APA"
  edition: "7th"
# … only the 7th-edition deltas …
```

```yaml
# styles/embedded/apa-8th.yaml
extends: apa-author-date-archetype-core
info:
  short_name: "APA"
  edition: "8th"
# … only the 8th-edition deltas …
```

Archetypes follow the existing `*-Core` pattern from publisher families and
are hidden per the definition above.

### Rationale

- **Fragile-base-class risk.** A fix to APA 7's bibliography template would
  silently re-tune APA 8 under edition-on-edition stacking. The two editions
  are conceptually peers, not parent-and-child.
- **Structural shifts happen.** MLA 7 → 8 reorganized around a container
  model; Chicago 17 changed `ibid` semantics. A spec that mandates
  edition-on-edition inheritance encodes assumptions the next spec revision
  may invalidate.
- **Wrappers pin to an edition deliberately.** A publisher wrapper
  (`extends: apa-7th`) does not auto-follow APA into the next edition. Bumping
  a wrapper to a new edition is an explicit, audited operation, which matches
  how publishers actually update house style guides (manual, dated, with a
  changelog).
- **Current `StyleBase` shape already encodes this.** Every Tier-1 base today
  (`apa-7th`, `chicago-notes-18th`, `chicago-author-date-18th`, `ieee`,
  `american-medical-association`, `modern-language-association`) is its own
  root with no `extends:` field. The policy formalizes the existing layout.

### Example: APA 8 in 2030

**Case A — default (no convergence factored out):**

| File | `extends:` | Notes |
|---|---|---|
| `styles/embedded/apa-7th.yaml` | none | retained as the prior-edition base per §2 |
| `styles/embedded/apa-8th.yaml` | none | new Tier-1 base; full structure |
| `styles/journal-foo.yaml` | `apa-7th` | wrapper stays on APA 7 until its maintainer bumps it |

**Case B — archetype factored out** (only if APA 7 and APA 8 turn out to share
enough structure to justify it):

| File | `extends:` | Notes |
|---|---|---|
| `styles/embedded/apa-author-date-archetype-core.yaml` | none | hidden archetype; not surfaced in the registry |
| `styles/embedded/apa-7th.yaml` | `apa-author-date-archetype-core` | carries only 7th-edition deltas |
| `styles/embedded/apa-8th.yaml` | `apa-author-date-archetype-core` | carries only 8th-edition deltas |
| `styles/journal-foo.yaml` | `apa-7th` | unchanged — wrappers still pin to a specific edition |

In both cases the rule stands: edition N+1 never `extends:` edition N.

External references: [Sonatype's hierarchy and inheritance best
practices](https://help.sonatype.com/en/hierarchy-and-inheritance-best-practices.html)
and the standard [composition-over-inheritance
argument](https://en.wikipedia.org/wiki/Composition_over_inheritance) — both
warn that deep inheritance chains create fragile-base-class drift, and
recommend that configuration hierarchies stay shallow with a clear single
source of truth.

---

## §2 What gets embedded; when editions drop off

**Recommendation.** Embed the current edition and the immediately prior
edition for each major family. Older editions move to the filesystem or are
removed.

### Rule

For each major style family the engine embeds two compiled-in editions:

- **N** — the current edition.
- **N−1** — the immediately prior edition.

Editions older than N−1 are removed from the `StyleBase` enum. If real user
demand exists they live as standalone YAML under `styles/legacy-editions/`;
otherwise they are removed outright.

Hidden `*-Core` family roots remain embedded as long as any embedded
profile or wrapper depends on them. When the last dependent ships an
inlined or reparented replacement, the core is removed in the same release.

A `StyleBase` variant must be embedded only when **all three** conditions
hold:

1. It is named in the current `scripts/report-core.js` priority cohort, **or** has projected
   non-trivial real-world usage based on the source CSL corpus.
2. At least one shipped style declares `extends: <key>` against it, **or** the
   variant is itself the canonical handle for the family (the user-facing
   Tier-1 base).
3. It passes the portfolio quality gate (`scripts/check-core-quality.js`
   against the current baseline).

Removal of a `StyleBase` variant is **out of scope** for the SoftDegrade
contract in `FORWARD_COMPATIBILITY.md`. A variant removal requires a major
schema bump and a release-notes entry naming the retirement.

### Rationale

- **Bounded binary size.** Compiled-in styles cost binary footprint in every
  language binding, WASM build, and FFI library. A bounded N + N−1 policy
  keeps that cost predictable as the corpus grows.
- **Matches publisher reality.** When a major publisher releases edition
  N+1, institutions typically transition over 2–4 years. Keeping N−1 covers
  in-flight manuscripts; keeping N−2 rarely does, and the legacy filesystem
  path remains for the rare case.
- **Avoids speculative retention.** Without telemetry we cannot prove which
  editions are used; "keep everything until usage drops" defers the decision
  indefinitely and is observed as worse than a simple cadence-based rule.

### Example: 2030 release with APA 8 shipping

| Edition | Status in `StyleBase` |
|---|---|
| APA 8 | embedded (current) |
| APA 7 | embedded (prior) |
| APA 6 | removed from `StyleBase`; available as `styles/legacy-editions/apa-6th.yaml` if demand exists |

---

## §3 Demoting a base when it drops out

**Recommendation.** Retirement is a one-PR coordinated workflow with CI
integrity checks. Never an ad-hoc edit.

### Rule

When a `StyleBase` variant is retired, every style in the corpus that declares
`extends: <retired-key>` must be rewritten in the same PR. The author chooses
one of three strategies per descendant:

- **Reparent.** Update `extends:` to point at a replacement base
  (typically N or a hidden archetype).
- **Inline.** Replace `extends:` with the resolved style emitted by
  `Style::into_resolved()`, dropping the inheritance edge.
- **Promote to archetype.** If two or more descendants share structure
  inherited from the retired base, introduce a hidden archetype core and have
  the descendants extend that.

A `deprecated:` field on `StyleBase` entries marks editions scheduled for
removal at a known future release. While the deprecation is active, the engine
emits a SoftDegrade warning at load time for every style that still extends
the retired key, with the planned removal version in the message.

CI integrity check: no `extends:` may reference a `StyleBase` variant that
does not exist. This protects against partial retirements.

### Rationale

- **No silent breakage.** The single-PR rule prevents a corpus state where
  some descendants are reparented and others are not.
- **Explicit cost of removal.** Removing a base touches the descendants the
  author chose to keep; that count is the real cost signal, surfaced in the
  diff and the bean.
- **Reproducible by tooling.** A future CLI helper
  (`citum registry retire <key> --strategy=…`) can produce the diff
  mechanically. The spec mandates the workflow; the helper is a follow-up
  bean (see §7) and not blocking.

### Example: retiring APA 6 when APA 8 ships

1. Audit: `grep -rl "extends: apa-6th" styles/` returns the descendant set.
2. For each descendant, pick reparent / inline / archetype.
3. PR rewrites all descendants, removes `StyleBase::Apa6th`, removes
   `styles/embedded/apa-6th.yaml`, bumps the major schema version, and adds a
   release-notes entry.
4. CI integrity check passes; oracle fidelity preserved.

---

## §4 Explicit style version field

**Recommendation.** Keep the fields that exist. Do not introduce a new style
version.

### Rule

The structured edition identity is the `StyleBase` *key*
(`apa-7th`, `chicago-notes-18th`, …). The display surface is `info.edition`
("7th", "18th edition"), free text by design. Engine compatibility is
declared by `info.citum_version`, an optional
`semver::VersionReq`-compatible string (see `citum_version` in
`crates/citum-schema-style/src/style/metadata.rs`).

Within-edition refinements (e.g. APA 7 errata, Chicago corrections issued
between manual editions) are tracked by Citum repo git history, not by a
style-document version number.

Proposals that add a third version field to the style document — e.g.
`info.version: "1.2.0"` — are rejected by this spec. They would duplicate
`STYLE_SCHEMA_VERSION` (the Citum schema version, `version.rs`) and
`info.citum_version` (the engine requirement), and would offer no signal that
git history does not already provide.

If real demand emerges for in-edition revision tracking, add a single
`info.edition_revision: Option<u32>` later. Do not introduce it speculatively.

### Rationale

- **One identity, one display, one engine requirement.** Three fields are
  enough; a fourth invites confusion about which is authoritative.
- **Edition keys already sort.** The `StyleBase` key carries the structured
  ordinal (`apa-7th` ≺ `apa-8th`); `info.edition` carries the human label.
  Tools that need to compare editions compare keys.
- **Git history is the revision log.** A style author who fixes a typo bumps
  no field; the git commit is the revision. This matches how the project
  already handles all other documents in the repo.

---

## §5 Tooling impact

The spec does not require any tooling change to be Active. The two changes
below are recommended and tracked as follow-up beans in §7.

### `citum-migrate`

Today edition detection is manual: `InfoExtractor::extract` in
`crates/citum-migrate/src/info_extractor.rs` sets `short_name` and `edition`
to `None` and relies on the operator to fill in well-known styles. Two enhancements follow from this policy:

- **Detect edition tokens in CSL source.** Parse `<info><title>` for tokens
  like "7th edition", "Seventh Edition", "18th ed." and route to the matching
  `StyleBase` automatically.
- **Preserve source edition by default.** When CSL declares an older edition
  than the available `StyleBase`, emit a structured "edition upgrade" warning
  and produce the older edition as a standalone style under
  `styles/legacy-editions/`. Do not silently upgrade to the current edition.
- **Edition discriminator on `FixupFamily`.** `base_detector::FixupFamily`
  currently distinguishes families (Apa, Chicago, …) but not editions. APA 6
  and APA 7 differ in publisher-place presentation and DOI formatting; the
  discriminator is needed before APA 6 input fidelity becomes measurable.

### Skills (`.claude/skills/`)

- `style-evolve` and its sub-skills (`style-maintain`,
  `style-migrate-enhance`) must record the chosen edition base in the bean
  spec when authoring a publisher or journal wrapper, and must never silently
  re-parent a wrapper across editions.
- `style-evolve` documents the §3 retirement workflow in its skill catalogue
  so retirements are reproducible.

No skill rewrites are required for this spec PR; only doc cross-references.

---

## §6 Acceptance criteria

- [x] `Status: Active`, `Version: 1.0`.
- [x] Each of §§1–5 states a **Recommendation**, **Rule**, **Rationale**, and
      worked **Example**.
- [x] Cross-references back from `STYLE_PRESET_ARCHITECTURE.md`,
      `STYLE_TAXONOMY.md`, and `FORWARD_COMPATIBILITY.md`.
- [x] No code changes in this PR.
- [x] Bean `csl26-atyp` opened for the spec, marked completed on merge.

---

## §7 Follow-up beans (deferred, not part of this PR)

| Title | Type | Trigger |
|---|---|---|
| Retire-base CLI workflow + CI integrity check | feature | needed before the first `StyleBase` variant is ever retired |
| `citum-migrate` edition detection from CSL title tokens | feature | small surface area; useful now |
| `FixupFamily` edition discriminator (Apa6 vs Apa7) | task | only when APA 6 input becomes a measured fidelity gap |
| Hidden archetype roots for APA / Chicago / MLA families | task | only when a second edition of one of those families is on deck |

None of these are required for the spec to be Active. The spec describes the
policy; the beans land mechanism as the need arrives.

---

## Changelog

- v1.1 (2026-07-28): Cross-referenced
  [STYLE_INHERITANCE.md](./STYLE_INHERITANCE.md), which owns merge semantics
  and portfolio tiering; edition workflow remains owned here.
- v1.0 (2026-05-21): Initial spec. Codifies edition inheritance shape
  (siblings under archetype), embedding retention (N + N−1), retirement
  workflow, version-field policy (no new field), and tooling impact.
