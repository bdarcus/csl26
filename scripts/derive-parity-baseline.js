#!/usr/bin/env node
/**
 * scripts/derive-parity-baseline.js
 *
 * Derives scripts/report-data/embedded-parity-baseline.json from a
 * `report-core.js --all-features` report. The baseline previously had no
 * generator — it was hand-assembled from a report run — which made it one of
 * the pinned artifacts left out of scope when tests/fixtures/ changes (see
 * docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md).
 *
 * Output shape must match the existing committed file field-for-field:
 * `.github/workflows/fidelity.yml` and `check-core-quality.js
 * --parity-baseline` both read `styles[].exactParity.{passed,total}`.
 *
 * Usage:
 *   node scripts/report-core.js --all-features > /tmp/core-report.json
 *   node scripts/derive-parity-baseline.js \
 *     --report /tmp/core-report.json \
 *     --out scripts/report-data/embedded-parity-baseline.json
 */

'use strict';

const fs = require('fs');
const path = require('path');

const PROJECT_ROOT = path.resolve(__dirname, '..');
const DEFAULT_OUT = path.join(PROJECT_ROOT, 'scripts', 'report-data', 'embedded-parity-baseline.json');

// Preserved verbatim from the existing committed baseline so regenerating it
// doesn't silently reword the contract this file is read under.
const PURPOSE =
  'Hard per-style exact-parity floor-gate baseline for the embedded tier, ' +
  'consumed by scripts/check-core-quality.js --parity-baseline (see ' +
  'docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md). It also ' +
  'records headline fidelity. Sub-1.0 fidelity ratchets are tracked ' +
  'separately per-style in scripts/report-data/verification-policy.yaml ' +
  '(min_pass_rate floors). Regenerate this file after each parity-tuning ' +
  'wave; the gate never goes backward on passed counts or total ' +
  '(fixture-drift guard).';

function parseArgs(argv) {
  const opts = { reportPath: null, outPath: DEFAULT_OUT };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--report') {
      opts.reportPath = path.resolve(argv[++i]);
    } else if (argv[i] === '--out') {
      opts.outPath = path.resolve(argv[++i]);
    }
  }
  return opts;
}

/**
 * True if a style's report record can't back a ratcheted floor: report-core.js
 * omits `exactParity` entirely on its error path, and defaulting a missing
 * measurement to 0 would silently reset that style's floor to 0/0 instead of
 * failing loudly.
 */
function isUnmeasurable(style) {
  return Boolean(style.error) || typeof style.exactParity?.total !== 'number';
}

/** Build the baseline document from a report-core.js report object. */
function deriveParityBaseline(report) {
  const embeddedStyles = (report.styles || [])
    .filter((style) => style.tier === 'embedded')
    .sort((a, b) => a.name.localeCompare(b.name));

  const unmeasurable = embeddedStyles.filter(isUnmeasurable);
  if (unmeasurable.length > 0) {
    throw new Error(
      `Refusing to derive a baseline: no exact-parity measurement for ` +
      `${unmeasurable.map((s) => s.name).join(', ')}. Fix the underlying ` +
      `report error and re-run rather than writing a 0/0 floor.`
    );
  }

  const styles = {};
  for (const style of embeddedStyles) {
    styles[style.name] = {
      tier: style.tier,
      fidelityScore: style.fidelityScore,
      qualityScore: style.qualityScore,
      citations: {
        passed: style.citations?.passed ?? 0,
        total: style.citations?.total ?? 0,
      },
      bibliography: {
        passed: style.bibliography?.passed ?? 0,
        total: style.bibliography?.total ?? 0,
      },
      exactParity: {
        passed: style.exactParity.passed,
        total: style.exactParity.total,
        rate: style.exactParity.rate ?? null,
      },
    };
  }

  return {
    generated: report.generated,
    commit: report.commit,
    source: 'scripts/report-core.js',
    purpose: PURPOSE,
    styles,
  };
}

function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (!opts.reportPath) {
    process.stderr.write(
      'Usage: node scripts/derive-parity-baseline.js --report <report.json> [--out <path>]\n'
    );
    process.exit(1);
  }

  const report = JSON.parse(fs.readFileSync(opts.reportPath, 'utf8'));
  let baseline;
  try {
    baseline = deriveParityBaseline(report);
  } catch (err) {
    process.stderr.write(`${err.message}\n`);
    process.exit(1);
  }

  fs.mkdirSync(path.dirname(opts.outPath), { recursive: true });
  fs.writeFileSync(opts.outPath, JSON.stringify(baseline, null, 2) + '\n', 'utf8');
  process.stderr.write(`Wrote ${embeddedCount(baseline)} embedded-tier styles to ${opts.outPath}\n`);
}

function embeddedCount(baseline) {
  return Object.keys(baseline.styles).length;
}

if (require.main === module) {
  main();
}

module.exports = { deriveParityBaseline, parseArgs };
