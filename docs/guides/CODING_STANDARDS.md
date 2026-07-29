# Citum Coding Standards

## Test Style

Use the right test style based on scope and intent:

| Scenario | Location | Style |
|----------|----------|-------|
| Pure logic, type conversions, single-function correctness | `#[cfg(test)]` inline in the module | Plain `#[test]` with descriptive names |
| Single-scenario integration check | `tests` directory | Plain `#[test]` |
| User-observable behavior with multiple parameterised cases | `tests` directory | `#[rstest]` with `given_…_when_…_then_…` naming |

**BDD naming rule** (`given_…_when_…_then_…`) is reserved for integration tests in `tests` that exercise a cross-module behavior path and have at least two parameterised variants. Do not apply BDD naming to inline unit tests or single-scenario integration tests — it adds noise without signal.

```rust
// Unit test — inline, plain name
#[cfg(test)]
mod tests {
    #[test]
    fn strips_trailing_period_from_title() { … }
}

// Integration test — single scenario, plain name
#[test]
fn bibliography_emits_doi_when_present() { … }

// Integration test — parameterised behavior, BDD name
#[rstest]
#[case("en-US", "Smith")]
#[case("de-DE", "Smith")]
fn given_a_multilingual_author_when_rendering_a_citation_then_family_name_is_used(
    #[case] locale: &str,
    #[case] expected: &str,
) { … }
```

## Test Independence

Tests must prove the behavior from an independent source, not mirror the current
implementation output.

- For behavior fixes, the test should fail against the old behavior unless the
  PR documents why the old behavior cannot be reproduced locally.
- Prefer expected values from literals, fixtures, oracle output, specs, or
  registered divergence decisions. Do not derive `expected` from `actual`,
  `result`, `rendered`, or other values produced by the code under test.
- **Never use `contains()` in assertions on rendered output.** Use `assert_eq!`
  with the full expected string. If a partial match is genuinely needed (e.g.,
  format-agnostic URL presence or locale-variable text), the substring must be
  ≥ 30 characters and the test name must signal it (`_contains_` or
  `_partial_`). Short `contains()` checks — anything under 30 chars — verify
  almost nothing and mask real regressions. Enforced by
  `audit-rust-review-smells.py` (`render-output-contains-assertion`).
- Fixture changes must explain the missing shape they add. When fixture data
  changes expected behavior, pair it with the smallest Rust or oracle check that
  exercises the new shape.
- Ignored tests need a reason and a tracking path. Do not leave `#[ignore]`
  as an unreviewed way to keep the suite green.

Use the advisory review-smell audit before opening PRs that touch Rust tests:

```bash
python3 scripts/audit-rust-review-smells.py --changed
```

## What makes a test worth keeping

This is the shared bar every test in this repo is held to. The
[`test-soundness-review`](../../.skills/test-soundness-review/SKILL.md) skill,
the [`test-coverage`](../../.claude/skills/test-coverage/SKILL.md) skill, and
human reviewers all classify tests against these three expectations, so the
words mean the same thing everywhere. A test that fails any of them is a defect,
not just a style nit.

1. **It makes an independent behavioural claim.** The expected value comes from
   a source other than the code under test — a literal, fixture, oracle output,
   spec, or registered divergence. (Full rule above in § "Test Independence".)
   A test that mirrors current output proves only that the code does what it
   currently does.

2. **It isn't there just for its own sake.** Redundant, tautological, and
   coverage-theatre tests dilute signal and cost maintenance. Prefer **deleting
   or merging** them over keeping them. A test is **redundant** when it adds no
   dimension a sibling doesn't already cover:
   - a near-duplicate on the same fixture and same assertion dimension (no new
     type, field shape, position, or edge condition);
   - a tautology — `assert!(true)`, asserting a literal you just constructed, or
     round-tripping a value through no transformation;
   - a test of the language or a library rather than Citum behaviour;
   - coverage theatre — exists only to touch a line, with no observable
     behaviour asserted.

3. **It is classifiable against a spec.** You can point to the spec section it
   verifies and say whether it passes or fails *that section*. If the governing
   spec is ambiguous, contradictory, or silent on the behaviour, the spec is the
   bug — fix the spec (or escalate the decision) rather than letting the test
   silently pick one reading. The soundness skill halts on such cases by design.

