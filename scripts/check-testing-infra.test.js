const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('fs');
const os = require('os');
const path = require('path');

const {
  validateCoreQualityBaseline,
  validateCoverageManifest,
  validateOracleBaseline,
  validateVerificationPolicyFiles,
} = require('./check-testing-infra');

function makeTempProject() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'citum-testing-infra-'));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function writeFile(filePath, contents = '') {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents, 'utf8');
}

function writeYaml(filePath, contents) {
  writeFile(filePath, `${contents.trim()}\n`);
}

function baseManifest() {
  return {
    version: 1,
    fixtures: [
      {
        fixture: 'tests/fixtures/references-expanded.json',
        domain: 'core',
        kind: 'references',
        reference_types: ['book'],
        citation_scenarios: [],
        rendering_risks: ['core rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'core',
      },
      {
        fixture: 'tests/fixtures/citations-note-expanded.json',
        domain: 'note',
        kind: 'citations',
        'pairs-with': 'tests/fixtures/references-expanded.json',
        reference_types: ['book'],
        citation_scenarios: ['single item'],
        rendering_risks: ['note rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'note',
      },
      {
        fixture: 'tests/fixtures/references-note-positions.json',
        domain: 'note',
        kind: 'references',
        reference_types: ['article-journal'],
        citation_scenarios: [],
        rendering_risks: ['repeated-note rendering'],
        used_by: ['scripts/audit-note-positions.js'],
        ci_status: 'required',
        notes: 'note positions refs',
      },
      {
        fixture: 'tests/fixtures/citations-note-positions.json',
        domain: 'note',
        kind: 'citations',
        'pairs-with': 'tests/fixtures/references-note-positions.json',
        reference_types: ['article-journal'],
        citation_scenarios: ['first', 'ibid', 'subsequent'],
        rendering_risks: ['repeated-note rendering'],
        used_by: ['scripts/audit-note-positions.js'],
        ci_status: 'required',
        notes: 'note positions cites',
      },
      {
        fixture: 'tests/fixtures/references-legal.json',
        domain: 'legal',
        kind: 'references',
        reference_types: ['legal_case'],
        citation_scenarios: [],
        rendering_risks: ['legal rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'legal',
      },
      {
        fixture: 'tests/fixtures/references-scientific.json',
        domain: 'scientific',
        kind: 'references',
        reference_types: ['dataset'],
        citation_scenarios: [],
        rendering_risks: ['scientific rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'scientific',
      },
      {
        fixture: 'tests/fixtures/references-multilingual.yaml',
        domain: 'multilingual',
        kind: 'references',
        reference_types: ['book'],
        citation_scenarios: [],
        rendering_risks: ['multilingual rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'multilingual',
      },
      {
        fixture: 'tests/fixtures/grouping/primary-secondary.json',
        domain: 'grouping',
        kind: 'references',
        reference_types: ['manuscript'],
        citation_scenarios: [],
        rendering_risks: ['grouping rendering'],
        used_by: ['scripts/oracle.js'],
        ci_status: 'required',
        notes: 'grouping',
      },
    ],
  };
}

function baselineMetadata() {
  return {
    timestamp: '2026-02-27T00:00:00.000Z',
    gitCommit: 'abcdef0',
    generator: 'scripts/report-core.js',
    fixture: 'tests/fixtures/references-expanded.json',
    styleSelector: 'core-styles',
    styles: ['apa-7th'],
  };
}

function seedProject(root) {
  writeJson(path.join(root, 'tests/fixtures/coverage-manifest.json'), baseManifest());
  writeJson(path.join(root, 'tests/fixtures/references-expanded.json'), { ITEM: { type: 'book' } });
  writeJson(path.join(root, 'tests/fixtures/citations-note-expanded.json'), []);
  writeJson(path.join(root, 'tests/fixtures/references-note-positions.json'), { ITEM: { type: 'article-journal' } });
  writeJson(path.join(root, 'tests/fixtures/citations-note-positions.json'), []);
  writeJson(path.join(root, 'tests/fixtures/references-legal.json'), {});
  writeJson(path.join(root, 'tests/fixtures/references-scientific.json'), {});
  writeFile(path.join(root, 'tests/fixtures/references-multilingual.yaml'), '[]\n');
  writeJson(path.join(root, 'tests/fixtures/grouping/primary-secondary.json'), []);
  writeFile(path.join(root, 'scripts/oracle.js'), '#!/usr/bin/env node\n');
  writeFile(path.join(root, 'scripts/audit-note-positions.js'), '#!/usr/bin/env node\n');
  writeJson(path.join(root, 'scripts/report-data/oracle-top10-baseline.json'), {
    styleBreakdown: [{ style: 'apa', citations: '1/1', bibliography: '1/1' }],
    metadata: {
      ...baselineMetadata(),
      generator: 'scripts/oracle-batch-aggregate.js',
    },
  });
  writeJson(path.join(root, 'scripts/report-data/core-quality-baseline.json'), {
    generated: '2026-02-27',
    commit: 'abcdef0',
    source: 'scripts/report-core.js',
    styles: { 'apa-7th': { fidelityScore: 1, quality: 90, concision: 90, presetUsage: 90 } },
    metadata: baselineMetadata(),
  });
  writeYaml(path.join(root, 'scripts/report-data/verification-policy.yaml'), `
version: 1
defaults:
  authority: citeproc-js
  secondary: []
  scopes:
    - citation
    - bibliography
divergences:
  div-004:
    scopes:
      - citation
      - bibliography
    tags:
      - missing-name-title-sort-order
      - sort-derived-numeric-citation-label
    note: Missing-name sort divergence.
styles:
  apa-7th:
    fixture_family: author-date
    secondary:
      - biblatex
`);
  writeYaml(path.join(root, 'scripts/report-data/fixture-sufficiency.yaml'), `
version: 1
defaults:
  release_report_fixture: core
families:
  author-date:
    default_report_sufficient: false
    required_reference_types:
      - article-journal
    required_scenarios:
      - same-author same-year collisions
    fixture_sets:
      - core
`);
}

test('validation passes for a complete testing-infra fixture set', () => {
  const root = makeTempProject();
  seedProject(root);

  assert.doesNotThrow(() => validateCoverageManifest(root));
  assert.doesNotThrow(() => validateOracleBaseline(root));
  assert.doesNotThrow(() => validateCoreQualityBaseline(root));
  assert.doesNotThrow(() => validateVerificationPolicyFiles(root));
});

test('coverage manifest fails when a fixture path is missing', () => {
  const root = makeTempProject();
  seedProject(root);
  fs.rmSync(path.join(root, 'tests/fixtures/references-scientific.json'));

  assert.throws(() => validateCoverageManifest(root), /Fixture path does not exist/);
});

test('coverage manifest fails when a used_by path is missing', () => {
  const root = makeTempProject();
  seedProject(root);
  const manifestPath = path.join(root, 'tests/fixtures/coverage-manifest.json');
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  manifest.fixtures[0].used_by = ['scripts/missing.js'];
  writeJson(manifestPath, manifest);

  assert.throws(() => validateCoverageManifest(root), /Referenced owner path does not exist/);
});

test('coverage manifest fails when a citations fixture is missing pairs-with', () => {
  const root = makeTempProject();
  seedProject(root);
  const manifestPath = path.join(root, 'tests/fixtures/coverage-manifest.json');
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const citationsEntry = manifest.fixtures.find((entry) => entry.kind === 'citations');
  delete citationsEntry['pairs-with'];
  writeJson(manifestPath, manifest);

  assert.throws(() => validateCoverageManifest(root), /must set pairs-with/);
});

test('coverage manifest fails when pairs-with points at a non-references fixture', () => {
  const root = makeTempProject();
  seedProject(root);
  const manifestPath = path.join(root, 'tests/fixtures/coverage-manifest.json');
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const citationsEntry = manifest.fixtures.find(
    (entry) => entry.fixture === 'tests/fixtures/citations-note-expanded.json',
  );
  citationsEntry['pairs-with'] = 'tests/fixtures/citations-note-positions.json';
  writeJson(manifestPath, manifest);

  assert.throws(() => validateCoverageManifest(root), /must be a references fixture/);
});

test('coverage manifest fails when pairs-with targets an unknown fixture', () => {
  const root = makeTempProject();
  seedProject(root);
  const manifestPath = path.join(root, 'tests/fixtures/coverage-manifest.json');
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  const citationsEntry = manifest.fixtures.find(
    (entry) => entry.fixture === 'tests/fixtures/citations-note-expanded.json',
  );
  citationsEntry['pairs-with'] = 'tests/fixtures/references-unknown.json';
  writeJson(manifestPath, manifest);

  assert.throws(() => validateCoverageManifest(root), /pairs-with target does not exist/);
});

test('baseline validation fails when metadata fields are missing', () => {
  const root = makeTempProject();
  seedProject(root);
  const baselinePath = path.join(root, 'scripts/report-data/core-quality-baseline.json');
  const baseline = JSON.parse(fs.readFileSync(baselinePath, 'utf8'));
  delete baseline.metadata.generator;
  writeJson(baselinePath, baseline);

  assert.throws(() => validateCoreQualityBaseline(root), /missing generator/);
});

test('coverage manifest fails when structure is malformed', () => {
  const root = makeTempProject();
  seedProject(root);
  writeJson(path.join(root, 'tests/fixtures/coverage-manifest.json'), { version: 1, fixtures: {} });

  assert.throws(() => validateCoverageManifest(root), /fixtures must be an array/);
});

test('verification policy fails when it references an unknown fixture family', () => {
  const root = makeTempProject();
  seedProject(root);
  writeYaml(path.join(root, 'scripts/report-data/verification-policy.yaml'), `
version: 1
defaults:
  authority: citeproc-js
  secondary: []
  scopes:
    - citation
    - bibliography
divergences:
  div-004:
    scopes:
      - citation
      - bibliography
    tags:
      - missing-name-title-sort-order
      - sort-derived-numeric-citation-label
styles:
  apa-7th:
    fixture_family: missing-family
`);

  assert.throws(() => validateVerificationPolicyFiles(root), /unknown fixture family/);
});

test('verification policy fails when a registered divergence is malformed', () => {
  const root = makeTempProject();
  seedProject(root);
  writeYaml(path.join(root, 'scripts/report-data/verification-policy.yaml'), `
version: 1
defaults:
  authority: citeproc-js
  secondary: []
  scopes:
    - citation
    - bibliography
divergences:
  div-004:
    scopes: citation
    tags:
      - missing-name-title-sort-order
styles:
  apa-7th:
    fixture_family: author-date
`);

  assert.throws(() => validateVerificationPolicyFiles(root), /divergences\.div-004\.scopes must be an array/);
});
