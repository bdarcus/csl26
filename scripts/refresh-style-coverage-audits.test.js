'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const path = require('node:path');

const { PROJECT_ROOT } = require('./lib/style-coverage-audits');
const {
  parseArgs,
  refreshRegisteredAudits,
} = require('./refresh-style-coverage-audits');

test('refresh command accepts the source-built Citum binary path', () => {
  assert.deepEqual(parseArgs(['--citum-bin', 'target/debug/citum']), {
    citumBin: `${PROJECT_ROOT}/target/debug/citum`,
  });
});

test('refresh command regenerates every registered audit', () => {
  const calls = [];
  const provenance = {
    coverage_audits: [
      {
        manifest: 'audit/one/manifest.yaml',
        packet: 'audit/one/packet.json',
        markdown: 'audit/one/packet.md',
      },
      {
        manifest: 'audit/two/manifest.yaml',
        packet: 'audit/two/packet.json',
        markdown: 'audit/two/packet.md',
      },
    ],
  };

  const result = refreshRegisteredAudits(
    { citumBin: '/tmp/citum' },
    {
      provenance,
      execFileSync(command, args, options) {
        calls.push({ command, args, options });
      },
    },
  );

  assert.equal(result.registrations.length, 2);
  assert.equal(calls.length, 2);
  assert.equal(calls[0].args.at(-1), '/tmp/citum');
  assert.equal(calls[0].args.includes('--check'), false);
  assert.equal(calls[1].args.includes(path.join(PROJECT_ROOT, 'audit/two/packet.md')), true);
});
