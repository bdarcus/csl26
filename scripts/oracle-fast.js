#!/usr/bin/env node
/**
 * scripts/oracle-fast.js
 *
 * Snapshot-based oracle: loads pre-computed citeproc-js output from
 * tests/snapshots/csl/<style>.json and diffs against the live Citum renderer.
 * Drop-in replacement for oracle.js in report-core.js non-migrate runs.
 *
 * Requires a current snapshot. Exits 2 if snapshot is missing, 3 if stale.
 * To regenerate: node scripts/oracle-snapshot.js <style.csl>
 *
 * Usage:
 *   node scripts/oracle-fast.js <style.csl>
 *   node scripts/oracle-fast.js <style.csl> --json
 *   node scripts/oracle-fast.js <style.csl> --verbose
 *
 * Exit codes:
 *   0 — all citations and bibliography match
 *   1 — mismatches found
 *   2 — snapshot file missing
 *   3 — snapshot stale (fixture or CSL changed)
 */

'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const {
  compareText,
  normalizeText,
  parseComponents,
  compareComponents,
  compareCitationComponents,
  analyzeOrdering,
  findRefDataForEntry,
} = require('./oracle-utils');
const {
  renderWithCitumProcessor,
  bibliographyComparisonMatches,
  matchBibliographyEntries: matchLiveBibliographyEntries,
} = require('./oracle');
const { attachRegisteredDivergenceAdjustments } = require('./lib/oracle-divergences');

const PROJECT_ROOT = path.resolve(__dirname, '..');
const SNAPSHOT_DIR = path.join(PROJECT_ROOT, 'tests', 'snapshots', 'csl');
const DEFAULT_REFS_FIXTURE = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'references-expanded.json');
const DEFAULT_CITATIONS_FIXTURE = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'citations-expanded.json');

const STRICT_CITATION_IDS = new Set([
  'et-al-single-long-list',
  'disambiguate-add-names-et-al',
  'disambiguate-year-suffix',
  'et-al-with-locator',
]);

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  const opts = {
    stylePath: null,
    jsonOutput: false,
    verbose: false,
    caseSensitive: true,
    allFeatures: false,
    citumBin: null,
    locale: null,
    refsFixture: DEFAULT_REFS_FIXTURE,
    citationsFixture: DEFAULT_CITATIONS_FIXTURE,
  };
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === '--json') opts.jsonOutput = true;
    else if (a === '--verbose') opts.verbose = true;
    else if (a === '--case-sensitive') opts.caseSensitive = true;
    else if (a === '--case-insensitive') opts.caseSensitive = false;
    else if (a === '--all-features') opts.allFeatures = true;
    else if (a === '--citum-bin') opts.citumBin = path.resolve(args[++i]);
    else if (a === '--locale') opts.locale = args[++i];
    else if (a === '--refs-fixture') opts.refsFixture = path.resolve(args[++i]);
    else if (a === '--citations-fixture') opts.citationsFixture = path.resolve(args[++i]);
    else if (!a.startsWith('--') && !opts.stylePath) opts.stylePath = path.resolve(a);
  }
  return opts;
}

// ---------------------------------------------------------------------------
// Fixture hashing (must match oracle-snapshot.js)
// ---------------------------------------------------------------------------

function fixtureHash(refsFixture, citationsFixture) {
  const h = crypto.createHash('sha256');
  h.update(fs.readFileSync(refsFixture));
  h.update(fs.readFileSync(citationsFixture));
  return h.digest('hex').slice(0, 16);
}

// ---------------------------------------------------------------------------
// Snapshot loading with staleness guard
// ---------------------------------------------------------------------------

class SnapshotMissingError extends Error {}
class SnapshotStaleError extends Error {}

/**
 * Load snapshot for a CSL style, validating the fixture_hash.
 * Returns the parsed snapshot or throws SnapshotMissingError / SnapshotStaleError.
 */
