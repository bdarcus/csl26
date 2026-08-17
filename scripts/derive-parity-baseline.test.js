// Coverage for scripts/derive-parity-baseline.js — the generator for
// scripts/report-data/embedded-parity-baseline.json (see
// docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md). The output
// shape is a contract: .github/workflows/fidelity.yml and
// check-core-quality.js --parity-baseline both read styles[].exactParity
// positionally, so the shape assertions here matter as much as the values.
const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { deriveParityBaseline } = require('./derive-parity-baseline');

const SCRIPT = path.join(__dirname, 'derive-parity-baseline.js');

function tmpJsonPath(name, data) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dpb-test-'));
  const filePath = path.join(dir, name);
  fs.writeFileSync(filePath, JSON.stringify(data));
  return filePath;
}

function styleRecord(overrides = {}) {
  return {
    name: 'apa-7th',
    tier: 'embedded',
    fidelityScore: 1,
    qualityScore: 0.972,
    citations: { passed: 20, total: 20 },
    bibliography: { passed: 47, total: 47 },
    exactParity: { passed: 33, total: 67, notComparable: 0, divergenceExcluded: 0, rate: 0.493 },
    ...overrides,
  };
}

test('keeps only embedded-tier styles, sorted by name', () => {
  const report = {
    generated: '2026-08-16T00:00:00.000Z',
    commit: 'abc1234',
    styles: [
      styleRecord({ name: 'zzz-exemplar', tier: 'exemplar' }),
      styleRecord({ name: 'ieee' }),
      styleRecord({ name: 'apa-7th' }),
    ],
  };

  const baseline = deriveParityBaseline(report);

  assert.deepEqual(Object.keys(baseline.styles), ['apa-7th', 'ieee']);
});

test('carries generated/commit through and preserves the fixed purpose text', () => {
  const report = { generated: '2026-08-16T00:00:00.000Z', commit: 'abc1234', styles: [styleRecord()] };

  const baseline = deriveParityBaseline(report);

  assert.equal(baseline.generated, '2026-08-16T00:00:00.000Z');
  assert.equal(baseline.commit, 'abc1234');
  assert.equal(baseline.source, 'scripts/report-core.js');
  assert.match(baseline.purpose, /Hard per-style exact-parity floor-gate baseline/);
});

test('per-style shape matches the fields fidelity.yml and check-core-quality.js read', () => {
  const report = { generated: 'x', commit: 'y', styles: [styleRecord()] };

  const baseline = deriveParityBaseline(report);
  const entry = baseline.styles['apa-7th'];

  assert.deepEqual(Object.keys(entry).sort(), [
    'bibliography',
    'citations',
    'exactParity',
    'fidelityScore',
    'qualityScore',
    'tier',
  ].sort());
  assert.deepEqual(entry.exactParity, { passed: 33, total: 67, rate: 0.493 });
  assert.deepEqual(entry.citations, { passed: 20, total: 20 });
  assert.deepEqual(entry.bibliography, { passed: 47, total: 47 });
});

test('throws rather than defaulting a style with a report error to a 0/0 floor', () => {
  const report = {
    generated: 'x',
    commit: 'y',
    styles: [styleRecord({ name: 'apa-7th', error: 'style file not found', exactParity: undefined })],
  };

  assert.throws(() => deriveParityBaseline(report), /apa-7th/);
});

test('throws when an embedded style is missing exactParity entirely', () => {
  const report = {
    generated: 'x',
    commit: 'y',
    styles: [styleRecord({ name: 'ieee', exactParity: undefined })],
  };

  assert.throws(() => deriveParityBaseline(report), /ieee/);
});

test('CLI exits non-zero and writes nothing when a style is unmeasurable', () => {
  const report = {
    generated: 'x',
    commit: 'y',
    styles: [styleRecord({ error: 'boom', exactParity: undefined })],
  };
  const reportPath = tmpJsonPath('report.json', report);
  const outPath = path.join(path.dirname(reportPath), 'out.json');

  const result = spawnSync(process.execPath, [SCRIPT, '--report', reportPath, '--out', outPath], {
    encoding: 'utf8',
  });

  assert.notEqual(result.status, 0);
  assert.equal(fs.existsSync(outPath), false);
});

test('CLI writes the derived baseline to --out', () => {
  const report = { generated: 'x', commit: 'y', styles: [styleRecord()] };
  const reportPath = tmpJsonPath('report.json', report);
  const outPath = path.join(path.dirname(reportPath), 'out.json');

  const result = spawnSync(process.execPath, [SCRIPT, '--report', reportPath, '--out', outPath], {
    encoding: 'utf8',
  });

  assert.equal(result.status, 0, result.stderr);
  const written = JSON.parse(fs.readFileSync(outPath, 'utf8'));
  assert.deepEqual(Object.keys(written.styles), ['apa-7th']);
});

test('CLI without --report prints usage and exits non-zero', () => {
  const result = spawnSync(process.execPath, [SCRIPT], { encoding: 'utf8' });

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Usage:/);
});
