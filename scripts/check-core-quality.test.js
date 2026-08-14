// Regression coverage for the exact-parity floor gate added in
// docs/architecture/audits/2026-07-31_EXACT_PARITY_REFOCUS.md. These tests
// invoke the CLI directly (rather than importing internals) because the gate's
// contract is its process exit code and stderr annotations, not its internal
// functions.
const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const SCRIPT = path.join(__dirname, 'check-core-quality.js');

function writeTmpJson(name, data) {
  const filePath = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'ccq-test-')), name);
  fs.writeFileSync(filePath, JSON.stringify(data));
  return filePath;
}

function baseStyle(overrides = {}) {
  return {
    name: 'apa-7th',
    fidelityScore: 1,
    qualityBreakdown: {},
    exactParity: { passed: 89, total: 146, notComparable: 0, divergenceExcluded: 0 },
    ...overrides,
  };
}

function runGate(args) {
  const result = spawnSync(process.execPath, [SCRIPT, ...args], { encoding: 'utf8' });
  return { status: result.status, stdout: result.stdout || '', stderr: result.stderr || '' };
}

test('exact-parity gate passes when passed count meets the baseline floor', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });

  const result = runGate(['--report', report, '--parity-baseline', parityBaseline]);

  assert.equal(result.status, 0);
  assert.match(result.stdout, /Core quality gate passed/);
});

test('exact-parity gate fails when passed count drops below the baseline floor', () => {
  const report = writeTmpJson('report.json', {
    styles: [baseStyle({ exactParity: { passed: 84, total: 146, notComparable: 0, divergenceExcluded: 0 } })],
  });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });

  const result = runGate(['--report', report, '--parity-baseline', parityBaseline]);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Exact-parity gate failed for apa-7th: passed=84 < baseline floor 89/);
});

test('exact-parity gate treats a style with a measurement error as unmeasurable, not a fixture-drift or regression finding', () => {
  // Reproduces the non-determinism observed under default report-core.js
  // parallelism: a snapshot-oracle failure (exit 2) yields a partial
  // exactParity result on an otherwise-successful report run. The gate must
  // not compare that partial result against the baseline at all.
  const report = writeTmpJson('report.json', {
    styles: [
      baseStyle({
        error: 'Snapshot oracle failed for apa-7th: exit 2',
        exactParity: { passed: 40, total: 80, notComparable: 0, divergenceExcluded: 0 },
      }),
    ],
  });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });

  const result = runGate(['--report', report, '--parity-baseline', parityBaseline]);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Exact-parity not measurable for apa-7th \(re-run, do not trust this run's numbers\)/);
  assert.doesNotMatch(result.stderr, /fixture drift/);
  assert.doesNotMatch(result.stderr, /baseline floor/);
});

test('exact-parity gate fails loudly on fixture-count drift instead of silently moving the floor', () => {
  const report = writeTmpJson('report.json', {
    styles: [baseStyle({ exactParity: { passed: 89, total: 143, notComparable: 0, divergenceExcluded: 0 } })],
  });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });

  const result = runGate(['--report', report, '--parity-baseline', parityBaseline]);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Exact-parity fixture drift for apa-7th: total 143 != baseline total 146/);
});

test('exact-parity gate fails when a baselined style is missing from the report', () => {
  const report = writeTmpJson('report.json', { styles: [] });

  // report-core.js always errors on an empty style list before the parity
  // gate runs, so exercise the missing-style path with a non-empty report
  // that simply omits the baselined style.
  const reportWithOtherStyle = writeTmpJson('report.json', {
    styles: [baseStyle({ name: 'ieee', exactParity: { passed: 84, total: 149 } })],
  });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });

  const result = runGate(['--report', reportWithOtherStyle, '--parity-baseline', parityBaseline]);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Missing exact-parity baseline core style in report: apa-7th/);
});

