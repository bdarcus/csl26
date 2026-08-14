#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const {
  loadFixtureSufficiency,
  loadVerificationPolicy,
} = require('./lib/verification-policy');

const PROJECT_ROOT = path.resolve(__dirname, '..');
const REQUIRED_DOMAINS = new Set([
  'core',
  'note',
  'legal',
  'scientific',
  'multilingual',
  'grouping',
]);
const ALLOWED_CI_STATUS = new Set(['required', 'advisory', 'manual']);
const REQUIRED_METADATA_FIELDS = ['timestamp', 'gitCommit', 'generator', 'fixture'];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function isNonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

function ensureStringArray(value, label) {
  assert(Array.isArray(value), `${label} must be an array`);
  assert(value.length > 0, `${label} must not be empty`);
  for (const entry of value) {
    assert(isNonEmptyString(entry), `${label} entries must be non-empty strings`);
  }
}

function validateManifestEntry(entry, seenFixtures, seenDomains, projectRoot) {
  assert(entry && typeof entry === 'object' && !Array.isArray(entry), 'Fixture entries must be objects');
  assert(isNonEmptyString(entry.fixture), 'Fixture entry is missing fixture path');
  assert(!seenFixtures.has(entry.fixture), `Duplicate fixture entry: ${entry.fixture}`);
  seenFixtures.add(entry.fixture);

  assert(isNonEmptyString(entry.domain), `Fixture ${entry.fixture} is missing domain`);
  assert(REQUIRED_DOMAINS.has(entry.domain), `Fixture ${entry.fixture} has invalid domain: ${entry.domain}`);
  seenDomains.add(entry.domain);

  assert(isNonEmptyString(entry.kind), `Fixture ${entry.fixture} is missing kind`);
  ensureStringArray(entry.reference_types, `${entry.fixture} reference_types`);
  assert(Array.isArray(entry.citation_scenarios), `${entry.fixture} citation_scenarios must be an array`);
  ensureStringArray(entry.rendering_risks, `${entry.fixture} rendering_risks`);
  ensureStringArray(entry.used_by, `${entry.fixture} used_by`);
  assert(ALLOWED_CI_STATUS.has(entry.ci_status), `Fixture ${entry.fixture} has invalid ci_status: ${entry.ci_status}`);
  assert(isNonEmptyString(entry.notes), `Fixture ${entry.fixture} is missing notes`);

  const fixturePath = path.resolve(projectRoot, entry.fixture);
  assert(fs.existsSync(fixturePath), `Fixture path does not exist: ${entry.fixture}`);

  for (const scriptPath of entry.used_by) {
    const resolved = path.resolve(projectRoot, scriptPath);
    assert(fs.existsSync(resolved), `Referenced owner path does not exist for ${entry.fixture}: ${scriptPath}`);
  }

  // Every citations fixture must record which references fixture it renders
  // against, so contributors can navigate the fixture directory without
  // reverse-engineering the pairing from scripts. See
  // docs/guides/RENDERING_WORKFLOW.md.
  if (entry.kind === 'citations') {
    assert(isNonEmptyString(entry['pairs-with']), `Fixture ${entry.fixture} is a citations fixture and must set pairs-with`);
    const pairedPath = path.resolve(projectRoot, entry['pairs-with']);
    assert(fs.existsSync(pairedPath), `pairs-with target does not exist for ${entry.fixture}: ${entry['pairs-with']}`);
  }
}

// Cross-check `pairs-with` targets once every entry is known: the target
// must itself be a manifest entry of kind `references`. Run as a second
// pass over the full fixture list so pairing order in the manifest doesn't
// matter.
function validatePairsWithTargets(fixtures) {
  const byPath = new Map(fixtures.map((entry) => [entry.fixture, entry]));
  for (const entry of fixtures) {
    if (entry.kind !== 'citations') continue;
    const target = byPath.get(entry['pairs-with']);
    assert(target, `pairs-with target for ${entry.fixture} is not a manifest entry: ${entry['pairs-with']}`);
    assert(
      target.kind === 'references',
      `pairs-with target for ${entry.fixture} must be a references fixture, got kind=${target.kind}: ${entry['pairs-with']}`,
    );
  }
}

