const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  validateVerificationPolicy,
  resolveRegisteredDivergence,
  resolveStyleData,
  loadVerificationPolicy,
  resolveVerificationPolicy,
  resolveScopeAuthority,
} = require('./lib/verification-policy');
const { getEffectiveVerificationScopes } = require('./lib/style-verification');
const { loadReportProvenance } = require('./lib/report-metadata');
const {
  buildNoteStyleLookup,
  collectTemplateScopes,
  computeComponentMatchRate,
  computeConcisionScore,
  computeFallbackRobustness,
  computePresetUsageScore,
  discoverCoreStyles,
  computeFidelityScore,
  buildEmptyOracleResult,
  cloneOracleResult,
  executeBenchmarkRuns,
  equivalentText,
  expandCompoundBibEntries,
  formatAuthorityLabel,
  getEffectiveOracleSection,
  getCslSnapshotStatus,
  getComparisonEntryTexts,
  generateHtml,
  generateReport,
  loadStyleYaml,
  mapWithConcurrency,
  mergeBenchmarkRunIntoOracle,
  mergeDivergenceSummaries,
  mergeOracleResults,
  parseArgs,
  preflightSnapshots,
  resolveSelectedStyles,
  runCachedJsonJob,
  selectPrimaryComparator,
  selectQualityAuthorshipData,
  hasRootExtends,
  toPublishedBenchmarkRunRecord,
  determineBenchmarkStatus,
  summarizeBibliographyPairing,
  summarizeExactParity,
} = require('./report-core');

const projectRoot = path.resolve(__dirname, '..');
const hasLegacyStyles = fs.existsSync(path.join(projectRoot, 'styles-legacy', 'apa.csl'));

function loadStyleMap() {
  return new Map(discoverCoreStyles().map((style) => [style.name, style]));
}

test('discoverCoreStyles classifies representative style origins and CSL reach', () => {
  const styles = loadStyleMap();
  const provenance = loadReportProvenance();

  assert.equal(styles.get('apa-7th').originLabel, provenance.defaults.labels['csl-derived']);
  assert.equal(styles.get('apa-7th').cslReach, 783);
  assert.equal(styles.get('apa-7th').hasBibliography, true);

  assert.equal(styles.get('apa-7th').tier, 'embedded');
  assert.equal(styles.get('american-chemical-society').tier, 'exemplar');

  const unknownOrigins = [...styles.values()].filter((style) => style.originLabel === 'Unknown');
  assert.deepEqual(unknownOrigins, []);
});

test('discoverCoreStyles keeps wrapper style baseline identity while resolving preset behavior', () => {
  const styles = loadStyleMap();
  const chicagoNotes = styles.get('chicago-notes-18th');

  assert.equal(chicagoNotes.sourceName, 'chicago-notes');
  assert.equal(chicagoNotes.format, 'note');
  assert.equal(chicagoNotes.hasBibliography, false);
});

test('discoverCoreStyles skips hidden embedded core wrappers', () => {
  const styles = discoverCoreStyles();

  assert.equal(styles.some((style) => style.name.endsWith('-core')), false);
  assert.equal(styles.some((style) => style.name === 'chicago-18-base'), false);
});

test('discoverCoreStyles exposes complete family and registry metadata', () => {
  const styles = loadStyleMap();
  const chicago = styles.get('chicago-author-date-18th');

  assert.deepEqual(chicago.inheritance.chain, [
    'chicago-author-date-18th',
    'chicago-18-base',
  ]);
  assert.equal(chicago.inheritance.familyRoot, 'chicago-18-base');
  assert.equal(chicago.inheritance.implementationForm, 'structural-wrapper');
  assert.equal(chicago.registry.kind, 'base');
  assert.equal(chicago.registry.aliases.includes('chicago-author-date'), true);
});

test('summarizeExactParity remains pre-divergence', () => {
  const summary = summarizeExactParity({
    citations: {
      total: 1,
      entries: [{ exactMatch: false }],
    },
    bibliography: {
      total: 1,
      entries: [
        { exactMatch: true },
        { exactMatch: null, exactParityEligible: false },
      ],
    },
    adjusted: {
      citations: {
        total: 1,
        passed: 1,
        entries: [{ exactMatch: true, match: true }],
      },
    },
  });

  assert.deepEqual(summary, {
    passed: 1,
    total: 2,
    notComparable: 1,
    rate: 0.5,
    status: 'unadjudicated',
    gating: false,
  });
});

test('bibliography pairing summaries distinguish paired, unresolved, and ID-proven observations', () => {
  const summary = summarizeBibliographyPairing({
    bibliography: {
      entries: [
        {
          expected: 'Benchmark text',
          actual: 'Citum text',
          pairingMethod: 'position',
          comparisonState: 'paired',
        },
        {
          expected: 'Unresolved benchmark candidate',
          actual: null,
          pairingMethod: 'position',
          comparisonState: 'unresolved-unpaired',
        },
        {
          id: 'oracle-only',
          oracle: 'ID-proven benchmark output',
          citum: null,
          pairingMethod: 'id',
          comparisonState: 'oracle-only',
        },
        {
          id: 'citum-only',
          oracle: null,
          citum: 'ID-proven Citum output',
          pairingMethod: 'id',
          comparisonState: 'citum-only',
        },
      ],
    },
  });

  assert.deepEqual(summary, {
    paired: 1,
    unresolvedUnpaired: 1,
    idProvenOracleOnly: 1,
    idProvenCitumOnly: 1,
    totalObservations: 4,
  });
});

test('resolveSelectedStyles filters to requested style names and rejects unknown styles', () => {
  const coreStyles = discoverCoreStyles();

  const selected = resolveSelectedStyles(coreStyles, ['chicago-author-date-18th', 'apa-7th']);
  assert.deepEqual(selected.map((style) => style.name), ['apa-7th', 'chicago-author-date-18th']);

  assert.throws(
    () => resolveSelectedStyles(coreStyles, ['not-a-style']),
    /Unknown style name\(s\) for --styles: not-a-style/
  );
});

