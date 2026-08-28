const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const yaml = require('js-yaml');

const {
  mergeStructuredResults,
  parseArgs,
  shouldUseStructuredOracle,
} = require('./oracle-yaml');
const { resolveYamlVerificationPlan } = require('./lib/style-verification');
const { deepMerge, resolveStyleData } = require('./lib/verification-policy');

function planFor(styleFile, overrides = {}) {
  const isEmbedded = [
    'apa-7th.yaml',
    'chicago-notes-18th.yaml',
    'chicago-author-date-18th.yaml',
    'ieee.yaml',
    'taylor-and-francis-chicago-author-date.yaml',
  ].includes(styleFile);
  const subDir = isEmbedded ? 'embedded' : '';

  return resolveYamlVerificationPlan({
    yamlPath: path.join(__dirname, '..', 'styles', subDir, styleFile),
    styleFormat: overrides.styleFormat,
    hasBibliography: overrides.hasBibliography,
  });
}

test('oracle-yaml parses family-aware CLI flags', () => {
  const options = parseArgs([
    'styles/embedded/chicago-notes-18th.yaml',
    '--legacy-csl',
    'styles-legacy/chicago-notes.csl',
    '--fixture-family',
    'note-humanities',
    '--refs-fixture',
    'tests/fixtures/references-humanities-note.json',
    '--citations-fixture',
    'tests/fixtures/citations-humanities-note.json',
    '--json',
  ]);

  assert.equal(options.yamlPath, 'styles/embedded/chicago-notes-18th.yaml');
  assert.equal(options.legacyCslPath, 'styles-legacy/chicago-notes.csl');
  assert.equal(options.fixtureFamily, 'note-humanities');
  assert.equal(options.refsFixture, 'tests/fixtures/references-humanities-note.json');
  assert.equal(options.citationsFixture, 'tests/fixtures/citations-humanities-note.json');
  assert.equal(options.jsonOutput, true);
  assert.equal(options.caseSensitive, true);
});

test('oracle-yaml parses case-sensitivity override flags', () => {
  const insensitive = parseArgs(['styles/embedded/apa-7th.yaml', '--case-insensitive']);
  const sensitive = parseArgs(['styles/embedded/apa-7th.yaml', '--case-sensitive']);

  assert.equal(insensitive.caseSensitive, false);
  assert.equal(sensitive.caseSensitive, true);
});

test('oracle-yaml resolves apa-7th to apa.csl and author-date fixtures', () => {
  const plan = planFor('apa-7th.yaml', { styleFormat: 'author-date', hasBibliography: true });

  assert.equal(path.basename(plan.legacyCslPath), 'apa.csl');
  assert.equal(path.basename(plan.baseRun.refsFixture), 'references-expanded.json');
  assert.equal(path.basename(plan.baseRun.citationsFixture), 'citations-expanded.json');
  assert.deepEqual(plan.familyRuns.map((run) => run.setName), ['author-date', 'secondary-roles']);
});

test('oracle-yaml resolves chicago-notes to note fixtures and citation-only scope', () => {
  const plan = planFor('chicago-notes-18th.yaml', { styleFormat: 'note', hasBibliography: false });

  assert.equal(path.basename(plan.legacyCslPath), 'chicago-notes.csl');
  assert.equal(path.basename(plan.baseRun.citationsFixture), 'citations-note-expanded.json');
  assert.deepEqual(plan.familyRuns.map((run) => run.setName), ['humanities-note', 'secondary-roles']);
  assert.deepEqual(plan.effectiveScopes, ['citation']);
});

test('oracle-yaml resolves chicago-author-date and ieee to their scoped family fixtures', () => {
  const chicagoPlan = planFor('chicago-author-date-18th.yaml', { styleFormat: 'author-date', hasBibliography: true });
  const ieeePlan = planFor('ieee.yaml', { styleFormat: 'numeric', hasBibliography: true });

  assert.equal(path.basename(chicagoPlan.legacyCslPath), 'chicago-author-date.csl');
  assert.deepEqual(chicagoPlan.familyRuns.map((run) => run.setName), ['author-date', 'secondary-roles']);

  assert.equal(path.basename(ieeePlan.legacyCslPath), 'ieee.csl');
  assert.deepEqual(ieeePlan.familyRuns.map((run) => run.setName), ['physics-numeric', 'secondary-roles']);
});

test('oracle-yaml keeps preset-backed wrapper styles mapped to their filename baseline', () => {
  const yamlPath = path.join(__dirname, '..', 'styles', 'embedded', 'chicago-notes-18th.yaml');
  const rawStyleData = yaml.load(fs.readFileSync(yamlPath, 'utf8')) || {};
  const plan = resolveYamlVerificationPlan({
    yamlPath,
    styleData: rawStyleData,
    resolvedStyleData: resolveStyleData(rawStyleData),
    styleFormat: 'note',
    hasBibliography: true,
  });

  assert.equal(path.basename(plan.legacyCslPath), 'chicago-notes.csl');
});

test('resolveStyleData deep-merges preset wrappers with local overrides', () => {
  const yamlPath = path.join(
    __dirname,
    '..',
    'styles',
    'embedded',
    'taylor-and-francis-chicago-author-date.yaml'
  );
  const rawStyleData = yaml.load(fs.readFileSync(yamlPath, 'utf8')) || {};
  const resolved = resolveStyleData(rawStyleData);

  assert.equal(resolved.options['range-format'], 'expanded');
  // Inherits chicago-author-date-18th's processing, now the `author-date-full`
  // string preset (names + add-givenname + primary-name + year-suffix) rather than
  // an inline disambiguate map. The Rust engine expands the preset at render time.
  assert.equal(resolved.options.processing, 'author-date-full');
  assert.equal(resolved.citation.options.contributors.shorten.min, 4);
  assert.equal(
    resolved.bibliography['type-variants']['motion-picture'][4].prefix,
    'Directed by '
  );
});

