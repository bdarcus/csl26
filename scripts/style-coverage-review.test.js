'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const yaml = require('js-yaml');

const {
  buildCoveragePacket,
  markdownPacket,
  refreshManifestHashes,
  stableJson,
  validateBaselineRuntime,
  validateManifest,
  writeManifestYaml,
} = require('./style-coverage-review');

const PROJECT_ROOT = path.dirname(__dirname);
const FIXTURE_ROOT = path.join(PROJECT_ROOT, 'tests', 'fixtures', 'style-coverage');

function readJson(name) {
  return JSON.parse(fs.readFileSync(path.join(FIXTURE_ROOT, name), 'utf8'));
}

function readManifest() {
  return yaml.load(fs.readFileSync(path.join(FIXTURE_ROOT, 'manifest.yaml'), 'utf8'));
}

function buildFixturePacket(overrides = {}) {
  return buildCoveragePacket({
    manifest: readManifest(),
    manifestPath: path.join(FIXTURE_ROOT, 'manifest.yaml'),
    resolvedStyle: readJson('resolved-style.json'),
    report: readJson('report.json'),
    references: readJson('references.json'),
    citations: readJson('citations.json'),
    runtime: {
      worktreeDirty: true,
      resolverVersion: 'citum 0.79.0',
    },
    ...overrides,
  });
}

test('coverage packet matches the complete checked-in golden files byte for byte', () => {
  const packet = buildFixturePacket();

  assert.equal(
    stableJson(packet),
    fs.readFileSync(path.join(FIXTURE_ROOT, 'packet.json'), 'utf8')
  );
  assert.equal(
    markdownPacket(packet),
    fs.readFileSync(path.join(FIXTURE_ROOT, 'packet.md'), 'utf8')
  );
  assert.equal(packet.observations.length, 8);
  assert.equal(packet.observations.at(-1).row, 8);
});

test('coverage dimensions remain independent', () => {
  const packet = buildFixturePacket();
  const byFieldAndSurface = new Map(packet.observations.map((observation) => [
    `${observation.surface}/${observation.referenceId}/${observation.field}`,
    observation,
  ]));

  assert.deepEqual(
    {
      relevance: byFieldAndSurface.get('bibliography/ITEM-1/license').relevance,
      comparison: byFieldAndSurface.get('bibliography/ITEM-1/license').comparisonEligibility,
    },
    { relevance: 'excluded', comparison: 'comparable' }
  );
  assert.deepEqual(
    {
      render: byFieldAndSurface.get('citation/ITEM-1/publisher').renderDisposition,
      comparison: byFieldAndSurface.get('citation/ITEM-1/publisher').comparisonEligibility,
    },
    { render: 'suppressed', comparison: 'comparable' }
  );
  assert.deepEqual(
    {
      render: byFieldAndSurface.get('bibliography/ITEM-2/title').renderDisposition,
      comparison: byFieldAndSurface.get('bibliography/ITEM-2/title').comparisonEligibility,
    },
    { render: 'fallback', comparison: 'not-comparable' }
  );
  assert.deepEqual(packet.summary.joinedExactParity, {
    passed: 1,
    total: 2,
    notComparable: 1,
  });
});

test('coverage observations use stable codepoint-sorted identities', () => {
  const packet = buildFixturePacket();
  const identities = packet.observations.map((observation) => observation.observationId);
  const sorted = [...identities].sort((left, right) => left < right ? -1 : left > right ? 1 : 0);

  assert.deepEqual(identities, sorted);
  assert.equal(new Set(identities).size, identities.length);
  assert.equal(
    identities[6],
    'coverage-fixture/citation/minimal-citations/ITEM-1/book/publisher/cite-1%3A1'
  );
});

test('coverage packet rejects an exact-parity join containing only null results', () => {
  const report = readJson('report.json');
  for (const entry of [
    ...report.styles[0].citationEntries,
    ...report.styles[0].oracleDetail,
  ]) {
    entry.exactMatch = null;
    entry.exactParityEligible = false;
  }

  assert.throws(
    () => buildFixturePacket({ report }),
    /join produced no non-null exactMatch values for baseline/
  );
});

test('coverage packet rejects observation-count drift', () => {
  const manifest = readManifest();
  manifest['expected-observations'] = 7;

  assert.throws(
    () => buildFixturePacket({ manifest }),
    /Observation count mismatch: expected 7, got 8.*--update-manifest/s
  );
});

test('coverage packet tolerates observation-count drift when refresh is requested', () => {
  const manifest = readManifest();
  manifest['expected-observations'] = 7;

  const packet = buildFixturePacket({ manifest, enforceExpectedObservations: false });

  assert.equal(packet.observations.length, 8);
});

test('refreshManifestHashes re-pins every hash against the files on disk', () => {
  const manifest = readManifest();
  manifest.style.sha256 = 'stale';
  manifest.style.chain[0].sha256 = 'stale';
  manifest.fixtures.references.sha256 = 'stale';
  manifest.fixtures.citations.sha256 = 'stale';
  manifest.authority.report.sha256 = 'stale';
  manifest.authority.inputs[0].sha256 = 'stale';

  const refreshed = refreshManifestHashes(manifest);
  const canonical = readManifest();

  assert.equal(refreshed.style.sha256, canonical.style.sha256);
  assert.equal(refreshed.style.chain[0].sha256, canonical.style.chain[0].sha256);
  assert.equal(refreshed.fixtures.references.sha256, canonical.fixtures.references.sha256);
  assert.equal(refreshed.fixtures.citations.sha256, canonical.fixtures.citations.sha256);
  assert.equal(refreshed.authority.report.sha256, canonical.authority.report.sha256);
  assert.equal(refreshed.authority.inputs[0].sha256, canonical.authority.inputs[0].sha256);
  // source.revision / worktree-clean are a maintainer's baseline declaration, not a hash pin.
  assert.equal(refreshed.source.revision, 'fixture');
  assert.equal(refreshed.source['worktree-clean'], false);
});

test('writeManifestYaml round-trips through validateManifest', () => {
  const manifest = readManifest();
  refreshManifestHashes(manifest);
  const tmpPath = path.join(FIXTURE_ROOT, '.tmp-manifest-roundtrip.yaml');

  try {
    writeManifestYaml(tmpPath, manifest);
    const reloaded = validateManifest(yaml.load(fs.readFileSync(tmpPath, 'utf8')));
    assert.deepEqual(reloaded, manifest);
  } finally {
    fs.rmSync(tmpPath, { force: true });
  }
});

test('coverage manifest requires a rationale for every exclusion', () => {
  const manifest = readManifest();
  delete manifest.relevance.excluded[0].rationale;

  assert.throws(
    () => validateManifest(manifest),
    /Invalid coverage manifest.*rationale/
  );
});

test('baseline provenance pins audited inputs rather than the generator HEAD', () => {
  const manifest = readManifest();
  manifest.source['worktree-clean'] = true;
  manifest.source.revision = 'audited-source-revision';

  assert.doesNotThrow(() => validateBaselineRuntime(manifest, {
    revision: 'later-generator-revision',
    worktreeDirty: false,
  }));
  assert.throws(
    () => validateBaselineRuntime(manifest, {
      revision: 'audited-source-revision',
      worktreeDirty: true,
    }),
    /Baseline manifest requires a clean worktree/
  );
});
