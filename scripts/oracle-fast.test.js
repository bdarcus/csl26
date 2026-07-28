const test = require('node:test');
const assert = require('node:assert/strict');

const { compareText } = require('./oracle-utils');
const { bibliographyComparisonMatches: oracleBibliographyComparisonMatches } = require('./oracle');
const {
  bibliographyComparisonMatches: fastBibliographyComparisonMatches,
  matchBibliographyEntries,
} = require('./oracle-fast');

test('oracle-fast.js is wired to the same strict bibliography gate as oracle.js, not a lenient fallback', () => {
  assert.equal(
    fastBibliographyComparisonMatches,
    oracleBibliographyComparisonMatches,
    'oracle-fast.js must reuse oracle.js\'s STRICT_BIBLIOGRAPHY_STYLES gate for the bibliography pairing loop'
  );

  // A GB/T entry that's similarity-close but not exact (missing terminal period)
  // must fail under the gate oracle-fast.js now uses -- this is exactly the class
  // of defect that was previously invisible to report-core.js's fidelity score.
  const comparison = compareText(
    'New York：Bantam Dell Publishing Group，1988.',
    'New York：Bantam Dell Publishing Group，1988'
  );
  assert.equal(comparison.match, true, 'similarity fallback alone would have hidden this');
  assert.equal(fastBibliographyComparisonMatches('gb-t-7714-2025-numeric', comparison), false);
  assert.equal(fastBibliographyComparisonMatches('apa-7th', comparison), true);
});

test('oracle-fast similarity pairing marks unmatched outputs as metric-ineligible', () => {
  const pairs = matchBibliographyEntries(
    ['Alpha bibliography record'],
    ['Zulu completely unrelated output']
  );

  assert.equal(pairs.length, 2);
  assert.equal(pairs.every((pair) => pair.pairingMethod === 'similarity'), true);
  assert.equal(pairs.every((pair) => pair.comparisonState === 'unresolved-unpaired'), true);
  assert.equal(pairs.every((pair) => pair.compatibilityEligible === false), true);
});
