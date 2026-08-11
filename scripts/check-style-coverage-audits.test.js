'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const yaml = require('js-yaml');

const { validateReportProvenance } = require('./lib/report-metadata');
const {
  buildCoverageAuditView,
  validateCoveragePacket,
  validatePacketSchema,
  verifyManifestFiles,
} = require('./lib/style-coverage-audits');
const {
  runRegenerationCheck,
  selectRegistrations,
} = require('./check-style-coverage-audits');

const PROJECT_ROOT = path.dirname(__dirname);
const FIXTURE_ROOT = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'style-coverage');

function readJson(name) {
  return JSON.parse(fs.readFileSync(path.join(FIXTURE_ROOT, name), 'utf8'));
}

function readManifest() {
  return yaml.load(fs.readFileSync(path.join(FIXTURE_ROOT, 'manifest.yaml'), 'utf8'));
}

function registration(overrides = {}) {
  return {
    style_id: 'coverage-fixture',
    manifest: 'tests/fixtures/style-coverage/manifest.yaml',
    packet: 'tests/fixtures/style-coverage/packet.json',
    markdown: 'tests/fixtures/style-coverage/packet.md',
    adjudication_record: 'tests/fixtures/style-coverage/packet.md',
    adjudication_href: 'https://example.test/coverage-adjudication.md',
    ...overrides,
  };
}

function baselineFixture() {
  const manifest = readManifest();
  const packet = readJson('packet.json');
  manifest.source['worktree-clean'] = true;
  packet.auditManifest.source.worktreeClean = true;
  packet.auditManifest.source.baselineEligible = true;
  return { manifest, packet };
}

test('report metadata validates unique, human-readable coverage audit registrations', () => {
  const base = {
    version: 1,
    defaults: {
      labels: { 'csl-derived': 'CSL', 'biblatex-derived': 'BibLaTeX', 'citum-native': 'Citum' },
      sort_ranks: { 'csl-derived': 1, 'biblatex-derived': 2, 'citum-native': 3 },
    },
    styles: {},
    coverage_audits: [registration()],
  };

  assert.doesNotThrow(() => validateReportProvenance(base));
  assert.throws(
    () => validateReportProvenance({ ...base, coverage_audits: [registration(), registration()] }),
    /duplicates coverage-fixture/
  );
  assert.throws(
    () => validateReportProvenance({
      ...base,
      coverage_audits: [registration({ adjudication_href: 'packet.json' })],
    }),
    /human-readable record/
  );
});

test('coverage packet semantic validation accepts the complete fixture partition', () => {
  const { manifest, packet } = baselineFixture();

  assert.doesNotThrow(() => validateCoveragePacket(packet, registration(), manifest));
});

test('coverage packet schema rejects malformed observations', () => {
  const packet = readJson('packet.json');
  delete packet.observations[0].surface;

  assert.throws(() => validatePacketSchema(packet), /Invalid coverage packet schema.*surface/);
});

test('coverage packet rejects duplicate and missing observation identities', () => {
  const duplicate = baselineFixture();
  duplicate.packet.observations[1] = {
    ...duplicate.packet.observations[0],
    row: 2,
  };
  assert.throws(
    () => validateCoveragePacket(duplicate.packet, registration(), duplicate.manifest),
    /duplicate observation identity/
  );

  const missing = baselineFixture();
  missing.packet.observations.pop();
  assert.throws(
    () => validateCoveragePacket(missing.packet, registration(), missing.manifest),
    /missing or extra observations/
  );
});

test('coverage packet rejects count drift and mismatched style IDs', () => {
  const drift = baselineFixture();
  drift.packet.summary.renderDisposition.rendered += 1;
  assert.throws(
    () => validateCoveragePacket(drift.packet, registration(), drift.manifest),
    /render-disposition count drift/
  );

  const mismatched = baselineFixture();
  assert.throws(
    () => validateCoveragePacket(
      mismatched.packet,
      registration({ style_id: 'another-style' }),
      mismatched.manifest
    ),
    /manifest style ID is coverage-fixture/
  );
});

test('coverage checker rejects stale registered input hashes', () => {
  const manifest = readManifest();
  manifest.style.sha256 = '0'.repeat(64);

  assert.throws(() => verifyManifestFiles(manifest), /style hash mismatch/);
});

test('coverage checker requests byte-for-byte regeneration with the source-built binary', () => {
  const calls = [];
  runRegenerationCheck(registration(), '/tmp/source-built-citum', {
    execFileSync(command, args) {
      calls.push({ command, args });
      return '';
    },
  });

  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, process.execPath);
  assert.equal(calls[0].args.includes('--check'), true);
  assert.equal(calls[0].args.at(calls[0].args.indexOf('--citum-bin') + 1), '/tmp/source-built-citum');

  assert.throws(
    () => runRegenerationCheck(registration(), '/tmp/source-built-citum', {
      execFileSync() {
        const error = new Error('regeneration failed');
        error.stderr = 'JSON packet is stale';
        throw error;
      },
    }),
    /byte regeneration failed: JSON packet is stale/
  );
});

test('coverage audit view groups fields by stable output identity with collapsed evidence data', () => {
  const { manifest, packet } = baselineFixture();
  validateCoveragePacket(packet, registration(), manifest);
  const report = readJson('report.json');
  const view = buildCoverageAuditView(packet, registration(), report);
  const bibliography = view.outputGroups.find((group) => (
    group.surface === 'bibliography' && group.outputId === 'ITEM-1'
  ));

  assert.equal(view.status, 'current');
  assert.equal(bibliography.fields.length, 3);
  assert.equal(bibliography.fields.every((field) => field.observationId.includes('coverage-fixture/')), true);
  assert.equal(bibliography.dispositions.includes('excluded'), true);
  assert.equal(bibliography.comparisonState, 'mismatch');
  assert.equal(typeof bibliography.exactEvidence.oracle, 'string');
  assert.equal(view.adjudicationRecord.href.endsWith('.json'), false);
});

test('coverage audit view measures current before/after output evidence separately from the packet', () => {
  const { manifest, packet } = baselineFixture();
  validateCoveragePacket(packet, registration(), manifest);
  const report = readJson('report.json');
  const view = buildCoverageAuditView(packet, registration(), report, report);
  assert.equal(view.postChangeEvidence.status, 'measured');
  assert.deepEqual(view.postChangeEvidence.beforeExactParity, view.postChangeEvidence.afterExactParity);
  assert.equal(view.postChangeEvidence.changedOutputs.length, 0);
  assert.equal(view.outputGroups.every((group) => group.postChangeEvidence), true);

  const changedReport = structuredClone(report);
  changedReport.styles[0].oracleDetail[0].exactCitum += ' changed';
  const changedView = buildCoverageAuditView(packet, registration(), report, changedReport);
  assert.equal(changedView.postChangeEvidence.changedOutputs.length, 1);
  assert.equal(changedView.postChangeEvidence.changedOutputs[0].surface, 'bibliography');
});

test('status lookup reports unregistered styles without selecting arbitrary audit directories', () => {
  const provenance = { coverage_audits: [registration()] };

  assert.deepEqual(selectRegistrations(provenance, 'missing-style'), []);
  assert.deepEqual(selectRegistrations(provenance, 'coverage-fixture'), [registration()]);
});