test('parseArgs accepts either --style or --styles and rejects invalid selector usage', () => {
  const originalArgv = process.argv;

  try {
    process.argv = ['node', 'scripts/report-core.js', '--style', 'chicago-author-date-18th'];
    assert.equal(parseArgs().styleName, 'chicago-author-date-18th');

    process.argv = ['node', 'scripts/report-core.js', '--styles', 'chicago-author-date-18th, apa-7th'];
    assert.deepEqual(parseArgs().styles, ['chicago-author-date-18th', 'apa-7th']);

    process.argv = ['node', 'scripts/report-core.js', '--style'];
    assert.throws(() => parseArgs(), /Missing value for --style/);

    process.argv = ['node', 'scripts/report-core.js', '--styles'];
    assert.throws(() => parseArgs(), /Missing value for --styles/);

    process.argv = ['node', 'scripts/report-core.js', '--styles', '   '];
    assert.throws(() => parseArgs(), /Missing value for --styles/);

    process.argv = ['node', 'scripts/report-core.js', '--style', 'chicago-author-date-18th', '--styles', 'apa-7th'];
    assert.throws(() => parseArgs(), /Flags --style and --styles are mutually exclusive/);
  } finally {
    process.argv = originalArgv;
  }
});

test('buildNoteStyleLookup indexes shipped note styles', () => {
  const noteStyles = buildNoteStyleLookup();

  assert.equal(noteStyles.has('chicago-notes-18th'), true);
  assert.equal(noteStyles.get('chicago-notes-18th').style.options.processing, 'note');
  assert.equal(Boolean(noteStyles.get('chicago-notes-18th').style.bibliography), true);
  assert.equal(noteStyles.has('apa-7th'), false);
});

test('collectTemplateScopes includes type-variants and type-templates', () => {
  const { scopes, variantSelectorCount } = collectTemplateScopes({
    citation: {
      template: [{ contributor: 'author' }],
      integral: {
        'type-variants': {
          book: [{ contributor: 'author' }],
        },
      },
      'non-integral': {
        'type-variants': {
          'book, chapter': [{ title: 'primary' }],
        },
      },
    },
    bibliography: {
      template: [{ title: 'primary' }],
      'type-variants': {
        article: [{ variable: 'publisher' }],
      },
      'type-templates': {
        dataset: [{ variable: 'url' }],
      },
    },
  });

  assert.equal(scopes.some((scope) => scope.name === 'citation.template'), true);
  assert.equal(scopes.some((scope) => scope.name === 'citation.integral.type-variants.book'), true);
  assert.equal(
    scopes.some((scope) => scope.name === 'citation.non-integral.type-variants.book, chapter'),
    true
  );
  assert.equal(scopes.some((scope) => scope.name === 'bibliography.type-variants.article'), true);
  assert.equal(scopes.some((scope) => scope.name === 'bibliography.type-templates.dataset'), true);
  assert.equal(variantSelectorCount, 4);
});

test('collectTemplateScopes includes resolved Template V3 diff variants', () => {
  const { scopes, variantSelectorCount } = collectTemplateScopes(
    resolveStyleData({
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
    })
  );

  assert.equal(scopes.some((scope) => scope.name === 'bibliography.type-variants.book'), true);
  assert.equal(variantSelectorCount, 1);
});

test('collectTemplateScopes scores authored Template V3 diff components', () => {
  const { scopes, variantSelectorCount } = collectTemplateScopes({
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
          remove: [
            { match: { contributor: 'author' } },
          ],
          add: [
            { after: { title: 'primary' }, component: { date: 'issued', form: 'year' } },
          ],
        },
      },
    },
  });

  const scope = scopes.find((candidate) => candidate.name === 'bibliography.type-variants.book');

  assert.equal(Boolean(scope), true);
  assert.deepEqual(scope.components, [
    { title: 'primary', suffix: '.', __isDiff: true },
    { contributor: 'author', suppress: true, __isDiff: true },
    { date: 'issued', form: 'year', __isDiff: true },
  ]);
  assert.equal(variantSelectorCount, 1);
});

test('computeConcisionScore penalizes duplicate-heavy type-variant structures', () => {
  const duplicatedStyle = {
    citation: {
      'non-integral': {
        'type-variants': {
          article: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
          book: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
          chapter: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
          report: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
          thesis: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
          webpage: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
        },
      },
    },
    bibliography: {
      template: [
        { contributor: 'author', form: 'long' },
        { date: 'issued', form: 'year' },
        { title: 'primary' },
      ],
      'type-variants': {
        article: [{ contributor: 'author', form: 'long' }, { title: 'primary' }],
        book: [{ contributor: 'author', form: 'long' }, { title: 'primary' }],
        chapter: [{ contributor: 'author', form: 'long' }, { title: 'primary' }],
      },
    },
  };

  const score = computeConcisionScore(duplicatedStyle, 'author-date');

  assert.equal(score.variantSelectors, 9);
  assert.ok(score.exactDuplicateScopes >= 6);
  assert.ok(score.score < 70, `expected concision below 70, got ${score.score}`);
});

test('computeConcisionScore rewards preset-backed compact structures', () => {
  const compactStyle = {
    citation: {
      'extends': 'apa',
      template: [{ contributor: 'author' }, { date: 'issued', form: 'year' }],
    },
    bibliography: {
      'extends': 'apa',
      template: [
        { contributor: 'author', form: 'long' },
        { date: 'issued', form: 'year' },
        { title: 'primary' },
        { variable: 'doi' },
      ],
    },
  };

  const score = computeConcisionScore(compactStyle, 'author-date');

  assert.equal(score.variantSelectors, 0);
  assert.equal(score.exactDuplicateScopes, 0);
  assert.ok(score.score >= 90, `expected concision >= 90, got ${score.score}`);
});

test('selectQualityAuthorshipData keeps root wrapper authorship for SQI', () => {
  const authored = {
    extends: 'apa-7th',
  };
  const resolved = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
    },
  };

  assert.equal(selectQualityAuthorshipData(authored, resolved), authored);
  assert.equal(hasRootExtends(authored), true);
});

test('selectQualityAuthorshipData still falls back for template-free non-wrappers', () => {
  const authored = {
    info: {
      title: 'Template-free draft',
    },
  };
  const resolved = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
    },
  };

  assert.equal(selectQualityAuthorshipData(authored, resolved), resolved);
  assert.equal(hasRootExtends(authored), false);
});

test('computeConcisionScore treats pure root wrappers as inherited presets', () => {
  const score = computeConcisionScore({ extends: 'elsevier-with-titles-core' }, 'numeric');

  assert.equal(score.score, 100);
  assert.equal(score.totalComponents, 0);
  assert.equal(score.inheritedPreset, 'elsevier-with-titles-core');
  assert.match(score.note, /root extends/);
});

