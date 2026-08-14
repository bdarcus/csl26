#!/usr/bin/env node
/**
 * Validate core style quality report against project gates.
 *
 * Gates:
 * 1) Hard fail when any core style fidelity drops below 1.0.
 * 2) Hard fail when any embedded-core style's exact-parity `passed` count
 *    drops below its recorded floor, or its `total` drifts from baseline
 *    (fixture-count change — see --parity-baseline).
 * 3) Warn (non-failing by default) on SQI metric failures and notable drift.
 *
 * Usage:
 *   node scripts/check-core-quality.js --report /tmp/core-report.json \
 *     --baseline scripts/report-data/core-quality-baseline.json \
 *     --parity-baseline scripts/report-data/embedded-parity-baseline.json
 */

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

const DEFAULTS = {
  maxConcisionDrop: 8,
  maxPresetDrop: 10,
  strictWarnings: false,
  crossEntryAudit: false,
};

function parseArgs(argv) {
  const args = {
    report: null,
    baseline: null,
    parityBaseline: null,
    parityAdjudication: null,
    maxConcisionDrop: DEFAULTS.maxConcisionDrop,
    maxPresetDrop: DEFAULTS.maxPresetDrop,
    strictWarnings: DEFAULTS.strictWarnings,
    crossEntryAudit: DEFAULTS.crossEntryAudit,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--report') {
      args.report = argv[++i];
    } else if (arg === '--baseline') {
      args.baseline = argv[++i];
    } else if (arg === '--parity-baseline') {
      args.parityBaseline = argv[++i];
    } else if (arg === '--parity-adjudication') {
      args.parityAdjudication = argv[++i];
    } else if (arg === '--max-concision-drop') {
      args.maxConcisionDrop = Number(argv[++i]);
    } else if (arg === '--max-preset-drop') {
      args.maxPresetDrop = Number(argv[++i]);
    } else if (arg === '--strict-warnings') {
      args.strictWarnings = true;
    } else if (arg === '--cross-entry-audit') {
      args.crossEntryAudit = true;
    } else if (arg === '-h' || arg === '--help') {
      printUsage();
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  if (!args.report) {
    throw new Error('Missing required --report path');
  }

  return args;
}

function printUsage() {
  // Keep output short for CI logs.
  console.log(
    'Usage: node scripts/check-core-quality.js --report <path> [--baseline <path>] ' +
      '[--parity-baseline <path>] [--parity-adjudication <path>]'
  );
}

function readJson(filePath) {
  const absolute = path.resolve(filePath);
  const raw = fs.readFileSync(absolute, 'utf8');
  return JSON.parse(raw);
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function annotateWarning(message) {
  if (process.env.GITHUB_ACTIONS === 'true') {
    console.warn(`::warning::${message}`);
  } else {
    console.warn(`WARN: ${message}`);
  }
}

function annotateError(message) {
  if (process.env.GITHUB_ACTIONS === 'true') {
    console.error(`::error::${message}`);
  } else {
    console.error(`ERROR: ${message}`);
  }
}

function styleMetric(style, keyPath) {
  let current = style;
  for (const key of keyPath) {
    if (!current || typeof current !== 'object') return null;
    current = current[key];
  }
  return typeof current === 'number' ? current : null;
}

function run() {
  let args;
  try {
    args = parseArgs(process.argv.slice(2));
  } catch (error) {
    annotateError(error.message);
    printUsage();
    process.exit(2);
  }

  // Optional cross-entry parity audit (subsequent-author-substitute + disambiguation)
  if (args.crossEntryAudit) {
    const auditScript = path.resolve(__dirname, 'audit-cross-entry-parity.js');
    try {
      const auditOutput = execFileSync(process.execPath, [auditScript, '--json'], {
        encoding: 'utf8',
      });
      const auditResult = JSON.parse(auditOutput);
      if (auditResult.summary.offenders > 0) {
        for (const offender of auditResult.offenders) {
          for (const issue of offender.issues) {
            annotateError(`Cross-entry parity [${offender.styleId}]: ${issue}`);
          }
        }
        process.exit(1);
      }
      console.log(
        `Cross-entry parity audit passed (${auditResult.summary.checked} checked, 0 offenders)`
      );
    } catch (err) {
      if (err.status === 1 && err.stdout) {
        // audit exited 1 = offenders found — parse JSON and emit per-issue annotations
        try {
          const auditResult = JSON.parse(err.stdout);
          for (const offender of auditResult.offenders || []) {
            for (const issue of offender.issues || []) {
              annotateError(`Cross-entry parity [${offender.styleId}]: ${issue}`);
            }
          }
        } catch (_parseErr) {
          // stdout was not JSON — fall back to raw output
          annotateError(`Cross-entry parity audit failed: ${err.stdout}`);
        }
        process.exit(1);
      }
      annotateError(`Failed to run cross-entry parity audit: ${err.message}`);
      process.exit(2);
    }
  }

  let report;
  try {
    report = readJson(args.report);
  } catch (error) {
    annotateError(`Failed to read report JSON: ${error.message}`);
    process.exit(2);
  }

  const styles = Array.isArray(report.styles) ? report.styles : [];
  if (styles.length === 0) {
    annotateError('Report has no styles; cannot evaluate quality gates');
    process.exit(2);
  }

  const styleMap = new Map(styles.map((style) => [style.name, style]));
  let baselineStyleNames = null;
  let baseline = null;

  if (args.baseline) {
    try {
      baseline = readJson(args.baseline);
    } catch (error) {
      annotateWarning(`Baseline unavailable (${args.baseline}): ${error.message}`);
    }
  }

  if (baseline && baseline.styles && typeof baseline.styles === 'object') {
    baselineStyleNames = Object.keys(baseline.styles);
  }

  // Exact-parity floor gate: a style's exactParity.passed count may never drop
  // below the value recorded in --parity-baseline (per-style, set at current
  // measurement — see docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md).
  // exactParity.total must match exactly so a fixture-count change (which shifts
  // the denominator) fails loudly instead of silently moving the floor, mirroring
  // check-oracle-regression.js's fixture-drift guard for the top-10 set.
  let parityBaseline = null;
  if (args.parityBaseline) {
    try {
      parityBaseline = readJson(args.parityBaseline);
    } catch (error) {
      annotateError(`Failed to read parity baseline (${args.parityBaseline}): ${error.message}`);
      process.exit(2);
    }
    if (!isRecord(parityBaseline) || !isRecord(parityBaseline.styles)) {
      annotateError('Invalid parity baseline: expected an object with an object "styles" field');
      process.exit(2);
    }
  }

  const parityBaselineStyleNames = parityBaseline ? Object.keys(parityBaseline.styles) : [];
  const missingParityBaselineStyles = parityBaselineStyleNames.filter((name) => !styleMap.has(name));
  const parityRegressions = [];
  const parityTotalDrift = [];
  const parityUnmeasurable = [];
  for (const name of parityBaselineStyleNames) {
    const style = styleMap.get(name);
    if (!style) continue;
    // A style with an oracle/quality error produced a partial or failed
    // measurement this run (e.g. a stale/missing citeproc snapshot returning
    // exit 2) — its exactParity numbers are not trustworthy evidence of a
    // real regression. Report it as its own failure instead of letting a
    // measurement gap masquerade as fixture drift or a parity regression.
    const measurementError = style.error || style.qualityBreakdown?.error;
    if (measurementError) {
      parityUnmeasurable.push({ name, error: measurementError });
      continue;
    }
    const baselineParity = parityBaseline.styles[name]?.exactParity;
    const currentParity = style.exactParity;
    if (!baselineParity || typeof baselineParity.passed !== 'number') continue;
    const currentPassed = Number(currentParity?.passed ?? NaN);
    const currentTotal = Number(currentParity?.total ?? NaN);
    if (Number.isFinite(baselineParity.total) && currentTotal !== baselineParity.total) {
      parityTotalDrift.push({ name, baselineTotal: baselineParity.total, currentTotal });
      continue;
    }
    if (!Number.isFinite(currentPassed) || currentPassed < baselineParity.passed) {
      parityRegressions.push({ name, baselinePassed: baselineParity.passed, currentPassed });
    }
  }

  let parityAdjudication = null;
  if (args.parityAdjudication) {
    try {
      parityAdjudication = readJson(args.parityAdjudication);
    } catch (error) {
      annotateError(`Failed to read parity adjudication ledger (${args.parityAdjudication}): ${error.message}`);
      process.exit(2);
    }
    if (!isRecord(parityAdjudication) || !isRecord(parityAdjudication.entries)) {
      annotateError('Invalid parity adjudication ledger: expected an object with an object "entries" field');
      process.exit(2);
    }
  }
  const adjudicationEntries = [];
  if (parityAdjudication) {
    for (const [styleName, styleEntries] of Object.entries(parityAdjudication.entries)) {
      if (!isRecord(styleEntries)) {
        annotateError(`Invalid parity adjudication entries for ${styleName}: expected an object`);
        process.exit(2);
      }
      for (const [entryId, entry] of Object.entries(styleEntries)) {
        if (!isRecord(entry)) {
          annotateError(`Invalid parity adjudication entry ${styleName}/${entryId}: expected an object`);
          process.exit(2);
        }
        adjudicationEntries.push(entry);
      }
    }
  }
  const unclearCount = adjudicationEntries.filter((entry) => entry?.state === 'unclear').length;
  const citumCorrectCount = adjudicationEntries.filter((entry) => entry?.state === 'citum-correct').length;
  const citeprocCorrectCount = adjudicationEntries.filter((entry) => entry?.state === 'citeproc-correct').length;
  for (const entry of adjudicationEntries) {
    if (entry?.state === 'citum-correct' && (!entry.authority || !entry.confirmedBy)) {
      annotateError(
        `Invalid parity adjudication entry: citum-correct requires "authority" and "confirmedBy" (user-only state)`
      );
      process.exit(2);
    }
    if (!['citeproc-correct', 'unclear', 'citum-correct'].includes(entry.state)) {
      annotateError(`Invalid parity adjudication state: ${entry.state ?? 'missing'}`);
      process.exit(2);
    }
  }

  const fidelityTargets = baselineStyleNames
    ? baselineStyleNames
        .map((name) => styleMap.get(name))
        .filter(Boolean)
    : styles;
  const missingBaselineStyles = baselineStyleNames
    ? baselineStyleNames.filter((name) => !styleMap.has(name))
    : [];
  const fidelityFailures = fidelityTargets.filter((style) => Number(style.fidelityScore) < 1.0);
  const metricFailures = styles.filter((style) => {
    if (style.error) return true;
    if (!style.qualityBreakdown) return true;
    if (style.qualityBreakdown.error) return true;
    return false;
  });

  let warningCount = 0;
  for (const style of metricFailures) {
    warningCount += 1;
    annotateWarning(
      `SQI metric failure in ${style.name}: ${style.error || style.qualityBreakdown?.error || 'unknown'}`
    );
  }

  // Positional bibliography-order check (csl26-7u16): a diagnostic warning,
  // not a hard gate, until a corpus sweep establishes the real scale. An
  // *explained* mismatch (a registered divergence such as div-004 accounts
  // for it) is not reported here — only unexplained ones, which indicate a
  // real, uninvestigated citum/citeproc-js ordering disagreement.
  const unexplainedOrderMismatches = styles.filter(
    (style) => style.bibliographyOrderMismatch?.mismatch && !style.bibliographyOrderMismatch?.explained
  );
  for (const style of unexplainedOrderMismatches) {
    warningCount += 1;
    annotateWarning(
      `Unexplained bibliography order mismatch in ${style.name}: Citum and citeproc-js render the same ` +
        `bibliography entries in a different sequence, with no registered divergence explaining it`
    );
  }

  if (baseline && baseline.styles && typeof baseline.styles === 'object') {
      for (const [name, baselineMetrics] of Object.entries(baseline.styles)) {
        const style = styleMap.get(name);
        if (!style) continue;

        const currentConcision = styleMetric(style, ['qualityBreakdown', 'subscores', 'concision', 'score']);
        const currentPreset = styleMetric(style, ['qualityBreakdown', 'subscores', 'presetUsage', 'score']);
        const baselineConcision = Number(baselineMetrics.concision);
        const baselinePreset = Number(baselineMetrics.presetUsage);

        if (Number.isFinite(currentConcision) && Number.isFinite(baselineConcision)) {
          const delta = currentConcision - baselineConcision;
          if (delta < -Math.abs(args.maxConcisionDrop)) {
            warningCount += 1;
            annotateWarning(
              `Concision regression in ${name}: ${currentConcision.toFixed(1)} (baseline ${baselineConcision.toFixed(1)}, delta ${delta.toFixed(1)})`
            );
          }
        }

        if (Number.isFinite(currentPreset) && Number.isFinite(baselinePreset)) {
          const delta = currentPreset - baselinePreset;
          if (delta < -Math.abs(args.maxPresetDrop)) {
            warningCount += 1;
            annotateWarning(
              `Preset usage regression in ${name}: ${currentPreset.toFixed(1)} (baseline ${baselinePreset.toFixed(1)}, delta ${delta.toFixed(1)})`
            );
          }
        }
      }
  }

  if (missingBaselineStyles.length > 0) {
    for (const name of missingBaselineStyles) {
      annotateError(`Missing baseline core style in report: ${name}`);
    }
    process.exit(1);
  }

  if (fidelityFailures.length > 0) {
    for (const style of fidelityFailures) {
      annotateError(`Core fidelity gate failed for ${style.name}: ${style.fidelityScore}`);
    }
    process.exit(1);
  }

  if (missingParityBaselineStyles.length > 0) {
    for (const name of missingParityBaselineStyles) {
      annotateError(`Missing exact-parity baseline core style in report: ${name}`);
    }
    process.exit(1);
  }

  if (parityUnmeasurable.length > 0) {
    for (const entry of parityUnmeasurable) {
      annotateError(
        `Exact-parity not measurable for ${entry.name} (re-run, do not trust this run's numbers): ${entry.error}`
      );
    }
    process.exit(1);
  }

  if (parityTotalDrift.length > 0) {
    for (const drift of parityTotalDrift) {
      annotateError(
        `Exact-parity fixture drift for ${drift.name}: total ${drift.currentTotal} != baseline total ${drift.baselineTotal} ` +
          `(regenerate scripts/report-data/embedded-parity-baseline.json if this is an intended fixture change)`
      );
    }
    process.exit(1);
  }

  if (parityRegressions.length > 0) {
    for (const regression of parityRegressions) {
      annotateError(
        `Exact-parity gate failed for ${regression.name}: passed=${regression.currentPassed} ` +
          `< baseline floor ${regression.baselinePassed}`
      );
    }
    process.exit(1);
  }

  if (warningCount > 0 && args.strictWarnings) {
    annotateError(`Quality warnings elevated to failure (${warningCount})`);
    process.exit(1);
  }

  if (unclearCount > 0) {
    annotateWarning(
      `${unclearCount} parity residual(s) recorded as "unclear" in the adjudication ledger — escalate to the user, do not exclude unilaterally`
    );
  }

  console.log(
    `Core quality gate passed (${styles.length} styles, fidelity=1.0 for all, ` +
      `exact-parity>=baseline for ${parityBaselineStyleNames.length} embedded-core styles, ` +
      `adjudication: ${citeprocCorrectCount} citeproc-correct, ${citumCorrectCount} citum-correct, ${unclearCount} unclear, ` +
      `warnings=${warningCount})`
  );
}

run();