test('adjudication ledger rejects a citum-correct entry missing authority or confirmedBy', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });
  const ledger = writeTmpJson('ledger.json', {
    entries: { 'apa-7th': { 'entry-1': { state: 'citum-correct', class: 'punctuation' } } },
  });

  const result = runGate([
    '--report',
    report,
    '--parity-baseline',
    parityBaseline,
    '--parity-adjudication',
    ledger,
  ]);

  assert.equal(result.status, 2);
  assert.match(result.stderr, /citum-correct requires "authority" and "confirmedBy"/);
});

test('adjudication ledger accepts a fully-specified citum-correct entry and reports the unclear queue', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const parityBaseline = writeTmpJson('parity-baseline.json', {
    styles: { 'apa-7th': { exactParity: { passed: 89, total: 146 } } },
  });
  const ledger = writeTmpJson('ledger.json', {
    entries: {
      'apa-7th': {
        'entry-1': {
          state: 'citum-correct',
          class: 'punctuation',
          authority: 'APA 7th manual, section 9.8',
          confirmedBy: 'bdarcus',
        },
        'entry-2': { state: 'unclear', class: 'date-format' },
      },
    },
  });

  const result = runGate([
    '--report',
    report,
    '--parity-baseline',
    parityBaseline,
    '--parity-adjudication',
    ledger,
  ]);

  assert.equal(result.status, 0);
  assert.match(result.stderr, /1 parity residual\(s\) recorded as "unclear"/);
  assert.match(result.stdout, /0 citeproc-correct, 1 citum-correct, 1 unclear/);
});

test('exact-parity gate fails closed when the baseline cannot be read', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const missingBaseline = path.join(os.tmpdir(), `missing-parity-baseline-${Date.now()}.json`);

  const result = runGate(['--report', report, '--parity-baseline', missingBaseline]);

  assert.equal(result.status, 2);
  assert.match(result.stderr, /Failed to read parity baseline/);
});

test('adjudication ledger fails closed when it cannot be read', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const missingLedger = path.join(os.tmpdir(), `missing-parity-ledger-${Date.now()}.json`);

  const result = runGate(['--report', report, '--parity-adjudication', missingLedger]);

  assert.equal(result.status, 2);
  assert.match(result.stderr, /Failed to read parity adjudication ledger/);
});

test('adjudication ledger rejects a null entries map', () => {
  const report = writeTmpJson('report.json', { styles: [baseStyle()] });
  const ledger = writeTmpJson('ledger.json', { entries: null });

  const result = runGate(['--report', report, '--parity-adjudication', ledger]);

  assert.equal(result.status, 2);
  assert.match(result.stderr, /Invalid parity adjudication ledger/);
});

// csl26-7u16: the positional bibliography-order check is a diagnostic
// warning here, not a hard gate, until a corpus sweep establishes scale.
test('an unexplained bibliography order mismatch warns but does not fail the gate', () => {
  const report = writeTmpJson('report.json', {
    styles: [
      baseStyle({
        bibliographyOrderMismatch: { mismatch: true, explained: false, explainedBy: null },
      }),
    ],
  });

  const result = runGate(['--report', report]);

  assert.equal(result.status, 0);
  assert.match(result.stderr, /Unexplained bibliography order mismatch in apa-7th/);
  assert.match(result.stdout, /Core quality gate passed/);
});

test('a bibliography order mismatch explained by a registered divergence does not warn', () => {
  const report = writeTmpJson('report.json', {
    styles: [
      baseStyle({
        bibliographyOrderMismatch: { mismatch: true, explained: true, explainedBy: 'div-004' },
      }),
    ],
  });

  const result = runGate(['--report', report]);

  assert.equal(result.status, 0);
  assert.doesNotMatch(result.stderr, /bibliography order mismatch/);
});

test('--strict-warnings escalates an unexplained bibliography order mismatch to a failure', () => {
  const report = writeTmpJson('report.json', {
    styles: [
      baseStyle({
        bibliographyOrderMismatch: { mismatch: true, explained: false, explainedBy: null },
      }),
    ],
  });

  const result = runGate(['--report', report, '--strict-warnings']);

  assert.equal(result.status, 1);
  assert.match(result.stderr, /Unexplained bibliography order mismatch in apa-7th/);
  assert.match(result.stderr, /Quality warnings elevated to failure \(1\)/);
});
