#!/usr/bin/env node
/**
 * Validate core style quality report against project gates.
 *
 * Gates:
 * 1) Hard fail when any core style fidelity drops below 1.0.
 * 2) Warn (non-failing by default) on SQI metric failures and notable drift.
 *
 * Usage:
 *   node scripts/check-core-quality.js --report /tmp/core-report.json \
 *     --baseline scripts/report-data/core-quality-baseline.json
 */

const fs = require('fs');
const path = require('path');

const DEFAULTS = {
  maxConcisionDrop: 8,
  maxPresetDrop: 10,
  strictWarnings: false,
};

function parseArgs(argv) {
  const args = {
    report: null,
    baseline: null,
    maxConcisionDrop: DEFAULTS.maxConcisionDrop,
    maxPresetDrop: DEFAULTS.maxPresetDrop,
    strictWarnings: DEFAULTS.strictWarnings,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--report') {
      args.report = argv[++i];
    } else if (arg === '--baseline') {
      args.baseline = argv[++i];
    } else if (arg === '--max-concision-drop') {
      args.maxConcisionDrop = Number(argv[++i]);
    } else if (arg === '--max-preset-drop') {
      args.maxPresetDrop = Number(argv[++i]);
    } else if (arg === '--strict-warnings') {
      args.strictWarnings = true;
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
  console.log('Usage: node scripts/check-core-quality.js --report <path> [--baseline <path>]');
}

function readJson(filePath) {
  const absolute = path.resolve(filePath);
  const raw = fs.readFileSync(absolute, 'utf8');
  return JSON.parse(raw);
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

  if (warningCount > 0 && args.strictWarnings) {
    annotateError(`Quality warnings elevated to failure (${warningCount})`);
    process.exit(1);
  }

  console.log(
    `Core quality gate passed (${styles.length} styles, fidelity=1.0 for all, warnings=${warningCount})`
  );
}

run();