test('resolveStyleData resolves Template V3 diff type variants', () => {
  const resolved = resolveStyleData({
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
        { variable: 'publisher' },
      ],
      'type-variants': {
        book: {
          modify: [
            { match: { title: 'primary' }, prefix: 'In ' },
          ],
          remove: [
            { match: { variable: 'publisher' } },
          ],
          add: [
            { after: { title: 'primary' }, component: { date: 'issued', form: 'year' } },
          ],
        },
      },
    },
  });

  assert.deepEqual(resolved.bibliography['type-variants'].book, [
    { contributor: 'author' },
    { title: 'primary', prefix: 'In ' },
    { date: 'issued', form: 'year' },
  ]);
});

test('resolveStyleData matches Template V3 grouped selectors recursively', () => {
  const group = [
    { number: 'citation-number', wrap: { punctuation: 'brackets' } },
    { contributor: 'author', form: 'long' },
  ];
  const resolved = resolveStyleData({
    bibliography: {
      template: [
        { delimiter: '', group },
        { title: 'primary' },
      ],
      'type-variants': {
        book: {
          modify: [
            { match: { group }, suffix: '.' },
          ],
        },
      },
    },
  });

  assert.deepEqual(resolved.bibliography['type-variants'].book, [
    { delimiter: '', group, suffix: '.' },
    { title: 'primary' },
  ]);
});

test('resolveStyleData lets child diff variants inherit same-key parent variants', () => {
  const merged = deepMerge(
    {
      bibliography: {
        template: [
          { contributor: 'author' },
          { title: 'primary' },
        ],
        'type-variants': {
          book: [
            { contributor: 'editor' },
            { title: 'primary' },
          ],
        },
      },
    },
    {
      bibliography: {
        'type-variants': {
          book: {
            modify: [
              { match: { title: 'primary' }, suffix: '.' },
            ],
          },
        },
      },
    }
  );

  const resolved = resolveStyleData(merged);

  assert.deepEqual(resolved.bibliography['type-variants'].book, [
    { contributor: 'editor' },
    { title: 'primary', suffix: '.' },
  ]);
});

test('resolveStyleData lets child diff variants inherit same-key parent diffs', () => {
  const merged = deepMerge(
    {
      bibliography: {
        template: [
          { contributor: 'author' },
          { title: 'primary' },
          { variable: 'publisher' },
        ],
        'type-variants': {
          book: {
            remove: [
              { match: { variable: 'publisher' } },
            ],
          },
        },
      },
    },
    {
      bibliography: {
        'type-variants': {
          book: {
            modify: [
              { match: { title: 'primary' }, suffix: '.' },
            ],
          },
        },
      },
    }
  );

  const resolved = resolveStyleData(merged);

  assert.deepEqual(resolved.bibliography['type-variants'].book, [
    { contributor: 'author' },
    { title: 'primary', suffix: '.' },
  ]);
});

test('resolveStyleData does not mutate authored type variant diffs', () => {
  const style = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
      'type-variants': {
        book: {
          modify: [
            { match: { title: 'primary' }, suffix: '.' },
          ],
        },
      },
    },
  };

  resolveStyleData(style);

  assert.deepEqual(style.bibliography['type-variants'].book, {
    modify: [
      { match: { title: 'primary' }, suffix: '.' },
    ],
  });
});

test('oracle-yaml sums component summary counts across structured family runs', () => {
  const main = {
    citations: { total: 1, passed: 1, failed: 0, entries: [] },
    bibliography: { total: 1, passed: 1, failed: 0, entries: [] },
    citationsByType: { book: { total: 1, passed: 1 } },
    componentSummary: { title: 2, author: 1 },
    orderingIssues: 1,
  };
  const extra = {
    citations: { total: 2, passed: 2, failed: 0, entries: [] },
    bibliography: { total: 3, passed: 2, failed: 1, entries: [] },
    citationsByType: { book: { total: 2, passed: 2 }, article: { total: 1, passed: 1 } },
    componentSummary: { title: 3, year: 4 },
    orderingIssues: 2,
  };

  mergeStructuredResults(main, extra);

  assert.deepEqual(main.componentSummary, { title: 5, author: 1, year: 4 });
  assert.deepEqual(main.citationsByType, {
    book: { total: 3, passed: 3 },
    article: { total: 1, passed: 1 },
  });
  assert.equal(main.orderingIssues, 3);
});

test('oracle-yaml disables structured oracle when legacy CSL is explicitly overridden', () => {
  const stylePlan = planFor('apa-7th.yaml', { styleFormat: 'author-date', hasBibliography: true });

  assert.equal(
    shouldUseStructuredOracle(
      { yamlPath: 'styles/embedded/apa-7th.yaml', legacyCslPath: 'styles-legacy/chicago-author-date.csl' },
      stylePlan
    ),
    false
  );
  assert.equal(
    shouldUseStructuredOracle(
      { yamlPath: 'styles/embedded/apa-7th.yaml', legacyCslPath: null },
      stylePlan
    ),
    stylePlan.canUseStructuredOracle
  );
});
