const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const yaml = require('js-yaml');

const WORKSPACE_ROOT = path.resolve(__dirname, '..');
const REGISTRY_PATH = path.join(
  WORKSPACE_ROOT,
  'crates',
  'citum-schema-style',
  'embedded',
  'registry',
  'default.yaml'
);
const CORE_URL_PREFIX = 'https://raw.githubusercontent.com/citum/citum-core/main/styles/';
const STYLES_ROOT = path.join(WORKSPACE_ROOT, 'styles');

// Regression guard for the 2026-07 community-corpus split (see
// docs/architecture/audits/): 123 registry entries kept pointing at
// citum-core paths for styles that had moved to citum/citum-styles, so
// every non-embedded registry fetch for those ids 404'd. This asserts every
// citum-core-hosted registry URL still resolves to a real file on disk, so
// that class of breakage cannot recur silently.
test('every citum-core registry URL resolves to a file that exists under styles/', () => {
  const doc = yaml.load(fs.readFileSync(REGISTRY_PATH, 'utf8'));
  assert.ok(Array.isArray(doc.styles), 'registry default.yaml must have a styles array');

  const missing = [];
  for (const entry of doc.styles) {
    if (!entry.url || !entry.url.startsWith(CORE_URL_PREFIX)) continue;
    const relativePath = entry.url.slice(CORE_URL_PREFIX.length);
    const absolutePath = path.join(STYLES_ROOT, relativePath);
    if (!fs.existsSync(absolutePath)) {
      missing.push(`${entry.id} -> ${relativePath}`);
    }
  }

  assert.deepEqual(
    missing,
    [],
    `registry entries point at missing citum-core style files:\n${missing.join('\n')}`
  );
});