function loadSnapshot(stylePath, refsFixture, citationsFixture) {
  const styleName = path.basename(stylePath, '.csl');
  const snapPath = path.join(SNAPSHOT_DIR, `${styleName}.json`);

  if (!fs.existsSync(snapPath)) {
    throw new SnapshotMissingError(
      `Snapshot missing for ${styleName}.\n` +
      `  Run: node scripts/oracle-snapshot.js ${stylePath}`
    );
  }

  const snap = JSON.parse(fs.readFileSync(snapPath, 'utf8'));
  const currentHash = fixtureHash(refsFixture, citationsFixture);

  if (snap.fixture_hash !== currentHash) {
    throw new SnapshotStaleError(
      `Snapshot stale for ${styleName} (fixture changed).\n` +
      `  Run: node scripts/oracle-snapshot.js ${stylePath}`
    );
  }

  return snap;
}

// ---------------------------------------------------------------------------
// Comparison logic (mirrors oracle.js)
// ---------------------------------------------------------------------------

function equivalentText(a, b, options = {}) {
  return compareText(a, b, options).match;
}

function extractYearSuffixes(text) {
  return normalizeText(text).match(/\b\d{4}[a-z]\b/gi) || [];
}

function hasEtAl(text) {
  return /\bet al\b/i.test(normalizeText(text));
}

function splitCitationCluster(text) {
  return normalizeText(text)
    .replace(/^\(/, '').replace(/\)$/, '')
    .split(/\s*;\s*/).map((p) => p.trim()).filter(Boolean);
}

function extractLocatorNumber(text) {
  const m = normalizeText(text).match(/\b(?:p|pp|section|sec)\.?\s*(\d+)\b/i);
  return m ? m[1] : null;
}

function equivalentCitationText(oracleText, citumText, citationId, options = {}) {
  if (options.caseSensitive !== false && compareText(oracleText, citumText, options).caseMismatch) {
    return false;
  }
  if (!STRICT_CITATION_IDS.has(citationId)) return equivalentText(oracleText, citumText, options);

  const oN = normalizeText(oracleText);
  const cN = normalizeText(citumText);
  if (hasEtAl(oN) && !hasEtAl(cN)) return false;
  if (extractYearSuffixes(oN).length > 0 && extractYearSuffixes(cN).length === 0) return false;
  if (citationId === 'disambiguate-add-names-et-al') {
    if (hasEtAl(oN) || extractYearSuffixes(oN).length > 0) {
      const parts = splitCitationCluster(cN);
      if (parts.length < 2 || new Set(parts).size !== parts.length) return false;
    }
  }
  if (citationId === 'et-al-with-locator') {
    const oL = extractLocatorNumber(oN);
    const cL = extractLocatorNumber(cN);
    if (oL && oL !== cL) return false;
  }
  return true;
}

/**
 * Pair bibliography entries by ID when both outputs have complete IDs.
 * Falls back to the live oracle's neutral similarity matcher otherwise.
 */
