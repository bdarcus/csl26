const test = require('node:test');
const assert = require('node:assert/strict');

const { compareText } = require('./oracle-utils');
const { bibliographyComparisonMatches: oracleBibliographyComparisonMatches } = require('./oracle');
const {
  bibliographyComparisonMatches: fastBibliographyComparisonMatches,
  matchBibliographyEntries,
  parseArgs,
} = require('./oracle-fast');

test('oracle-fast parses report-provided Citum runtime options', () => {
  const originalArgv = process.argv;
  process.argv = [
    originalArgv[0],
    'scripts/oracle-fast.js',
    'styles-legacy/apa.csl',
    '--citum-bin',
    'target/debug/citum',
    '--locale',
    'fr-FR',
  ];

  try {
    const options = parseArgs();
    assert.equal(options.citumBin, require('path').resolve('target/debug/citum'));
    assert.equal(options.locale, 'fr-FR');
  } finally {
    process.argv = originalArgv;
  }
});

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

test('oracle-fast pairs complete ID-bearing rows by ID instead of text or position', () => {
  const pairs = matchBibliographyEntries(
    ['Oracle row A', 'Oracle row B'],
    ['Citum row B', 'Citum row A'],
    ['ITEM-A', 'ITEM-B'],
    ['ITEM-B', 'ITEM-A']
  );

  assert.deepEqual(
    pairs.map(({ id, oracle, citum, pairingMethod, comparisonState }) => ({
      id, oracle, citum, pairingMethod, comparisonState,
    })),
    [{
      id: 'ITEM-A',
      oracle: 'Oracle row A',
      citum: 'Citum row A',
      pairingMethod: 'id',
      comparisonState: 'paired',
    }, {
      id: 'ITEM-B',
      oracle: 'Oracle row B',
      citum: 'Citum row B',
      pairingMethod: 'id',
      comparisonState: 'paired',
    }]
  );
});

test('oracle-fast classifies a complete-ID oracle-only row as a compatibility failure', () => {
  const pairs = matchBibliographyEntries(
    ['Shared oracle row', 'Missing Citum row'],
    ['Shared Citum row'],
    ['ITEM-SHARED', 'ITEM-ORACLE'],
    ['ITEM-SHARED']
  );
  const oracleOnly = pairs.find((pair) => pair.id === 'ITEM-ORACLE');

  assert.deepEqual(oracleOnly, {
    id: 'ITEM-ORACLE',
    oracle: 'Missing Citum row',
    citum: null,
    score: 0,
    pairingMethod: 'id',
    comparisonState: 'oracle-only',
    compatibilityEligible: true,
  });
});

test('oracle-fast classifies a complete-ID Citum-only row as a compatibility failure', () => {
  const pairs = matchBibliographyEntries(
    ['Shared oracle row'],
    ['Shared Citum row', 'Extra Citum row'],
    ['ITEM-SHARED'],
    ['ITEM-SHARED', 'ITEM-CITUM']
  );
  const citumOnly = pairs.find((pair) => pair.id === 'ITEM-CITUM');

  assert.deepEqual(citumOnly, {
    id: 'ITEM-CITUM',
    oracle: null,
    citum: 'Extra Citum row',
    score: 0,
    pairingMethod: 'id',
    comparisonState: 'citum-only',
    compatibilityEligible: true,
  });
});

test('oracle-fast falls back to neutral similarity pairing when either ID list is incomplete', () => {
  const pairs = matchBibliographyEntries(
    ['Alpha bibliography record', 'Beta bibliography record'],
    ['Beta bibliography record', 'Alpha bibliography record'],
    ['ITEM-A', null],
    ['ITEM-A', 'ITEM-B']
  );

  assert.equal(pairs.every((pair) => pair.pairingMethod === 'similarity'), true);
  assert.equal(pairs.every((pair) => pair.comparisonState === 'paired'), true);
  assert.deepEqual(
    pairs.map(({ oracle, citum }) => ({ oracle, citum })),
    [{
      oracle: 'Alpha bibliography record',
      citum: 'Alpha bibliography record',
    }, {
      oracle: 'Beta bibliography record',
      citum: 'Beta bibliography record',
    }]
  );
});

test('oracle-fast does not infer complete IDs for an empty legacy snapshot', () => {
  const pairs = matchBibliographyEntries(
    [],
    ['Citum-only text without ID-bearing benchmark metadata'],
    undefined,
    ['ITEM-CITUM']
  );

  assert.deepEqual(pairs, [{
    oracle: null,
    citum: 'Citum-only text without ID-bearing benchmark metadata',
    score: 0,
    pairingMethod: 'similarity',
    comparisonState: 'unresolved-unpaired',
    compatibilityEligible: false,
  }]);
});
