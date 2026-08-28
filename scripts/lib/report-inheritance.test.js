'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  buildFamilyAggregates,
  buildInheritanceIndex,
  classifyImplementation,
  loadMeasurementEvidence,
} = require('./report-inheritance');

function writeFixture(root, relativePath, content) {
  const target = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
  return target;
}

test('inheritance index resolves hidden roots, forms, aliases, and latest evidence', (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'citum-report-inheritance-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const styles = path.join(root, 'styles');
  const embedded = path.join(styles, 'embedded');
  const reportData = path.join(root, 'report-data');

  writeFixture(root, 'styles/embedded/family-base.yaml', 'options:\n  processing: author-date\n');
  writeFixture(
    root,
    'styles/embedded/family-core.yaml',
    'extends: family-base\nbibliography:\n  template:\n  - title: primary\n'
  );
  writeFixture(
    root,
    'styles/member.yaml',
    'extends: family-core\noptions:\n  range-format: expanded\n'
  );
  const registryPath = writeFixture(
    root,
    'registry.yaml',
    'styles:\n- id: member\n  kind: profile\n  aliases:\n  - member-short\n  - member-journal\n'
  );
  writeFixture(
    root,
    'report-data/alias-candidates-band-registered-2026-07-01.tsv',
    'candidate_id\tbest_target\tsimilarity\tcitation_match\tbib_match\tevidence_url\tconfidence_note\tband\nmember\told\t0.8\t1\t1\t\t\tnear-clone\n'
  );
  writeFixture(
    root,
    'report-data/alias-candidates-band-registered-2026-07-20.tsv',
    'candidate_id\tbest_target\tsimilarity\tcitation_match\tbib_match\tevidence_url\tconfidence_note\tband\nmember\tfamily-core\t1\t1\t1\thttps://example.test\tverified\talias\n'
  );
  writeFixture(
    root,
    'report-data/delta-derivability-styles-2026-07-20.tsv',
    'candidate_id\ttarget_id\tsimilarity\tstandalone_fidelity\twrapper_fidelity\twrapper_bytes\twrapper_key_count\tverdict\tdetail\nmember\tfamily-core\t1\t1\t1\t120\t2\tdelta-expressible\tclean\n'
  );

  const index = buildInheritanceIndex({
    styleRoots: [styles, embedded],
    registryPath,
    reportDataDir: reportData,
  });
  const member = index.records.get('member');

  assert.deepEqual(member.inheritance.chain, ['member', 'family-core', 'family-base']);
  assert.equal(member.inheritance.familyRoot, 'family-base');
  assert.equal(member.inheritance.implementationForm, 'config-wrapper');
  assert.equal(index.records.get('family-core').inheritance.implementationForm, 'structural-wrapper');
  assert.deepEqual(member.registry.aliases, ['member-journal', 'member-short']);
  assert.equal(member.registry.aliasCount, 2);
  assert.equal(member.measurementEvidence.behavioralBand.target, 'family-core');
  assert.equal(member.measurementEvidence.behavioralBand.band, 'alias');
  assert.equal(member.measurementEvidence.derivability.verdict, 'delta-expressible');
});

test('inheritance index reports missing parents and cycles without failing', (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'citum-report-broken-chain-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const styles = path.join(root, 'styles');

  writeFixture(root, 'styles/missing.yaml', 'extends: absent-parent\n');
  writeFixture(root, 'styles/cycle-a.yaml', 'extends: cycle-b\n');
  writeFixture(root, 'styles/cycle-b.yaml', 'extends: cycle-a\n');
  const index = buildInheritanceIndex({
    styleRoots: [styles],
    registryPath: path.join(root, 'absent-registry.yaml'),
    reportDataDir: path.join(root, 'absent-report-data'),
  });

  assert.deepEqual(index.records.get('missing').inheritance.chain, ['missing', 'absent-parent']);
  assert.equal(index.records.get('missing').inheritance.missingParent, 'absent-parent');
  assert.equal(index.records.get('missing').inheritance.complete, false);
  assert.deepEqual(index.records.get('cycle-a').inheritance.cycle, ['cycle-a', 'cycle-b', 'cycle-a']);
  assert.equal(index.records.get('cycle-b').inheritance.familyRoot, 'cycle-a');
  assert.equal(index.records.get('missing').measurementEvidence.behavioralBand, null);
});

test('implementation form ignores option and sort-only wrapper configuration', () => {
  assert.equal(classifyImplementation({ options: { processing: 'numeric' } }), 'standalone');
  assert.equal(
    classifyImplementation({
      extends: 'parent',
      citation: { sort: [{ key: 'issued' }] },
    }),
    'config-wrapper'
  );
  assert.equal(
    classifyImplementation({
      extends: 'parent',
      citation: { 'non-integral': { template: [{ contributor: 'author' }] } },
    }),
    'structural-wrapper'
  );
});

test('optional evidence loader returns unavailable sources when artifacts are absent', (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'citum-report-no-evidence-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));

  const evidence = loadMeasurementEvidence(root);
  assert.equal(evidence.sources.behavioralBands, null);
  assert.equal(evidence.sources.derivability, null);
  assert.equal(evidence.bands.size, 0);
  assert.equal(evidence.derivability.size, 0);
});

test('family aggregates order by reach then root and sort members deterministically', () => {
  const families = buildFamilyAggregates([
    {
      name: 'zeta-member',
      cslReach: 3,
      inheritance: { familyRoot: 'family-z' },
      registry: { aliases: ['z'] },
      citations: { passed: 2, total: 2 },
      bibliography: { passed: 1, total: 2 },
      exactParity: { passed: 2, total: 4, notComparable: 1 },
      pairingSummary: { paired: 3, unresolvedUnpaired: 1, totalObservations: 4 },
    },
    {
      name: 'alpha-member',
      cslReach: 7,
      inheritance: { familyRoot: 'family-z' },
      registry: { aliases: ['a'] },
      citations: { passed: 1, total: 2 },
      bibliography: { passed: 2, total: 2 },
      exactParity: { passed: 3, total: 4, notComparable: 2 },
      pairingSummary: { paired: 4, idProvenOracleOnly: 1, totalObservations: 5 },
    },
    {
      name: 'solo',
      cslReach: 9,
      inheritance: { familyRoot: 'family-a' },
      registry: { aliases: [] },
      citations: { passed: 1, total: 1 },
      bibliography: { passed: 1, total: 1 },
      exactParity: { passed: 2, total: 2 },
      pairingSummary: { paired: 2, totalObservations: 2 },
    },
  ]);

  assert.deepEqual(families.map((family) => family.root), ['family-z', 'family-a']);
  assert.deepEqual(families[0].members, ['alpha-member', 'zeta-member']);
  assert.equal(families[0].aggregateCslReach, 10);
  assert.deepEqual(families[0].compatibility, {
    passed: 6,
    total: 8,
    notComparable: 0,
    unresolvedPairing: 1,
  });
  assert.deepEqual(families[0].exactParity, {
    passed: 5,
    total: 8,
    notComparable: 3,
    unresolvedPairing: 0,
  });
  assert.deepEqual(families[0].pairing, {
    paired: 7,
    unresolvedUnpaired: 1,
    idProvenOracleOnly: 1,
    idProvenCitumOnly: 0,
    totalObservations: 9,
  });
});
