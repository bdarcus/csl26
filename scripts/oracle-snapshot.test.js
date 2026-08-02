const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const os = require('os');
const path = require('path');

const { bibliographyIdsForRenderedRows } = require('./lib/citeproc-bibliography');
const {
  buildSnapshot,
  isSnapshotCurrent,
  renderWithCiteprocJs,
} = require('./oracle-snapshot');

const projectRoot = path.resolve(__dirname, '..');
const hasLegacyStyles = fs.existsSync(path.join(projectRoot, 'styles-legacy', 'apa.csl'));

test('citeproc bibliography IDs omit the failed item identified by bibliography_errors', () => {
  const ids = bibliographyIdsForRenderedRows({
    entry_ids: [['ITEM-A'], ['ITEM-FAILED'], ['ITEM-C']],
    bibliography_errors: [{ index: 1, itemID: 'ITEM-FAILED', error_code: 1 }],
  }, ['Rendered A', 'Rendered C']);

  assert.deepEqual(ids, ['ITEM-A', 'ITEM-C']);
});

test('citeproc bibliography ID alignment is safe for multiple errors in any index order', () => {
  const params = {
    entry_ids: [['ITEM-A'], ['ITEM-B'], ['ITEM-C'], ['ITEM-D'], ['ITEM-E']],
    bibliography_errors: [
      { index: 3, itemID: 'ITEM-D', error_code: 1 },
      { index: 1, itemID: 'ITEM-B', error_code: 1 },
    ],
  };

  assert.deepEqual(
    bibliographyIdsForRenderedRows(params, ['Rendered A', 'Rendered C', 'Rendered E']),
    ['ITEM-A', 'ITEM-C', 'ITEM-E']
  );
  assert.deepEqual(
    bibliographyIdsForRenderedRows({
      ...params,
      bibliography_errors: [...params.bibliography_errors].reverse(),
    }, ['Rendered A', 'Rendered C', 'Rendered E']),
    ['ITEM-A', 'ITEM-C', 'ITEM-E']
  );
});

test('APA expanded fixture omits the failed ITEM-28 ID with its unrendered row', {
  skip: !hasLegacyStyles,
}, () => {
  const refsPath = path.join(__dirname, '..', 'tests', 'fixtures', 'references-expanded.json');
  const refs = JSON.parse(fs.readFileSync(refsPath, 'utf8'));
  const testItems = Object.fromEntries(
    Object.entries(refs).filter(([id]) => id !== 'comment')
  );
  const stylePath = path.join(__dirname, '..', 'styles-legacy', 'apa.csl');

  const rendered = renderWithCiteprocJs(stylePath, testItems, []);

  assert.equal(rendered.bibliography.length, 46);
  assert.equal(rendered.bibliographyIds.length, 46);
  assert.equal(rendered.bibliographyIds.includes('ITEM-28'), false);
});

test('oracle snapshot rendering preserves bibliography IDs in rendered row order', {
  skip: !hasLegacyStyles,
}, () => {
  const stylePath = path.join(__dirname, '..', 'styles-legacy', 'apa.csl');
  const testItems = {
    'ITEM-Z': {
      id: 'ITEM-Z',
      type: 'book',
      title: 'Zulu Work',
      author: [{ family: 'Zulu', given: 'Zoe' }],
      issued: { 'date-parts': [[2020]] },
    },
    'ITEM-A': {
      id: 'ITEM-A',
      type: 'book',
      title: 'Alpha Work',
      author: [{ family: 'Alpha', given: 'Ada' }],
      issued: { 'date-parts': [[2021]] },
    },
  };

  const rendered = renderWithCiteprocJs(stylePath, testItems, []);

  assert.deepEqual(rendered.bibliographyIds, ['ITEM-A', 'ITEM-Z']);
  assert.equal(rendered.bibliography.length, 2);
  assert.match(rendered.bibliography[0], /Alpha Work/);
  assert.match(rendered.bibliography[1], /Zulu Work/);
});

test('oracle snapshot version 2 serializes bibliography IDs beside their rows', () => {
  const snapshot = buildSnapshot('sample', 'fixture-hash', 'csl-hash', {
    citations: { sample: '(Alpha, 2021)' },
    bibliography: ['Alpha bibliography row'],
    bibliographyIds: ['ITEM-A'],
  });

  assert.equal(snapshot.version, 2);
  assert.deepEqual(snapshot.bibliography, ['Alpha bibliography row']);
  assert.deepEqual(snapshot.bibliography_ids, ['ITEM-A']);
});

test('oracle snapshot version requires complete ID metadata before skipping regeneration', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'citum-oracle-snapshot-'));
  const snapshotPath = path.join(tempDir, 'sample.json');
  const baseSnapshot = {
    fixture_hash: 'fixture-hash',
    csl_hash: 'csl-hash',
    bibliography: ['First row', 'Second row'],
  };

  try {
    fs.writeFileSync(snapshotPath, JSON.stringify({
      ...baseSnapshot,
      version: 1,
    }));
    assert.equal(isSnapshotCurrent(snapshotPath, 'fixture-hash', 'csl-hash'), false);

    fs.writeFileSync(snapshotPath, JSON.stringify({
      ...baseSnapshot,
      version: 2,
      bibliography_ids: ['ITEM-1'],
    }));
    assert.equal(isSnapshotCurrent(snapshotPath, 'fixture-hash', 'csl-hash'), false);

    fs.writeFileSync(snapshotPath, JSON.stringify({
      ...baseSnapshot,
      version: 2,
      bibliography_ids: ['ITEM-1', 'ITEM-1'],
    }));
    assert.equal(isSnapshotCurrent(snapshotPath, 'fixture-hash', 'csl-hash'), false);

    fs.writeFileSync(snapshotPath, JSON.stringify({
      ...baseSnapshot,
      version: 2,
      bibliography_ids: ['ITEM-1', 'ITEM-2'],
    }));
    assert.equal(isSnapshotCurrent(snapshotPath, 'fixture-hash', 'csl-hash'), true);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