function validateCoverageManifest(projectRoot = PROJECT_ROOT) {
  const manifestPath = path.join(projectRoot, 'tests', 'fixtures', 'coverage-manifest.json');
  const manifest = readJson(manifestPath);

  assert(manifest && typeof manifest === 'object' && !Array.isArray(manifest), 'coverage-manifest.json must be an object');
  assert(manifest.version === 1, 'coverage-manifest.json version must be 1');
  assert(Array.isArray(manifest.fixtures), 'coverage-manifest.json fixtures must be an array');
  assert(manifest.fixtures.length > 0, 'coverage-manifest.json fixtures must not be empty');

  const seenFixtures = new Set();
  const seenDomains = new Set();

  for (const entry of manifest.fixtures) {
    validateManifestEntry(entry, seenFixtures, seenDomains, projectRoot);
  }

  validatePairsWithTargets(manifest.fixtures);

  for (const domain of REQUIRED_DOMAINS) {
    assert(seenDomains.has(domain), `coverage-manifest.json is missing required domain: ${domain}`);
  }

  return manifest;
}

function validateMetadataFields(metadata, label) {
  assert(metadata && typeof metadata === 'object' && !Array.isArray(metadata), `${label} is missing metadata object`);
  for (const field of REQUIRED_METADATA_FIELDS) {
    assert(isNonEmptyString(metadata[field]), `${label} metadata is missing ${field}`);
  }
  const hasStyleList = Array.isArray(metadata.styles) && metadata.styles.length > 0;
  const hasStyleSelector = isNonEmptyString(metadata.styleSelector);
  assert(hasStyleList || hasStyleSelector, `${label} metadata requires styles or styleSelector`);
}

function validateOracleBaseline(projectRoot = PROJECT_ROOT) {
  const baselinePath = path.join(projectRoot, 'scripts', 'report-data', 'oracle-top10-baseline.json');
  const baseline = readJson(baselinePath);

  assert(Array.isArray(baseline.styleBreakdown), 'oracle-top10-baseline.json is missing styleBreakdown');
  assert(baseline.styleBreakdown.length > 0, 'oracle-top10-baseline.json styleBreakdown must not be empty');
  validateMetadataFields(baseline.metadata, 'oracle-top10-baseline.json');
  return baseline;
}

function validateCoreQualityBaseline(projectRoot = PROJECT_ROOT) {
  const baselinePath = path.join(projectRoot, 'scripts', 'report-data', 'core-quality-baseline.json');
  const baseline = readJson(baselinePath);

  assert(baseline.styles && typeof baseline.styles === 'object', 'core-quality-baseline.json is missing styles');
  assert(Object.keys(baseline.styles).length > 0, 'core-quality-baseline.json styles must not be empty');
  validateMetadataFields(baseline.metadata, 'core-quality-baseline.json');
  return baseline;
}

function validateVerificationPolicyFiles(projectRoot = PROJECT_ROOT) {
  const policyPath = path.join(projectRoot, 'scripts', 'report-data', 'verification-policy.yaml');
  const sufficiencyPath = path.join(projectRoot, 'scripts', 'report-data', 'fixture-sufficiency.yaml');

  assert(fs.existsSync(policyPath), `verification policy file not found: ${policyPath}`);
  assert(fs.existsSync(sufficiencyPath), `fixture sufficiency file not found: ${sufficiencyPath}`);

  const policy = loadVerificationPolicy(policyPath);
  const sufficiency = loadFixtureSufficiency(sufficiencyPath);

  for (const [styleName, stylePolicy] of Object.entries(policy.styles || {})) {
    const familyName = stylePolicy.fixture_family;
    if (familyName) {
      assert(
        Object.prototype.hasOwnProperty.call(sufficiency.families, familyName),
        `verification-policy.yaml style ${styleName} references unknown fixture family: ${familyName}`
      );
    }
  }

  return { policy, sufficiency };
}

function runChecks(projectRoot = PROJECT_ROOT) {
  validateCoverageManifest(projectRoot);
  validateOracleBaseline(projectRoot);
  validateCoreQualityBaseline(projectRoot);
  validateVerificationPolicyFiles(projectRoot);
}

function main() {
  try {
    runChecks();
    console.log('Testing infrastructure contracts are valid.');
  } catch (error) {
    console.error(`check-testing-infra failed: ${error.message}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  ALLOWED_CI_STATUS,
  PROJECT_ROOT,
  REQUIRED_DOMAINS,
  REQUIRED_METADATA_FIELDS,
  runChecks,
  validateCoreQualityBaseline,
  validateCoverageManifest,
  validateMetadataFields,
  validateOracleBaseline,
  validateVerificationPolicyFiles,
};