State of conformance across specs is tracked in
[`docs/architecture/TEST_SOUNDNESS_STATUS.md`](../architecture/TEST_SOUNDNESS_STATUS.md).

## String Ownership

Owned strings are normal at data model, serialization, FFI, and output
boundaries. They are suspicious when allocated only for lookup, comparison, or
short-lived formatting inside production paths.

- Prefer borrowed `&str` values for lookups, comparisons, and parser decisions.
  For example, avoid allocating a short `String` only to call `contains`.
- Allocate when constructing owned schema/data values, crossing FFI or serde
  boundaries, returning owned rendered output, or storing values beyond the
  borrowed input lifetime.
- Treat raw `.to_string()` counts as a triage signal only. Review categorized
  findings by path kind, especially `hot-path` production findings.
- Do not enable broad noisy Clippy restriction lints as hard failures without a
  baseline pass. Use them to inform targeted review.
- Hot-path allocation reductions are performance work: run before/after
  benchmarks and report the deltas when changing citation rendering,
  bibliography processing, style parsing, name formatting, date formatting, or
  substitution logic.

Use the advisory audit to find suspicious patterns:

```bash
python3 scripts/audit-rust-review-smells.py --all
python3 scripts/audit-rust-review-smells.py --all --json
```

## Serde Attributes Checklist

- Use `#[serde(rename_all = "kebab-case")]` for YAML/JSON compatibility
- Use `#[non_exhaustive]` for extensible enums
- Use `#[serde(deny_unknown_fields)]` on untagged enum variants to prevent misparse
- Prefer `Option<T>` with `skip_serializing_if` for optional fields
- Add `#[serde(flatten)]` for inline rendering options
- Comment non-obvious logic; reference CSL 1.0 spec where applicable
- Never deserialize `Style` directly with `serde_yaml::from_slice`/`from_str`,
  `serde_json::from_slice`, or `ciborium::de::from_reader`. Always go through
  `Style::from_yaml_str`, `Style::from_yaml_bytes`, or
  `Style::from_document_bytes` (`crates/citum-schema-style/src/style/model.rs`)
  so `raw_yaml` is populated — every `extends` overlay merge reads null-clear
  intent from that raw tree, not from typed `Option::None`. A direct bypass
  compiles and passes ordinary round-trip tests while silently breaking
  explicit-`null` inherited-field clearing (`csl26-j3zy`); a guard test
  (`crates/citum-schema-style/tests/raw_ingest_guard.rs`) forbids the
  turbofish form of this bypass but cannot catch one hidden behind
  return-type inference, so review new style load paths by hand.

## Verification Requirements

Different types of changes require different levels of verification to maintain quality while optimizing for development velocity.

| Change Type | Verification Required |
|-------------|----------------------|
| Config/Docs/Styles (<5 lines) | Syntax check only |
| Bugfixes (non-hot path) | `cargo fmt && cargo clippy && cargo test` |
| New features (cold path) | `cargo fmt && cargo clippy && cargo test` |
| Hot path refactoring | Pre-commit checks + **Benchmarks** (before/after) |
| Algorithm changes | Pre-commit checks + **Benchmarks** (regression check) |
| Format/Parser optimization | Pre-commit checks + **Benchmarks** (validated claim) |
| Performance claims | Pre-commit checks + **Benchmarks** (evidence-based) |

**Hot paths:** citation rendering, bibliography processing, style parsing, name formatting, date formatting, substitution logic

## Benchmark Workflow (Required for Performance/Refactor Work)

Benchmarks are **required** for performance-sensitive changes and hot-path refactors. Use the provided helper script to automate comparison.

```bash
# 1. Capture baseline (on main or before changes)
./scripts/bench-check.sh capture baseline

# 2. Make performance changes
# ... implement optimization ...

# 3. Compare after changes
./scripts/bench-check.sh compare baseline after

# 4. Include relevant deltas in commit message body
```

**Available benchmarks:**
- `cargo bench --bench rendering` - Citation/bibliography processing (APA-focused)
- `cargo bench --bench formats` - YAML/JSON/CBOR deserialization

Baseline files are stored in `.bench-baselines/` (gitignored, local-only). Use `critcmp` for manual comparisons if needed.
The legacy positional forms still work for now:
`./scripts/bench-check.sh baseline` and
`./scripts/bench-check.sh baseline after`.