test('computePresetUsageScore treats pure root wrappers as strong preset reuse', () => {
  const score = computePresetUsageScore({ extends: 'springer-vancouver-brackets-core' }, 100);

  assert.equal(score.score, 100);
  assert.equal(score.inheritedPreset, 'springer-vancouver-brackets-core');
  assert.match(score.note, /root extends/);
});

test('selectQualityAuthorshipData keeps authored Template V3 diff scopes', () => {
  const authored = {
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
  const resolved = resolveStyleData(authored);

  assert.equal(selectQualityAuthorshipData(authored, resolved), authored);
});

test('computeConcisionScore does not penalize surgical diff variants as cross-scope repeats', () => {
  const baseStyle = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { date: 'issued', form: 'year' },
        { title: 'primary' },
      ],
    },
  };
  const diffStyle = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { date: 'issued', form: 'year' },
        { title: 'primary' },
      ],
      'type-variants': {
        book: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        chapter: { remove: [{ match: { contributor: 'author' } }] },
        article: { add: [{ component: { variable: 'doi' } }] },
      },
    },
  };

  const baseScore = computeConcisionScore(baseStyle, 'author-date');
  const diffScore = computeConcisionScore(diffStyle, 'author-date');

  assert.equal(diffScore.crossScopeRepeats, baseScore.crossScopeRepeats,
    'modify ops referencing a base component must not register as cross-scope repeats');
  assert.equal(diffScore.totalComponents, baseScore.totalComponents,
    'diff scopes must not inflate the component count budget');
  assert.ok(diffScore.score >= baseScore.score - 0.5,
    `surgical diff variants should not reduce concision; base=${baseScore.score} diff=${diffScore.score}`);
});

test('computeConcisionScore reports but does not penalize parallel diff variants', () => {
  const styleData = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
      'type-variants': {
        book: { add: [{ component: { variable: 'doi' } }] },
        chapter: { add: [{ component: { variable: 'doi' } }] },
        article: { add: [{ component: { variable: 'doi' } }] },
        report: { add: [{ component: { variable: 'doi' } }] },
        thesis: { add: [{ component: { variable: 'doi' } }] },
      },
    },
  };

  const score = computeConcisionScore(styleData, 'author-date');

  assert.equal(score.diffVariantScopes, 5);
  assert.equal(score.diffVariantOperations, 5);
  assert.equal(score.exactDuplicateScopes, 0);
  assert.ok(score.score >= 95,
    `parallel diff variants should not be duplicate-penalized, got ${score.score}`);
});

