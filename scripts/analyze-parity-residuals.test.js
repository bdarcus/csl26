// Unit coverage for the exact-parity leverage-ranking tool (see the audit
// this tool was built for:
// docs/architecture/audits/2026-08-23_CHICAGO_PARITY_LEVERAGE_AUDIT.md).
// Tests use small, hand-constructed inputs with exact expected shapes —
// no substring-contains assertions on rendered text.
const test = require('node:test');
const assert = require('node:assert/strict');
const {
  diffOps,
  labelsFor,
  failingRows,
  greedySetCover,
  analyzeStyle,
  listLabel,
} = require('./analyze-parity-residuals.js');

test('diffOps: identical strings produce no opcodes', () => {
  assert.deepEqual(diffOps('same text', 'same text'), []);
});

test('diffOps: single mid-string replacement isolates the changed span', () => {
  const ops = diffOps('The Racial Dot Map', 'The racial dot map');
  // Every op should be a single-character case replace; no deletes/inserts.
  assert.ok(ops.length > 0);
  for (const op of ops) {
    assert.equal(op.tag, 'replace');
    assert.equal(op.a.length, 1);
    assert.equal(op.b.length, 1);
    assert.equal(op.a.toLowerCase(), op.b.toLowerCase());
  }
});

test('diffOps: trailing insert is reported as insert with empty oracle side', () => {
  const ops = diffOps('Foo, 2020.', 'Foo, 2020. Accessed July 1, 2020.');
  assert.equal(ops.length, 1);
  assert.equal(ops[0].tag, 'insert');
  assert.equal(ops[0].a, '');
  assert.equal(ops[0].b, ' Accessed July 1, 2020.');
});

test('labelsFor: title-case-not-applied is labeled A1 and not A2/A3', () => {
  const labels = labelsFor(
    'Mesopotamia: Between Two Rivers. 1957.',
    'Mesopotamia: between two rivers. 1957.'
  );
  assert.ok(labels.includes('A1 title-case not applied'));
  assert.ok(!labels.includes('A2 title-case over-applied / stop-word'));
  assert.ok(!labels.includes('A3 acronym/mixed-case'));
});

test('labelsFor: title-case-over-applied on a stop word is labeled A2', () => {
  const labels = labelsFor(
    'Language and Design in Pippa Passes.',
    'Language and Design In Pippa Passes.'
  );
  assert.ok(labels.includes('A2 title-case over-applied / stop-word'));
  assert.ok(!labels.includes('A1 title-case not applied'));
});

test('labelsFor: acronym case flip is labeled A3, distinct from A1/A2', () => {
  const labels = labelsFor('(PhD diss., Cornell University, 2020)', '(Phd diss., Cornell University, 2020)');
  assert.ok(labels.includes('A3 acronym/mixed-case'));
  assert.ok(!labels.includes('A1 title-case not applied'));
  assert.ok(!labels.includes('A2 title-case over-applied / stop-word'));
});

test('labelsFor: title quote boundary is detected when quotes appear or vanish', () => {
  const labels = labelsFor(
    '“The Role of Theory in Research.” 2018.',
    'The Role of Theory in Research. 2018.'
  );
  assert.ok(labels.includes('B title quote boundary'));
});

test('labelsFor: year-suffix letter divergence is isolated to class C', () => {
  const labels = labelsFor('Smith, Jane. 2019a. Title.', 'Smith, Jane. 2019b. Title.');
  assert.ok(labels.includes('C year-suffix letter'));
});

test('labelsFor: dropped month/day is labeled D', () => {
  const labels = labelsFor(
    'Rodriguez, Maria. 2024. "Title." New York Times, March 15.',
    'Rodriguez, Maria. 2024. "Title." New York Times.'
  );
  assert.ok(labels.includes('D date detail (month/day)'));
});

test('labelsFor: punctuation-only divergence is labeled N and nothing else', () => {
  const labels = labelsFor('Foo, Bar (2020): 42.', 'Foo, Bar (2020), 42.');
  assert.deepEqual(labels, ['N punctuation/delimiter only']);
});

test('labelsFor: rows with no matching rule fall back to Z unclassified', () => {
  const labels = labelsFor('Xyzzy plugh.', 'Plugh xyzzy quux.');
  assert.deepEqual(labels, ['Z unclassified']);
});