function matchBibliographyEntries(oracleBib, citumBib, oracleIds = null, citumIds = null) {
  const hasCompleteIds = (entries, ids) =>
    Array.isArray(ids) &&
    ids.length === entries.length &&
    ids.every((id) => id !== null && id !== undefined && id !== '') &&
    new Set(ids).size === ids.length;
  const useIds = hasCompleteIds(oracleBib, oracleIds) && hasCompleteIds(citumBib, citumIds);

  return matchLiveBibliographyEntries(
    oracleBib,
    citumBib,
    useIds ? oracleIds : [],
    useIds ? citumIds : []
  );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function run() {
  const opts = parseArgs();

  if (!opts.stylePath) {
    process.stderr.write('Usage: oracle-fast.js <style.csl> [--json] [--verbose]\n');
    process.exitCode = 2;
    return;
  }

  if (!fs.existsSync(opts.stylePath)) {
    process.stderr.write(`Style not found: ${opts.stylePath}\n`);
    process.exitCode = 2;
    return;
  }

  // 1. Load and validate snapshot
  let snapshot;
  try {
    snapshot = loadSnapshot(opts.stylePath, opts.refsFixture, opts.citationsFixture);
  } catch (err) {
    process.stderr.write(`oracle-fast: ${err.message}\n`);
    process.exitCode = err instanceof SnapshotStaleError ? 3 : 2;
    return;
  }

  // 2. Load fixtures for Citum rendering
  const refsData = JSON.parse(fs.readFileSync(opts.refsFixture, 'utf8'));
  const testItems = Object.fromEntries(
    Object.entries(refsData)
      .filter(([k]) => k !== 'comment')
  );
  const testCitations = JSON.parse(fs.readFileSync(opts.citationsFixture, 'utf8'));

  // 3. Render with Citum
  const citum = renderWithCitumProcessor(opts.stylePath, refsData, testItems, testCitations, opts);
  if (!citum || citum.error) {
    const reason = citum?.error ?? 'Processor execution error';
    if (opts.jsonOutput) {
      process.stdout.write(JSON.stringify({
        error: 'Citum rendering failed', reason,
        style: path.basename(opts.stylePath, '.csl'),
      }) + '\n');
    } else {
      process.stderr.write(`Citum rendering failed: ${reason}\n`);
    }
    process.exitCode = 2;
    return;
  }

  const styleName = path.basename(opts.stylePath, '.csl');

  // 4. Diff
  const pairs = matchBibliographyEntries(
    snapshot.bibliography,
    citum.bibliography,
    snapshot.bibliography_ids,
    citum.bibliographyIds
  );

  const rawResults = {
    style: styleName,
    oracleSource: 'citeproc-js',
    snapshotGeneratedBy: snapshot.generated_by,
    citations: { total: testCitations.length, passed: 0, failed: 0, entries: [] },
    citationsByType: {},
    bibliography: { total: 0, passed: 0, failed: 0, entries: [] },
    componentSummary: {},
    orderingIssues: 0,
  };

  for (const cite of testCitations) {
    const comparison = compareText(snapshot.citations[cite.id] || '', citum.citations[cite.id] || '', {
      caseSensitive: opts.caseSensitive,
    });
    const match = equivalentCitationText(comparison.expected, comparison.actual, cite.id, {
      caseSensitive: opts.caseSensitive,
    });
    if (match) rawResults.citations.passed++; else rawResults.citations.failed++;

    // Always run real component detection (matching or not) -- citation
    // text rarely carries more than contributors + year, so a passing-
    // entry shortcut (as the bibliography path below takes) would badly
    // overweight it. See oracle.js's citation loop for the same approach.
    const itemComparison = compareCitationComponents(
      comparison.expected,
      comparison.actual,
      cite.items || [],
      testItems
    );

    rawResults.citations.entries.push({
      id: cite.id,
      oracle: comparison.expected,
      citum: comparison.actual,
      rawOracle: comparison.rawExpected,
      rawCitum: comparison.rawActual,
      exactOracle: comparison.exactExpected,
      exactCitum: comparison.exactActual,
      exactMatch: comparison.exactMatch,
      exactAdjudication: comparison.exactAdjudication,
      match,
      caseMismatch: comparison.caseMismatch,
      components: itemComparison.segmented
        ? { matches: itemComparison.matches, differences: itemComparison.differences }
        : {},
      componentsSegmented: itemComparison.segmented,
    });

    for (const item of cite.items || []) {
      const type = testItems[item.id]?.type ?? 'unknown';
      if (!rawResults.citationsByType[type]) rawResults.citationsByType[type] = { total: 0, passed: 0 };
      rawResults.citationsByType[type].total++;
      if (match) rawResults.citationsByType[type].passed++;
    }
  }

  for (let i = 0; i < pairs.length; i++) {
    const pair = pairs[i];
    const entryResult = {
      index: i + 1,
      id: pair.id || null,
      oracle: pair.oracle ? normalizeText(pair.oracle) : null,
      citum: pair.citum ? normalizeText(pair.citum) : null,
      rawOracle: pair.oracle ?? null,
      rawCitum: pair.citum ?? null,
      exactOracle: null,
      exactCitum: null,
      exactMatch: null,
      exactAdjudication: 'not-comparable',
      match: pair.compatibilityEligible ? false : null,
      caseMismatch: false,
      pairingMethod: pair.pairingMethod,
      comparisonState: pair.comparisonState,
      compatibilityEligible: pair.compatibilityEligible,
      exactParityEligible: pair.comparisonState === 'paired',
      components: {},
      ordering: null,
      issues: [],
    };

    if (!pair.oracle) {
      if (pair.compatibilityEligible) {
        entryResult.issues.push({ issue: 'extra_entry', detail: 'ID-proven entry in Citum but not oracle' });
        rawResults.bibliography.total++;
        rawResults.bibliography.failed++;
      } else {
        entryResult.issues.push({ issue: 'unpaired_output', detail: 'Similarity pairing found no benchmark counterpart' });
      }
    } else if (!pair.citum) {
      if (pair.compatibilityEligible) {
        entryResult.issues.push({ issue: 'missing_entry', detail: 'ID-proven entry in oracle but not Citum' });
        rawResults.bibliography.total++;
        rawResults.bibliography.failed++;
      } else {
        entryResult.issues.push({ issue: 'unpaired_output', detail: 'Similarity pairing found no Citum counterpart' });
      }
    } else {
      rawResults.bibliography.total++;
      const comparison = compareText(pair.oracle, pair.citum, {
        caseSensitive: opts.caseSensitive,
      });
      entryResult.oracle = comparison.expected;
      entryResult.citum = comparison.actual;
      entryResult.rawOracle = comparison.rawExpected;
      entryResult.rawCitum = comparison.rawActual;
      entryResult.exactOracle = comparison.exactExpected;
      entryResult.exactCitum = comparison.exactActual;
      entryResult.exactMatch = comparison.exactMatch;
      entryResult.exactAdjudication = comparison.exactAdjudication;
      entryResult.caseMismatch = comparison.caseMismatch;
      if (bibliographyComparisonMatches(styleName, comparison, opts.caseSensitive)) {
        entryResult.match = true;
        rawResults.bibliography.passed++;
      } else {
        rawResults.bibliography.failed++;
        const refData = pair.id
          ? testItems[pair.id]
          : findRefDataForEntry(pair.oracle, testItems);
        if (refData) {
          const oComp = parseComponents(pair.oracle, refData);
          const cComp = parseComponents(pair.citum, refData);
          const { differences, matches } = compareComponents(oComp, cComp, refData);
          entryResult.components = { differences, matches };

          const oOrder = analyzeOrdering(pair.oracle, refData);
          const cOrder = analyzeOrdering(pair.citum, refData);
          if (JSON.stringify(oOrder) !== JSON.stringify(cOrder)) {
            entryResult.ordering = { oracle: oOrder, citum: cOrder };
            rawResults.orderingIssues++;
          }
          entryResult.issues = [...differences];
          for (const [key, count] of Object.entries(
            differences.reduce((acc, d) => { acc[`${d.component}:${d.issue}`] = (acc[`${d.component}:${d.issue}`] || 0) + 1; return acc; }, {})
          )) {
            rawResults.componentSummary[key] = (rawResults.componentSummary[key] || 0) + count;
          }
        }
      }
    }

    rawResults.bibliography.entries.push(entryResult);
  }

  const results = attachRegisteredDivergenceAdjustments(
    rawResults,
    snapshot.bibliography,
    citum.bibliographyOrderIds || [],
    testItems,
    testCitations,
    snapshot.bibliography_ids || null
  );

  // 5. Output
  if (opts.jsonOutput) {
    process.stdout.write(JSON.stringify(results, null, 2) + '\n');
  } else {
    process.stderr.write(`\n=== Fast Oracle: ${styleName} (${snapshot.generated_by}) ===\n\n`);
    process.stderr.write(`Citations:    ${results.citations.passed}/${results.citations.total}\n`);
    process.stderr.write(`Bibliography: ${results.bibliography.passed}/${results.bibliography.total}\n`);
    if (opts.verbose) {
      for (const e of results.citations.entries.filter((e) => !e.match)) {
        process.stderr.write(`  [FAIL] ${e.id}\n    oracle: ${e.oracle}\n    citum:  ${e.citum}\n`);
      }
    }
  }

  process.exitCode = results.citations.failed === 0 && results.bibliography.failed === 0 ? 0 : 1;
}

if (require.main === module) {
  run();
}

module.exports = {
  parseArgs,
  run,
  matchBibliographyEntries,
  // Re-exported so tests can confirm this module's bibliography pairing loop
  // is wired to the same strict-fidelity gate oracle.js uses for
  // STRICT_BIBLIOGRAPHY_STYLES, rather than falling back to the lenient
  // similarity-threshold match.
  bibliographyComparisonMatches,
};