test('computeFallbackRobustness treats type-variants as explicit type coverage', () => {
  const styleData = {
    bibliography: {
      template: [
        { contributor: 'author' },
        { title: 'primary' },
      ],
      'type-variants': {
        'article-journal': { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        book: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        chapter: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        report: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        thesis: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        'paper-conference': { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
        webpage: { modify: [{ match: { title: 'primary' }, suffix: '.' }] },
      },
    },
  };

  const result = computeFallbackRobustness(styleData);

  assert.equal(result.score, 100);
  assert.equal(result.assessedTypes, 0);
  assert.match(result.note, /type-variants/);
});

test('computeFallbackRobustness honors extends short-circuit', () => {
  const styleData = {
    bibliography: {
      'extends': 'apa',
      template: [{ contributor: 'author' }],
    },
  };

  const result = computeFallbackRobustness(styleData);

  assert.equal(result.score, 100);
  assert.match(result.note, /embedded bibliography preset/);
});

test('apa-7th concision regression reflects preset-first success', () => {
  const style = loadStyleMap().get('apa-7th');
  const loaded = loadStyleYaml(style.name);
  const concision = computeConcisionScore(loaded.resolvedStyleData, style.format);

  assert.equal(concision.variantSelectors, 26, 'resolved APA should reflect the embedded authored variant selectors');
  // APA's disambiguation now uses the `author-date-full` preset instead of an inline
  // custom block, and chapter container introductions use a localized message
  // phrase instead of a separate term-plus-title group, so concision improves.
  // The dataset variant's small concision dip (64 -> 63.4) is an intentional
  // tradeoff: the old " [Dataset]." title suffix was a hardcoded English
  // literal that dropped the version field entirely for titled datasets; the
  // type-label + term:version group correctly localizes and renders both, at
  // the cost of a few extra template components. See
  // docs/specs/TYPE_CLASSIFICATION_CENTRALIZATION.md.
  assert.equal(concision.score, 63.4, `expected embedded APA concision, got ${concision.score}`);
});

test('report-core exposes expected benchmark labels for representative styles', () => {
  const styles = loadStyleMap();
  const policy = loadVerificationPolicy();

  const cases = [
    ['apa-7th', 'citeproc-js', null],
    ['chem-acs', 'biblatex: chem-acs', 'chem-acs'],
    ['numeric-comp', 'biblatex: numeric-comp', 'numeric-comp'],
  ];

  for (const [styleName, expectedLabel, authorityId] of cases) {
    const style = styles.get(styleName);
    const stylePolicy = resolveVerificationPolicy(styleName, policy);
    const comparator = selectPrimaryComparator(style, stylePolicy);
    assert.equal(formatAuthorityLabel(comparator, authorityId || stylePolicy.authorityId), expectedLabel);
  }
});

test('verification policy exposes scope-specific authority for chemistry benchmark rows', () => {
  const policy = loadVerificationPolicy();
  const stylePolicy = resolveVerificationPolicy('chem-acs', policy);

  assert.equal(stylePolicy.authority, 'biblatex');
  assert.equal(stylePolicy.authorityId, 'chem-acs');
  assert.equal(stylePolicy.regressionBaseline, 'citum-baseline');

  const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');
  const bibliographyAuthority = resolveScopeAuthority(stylePolicy, 'bibliography');

  assert.deepEqual(citationAuthority, {
    authority: 'citeproc-js',
    authorityId: null,
    note: stylePolicy.note,
  });
  assert.deepEqual(bibliographyAuthority, {
    authority: 'biblatex',
    authorityId: 'chem-acs',
    note: stylePolicy.note,
  });
});

test('verification policy exposes registered divergence metadata for div-004', () => {
  const policy = loadVerificationPolicy();
  const divergence = resolveRegisteredDivergence(policy, 'div-004');

  assert.deepEqual(divergence.scopes, ['citation', 'bibliography']);
  assert.deepEqual(divergence.tags, [
    'missing-name-title-sort-order',
    'sort-derived-numeric-citation-label',
  ]);
  assert.match(divergence.note, /Missing-name works sort by title/);
});

test('verification policy exposes registered divergence metadata for div-005', () => {
  const policy = loadVerificationPolicy();
  const divergence = resolveRegisteredDivergence(policy, 'div-005');

  assert.deepEqual(divergence.scopes, ['citation']);
  assert.deepEqual(divergence.tags, [
    'citeproc-legacy-archive-gap',
    'structured-archival-manuscript-detail',
  ]);
  assert.match(divergence.note, /structured archival manuscript metadata/);
});

test('citation-only note styles do not advertise bibliography verification scopes', () => {
  const styles = loadStyleMap();
  const policy = loadVerificationPolicy();
  const chicagoNotesClassic = styles.get('chicago-notes-18th');
  const stylePolicy = resolveVerificationPolicy('chicago-notes-18th', policy);

  assert.equal(chicagoNotesClassic.hasBibliography, false);
  assert.deepEqual(getEffectiveVerificationScopes(stylePolicy, chicagoNotesClassic.hasBibliography), ['citation']);
});

test('verification policy validates and resolves ordered benchmark runs', () => {
  const policy = validateVerificationPolicy({
    version: 1,
    defaults: {
      authority: 'citeproc-js',
      secondary: [],
      scopes: ['citation', 'bibliography'],
    },
    styles: {
      'chicago-author-date-18th': {
        benchmark_runs: [
          {
            id: 'rich-bib',
            label: 'Rich bibliography',
            runner: 'citeproc-oracle',
            refs_fixture: 'tests/fixtures/test-items-library/chicago-18th.json',
            scope: 'bibliography',
            count_toward_fidelity: true,
          },
          {
            id: 'native-smoke',
            label: 'Native smoke',
            runner: 'native-smoke',
            refs_fixture: 'examples/comprehensive.yaml',
            scope: 'bibliography',
            count_toward_fidelity: false,
          },
        ],
      },
    },
  });

  const stylePolicy = resolveVerificationPolicy('chicago-author-date-18th', policy);
  assert.deepEqual(
    stylePolicy.benchmarkRuns.map((run) => run.id),
    ['rich-bib', 'native-smoke']
  );
  assert.equal(stylePolicy.benchmarkRuns[0].countTowardFidelity, true);
  assert.equal(stylePolicy.benchmarkRuns[1].runner, 'native-smoke');
});

test('repo verification policy exposes APA supplemental benchmark runs', () => {
  const policy = loadVerificationPolicy();
  const stylePolicy = resolveVerificationPolicy('apa-7th', policy);

  assert.deepEqual(
    stylePolicy.benchmarkRuns.map((run) => run.id),
    ['apa-zotero-bibliography', 'apa-test-library-diagnostic']
  );
  assert.equal(stylePolicy.benchmarkRuns[0].countTowardFidelity, false);
  assert.equal(stylePolicy.benchmarkRuns[1].countTowardFidelity, false);
});

test('verification policy rejects unsupported benchmark run combinations', () => {
  assert.throws(
    () => validateVerificationPolicy({
      version: 1,
      defaults: {
        authority: 'citeproc-js',
        secondary: [],
        scopes: ['citation', 'bibliography'],
      },
      styles: {
        sample: {
          benchmark_runs: [{
            id: 'bad-native-scope',
            label: 'Bad native scope',
            runner: 'native-smoke',
            refs_fixture: 'examples/comprehensive.yaml',
            scope: 'both',
            count_toward_fidelity: false,
            citations_fixture: 'tests/fixtures/citations-expanded.json',
          }],
        },
      },
    }),
    /scope must be bibliography for native-smoke/
  );

  assert.throws(
    () => validateVerificationPolicy({
      version: 1,
      defaults: {
        authority: 'citeproc-js',
        secondary: [],
        scopes: ['citation', 'bibliography'],
      },
      styles: {
        sample: {
          benchmark_runs: [{
            id: 'bad-native-count',
            label: 'Bad native count',
            runner: 'native-smoke',
            refs_fixture: 'examples/comprehensive.yaml',
            scope: 'bibliography',
            count_toward_fidelity: true,
          }],
        },
      },
    }),
    /count_toward_fidelity must be false for native-smoke/
  );

  // A citation-scope citeproc-oracle run still requires a citations_fixture.
  assert.throws(
    () => validateVerificationPolicy({
      version: 1,
      defaults: {
        authority: 'citeproc-js',
        secondary: [],
        scopes: ['citation', 'bibliography'],
      },
      styles: {
        sample: {
          benchmark_runs: [{
            id: 'citation-missing-fixture',
            label: 'Citation missing fixture',
            runner: 'citeproc-oracle',
            refs_fixture: 'tests/fixtures/references-expanded.json',
            scope: 'citation',
            count_toward_fidelity: false,
          }],
        },
      },
    }),
    /citations_fixture is required unless scope is bibliography/
  );
});

test('verification policy accepts citation-scope citeproc-oracle runs', () => {
  assert.doesNotThrow(() => validateVerificationPolicy({
    version: 1,
    defaults: {
      authority: 'citeproc-js',
      secondary: [],
      scopes: ['citation', 'bibliography'],
    },
    styles: {
      sample: {
        benchmark_runs: [{
          id: 'citation-only',
          label: 'Citation only',
          runner: 'citeproc-oracle',
          refs_fixture: 'tests/fixtures/references-expanded.json',
          citations_fixture: 'tests/fixtures/citations-expanded.json',
          scope: 'citation',
          count_toward_fidelity: false,
        }],
      },
    },
  }));
});

test('executeBenchmarkRuns preserves declaration order', async () => {
  const seen = [];
  const benchmarkRuns = [{ id: 'first' }, { id: 'second' }];

  const results = await executeBenchmarkRuns(benchmarkRuns, async (benchmarkRun) => {
    seen.push(benchmarkRun.id);
    await new Promise((resolve) => setTimeout(resolve, benchmarkRun.id === 'first' ? 5 : 0));
    return benchmarkRun.id;
  });

  assert.deepEqual(seen, ['first', 'second']);
  assert.deepEqual(results, ['first', 'second']);
});

test('mergeBenchmarkRunIntoOracle adds bibliography-only scoring totals without changing citations', () => {
  const base = cloneOracleResult(buildEmptyOracleResult({
    citations: { passed: 2, total: 2, entries: [{ id: 'c1', match: true }] },
    bibliography: { passed: 3, total: 4, entries: [{ index: 1, match: true }] },
    adjusted: {
      citations: { passed: 2, total: 2, entries: [{ id: 'c1', match: true }] },
      bibliography: { passed: 3, total: 4, entries: [{ index: 1, match: true }] },
      divergenceSummary: {},
    },
  }));
  const benchmarkOracle = buildEmptyOracleResult({
    bibliography: { passed: 5, total: 6, entries: [{ index: 2, match: false }] },
    adjusted: {
      citations: { passed: 0, total: 0, entries: [] },
      bibliography: { passed: 5, total: 6, entries: [{ index: 2, match: false }] },
      divergenceSummary: {},
    },
  });

  mergeBenchmarkRunIntoOracle(base, {
    countTowardFidelity: true,
    scope: 'bibliography',
    oracleResult: benchmarkOracle,
  });

  assert.deepEqual(base.citations.passed, 2);
  assert.deepEqual(base.citations.total, 2);
  assert.deepEqual(base.bibliography.passed, 8);
  assert.deepEqual(base.bibliography.total, 10);
});

test('mergeOracleResults combines bibliography-only oracle sections', () => {
  const main = buildEmptyOracleResult({
    bibliography: { passed: 1, total: 2, entries: [{ index: 1, match: true }] },
    adjusted: {
      citations: { passed: 0, total: 0, entries: [] },
      bibliography: { passed: 1, total: 2, entries: [{ index: 1, match: true }] },
      divergenceSummary: {},
    },
  });
  const extra = buildEmptyOracleResult({
    bibliography: { passed: 2, total: 3, entries: [{ index: 2, match: false }] },
    adjusted: {
      citations: { passed: 0, total: 0, entries: [] },
      bibliography: { passed: 2, total: 3, entries: [{ index: 2, match: false }] },
      divergenceSummary: {},
    },
  });

  mergeOracleResults(main, extra);
  assert.deepEqual(main.bibliography.passed, 3);
  assert.deepEqual(main.bibliography.total, 5);
  assert.deepEqual(main.citations.total, 0);
});

test('published benchmark run records are compact and repo-relative', () => {
  const published = toPublishedBenchmarkRunRecord({
    id: 'chicago-zotero-bibliography',
    label: 'Chicago Zotero bibliography',
    runner: 'citeproc-oracle',
    scope: 'bibliography',
    countTowardFidelity: true,
    refsFixture: path.join(projectRoot, 'tests', 'fixtures', 'test-items-library', 'chicago-18th.json'),
    citationsFixture: null,
    status: 'pass',
    error: null,
    citations: { passed: 0, total: 0, entries: [{ id: 'c1', match: true }] },
    bibliography: { passed: 12, total: 18, entries: [{ index: 1, match: false }] },
    bibliographyEntries: null,
    oracleResult: { bibliography: { entries: ['too-much-detail'] } },
  });

  assert.deepEqual(published, {
    id: 'chicago-zotero-bibliography',
    label: 'Chicago Zotero bibliography',
    runner: 'citeproc-oracle',
    scope: 'bibliography',
    countTowardFidelity: true,
    refsFixture: 'tests/fixtures/test-items-library/chicago-18th.json',
    citationsFixture: null,
    status: 'pass',
    error: null,
    citations: { passed: 0, total: 0 },
    bibliography: { passed: 12, total: 18 },
    bibliographyEntries: null,
  });
  assert.equal(Object.hasOwn(published, 'oracleResult'), false);
  assert.equal(Object.hasOwn(published, 'minPassRate'), false);
});

test('min_pass_rate resolves to minPassRate in resolved policy', () => {
  const policy = validateVerificationPolicy({
    version: 1,
    defaults: { authority: 'citeproc-js', secondary: [], scopes: ['citation', 'bibliography'] },
    styles: {
      'chicago-author-date-18th': {
        benchmark_runs: [{
          id: 'zotero-bib',
          label: 'Zotero bibliography',
          runner: 'citeproc-oracle',
          refs_fixture: 'tests/fixtures/test-items-library/chicago-18th.json',
          scope: 'bibliography',
          count_toward_fidelity: true,
          min_pass_rate: 0.73,
        }],
      },
    },
  });
  const stylePolicy = resolveVerificationPolicy('chicago-author-date-18th', policy);
  assert.equal(stylePolicy.benchmarkRuns[0].minPassRate, 0.73);
});

test('min_pass_rate validation rejects out-of-range values', () => {
  const base = {
    version: 1,
    defaults: { authority: 'citeproc-js', secondary: [], scopes: ['citation', 'bibliography'] },
    styles: {
      sample: {
        benchmark_runs: [{
          id: 'r',
          label: 'R',
          runner: 'citeproc-oracle',
          refs_fixture: 'tests/fixtures/test-items-library/chicago-18th.json',
          scope: 'bibliography',
          count_toward_fidelity: false,
        }],
      },
    },
  };
  assert.throws(
    () => validateVerificationPolicy({
      ...base,
      styles: { sample: { benchmark_runs: [{ ...base.styles.sample.benchmark_runs[0], min_pass_rate: 1.5 }] } },
    }),
    /min_pass_rate must be a number between 0 and 1/
  );
  assert.throws(
    () => validateVerificationPolicy({
      ...base,
      styles: { sample: { benchmark_runs: [{ ...base.styles.sample.benchmark_runs[0], min_pass_rate: -0.1 }] } },
    }),
    /min_pass_rate must be a number between 0 and 1/
  );
});

test('determineBenchmarkStatus returns pass/fail/ok/error based on threshold and result', () => {
  const passing = { bibliography: { passed: 8, total: 10 }, citations: { passed: 0, total: 0 } };
  const failing = { bibliography: { passed: 5, total: 10 }, citations: { passed: 0, total: 0 } };
  const errored = { error: 'citeproc-js crashed', bibliography: null, citations: null };

  assert.equal(determineBenchmarkStatus(errored, 0.73), 'error');
  assert.equal(determineBenchmarkStatus(passing, 0.73), 'pass');
  assert.equal(determineBenchmarkStatus(failing, 0.73), 'fail');
  assert.equal(determineBenchmarkStatus(passing, null), 'ok');
  assert.equal(determineBenchmarkStatus(failing, null), 'ok');
});

test('determineBenchmarkStatus judges conformance against adjusted counts when present, not raw', () => {
  // A raw mismatch masked by a registered divergence (e.g. div-010) must count
  // toward the gate — otherwise every registered divergence is purely
  // decorative and never actually keeps a style's declared min_pass_rate green.
  const maskedByDivergence = {
    bibliography: { passed: 5, total: 10 },
    citations: { passed: 0, total: 0 },
    adjusted: {
      bibliography: { passed: 10, total: 10 },
      citations: { passed: 0, total: 0 },
    },
  };

  assert.equal(determineBenchmarkStatus(maskedByDivergence, 1.0), 'pass');
});

test('comparison text helper supports both live-oracle and native-snapshot entry shapes', () => {
  assert.deepEqual(
    getComparisonEntryTexts({ oracle: 'benchmark text', citum: 'citum text' }),
    { benchmark: 'benchmark text', citum: 'citum text' }
  );
  assert.deepEqual(
    getComparisonEntryTexts({ expected: 'snapshot benchmark', actual: 'snapshot citum' }),
    { benchmark: 'snapshot benchmark', citum: 'snapshot citum' }
  );
});

test('equivalentText treats case-only differences as failures by default', () => {
  assert.equal(equivalentText('DNA repair', 'Dna repair'), false);
  assert.equal(equivalentText('DNA repair', 'Dna repair', { caseSensitive: false }), true);
});

test('generateHtml returns JSON string if template is missing', () => {
  const html = generateHtml({
    generated: '2026-03-11T00:00:00.000Z',
    commit: 'deadbee',
    metadata: {},
    totalImpact: 0,
    totalStyles: 1,
    citationsOverall: { passed: 1, total: 1 },
    bibliographyOverall: { passed: 0, total: 0 },
    qualityOverall: { score: 1 },
    styles: [
      {
        name: 'chicago-notes-18th',
        sourceName: 'chicago-notes',
        format: 'note',
        hasBibliography: false,
        originLabel: 'Test',
        authorityLabel: 'citeproc-js',
        fidelityScore: 1,
        citations: { passed: 1, total: 1 },
        bibliography: { passed: 0, total: 0 },
        qualityScore: 1,
        qualityBreakdown: {
          subscores: {
            typeCoverage: { score: 100 },
            fallbackRobustness: { score: 100 },
            concision: { score: 100 },
            presetUsage: { score: 100 },
          },
        },
        notePositionAudit: {
          regression: {
            status: 'pass',
            profile: 'ibid-and-subsequent',
            issues: [],
          },
          conformance: {
            status: 'pass',
            family: 'chicago-full-note',
            issues: [],
            unresolved: ['prose-integral'],
          },
        },
      },
    ],
  });

  assert.match(html, /"chicago-notes-18th"/);
  assert.match(html, /chicago-full-note/);
});

test('generateHtml groups families and exposes unadjudicated oracle text drift', () => {
  const longSharedPrefix = 'A'.repeat(110);
  const html = generateHtml({
    generated: '2026-07-28T00:00:00.000Z',
    commit: 'deadbee',
    metadata: {},
    totalImpact: 0.1,
    totalStyles: 1,
    citationsOverall: { passed: 0, total: 0 },
    bibliographyOverall: { passed: 1, total: 2, unresolvedPairing: 1 },
    exactParityOverall: { passed: 0, total: 1, notComparable: 2, rate: 0 },
    pairingOverall: {
      paired: 1,
      unresolvedUnpaired: 1,
      idProvenOracleOnly: 1,
      idProvenCitumOnly: 0,
      totalObservations: 3,
    },
    qualityOverall: { score: 1 },
    families: [{
      root: 'chicago-18-base',
      aggregateCslReach: 8,
      members: ['chicago-author-date-18th'],
      memberCount: 1,
      aliases: ['chicago-author-date'],
      aliasCount: 1,
    }],
    styles: [{
      name: 'chicago-author-date-18th',
      format: 'author-date',
      hasBibliography: true,
      cslReach: 8,
      originLabel: 'CSL-derived',
      benchmarkLabel: 'citeproc-js',
      bibliographyAuthorityLabel: 'citeproc-js',
      fidelityScore: 1,
      exactParity: { passed: 0, total: 1, notComparable: 2, rate: 0 },
      pairingSummary: {
        paired: 1,
        unresolvedUnpaired: 1,
        idProvenOracleOnly: 1,
        idProvenCitumOnly: 0,
        totalObservations: 3,
      },
      citations: { passed: 0, total: 0 },
      bibliography: { passed: 1, total: 1 },
      qualityScore: 1,
      qualityBreakdown: {
        score: 100,
        subscores: {
          typeCoverage: { score: 100 },
          fallbackRobustness: { score: 100 },
          concision: { score: 100 },
          presetUsage: { score: 100 },
        },
      },
      inheritance: {
        chain: ['chicago-author-date-18th', 'chicago-18-base'],
        familyRoot: 'chicago-18-base',
        implementationForm: 'structural-wrapper',
      },
      registry: {
        kind: 'base',
        aliases: ['chicago-author-date'],
        aliasCount: 1,
      },
      measurementEvidence: {
        behavioralBand: { band: 'near-clone', target: 'chicago-author-date' },
        derivability: { verdict: 'not-delta-expressible', target: 'chicago-author-date' },
      },
      benchmarkRunResults: [{
        id: 'native-smoke',
        label: 'Native Chicago bibliography render smoke test',
        runner: 'native-smoke',
        scope: 'bibliography',
        countTowardFidelity: false,
        refsFixture: 'examples/chicago-bib.yaml',
        status: 'pass',
        bibliographyEntries: 6,
      }],
      citationEntries: [{
        id: 'late-difference',
        rawOracle: `${longSharedPrefix}. Benchmark ending.`,
        rawCitum: `${longSharedPrefix}, Citum ending.`,
        exactOracle: `${longSharedPrefix}. Benchmark ending.`,
        exactCitum: `${longSharedPrefix}, Citum ending.`,
        exactMatch: false,
        exactAdjudication: 'unresolved',
        match: true,
      }],
      oracleDetail: [{
        index: 193,
        rawOracle: '<div>Smith, John. [1750?]. <i>Title of First Work</i>.</div>',
        rawCitum: 'Smith, John. 1750? _Title of First Work_.',
        exactOracle: 'Smith, John. [1750?]. Title of First Work.',
        exactCitum: 'Smith, John. 1750? Title of First Work.',
        exactMatch: false,
        match: true,
        exactParityEligible: true,
        pairingMethod: 'similarity',
        comparisonState: 'paired',
        evidenceRunId: 'benchmark:chicago-shared-corpus',
        evidenceRunLabel: 'Chicago shared corpus',
        evidenceAuthority: 'citeproc-js',
      }, {
        index: 194,
        rawOracle: null,
        rawCitum: 'Stephanos C. 2017.',
        exactOracle: '',
        exactCitum: 'Stephanos C. 2017.',
        exactMatch: null,
        exactAdjudication: 'not-comparable',
        match: null,
        exactParityEligible: false,
        compatibilityEligible: false,
        pairingMethod: 'similarity',
        comparisonState: 'unresolved-unpaired',
        evidenceRunId: 'benchmark:chicago-shared-corpus',
        evidenceRunLabel: 'Chicago shared corpus',
        evidenceAuthority: 'citeproc-js',
        issues: [{ issue: 'unpaired_output' }],
      }, {
        id: 'ITEM-MISSING',
        index: 195,
        rawOracle: '<div>Oracle-only identified entry.</div>',
        rawCitum: null,
        exactOracle: 'Oracle-only identified entry.',
        exactCitum: '',
        exactMatch: null,
        exactAdjudication: 'not-comparable',
        match: false,
        exactParityEligible: false,
        compatibilityEligible: true,
        pairingMethod: 'id',
        comparisonState: 'oracle-only',
        evidenceRunId: 'benchmark:chicago-shared-corpus',
        evidenceRunLabel: 'Chicago shared corpus',
        evidenceAuthority: 'citeproc-js',
        issues: [{ issue: 'missing_entry' }],
      }],
    }],
  });

  assert.match(html, /aggregate CSL reach 8/);
  assert.match(html, /structural-wrapper/);
  assert.match(html, /near-clone/);
  assert.match(html, /not-delta-expressible/);
  assert.match(html, /Oracle Text Parity/);
  assert.match(html, /Unresolved Oracle Drift/);
  assert.match(html, /<mark[^>]*>\. Benchmark<\/mark> ending\./);
  assert.match(html, /<mark[^>]*>, Citum<\/mark> ending\./);
  assert.match(html, /Smith, John\. <mark[^>]*>\[1750\?\]\.<\/mark>/);
  assert.match(html, /Chicago shared corpus/);
  assert.match(html, /1 paired · 1 unresolved · 1 ID-proven one-sided/);
  assert.match(html, /Unpaired outputs—pairing unresolved \(1\)/);
  assert.match(html, /These candidates are excluded from compatibility and oracle-text parity/);
  assert.match(html, /ID-proven output cardinality failures \(1\)/);
  assert.match(html, /Oracle-only identified entry/);
  assert.match(html, /N\/A/);
  assert.match(html, /Native Chicago bibliography render smoke test/);
  assert.match(html, /none \(render-only smoke test\)/);
  assert.doesNotMatch(html, /<mark[^>]*>Stephanos C\. 2017\.<\/mark>/);
  assert.doesNotMatch(html, /no benchmark entry/);
  assert.doesNotMatch(html, />∅</);
  assert.doesNotMatch(html, /&lt;div&gt;Smith/);
});

test('generateReport supports style-scoped official reports', {
  skip: !hasLegacyStyles,
  timeout: 180000,
}, async () => {
  const { report } = await generateReport({
    styleName: 'apa-7th',
    parallelism: 1,
  });

  assert.equal(report.totalStyles, 1);
  assert.deepEqual(report.metadata.styles, ['apa-7th']);
  assert.equal(report.metadata.styleSelector, 'style:apa-7th');
  assert.ok(report.metadata.richInputEvidence.headlineGate, 'should have headlineGate evidence');
});

test('generateReport supports multi-style selected reports', {
  skip: !hasLegacyStyles,
  timeout: 180000,
}, async () => {
  const { report } = await generateReport({
    styles: ['chicago-author-date-18th', 'apa-7th'],
    parallelism: 1,
  });

  assert.equal(report.totalStyles, 2);
  assert.deepEqual(report.metadata.styles, ['apa-7th', 'chicago-author-date-18th']);
  assert.equal(report.metadata.styleSelector, 'selected-styles');
});

test('effective oracle sections and fidelity prefer adjusted counts when present', () => {
  const oracleResult = {
    citations: { passed: 8, total: 10, entries: [] },
    bibliography: { passed: 9, total: 10, entries: [] },
    adjusted: {
      citations: { passed: 10, total: 10, entries: [] },
      bibliography: { passed: 10, total: 10, entries: [] },
      divergenceSummary: {
        'div-004': { adjustedCitations: 2 },
      },
    },
  };

  assert.deepEqual(getEffectiveOracleSection(oracleResult, 'citations'), oracleResult.adjusted.citations);
  assert.equal(computeFidelityScore(oracleResult), 1);
});

test('mergeDivergenceSummaries preserves counts and unions arrays', () => {
  const merged = mergeDivergenceSummaries(
    {
      'div-004': {
        adjustedCitations: 1,
        bibliographyOrderDifference: true,
        anonymousIds: ['ITEM-20'],
        tags: ['missing-name-title-sort'],
      },
    },
    {
      'div-004': {
        adjustedCitations: 2,
        bibliographyOrderDifference: false,
        anonymousIds: ['ITEM-21'],
        tags: ['sort-derived-numeric-citation-label'],
      },
    }
  );

  assert.deepEqual(merged, {
    'div-004': {
      adjustedCitations: 3,
      bibliographyOrderDifference: true,
      anonymousIds: ['ITEM-20', 'ITEM-21'],
      tags: ['missing-name-title-sort', 'sort-derived-numeric-citation-label'],
    },
  });
});

test('expandCompoundBibEntries splits merged biblatex compound blocks', () => {
  const entries = [
    '(1) First entry. (2) Second entry. (3) Third entry.',
    '(4) Standalone entry.',
  ];

  assert.deepEqual(expandCompoundBibEntries(entries), [
    '(1) First entry.',
    '(2) Second entry.',
    '(3) Third entry.',
    '(4) Standalone entry.',
  ]);
});

test('computeComponentMatchRate scores biblatex entries with populated components', () => {
  // Entry 1: text matches (counts as 11/11).
  // Entry 2: text mismatch but components.{matches,differences} populated by
  // the same heuristic used for citeproc-js styles — yields 3 matches / 4 total.
  const oracleResult = {
    bibliography: {
      entries: [
        { match: true },
        {
          match: false,
          components: {
            matches: [
              { component: 'contributors', status: 'match' },
              { component: 'title', status: 'match' },
              { component: 'year', status: 'match' },
            ],
            differences: [
              {
                component: 'doi',
                issue: 'missing',
                expected: '10.1234/foo',
                detail: 'Missing in Citum output',
              },
            ],
          },
        },
      ],
    },
  };

  const rate = computeComponentMatchRate(oracleResult);
  // (11 + 3) / (11 + 4) = 14/15 ≈ 0.933
  assert.equal(rate, 0.933);
});

test('computeComponentMatchRate returns null when no components are populated', () => {
  // Mirrors the pre-fix biblatex behaviour: mismatched entries without
  // entry.components produce no signal, total stays 0, result is null.
  const oracleResult = {
    bibliography: {
      entries: [
        { match: false },
        { match: false },
      ],
    },
  };

  assert.equal(computeComponentMatchRate(oracleResult), null);
});

test('equivalentText tolerates near-match snapshot formatting without masking drift', () => {
  assert.equal(
    equivalentText(
      '[3] Yann LeCun, Yoshua Bengio, and Geoffrey Hinton. “Deep Learning”. In: Nature 521 (2015), pp. 436–444.',
      '[3] Y. LeCun, Y. Bengio and G. Hinton, “Deep Learning”, Nature, 2015, 521, 436–444.'
    ),
    true
  );

  assert.equal(
    equivalentText(
      '[29] John Smith et al. “Adaptive Climate Risk Modeling in Coastal Cities”. In: Journal of Climate Analytics 12.2 (2021), pp. 101–119.',
      '[30] John Smith et al. “Adaptive Climate Risk Modeling for Inland Regions”. In: Journal of Climate Analytics 12.3 (2021), pp. 201–219.'
    ),
    false
  );
});

test('getCslSnapshotStatus reports missing and stale snapshots without invoking live oracle', () => {
  const refsFixture = path.join(projectRoot, 'tests', 'fixtures', 'references-expanded.json');
  const citationsFixture = path.join(projectRoot, 'tests', 'fixtures', 'citations-expanded.json');
  const missing = getCslSnapshotStatus('/tmp/definitely-missing-style.csl', refsFixture, citationsFixture);
  assert.equal(missing.ok, false);
  assert.equal(missing.status, 'missing');

  const staleCitationsFixture = path.join(os.tmpdir(), `citations-stale-${process.pid}.json`);
  fs.writeFileSync(staleCitationsFixture, fs.readFileSync(citationsFixture, 'utf8').replace('"id":', '"fixture-id":'));
  if (hasLegacyStyles) {
    const stale = getCslSnapshotStatus(
      path.join(projectRoot, 'styles-legacy', 'apa.csl'),
      refsFixture,
      staleCitationsFixture
    );
    assert.equal(stale.ok, false);
    assert.equal(stale.status, 'stale');
  }
  fs.rmSync(staleCitationsFixture, { force: true });
});

test('runCachedJsonJob invalidates when cache key changes', async () => {
  const cacheDir = fs.mkdtempSync(path.join(os.tmpdir(), 'report-cache-'));
  const runtime = {
    cacheDir,
    timings: new Map(),
    recordTiming(kind, durationMs, cacheHit = false) {
      const current = this.timings.get(kind) || { count: 0, totalMs: 0, cacheHits: 0 };
      current.count += 1;
      current.totalMs += durationMs;
      if (cacheHit) current.cacheHits += 1;
      this.timings.set(kind, current);
    },
  };
  let computes = 0;

  const first = await runCachedJsonJob(runtime, {
    kind: 'unit',
    cacheKey: { style: 'apa', fixture: 'core', hash: 'a' },
    async compute() {
      computes += 1;
      return { value: 'first' };
    },
  });
  const second = await runCachedJsonJob(runtime, {
    kind: 'unit',
    cacheKey: { style: 'apa', fixture: 'core', hash: 'a' },
    async compute() {
      computes += 1;
      return { value: 'first' };
    },
  });
  const third = await runCachedJsonJob(runtime, {
    kind: 'unit',
    cacheKey: { style: 'apa', fixture: 'core', hash: 'b' },
    async compute() {
      computes += 1;
      return { value: 'third' };
    },
  });

  fs.rmSync(cacheDir, { recursive: true, force: true });
  assert.deepEqual(first, { value: 'first' });
  assert.deepEqual(second, { value: 'first' });
  assert.deepEqual(third, { value: 'third' });
  assert.equal(computes, 2);
});

test('mapWithConcurrency preserves input ordering under parallel execution', async () => {
  const values = [40, 5, 20, 1];
  const results = await mapWithConcurrency(values, 2, async (delay, index) => {
    await new Promise((resolve) => setTimeout(resolve, delay));
    return `${index}:${delay}`;
  });

  assert.deepEqual(results, ['0:40', '1:5', '2:20', '3:1']);
});

test('preflightSnapshots reports missing citeproc snapshots for citeproc-backed styles', () => {
  const policy = loadVerificationPolicy();
  const stylesDir = fs.mkdtempSync(path.join(os.tmpdir(), 'report-preflight-'));
  fs.writeFileSync(path.join(stylesDir, 'chicago-author-date-18th.csl'), '<style></style>');
  const issues = preflightSnapshots(
    [
      {
        name: 'chicago-author-date-18th',
        sourceName: 'chicago-author-date-18th',
        format: 'author-date',
      },
      {
        name: 'missing-style',
        sourceName: 'definitely-missing-style',
        format: 'author-date',
      }
    ],
    policy,
    stylesDir
  );

  fs.rmSync(stylesDir, { recursive: true, force: true });
  assert.equal(issues.length, 1);
  assert.equal(issues[0].status, 'missing');
  assert.equal(issues[0].style, 'chicago-author-date-18th');
});
