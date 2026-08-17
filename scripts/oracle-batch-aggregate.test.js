// Regression coverage for the --save refusal on a partial run (see
// docs/architecture/audits/2026-08-16_FIXTURE_CHANGE_FAN_OUT.md and the
// PR review that flagged it): saving over a committed baseline when any
// requested style failed to render would silently drop that style from
// styleBreakdown, corrupting the ratchet.
const test = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const SCRIPT = path.join(__dirname, 'oracle-batch-aggregate.js');
const STYLES_DIR = path.join(__dirname, '..', 'styles-legacy');

test('refuses to save when a requested style fails to render', { skip: !fs.existsSync(STYLES_DIR) }, () => {
  const outPath = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'oba-test-')), 'out.json');

  const result = spawnSync(
    process.execPath,
    [SCRIPT, STYLES_DIR, '--styles', 'this-style-does-not-exist', '--save', outPath, '--json'],
    { encoding: 'utf8' }
  );

  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Refusing to write/);
  assert.equal(fs.existsSync(outPath), false);
});
