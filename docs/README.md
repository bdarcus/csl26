# Docs Map

This directory is organized for progressive disclosure: start with generated
status, then drill into strategy and implementation details only as needed.

## Start Here (Canonical)

1. `scripts/report-core.js` - canonical live style fidelity and SQI report source.
2. [`compat.html`](https://citum.github.io/citum-core/compat.html) - generated compatibility snapshot, including official supplemental rich benchmark evidence for configured styles.
3. `behavior-report.html` - published engine behavior coverage page generated in CI, with source locations for the selected engine behavior suites.
4. `migration-behavior-report.html` - published CSL-to-Citum migration behavior coverage page generated in CI, with source locations for reviewer-facing `citum-migrate` suites.
5. [`guides/RENDERING_WORKFLOW.md`](./guides/RENDERING_WORKFLOW.md) - operational rendering and verification workflow.
6. [`architecture/ROADMAP.md`](./architecture/ROADMAP.md) - archival strategic direction and phase sequencing.

Generate the live JSON status locally with `node scripts/report-core.js > /tmp/core-report.json`.
Generate the compatibility snapshot locally with `node scripts/report-core.js --write-html`.
Generate an official single-style report for workflow consumption with `node scripts/report-core.js --style <style-name>`.
Generate the local behavior report with `./scripts/test-report.sh`.
Generate the local CSL-to-Citum migration behavior report with `./scripts/migration-test-report.sh`.

## Active Guides

1. [`guides/CODING_STANDARDS.md`](./guides/CODING_STANDARDS.md)
2. [`guides/STYLE_EVOLVE_WORKFLOW.md`](./guides/STYLE_EVOLVE_WORKFLOW.md)
3. [`guides/DOMAIN_EXPERT.md`](./guides/DOMAIN_EXPERT.md)
4. [`guides/style-author-guide.md`](./guides/style-author-guide.md)

## Architecture (Entry)

Use [`architecture/README.md`](./architecture/README.md) to navigate active
architecture docs versus historical snapshots and audits.

## Specifications & Policies

- [`specs/`](./specs/) — Feature/design specifications (create before implementing non-trivial features)
- [`policies/`](./policies/) — Active behavioral rules for agents and contributors
- [`reference/`](./reference/) — Domain reference tables, including the generated data model
  ([`reference/DATA_MODEL.md`](./reference/DATA_MODEL.md)) and ingest mapping docs; see [`reference.html`](https://citum.github.io/citum-core/reference.html) for the browsable index
- [`guides/DOCUMENT_CLASSIFICATION.md`](./guides/DOCUMENT_CLASSIFICATION.md) — how to decide whether an existing doc belongs in specs, architecture, or policies

## Historical Snapshot Policy

Date-stamped architecture docs (for example `*_2026-02-21.md`) are snapshots of
specific execution windows. They are useful for audit history but are not
canonical status sources.