test('failingRows: only exact-parity-eligible bib rows and non-matching citation rows are counted', () => {
  const styleReport = {
    oracleDetail: [
      { id: 'b1', exactParityEligible: true, exactMatch: false, exactOracle: 'A', exactCitum: 'a' },
      { id: 'b2', exactParityEligible: true, exactMatch: true, exactOracle: 'B', exactCitum: 'B' },
      { id: 'b3', exactParityEligible: false, exactMatch: false, exactOracle: 'C', exactCitum: 'c' },
    ],
    citationEntries: [
      { id: 'c1', exactMatch: false, exactOracle: 'D', exactCitum: 'd' },
      { id: 'c2', exactMatch: true, exactOracle: 'E', exactCitum: 'E' },
    ],
  };
  const rows = failingRows(styleReport);
  assert.deepEqual(
    rows.map((r) => r.id),
    ['b1', 'c1']
  );
  assert.deepEqual(
    rows.map((r) => r.kind),
    ['bib', 'cite']
  );
});

test('greedySetCover: picks the label that fully explains the most rows first', () => {
  // 3 rows: two explained solely by "X", one needs both "X" and "Y".
  const rowLabels = [['X'], ['X'], ['X', 'Y']];
  const steps = greedySetCover(rowLabels);
  assert.equal(steps[0].label, 'X');
  assert.equal(steps[0].cumulativeFlipped, 2);
  assert.equal(steps[1].label, 'Y');
  assert.equal(steps[1].cumulativeFlipped, 3);
});

test('greedySetCover: never selects the Z unclassified bucket', () => {
  const rowLabels = [['Z unclassified'], ['Z unclassified'], ['X']];
  const steps = greedySetCover(rowLabels);
  assert.ok(!steps.some((s) => s.label === 'Z unclassified'));
});

test('analyzeStyle: end-to-end on a tiny synthetic report matches hand-computed labels', () => {
  const styleReport = {
    name: 'fixture-style',
    exactParity: { passed: 1, total: 3 },
    fidelityScore: 0.9,
    oracleDetail: [
      { id: 'r1', exactParityEligible: true, exactMatch: true, exactOracle: 'X', exactCitum: 'X' },
      {
        id: 'r2',
        exactParityEligible: true,
        exactMatch: false,
        exactOracle: 'Mesopotamia: Between Two Rivers. 1957.',
        exactCitum: 'Mesopotamia: between two rivers. 1957.',
      },
      {
        id: 'r3',
        exactParityEligible: true,
        exactMatch: false,
        exactOracle: '“The Role of Theory.” 2018.',
        exactCitum: 'The Role of Theory. 2018.',
      },
    ],
    citationEntries: [],
  };
  const result = analyzeStyle(styleReport, null);
  assert.equal(result.failingRows, 2);
  const labelMap = new Map(result.labelCounts.map((l) => [l.label, l.rows]));
  assert.equal(labelMap.get('A1 title-case not applied'), 1);
  assert.equal(labelMap.get('B title quote boundary'), 1);
  assert.equal(result.byType, null);
});

test('listLabel: drills a class down to its actual entries, deduped by id', () => {
  const styleReport = {
    name: 'fixture-style',
    oracleDetail: [
      {
        id: 'r1',
        exactParityEligible: true,
        exactMatch: false,
        exactOracle: 'Mesopotamia: Between Two Rivers.',
        exactCitum: 'Mesopotamia: between two rivers.',
      },
      // Same id repeated, as happens when a report merges multiple
      // benchmark runs covering the same fixture id -- must not be listed
      // twice.
      {
        id: 'r1',
        exactParityEligible: true,
        exactMatch: false,
        exactOracle: 'Mesopotamia: Between Two Rivers.',
        exactCitum: 'Mesopotamia: between two rivers.',
      },
      {
        id: 'r2',
        exactParityEligible: true,
        exactMatch: false,
        exactOracle: '“The Role of Theory.” 2018.',
        exactCitum: 'The Role of Theory. 2018.',
      },
    ],
    citationEntries: [],
  };
  const entries = listLabel(styleReport, 'A1 title-case not applied');
  assert.deepEqual(
    entries.map((e) => e.id),
    ['r1']
  );
  assert.ok(entries[0].labels.includes('A1 title-case not applied'));
});
